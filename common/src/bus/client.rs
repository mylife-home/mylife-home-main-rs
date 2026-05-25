use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures::sink::SinkExt;
use log::warn;
use mqttbytes::QoS;
use mqttbytes::v4::{
    Connect, ConnectReturnCode, Disconnect, Packet, PingReq, PingResp, Publish, Subscribe,
    SubscribeFilter, SubscribeReasonCode, Unsubscribe,
};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior, interval, timeout};
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, Encoder, Framed};

/// Keep-alive interval for the MQTT connection. The client will send a ping at
/// half this interval and expect the broker to respond within the full interval.
/// If the broker does not respond in time, the client will treat the connection
/// as lost and reconnect.
const KEEP_ALIVE: Duration = Duration::from_secs(30);

/// Timeout for establishing a connection and waiting for the CONNACK response.
/// If the client fails to connect or receive a valid CONNACK within this
/// duration, the connection attempt is considered failed and will be retried.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Base delay for reconnection attempts. The client uses exponential backoff
/// when reconnecting to the broker.
const RECONNECT_BASE_DELAY: Duration = Duration::from_secs(1);

/// Maximum delay for reconnection attempts. The client doubles the delay after
/// each failed attempt, up to this ceiling.
const RECONNECT_MAX_DELAY: Duration = Duration::from_secs(30);

/// Capacity for the command channel used to send work from the public client to
/// the internal worker task.
const COMMAND_QUEUE_CAPACITY: usize = 128;

/// Capacity for the broadcast channel used to publish events to subscribers.
const EVENT_QUEUE_CAPACITY: usize = 128;

/// Maximum allowed size for MQTT packets. This is used to prevent unbounded memory
/// usage when reading from the socket.
const MAX_PACKET_SIZE: usize = 1024 * 1024; // 1 MiB

/// MQTT events emitted by the client to indicate connection state changes,
/// inbound messages, and failures.
#[derive(Debug, Clone)]
pub enum MqttEvent {
    /// Emitted when the client successfully establishes a connection to the broker.
    Connected,
    /// Emitted when the client loses connection to the broker or the broker
    /// closes the connection.
    Disconnected { reason: String },
    /// Emitted when an error occurs while connecting, reading, writing, or
    /// processing MQTT packets.
    Error(Arc<MqttError>),
    /// Emitted when a message is received from a subscribed topic.
    Message {
        topic: String,
        payload: Bytes,
        retain: bool,
    },
}

/// Errors that can occur while configuring, connecting, or running the MQTT
/// client.
#[derive(Debug)]
pub enum MqttError {
    /// Indicates that the client was configured with invalid parameters.
    InvalidConfig { message: String },
    /// Indicates an I/O error while communicating with the broker.
    Io(std::io::Error),
    /// Indicates a protocol error while encoding or decoding MQTT packets.
    Codec(mqttbytes::Error),
    /// Indicates that the command channel has been closed.
    CommandClosed,
    /// Indicates that the command queue is full and a new command could not be
    /// accepted.
    CommandQueueFull,
    /// Indicates that the broker refused the connection attempt.
    ConnectionRefused { reason: String },
    /// Indicates that a subscription request failed.
    SubscriptionFailed { paths: Vec<String> },
    /// Indicates that an operation timed out.
    Timeout { reason: String },
}

impl fmt::Display for MqttError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig { message } => write!(f, "invalid config: {message}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Codec(error) => write!(f, "codec error: {error}"),
            Self::CommandClosed => write!(f, "mqtt command channel closed"),
            Self::CommandQueueFull => write!(f, "mqtt command queue full"),
            Self::ConnectionRefused { reason } => write!(f, "connection refused: {reason}"),
            Self::SubscriptionFailed { paths } => {
                write!(f, "subscription failed on paths {:?}", paths)
            }
            Self::Timeout { reason } => write!(f, "timeout: {reason}"),
        }
    }
}

impl std::error::Error for MqttError {}

impl From<std::io::Error> for MqttError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<mqttbytes::Error> for MqttError {
    fn from(value: mqttbytes::Error) -> Self {
        Self::Codec(value)
    }
}

#[derive(Debug)]
enum MqttCommand {
    Publish {
        topic: String,
        payload: Bytes,
        retain: bool,
    },
    Subscribe {
        paths: Vec<String>,
    },
    Unsubscribe {
        paths: Vec<String>,
    },
    Shutdown,
}

/// MQTT client for publishing messages, subscribing to topics, and tracking
/// broker connection state. The client runs an internal worker task that owns
/// the TCP connection and automatically reconnects when the broker becomes
/// unavailable.
#[derive(Debug)]
pub struct MqttClient {
    command_tx: mpsc::Sender<MqttCommand>,
    events_tx: broadcast::Sender<MqttEvent>,
    worker_handle: JoinHandle<()>,
}

impl MqttClient {
    /// Creates a new MQTT client and starts the background worker.
    ///
    /// The worker will attempt to connect to the broker at `server_address`,
    /// publish connection state through `events()`, and automatically reconnect
    /// with exponential backoff if the connection is lost.
    pub fn create(instance_name: String, server_address: String) -> Result<Self, MqttError> {
        if instance_name.trim().is_empty() {
            return Err(MqttError::InvalidConfig {
                message: String::from("instance_name must not be empty"),
            });
        }

        if server_address.trim().is_empty() {
            return Err(MqttError::InvalidConfig {
                message: String::from("server_address must not be empty"),
            });
        }

        let (command_tx, command_rx) = mpsc::channel(COMMAND_QUEUE_CAPACITY);
        let (events_tx, _) = broadcast::channel(EVENT_QUEUE_CAPACITY);
        let worker_events = events_tx.clone();

        let worker_handle = tokio::spawn(async move {
            let mut worker =
                IoWorker::new(instance_name, server_address, command_rx, worker_events);
            worker.run().await;
        });

        Ok(Self {
            command_tx,
            events_tx,
            worker_handle,
        })
    }

    /// Returns a receiver for MQTT events emitted by the worker.
    pub fn events(&self) -> broadcast::Receiver<MqttEvent> {
        self.events_tx.subscribe()
    }

    /// Enqueues a publish request for the worker.
    ///
    /// This method does not wait for the broker acknowledgment; transmission is
    /// handled asynchronously by the worker.
    pub fn publish(&self, topic: String, payload: Bytes, retain: bool) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Publish {
            topic,
            payload,
            retain,
        })
    }

    /// Enqueues a subscription request for the worker.
    pub fn subscribe(&self, paths: Vec<String>) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Subscribe { paths })
    }

    /// Enqueues an unsubscription request for the worker.
    pub fn unsubscribe(&self, paths: Vec<String>) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Unsubscribe { paths })
    }

    /// Gracefully shuts down the client and waits for the worker task to exit.
    pub async fn shutdown(self) {
        let _ = self.command_tx.send(MqttCommand::Shutdown).await;

        if let Err(err) = self.worker_handle.await {
            if err.is_panic() {
                std::panic::resume_unwind(err.into_panic());
            }

            if err.is_cancelled() {
                panic!("mqtt worker task was cancelled during shutdown");
            }

            panic!("mqtt worker task join error during shutdown: {err}");
        }
    }

    fn try_send_command(&self, command: MqttCommand) -> Result<(), MqttError> {
        self.command_tx
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => MqttError::CommandQueueFull,
                mpsc::error::TrySendError::Closed(_) => MqttError::CommandClosed,
            })
    }
}

/// Internal worker responsible for owning the TCP connection and performing the
/// MQTT read/write loop.
struct IoWorker {
    instance_name: String,
    server_address: String,
    command_rx: mpsc::Receiver<MqttCommand>,
    events_tx: broadcast::Sender<MqttEvent>,
    pending_subscription_paths: Option<Vec<String>>,
    connected: bool,
    shutting_down: bool,
    reconnect_delay: Duration,
}

impl IoWorker {
    fn new(
        instance_name: String,
        server_address: String,
        command_rx: mpsc::Receiver<MqttCommand>,
        events_tx: broadcast::Sender<MqttEvent>,
    ) -> Self {
        Self {
            instance_name,
            server_address,
            command_rx,
            events_tx,
            pending_subscription_paths: None,
            connected: false,
            shutting_down: false,
            reconnect_delay: Duration::ZERO,
        }
    }

    /// Runs the main connection loop.
    ///
    /// This method keeps the connection alive, handles inbound packets, processes
    /// outbound commands, and reconnects on failures.
    async fn run(&mut self) {
        let mut stream: Option<Framed<TcpStream, PacketCodec>> = None;
        let mut ping_interval = interval(KEEP_ALIVE / 2);
        ping_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            if self.shutting_down {
                break;
            }

            if !self.connected {
                self.close_stream(&mut stream).await;
                self.pending_subscription_paths = None;

                match self.connect_once().await {
                    Ok(new_stream) => {
                        stream = Some(new_stream);
                        self.connected = true;
                        // Clear command queue on reconnect to avoid processing stale commands that may have been enqueued during downtime
                        self.clear_command_queue();
                        self.reconnect_delay = Duration::ZERO;
                        self.emit_event(MqttEvent::Connected);
                    }
                    Err(error) => {
                        self.emit_event(MqttEvent::Error(Arc::new(error)));
                        self.reconnect_delay = self.next_reconnect_delay();
                        time::sleep(self.reconnect_delay).await;
                        continue;
                    }
                }
            }

            let Some(current_stream) = stream.as_mut() else {
                self.connected = false;
                continue;
            };

            tokio::select! {
                maybe_command = self.command_rx.recv() => {
                    match maybe_command {
                        Some(command) => {
                            if let Err(error) = self.handle_command(current_stream, command).await {
                                self.emit_event(MqttEvent::Error(Arc::new(error)));
                                self.connected = false;
                            }
                        }
                        None => {
                            self.emit_event(MqttEvent::Disconnected { reason: String::from("command channel closed") });
                            self.close_stream(&mut stream).await;
                            break;
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    if let Err(error) = current_stream.send(Packet::PingReq).await {
                        self.emit_event(MqttEvent::Error(Arc::new(error)));
                        self.connected = false;
                    }
                }
                read_result = current_stream.next() => {
                    match read_result {
                        None => {
                            self.connected = false;
                            self.emit_event(MqttEvent::Disconnected { reason: String::from("connection closed by peer") });
                        }
                        Some(Ok(packet)) => {
                            if let Err(error) = self.handle_incoming_packet(packet).await {
                                self.emit_event(MqttEvent::Error(Arc::new(error)));
                                self.connected = false;
                            }
                        }
                        Some(Err(error)) => {
                            self.emit_event(MqttEvent::Error(Arc::new(error.into())));
                            self.connected = false;
                        }
                    }
                }
            }
        }

        self.close_stream(&mut stream).await;
    }

    fn clear_command_queue(&mut self) {
        while let Ok(_) = self.command_rx.try_recv() {}
    }

    /// Closes and drops the current TCP stream if one is present.
    async fn close_stream(&self, stream: &mut Option<Framed<TcpStream, PacketCodec>>) {
        if let Some(current_stream) = stream.take() {
            let _ = current_stream.into_inner().shutdown().await;
        }
    }

    /// Computes the next reconnect delay using exponential backoff.
    fn next_reconnect_delay(&mut self) -> Duration {
        if self.reconnect_delay.is_zero() {
            self.reconnect_delay = RECONNECT_BASE_DELAY;
        } else {
            self.reconnect_delay = (self.reconnect_delay * 2).min(RECONNECT_MAX_DELAY);
        }

        self.reconnect_delay
    }

    /// Establishes a TCP connection, sends the CONNECT packet, and waits for a
    /// CONNACK before returning the connected stream and any leftover buffered
    /// bytes.
    async fn connect_once(&self) -> Result<Framed<TcpStream, PacketCodec>, MqttError> {
        let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&self.server_address))
            .await
            .map_err(|_| MqttError::Timeout {
                reason: String::from("connect timeout"),
            })??;
        stream.set_nodelay(true)?;

        let mut stream = Framed::new(stream, PacketCodec);

        stream.send(self.build_connect_packet()).await?;

        loop {
            let Some(res) =
                timeout(CONNECT_TIMEOUT, stream.next())
                    .await
                    .map_err(|_| MqttError::Timeout {
                        reason: String::from("connack timeout"),
                    })?
            else {
                return Err(MqttError::ConnectionRefused {
                    reason: String::from("connection closed by peer during handshake"),
                });
            };

            let packet = res?;

            match packet {
                Packet::ConnAck(connack) => {
                    if connack.code == ConnectReturnCode::Success {
                        return Ok(stream);
                    }

                    return Err(MqttError::ConnectionRefused {
                        reason: format!("broker refused connection: {:?}", connack.code),
                    });
                }
                other => {
                    return Err(MqttError::ConnectionRefused {
                        reason: format!("expected connack during handshake, got {other:?}"),
                    });
                }
            }
        }
    }

    /// Handles a single command received from the public client.
    async fn handle_command(
        &mut self,
        stream: &mut Framed<TcpStream, PacketCodec>,
        command: MqttCommand,
    ) -> Result<(), MqttError> {
        match command {
            MqttCommand::Publish {
                topic,
                payload,
                retain,
            } => {
                let packet = self.build_publish_packet(topic, payload, retain);
                stream.send(packet).await?;
            }
            MqttCommand::Subscribe { paths } => {
                self.pending_subscription_paths = Some(paths.clone());
                let packet = self.build_subscribe_packet(paths);
                stream.send(packet).await?;
            }
            MqttCommand::Unsubscribe { paths } => {
                let packet = self.build_unsubscribe_packet(paths);
                stream.send(packet).await?;
            }
            MqttCommand::Shutdown => {
                let packet = Packet::Disconnect;
                stream.send(packet).await?;
                self.shutting_down = true;
            }
        }

        Ok(())
    }

    /// Processes a single inbound MQTT packet.
    async fn handle_incoming_packet(&mut self, packet: Packet) -> Result<(), MqttError> {
        match packet {
            Packet::ConnAck(connack) => {
                if connack.code != ConnectReturnCode::Success {
                    return Err(MqttError::ConnectionRefused {
                        reason: format!("broker refused connection: {:?}", connack.code),
                    });
                }
            }
            Packet::Publish(publish) => {
                self.emit_event(MqttEvent::Message {
                    topic: publish.topic,
                    payload: publish.payload,
                    retain: publish.retain,
                });

                if publish.qos != QoS::AtMostOnce {
                    warn!(
                        "received QoS {:?} publish from broker; broker-side QoS handling is not fully implemented yet",
                        publish.qos
                    );
                }
            }
            Packet::SubAck(suback) => {
                let paths = self
                    .pending_subscription_paths
                    .take()
                    .unwrap_or_else(|| vec![String::from("<unknown>")]);
                let success = suback
                    .return_codes
                    .iter()
                    .all(|code| matches!(code, SubscribeReasonCode::Success(_)));

                if !success {
                    return Err(MqttError::SubscriptionFailed { paths });
                }
            }
            Packet::PingResp => {}
            Packet::Disconnect => {
                self.connected = false;
                self.emit_event(MqttEvent::Disconnected {
                    reason: String::from("broker sent disconnect"),
                });
            }
            other => {
                warn!("received unsupported packet from broker: {other:?}");
            }
        }

        Ok(())
    }

    fn build_connect_packet(&self) -> Packet {
        Packet::Connect(Connect {
            protocol: mqttbytes::Protocol::V4,
            keep_alive: KEEP_ALIVE.as_secs() as u16,
            client_id: self.instance_name.clone(),
            clean_session: true,
            last_will: None,
            login: None,
        })
    }

    fn build_publish_packet<'a>(&self, topic: String, payload: Bytes, retain: bool) -> Packet {
        Packet::Publish(Publish {
            pkid: 1,
            dup: false,
            qos: QoS::AtMostOnce,
            retain,
            topic: topic,
            payload,
        })
    }

    fn build_subscribe_packet(&self, paths: Vec<String>) -> Packet {
        Packet::Subscribe(Subscribe {
            pkid: 1,
            filters: paths
                .into_iter()
                .map(|path| SubscribeFilter {
                    path,
                    qos: QoS::AtMostOnce,
                })
                .collect(),
        })
    }

    fn build_unsubscribe_packet(&self, paths: Vec<String>) -> Packet {
        Packet::Unsubscribe(Unsubscribe {
            pkid: 1,
            topics: paths,
        })
    }

    fn emit_event(&self, event: MqttEvent) {
        // Best effort send; if there are no subscribers or the channel is full, we can just drop the event
        let _ = self.events_tx.send(event);
    }
}

struct PacketCodec;

impl Decoder for PacketCodec {
    type Item = Packet;
    type Error = MqttError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match mqttbytes::v4::read(src, MAX_PACKET_SIZE) {
            Ok(packet) => Ok(Some(packet)),
            Err(mqttbytes::Error::InsufficientBytes(_)) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

impl Encoder<Packet> for PacketCodec {
    type Error = MqttError;

    fn encode(&mut self, item: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let _ = match item {
            Packet::Connect(data) => data.write(dst),
            Packet::ConnAck(data) => data.write(dst),
            Packet::Publish(data) => data.write(dst),
            Packet::PubAck(data) => data.write(dst),
            Packet::PubRec(data) => data.write(dst),
            Packet::PubRel(data) => data.write(dst),
            Packet::PubComp(data) => data.write(dst),
            Packet::Subscribe(data) => data.write(dst),
            Packet::SubAck(data) => data.write(dst),
            Packet::Unsubscribe(data) => data.write(dst),
            Packet::UnsubAck(data) => data.write(dst),
            Packet::PingReq => PingReq.write(dst),
            Packet::PingResp => PingResp.write(dst),
            Packet::Disconnect => Disconnect.write(dst),
        }?;

        Ok(())
    }
}
