use std::collections::{HashSet, VecDeque};
use std::{fmt, panic};
use std::time::Duration;

use mqttrs::{
    Connect, ConnectReturnCode, Packet, Pid, Protocol, Publish, QoS, QosPid, Subscribe,
    SubscribeTopic, Unsubscribe, clone_packet, decode_slice, encode_slice,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior, interval, timeout};

/// Keep-alive interval for MQTT connection. The client will send a ping to the broker at half this interval, and
/// expect a response within the full interval. If the broker does not respond within the full interval, the client
/// will consider the connection lost and attempt to reconnect.
const KEEP_ALIVE: Duration = Duration::from_secs(30);

/// Timeout for establishing a connection to the broker and waiting for the CONNACK response. If the client fails to
/// connect or receive a valid CONNACK within this duration, it will consider the connection attempt failed and try
/// again (if applicable).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Base delay for reconnection attempts. When the client loses connection to the broker, it will wait this long
/// before attempting to reconnect. If the reconnection attempt fails, the client will double the delay and try again,
/// up to a maximum of `RECONNECT_MAX_DELAY`.
const RECONNECT_BASE_DELAY: Duration = Duration::from_secs(1);

/// Maximum delay for reconnection attempts. The client will use exponential backoff for reconnection attempts,
/// starting with `RECONNECT_BASE_DELAY` and doubling the delay after each failed attempt, up to this maximum.
const RECONNECT_MAX_DELAY: Duration = Duration::from_secs(30);

/// Capacity for the channel used to send commands from the `MqttClient` to the `IoWorker`. This should be large enough
/// to accommodate bursts of commands without overwhelming the service, but not so large as to consume excessive memory.
const RECEIVE_QUEUE_CAPACITY: usize = 128;

/// Capacity for the broadcast channel used to send events from the `IoWorker` to all subscribed `MqttClient` instances.
/// This should be large enough to accommodate bursts of events without losing messages, but not so large as to consume
/// excessive memory.
const TRANSMIT_QUEUE_CAPACITY: usize = 128;

#[derive(Debug, Clone)]
pub enum MqttEvent {
    Connected,
    Disconnected {
        reason: String,
    },
    Error(String),
    Message {
        topic: String,
        payload: Vec<u8>,
        retain: bool,
    },
    SubscriptionAcknowledged {
        topic: String,
    },
    SubscriptionFailed {
        topic: String,
        reason: String,
    },
}

#[derive(Debug)]
pub enum MqttError {
    InvalidConfig(String),
    Io(std::io::Error),
    Codec(mqttrs::Error),
    CommandClosed,
    CommandQueueFull,
    ConnectionRefused(String),
    Timeout(String),
}

impl fmt::Display for MqttError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(f, "invalid config: {message}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Codec(error) => write!(f, "codec error: {error}"),
            Self::CommandClosed => write!(f, "mqtt command channel closed"),
            Self::CommandQueueFull => write!(f, "mqtt command queue full"),
            Self::ConnectionRefused(reason) => write!(f, "connection refused: {reason}"),
            Self::Timeout(reason) => write!(f, "timeout: {reason}"),
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

pub struct MqttClient {
    command_tx: mpsc::Sender<MqttCommand>,
    events_tx: broadcast::Sender<MqttEvent>,
    worker_handle: JoinHandle<()>,
}

impl MqttClient {
    pub async fn connect(instance_name: String, server_address: String) -> Result<Self, MqttError> {
        if instance_name.trim().is_empty() {
            return Err(MqttError::InvalidConfig(
                "instance_name must not be empty".to_owned(),
            ));
        }
        if server_address.trim().is_empty() {
            return Err(MqttError::InvalidConfig(
                "server_address must not be empty".to_owned(),
            ));
        }

        let (command_tx, command_rx) = mpsc::channel(TRANSMIT_QUEUE_CAPACITY);
        let (events_tx, _) = broadcast::channel(RECEIVE_QUEUE_CAPACITY);
        let woker_events = events_tx.clone();

        let worker_handle = tokio::spawn(async move {
            let mut woker = IoWorker::new(instance_name, server_address, command_rx, woker_events);
            woker.run().await;
        });

        Ok(Self {
            command_tx,
            events_tx,
            worker_handle,
        })
    }

    pub fn events(&self) -> broadcast::Receiver<MqttEvent> {
        self.events_tx.subscribe()
    }

    /// Publish a message to the specified topic with the given payload and retain flag. This method will return an
    /// error if the command queue is full or closed, but it does not wait for the message to be sent to the broker.
    /// The client will handle sending the message in the background, and any errors that occur during transmission
    /// will be reported through the event channel.
    pub fn publish(&self, topic: &str, payload: &[u8], retain: bool) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Publish {
            topic: topic.to_owned(),
            payload: payload.to_vec(),
            retain,
        })
    }

    /// Subscribe to the specified topic. This method will return an error if the command queue is full or closed,
    /// but it does not wait for the subscription to be acknowledged by the broker. The client will handle sending the
    /// subscription request in the background, and the result of the subscription attempt (success or failure) will be
    /// reported through the event channel.
    pub fn subscribe(&self, topic: &str) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Subscribe {
            topic: topic.to_owned(),
        })
    }

    /// Unsubscribe from the specified topic. This method will return an error if the command queue is full or closed,
    /// but it does not wait for the unsubscription to be acknowledged by the broker. The client will handle sending the
    /// unsubscription request in the background, and the result of the unsubscription attempt (success or failure) will be
    /// reported through the event channel.
    pub fn unsubscribe(&self, topic: &str) -> Result<(), MqttError> {
        self.try_send_command(MqttCommand::Unsubscribe {
            topic: topic.to_owned(),
        })
    }

    fn try_send_command(&self, command: MqttCommand) -> Result<(), MqttError> {
        self.command_tx.try_send(command).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => MqttError::CommandQueueFull,
            mpsc::error::TrySendError::Closed(_) => MqttError::CommandClosed,
        })
    }

    /// Gracefully shut down the MQTT client by sending a disconnect command to the broker and closing the connection.
    /// This method will wait for the shutdown process to complete, and it will return an error if the command queue
    /// is closed or if any errors occur during shutdown.
    pub async fn shutdown(self) {
        // If the channel is closed already, the worker is likely already shut down, so we can ignore the error in that case.
        let _ = self.command_tx
            .send(MqttCommand::Shutdown)
            .await;

        if let Err(err) = self.worker_handle.await {
            if err.is_panic() {
                // Resume the panic on the main task
                panic::resume_unwind(err.into_panic());
            }

            if err.is_cancelled() {
                panic!("mqtt worker task was cancelled during shutdown");
            }

            panic!("mqtt worker task join error during shutdown: {err}");
        }
    }
}

struct IoWorker {
    instance_name: String,
    server_address: String,
    command_rx: mpsc::Receiver<MqttCommand>,
    events_tx: broadcast::Sender<MqttEvent>,
    pending_commands: VecDeque<MqttCommand>,
    pending_qos2: HashSet<u16>,
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
            pending_commands: VecDeque::new(),
            pending_qos2: HashSet::new(),
            pending_subscription_topic: None,
            connected: false,
            shutting_down: false,
            reconnect_delay: Duration::ZERO,
        }
    }

    async fn run(&mut self) {
        let mut stream: Option<TcpStream> = None;
        let mut recv_buf = Vec::new();
        let mut ping_interval = interval(KEEP_ALIVE / 2);
        ping_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            if self.shutting_down {
                break;
            }

            if !self.connected {
                if let Some(mut existing) = stream.take() {
                    let _ = existing.shutdown().await;
                }
                recv_buf.clear();

                match self.connect_once().await {
                    Ok((new_stream, new_recv_buf)) => {
                        stream = Some(new_stream);
                        recv_buf = new_recv_buf;
                        self.connected = true;
                        self.reconnect_delay = Duration::ZERO;
                        let _ = self.events_tx.send(MqttEvent::Connected);
                        if let Err(error) = self
                            .flush_pending_commands(stream.as_mut().unwrap(), &mut recv_buf)
                            .await
                        {
                            let _ = self.events_tx.send(MqttEvent::Error(error.to_string()));
                            self.connected = false;
                            continue;
                        }
                    }
                    Err(error) => {
                        let _ = self.events_tx.send(MqttEvent::Error(error.to_string()));
                        self.reconnect_delay = if self.reconnect_delay.is_zero() {
                            RECONNECT_BASE_DELAY
                        } else {
                            std::cmp::min(self.reconnect_delay * 2, RECONNECT_MAX_DELAY)
                        };
                        time::sleep(self.reconnect_delay).await;
                        continue;
                    }
                }
            }

            let Some(stream) = stream.as_mut() else {
                self.connected = false;
                continue;
            };

            let mut read_buf = [0u8; 4096];
            tokio::select! {
                maybe_command = self.command_rx.recv() => {
                    match maybe_command {
                        Some(command) => {
                            self.pending_commands.push_back(command);
                            if let Err(error) = self.flush_pending_commands(stream, &mut recv_buf).await {
                                let _ = self.events_tx.send(MqttEvent::Error(error.to_string()));
                                self.connected = false;
                            }
                        }
                        None => {
                            let _ = self.events_tx.send(MqttEvent::Disconnected { reason: "command channel closed".to_owned() });
                            let _ = stream.shutdown().await;
                            break;
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    if let Err(error) = self.send_packet(stream, &Packet::Pingreq).await {
                        let _ = self.events_tx.send(MqttEvent::Error(error.to_string()));
                        self.connected = false;
                    }
                }
                read_result = stream.read(&mut read_buf) => {
                    match read_result {
                        Ok(0) => {
                            self.connected = false;
                            let _ = self.events_tx.send(MqttEvent::Disconnected { reason: "connection closed by peer".to_owned() });
                        }
                        Ok(n) => {
                            recv_buf.extend_from_slice(&read_buf[..n]);
                            if let Err(error) = self.process_received_packets(stream, &mut recv_buf).await {
                                let _ = self.events_tx.send(MqttEvent::Error(error.to_string()));
                                self.connected = false;
                            }
                        }
                        Err(error) => {
                            let _ = self.events_tx.send(MqttEvent::Error(error.to_string()));
                            self.connected = false;
                        }
                    }
                }
            }
        }
    }

    async fn connect_once(&self) -> Result<(TcpStream, Vec<u8>), MqttError> {
        let mut stream = timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect(&self.server_address),
        )
        .await
        .map_err(|_| MqttError::Timeout("connect timeout".to_owned()))??;
        stream.set_nodelay(true)?;

        let connect_packet = self.build_connect_packet();
        self.send_packet(&mut stream, &connect_packet).await?;

        let mut recv_buf = Vec::new();
        let mut scratch = [0u8; 4096];

        loop {
            let n = timeout(CONNECT_TIMEOUT, stream.read(&mut scratch))
                .await
                .map_err(|_| MqttError::Timeout("connack timeout".to_owned()))??;
            if n == 0 {
                return Err(MqttError::ConnectionRefused(
                    "connection closed before connack".to_owned(),
                ));
            }

            recv_buf.extend_from_slice(&scratch[..n]);
            let mut clone_buf = vec![0u8; recv_buf.len().max(1024)];
            let packet_len = clone_packet(&recv_buf, &mut clone_buf)?;
            if packet_len == 0 {
                continue;
            }

            let packet = decode_slice(&clone_buf[..packet_len])?
                .ok_or_else(|| MqttError::Codec(mqttrs::Error::InvalidHeader))?;
            recv_buf.drain(..packet_len);

            match packet {
                Packet::Connack(connack) => {
                    if connack.code == ConnectReturnCode::Accepted {
                        return Ok((stream, recv_buf));
                    }
                    return Err(MqttError::ConnectionRefused(format!(
                        "broker refused connection: {:?}",
                        connack.code
                    )));
                }
                Packet::Publish(publish) => {
                    let _ = self.events_tx.send(MqttEvent::Message {
                        topic: publish.topic_name.to_owned(),
                        payload: publish.payload.to_vec(),
                        retain: publish.retain,
                    });
                    continue;
                }
                _ => {
                    return Err(MqttError::InvalidConfig(format!(
                        "expected connack during handshake, got {packet:?}"
                    )));
                }
            }
        }
    }

    async fn flush_pending_commands(
        &mut self,
        stream: &mut TcpStream,
        _recv_buf: &mut Vec<u8>,
    ) -> Result<(), MqttError> {
        while let Some(command) = self.pending_commands.pop_front() {
            self.handle_command(stream, command).await?;
            if self.shutting_down {
                break;
            }
        }

        if self.shutting_down {
            return Ok(());
        }

        Ok(())
    }

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
                let _ = stream.shutdown().await;
                self.shutting_down = true;
            }
        }

        Ok(())
    }

    async fn process_received_packets(
        &mut self,
        stream: &mut TcpStream,
        recv_buf: &mut Vec<u8>,
    ) -> Result<(), MqttError> {
        let mut clone_buf = vec![0u8; recv_buf.len().max(1024)];
        loop {
            let packet_len = clone_packet(recv_buf, &mut clone_buf)?;
            if packet_len == 0 {
                break;
            }

            let packet = decode_slice(&clone_buf[..packet_len])?
                .ok_or_else(|| MqttError::Codec(mqttrs::Error::InvalidHeader))?;
            recv_buf.drain(..packet_len);
            self.handle_incoming_packet(stream, packet).await?;
            clone_buf = vec![0u8; recv_buf.len().max(1024)];
        }
        Ok(())
    }

    async fn handle_incoming_packet(
        &mut self,
        stream: &mut TcpStream,
        packet: Packet<'_>,
    ) -> Result<(), MqttError> {
        match packet {
            Packet::Connack(connack) => {
                if connack.code != ConnectReturnCode::Accepted {
                    return Err(MqttError::ConnectionRefused(format!(
                        "broker refused connection: {:?}",
                        connack.code
                    )));
                }
            }
            Packet::Publish(publish) => {
                let _ = self.events_tx.send(MqttEvent::Message {
                    topic: publish.topic_name.to_owned(),
                    payload: publish.payload.to_vec(),
                    retain: publish.retain,
                });
                match publish.qospid {
                    QosPid::AtMostOnce => {}
                    QosPid::AtLeastOnce(pid) => {
                        self.send_packet(stream, &Packet::Puback(pid)).await?;
                    }
                    QosPid::ExactlyOnce(pid) => {
                        self.pending_qos2.insert(pid.get());
                        self.send_packet(stream, &Packet::Pubrec(pid)).await?;
                    }
                }
            }
            Packet::Pubrel(pid) => {
                self.send_packet(stream, &Packet::Pubcomp(pid)).await?;
                self.pending_qos2.remove(&pid.get());
            }
            Packet::Suback(suback) => {
                let topic = self
                    .pending_subscription_topic
                    .clone()
                    .unwrap_or_else(|| "<unknown>".to_owned());

                let success = suback
                    .return_codes
                    .iter()
                    .all(|code| matches!(code, mqttrs::SubscribeReturnCodes::Success(_)));
                if success {
                    let _ = self
                        .events_tx
                        .send(MqttEvent::SubscriptionAcknowledged { topic });
                } else {
                    let _ = self.events_tx.send(MqttEvent::SubscriptionFailed {
                        topic,
                        reason: "broker refused subscription".to_owned(),
                    });
                }
            }
            Packet::Pingresp => {}
            Packet::Disconnect => {
                self.connected = false;
                let _ = self.events_tx.send(MqttEvent::Disconnected {
                    reason: "broker sent disconnect".to_owned(),
                });
            }
            _ => {}
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

    async fn send_packet(
        &self,
        stream: &mut TcpStream,
        packet: &Packet<'_>,
    ) -> Result<(), MqttError> {
        let mut frame_buffer = [0u8; 8192];
        let len = encode_slice(packet, &mut frame_buffer)?;
        let frame = &frame_buffer[..len];

        stream.write_all(frame).await?;
        stream.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mqttrs::{Packet, decode_slice, encode_slice};

    #[test]
    fn connect_packet_roundtrip() {
        let connect = Connect {
            protocol: Protocol::MQTT311,
            keep_alive: 30,
            client_id: "test-client",
            clean_session: true,
            last_will: None,
            username: Some("user"),
            password: Some(b"secret"),
        };

        let packet: Packet = connect.into();
        let mut frame = [0u8; 1024];
        let len = encode_slice(&packet, &mut frame).unwrap();
        let decoded = decode_slice(&frame[..len]).unwrap().unwrap();

        match decoded {
            Packet::Connect(connect) => {
                assert_eq!(connect.client_id, "test-client");
                assert_eq!(connect.keep_alive, 30);
                assert_eq!(connect.username, Some("user"));
                assert_eq!(connect.password, Some(b"secret".as_ref()));
            }
            other => panic!("expected connect packet, got {other:?}"),
        }
    }
}
