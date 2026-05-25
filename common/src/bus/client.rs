use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use log::warn;
use mqttrs::{
    Connect, ConnectReturnCode, Packet, Pid, Protocol, Publish, QoS, QosPid, Subscribe,
    SubscribeTopic, Unsubscribe, clone_packet, decode_slice, encode_slice,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior, interval, timeout};

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

/// Read buffer size used while waiting for bytes from the TCP stream.
const READ_BUFFER_SIZE: usize = 4096;

/// Frame buffer size used when encoding MQTT packets before writing them to the
/// socket.
const FRAME_BUFFER_SIZE: usize = 8192;

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
        payload: Vec<u8>,
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
    Codec(mqttrs::Error),
    /// Indicates that the command channel has been closed.
    CommandClosed,
    /// Indicates that the command queue is full and a new command could not be
    /// accepted.
    CommandQueueFull,
    /// Indicates that the broker refused the connection attempt.
    ConnectionRefused { reason: String },
    /// Indicates that a subscription request failed.
    SubscriptionFailed { topic: String },
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
            Self::SubscriptionFailed { topic } => {
                write!(f, "subscription failed on topic '{topic}'")
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

impl From<mqttrs::Error> for MqttError {
    fn from(value: mqttrs::Error) -> Self {
        Self::Codec(value)
    }
}

#[derive(Debug)]
enum MqttCommand {
    Publish {
        topic: String,
        payload: Vec<u8>,
        retain: bool,
    },
    Subscribe {
        topic: String,
    },
    Unsubscribe {
        topic: String,
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
                message: "instance_name must not be empty".to_owned(),
            });
        }

        if server_address.trim().is_empty() {
            return Err(MqttError::InvalidConfig {
                message: "server_address must not be empty".to_owned(),
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
    pub fn publish(&self, topic: &str, payload: &[u8], retain: bool) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Publish {
            topic: topic.to_owned(),
            payload: payload.to_vec(),
            retain,
        })
    }

    /// Enqueues a subscription request for the worker.
    pub fn subscribe(&self, topic: &str) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Subscribe {
            topic: topic.to_owned(),
        })
    }

    /// Enqueues an unsubscription request for the worker.
    pub fn unsubscribe(&self, topic: &str) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Unsubscribe {
            topic: topic.to_owned(),
        })
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
    pending_subscription_topic: Option<String>,
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
            pending_subscription_topic: None,
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
        let mut stream: Option<TcpStream> = None;
        let mut recv_buf = Vec::with_capacity(8192);
        let mut read_buf = [0u8; READ_BUFFER_SIZE];
        let mut ping_interval = interval(KEEP_ALIVE / 2);
        ping_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            if self.shutting_down {
                break;
            }

            if !self.connected {
                self.close_stream(&mut stream).await;
                recv_buf.clear();
                self.pending_subscription_topic = None;

                match self.connect_once().await {
                    Ok((new_stream, buffered_data)) => {
                        stream = Some(new_stream);
                        recv_buf = buffered_data;
                        self.connected = true;
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
                            self.emit_event(MqttEvent::Disconnected { reason: "command channel closed".to_owned() });
                            self.close_stream(&mut stream).await;
                            break;
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    if let Err(error) = self.send_packet(current_stream, &Packet::Pingreq).await {
                        self.emit_event(MqttEvent::Error(Arc::new(error)));
                        self.connected = false;
                    }
                }
                read_result = current_stream.read(&mut read_buf) => {
                    match read_result {
                        Ok(0) => {
                            self.connected = false;
                            self.emit_event(MqttEvent::Disconnected { reason: "connection closed by peer".to_owned() });
                        }
                        Ok(bytes_read) => {
                            recv_buf.extend_from_slice(&read_buf[..bytes_read]);
                            if let Err(error) = self.process_received_packets(&mut recv_buf).await {
                                self.emit_event(MqttEvent::Error(Arc::new(error)));
                                self.connected = false;
                            }
                        }
                        Err(error) => {
                            self.emit_event(MqttEvent::Error(Arc::new(error.into())));
                            self.connected = false;
                        }
                    }
                }
            }
        }

        self.close_stream(&mut stream).await;
    }

    /// Closes and drops the current TCP stream if one is present.
    async fn close_stream(&self, stream: &mut Option<TcpStream>) {
        if let Some(mut current_stream) = stream.take() {
            let _ = current_stream.shutdown().await;
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
    async fn connect_once(&self) -> Result<(TcpStream, Vec<u8>), MqttError> {
        let mut stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&self.server_address))
            .await
            .map_err(|_| MqttError::Timeout {
                reason: "connect timeout".to_owned(),
            })??;
        stream.set_nodelay(true)?;

        self.send_packet(&mut stream, &self.build_connect_packet())
            .await?;

        let mut recv_buf = Vec::with_capacity(8192);
        let mut scratch = [0u8; READ_BUFFER_SIZE];
        let mut packet_buf = Vec::with_capacity(1024);

        loop {
            let bytes_read = timeout(CONNECT_TIMEOUT, stream.read(&mut scratch))
                .await
                .map_err(|_| MqttError::Timeout {
                    reason: "connack timeout".to_owned(),
                })??;

            if bytes_read == 0 {
                return Err(MqttError::ConnectionRefused {
                    reason: "connection closed before connack".to_owned(),
                });
            }

            recv_buf.extend_from_slice(&scratch[..bytes_read]);
            packet_buf.clear();
            packet_buf.resize(recv_buf.len().max(1024), 0);

            let packet_len = clone_packet(&recv_buf, &mut packet_buf)?;
            if packet_len == 0 {
                continue;
            }

            let packet = decode_slice(&packet_buf[..packet_len])?
                .ok_or_else(|| MqttError::Codec(mqttrs::Error::InvalidHeader))?;
            recv_buf.drain(..packet_len);

            match packet {
                Packet::Connack(connack) => {
                    if connack.code == ConnectReturnCode::Accepted {
                        return Ok((stream, recv_buf));
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
        stream: &mut TcpStream,
        command: MqttCommand,
    ) -> Result<(), MqttError> {
        match command {
            MqttCommand::Publish {
                topic,
                payload,
                retain,
            } => {
                let packet = self.build_publish_packet(&topic, &payload, retain);
                self.send_packet(stream, &packet).await?;
            }
            MqttCommand::Subscribe { topic } => {
                self.pending_subscription_topic = Some(topic.clone());
                let packet = self.build_subscribe_packet(&topic);
                self.send_packet(stream, &packet).await?;
            }
            MqttCommand::Unsubscribe { topic } => {
                let packet = self.build_unsubscribe_packet(&topic);
                self.send_packet(stream, &packet).await?;
            }
            MqttCommand::Shutdown => {
                let packet = Packet::Disconnect;
                self.send_packet(stream, &packet).await?;
                self.shutting_down = true;
                let _ = stream.shutdown().await;
            }
        }

        Ok(())
    }

    /// Decodes all complete MQTT packets currently buffered from the socket.
    async fn process_received_packets(&mut self, recv_buf: &mut Vec<u8>) -> Result<(), MqttError> {
        let mut packet_buf = Vec::with_capacity(1024);

        loop {
            packet_buf.clear();
            packet_buf.resize(recv_buf.len().max(1024), 0);

            let packet_len = clone_packet(recv_buf, &mut packet_buf)?;
            if packet_len == 0 {
                break;
            }

            let packet = decode_slice(&packet_buf[..packet_len])?
                .ok_or_else(|| MqttError::Codec(mqttrs::Error::InvalidHeader))?;
            recv_buf.drain(..packet_len);
            self.handle_incoming_packet(packet).await?;
        }

        Ok(())
    }

    /// Processes a single inbound MQTT packet.
    async fn handle_incoming_packet(&mut self, packet: Packet<'_>) -> Result<(), MqttError> {
        match packet {
            Packet::Connack(connack) => {
                if connack.code != ConnectReturnCode::Accepted {
                    return Err(MqttError::ConnectionRefused {
                        reason: format!("broker refused connection: {:?}", connack.code),
                    });
                }
            }
            Packet::Publish(publish) => {
                self.emit_event(MqttEvent::Message {
                    topic: publish.topic_name.to_owned(),
                    payload: publish.payload.to_vec(),
                    retain: publish.retain,
                });

                if !matches!(publish.qospid, QosPid::AtMostOnce) {
                    warn!(
                        "received QoS {:?} publish from broker; broker-side QoS handling is not fully implemented yet",
                        publish.qospid
                    );
                }
            }
            Packet::Suback(suback) => {
                let topic = self
                    .pending_subscription_topic
                    .take()
                    .unwrap_or_else(|| "<unknown>".to_owned());
                let success = suback
                    .return_codes
                    .iter()
                    .all(|code| matches!(code, mqttrs::SubscribeReturnCodes::Success(_)));

                if !success {
                    return Err(MqttError::SubscriptionFailed { topic });
                }
            }
            Packet::Pingresp => {}
            Packet::Disconnect => {
                self.connected = false;
                self.emit_event(MqttEvent::Disconnected {
                    reason: "broker sent disconnect".to_owned(),
                });
            }
            other => {
                warn!("received unsupported packet from broker: {other:?}");
            }
        }

        Ok(())
    }

    fn build_connect_packet(&self) -> Packet<'_> {
        Packet::Connect(Connect {
            protocol: Protocol::MQTT311,
            keep_alive: KEEP_ALIVE.as_secs() as u16,
            client_id: &self.instance_name,
            clean_session: true,
            last_will: None,
            username: None,
            password: None,
        })
    }

    fn build_publish_packet<'a>(
        &self,
        topic: &'a str,
        payload: &'a [u8],
        retain: bool,
    ) -> Packet<'a> {
        Packet::Publish(Publish {
            dup: false,
            qospid: QosPid::AtMostOnce,
            retain,
            topic_name: topic,
            payload,
        })
    }

    fn build_subscribe_packet(&self, topic: &str) -> Packet<'_> {
        Packet::Subscribe(Subscribe::new(
            Pid::new(),
            vec![SubscribeTopic {
                topic_path: topic.to_owned().into(),
                qos: QoS::AtMostOnce,
            }],
        ))
    }

    fn build_unsubscribe_packet(&self, topic: &str) -> Packet<'_> {
        Packet::Unsubscribe(Unsubscribe {
            pid: Pid::new(),
            topics: vec![topic.to_owned().into()],
        })
    }

    fn emit_event(&self, event: MqttEvent) {
        let _ = self.events_tx.send(event);
    }

    /// Encodes and writes a single MQTT packet to the stream.
    async fn send_packet(
        &self,
        stream: &mut TcpStream,
        packet: &Packet<'_>,
    ) -> Result<(), MqttError> {
        let mut frame_buffer = [0u8; FRAME_BUFFER_SIZE];
        let len = encode_slice(packet, &mut frame_buffer)?;
        stream.write_all(&frame_buffer[..len]).await?;
        stream.flush().await?;
        Ok(())
    }
}
