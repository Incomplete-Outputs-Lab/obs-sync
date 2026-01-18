use crate::sync::protocol::SyncMessage;
use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconnectionStatus {
    pub is_reconnecting: bool,
    pub attempt_count: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
}

pub struct SlaveClient {
    host: String,
    port: u16,
    ws_stream: Arc<RwLock<Option<WsStream>>>,
    should_reconnect: Arc<AtomicBool>,
    max_reconnect_attempts: u32,
    message_tx: Arc<RwLock<Option<mpsc::UnboundedSender<Message>>>>,
    sync_message_tx: Arc<RwLock<Option<mpsc::UnboundedSender<SyncMessage>>>>,
    reconnection_status: Arc<RwLock<ReconnectionStatus>>,
    current_attempt: Arc<AtomicU32>,
}

impl SlaveClient {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            ws_stream: Arc::new(RwLock::new(None)),
            should_reconnect: Arc::new(AtomicBool::new(true)),
            max_reconnect_attempts: 10,
            message_tx: Arc::new(RwLock::new(None)),
            sync_message_tx: Arc::new(RwLock::new(None)),
            reconnection_status: Arc::new(RwLock::new(ReconnectionStatus {
                is_reconnecting: false,
                attempt_count: 0,
                max_attempts: 10,
                last_error: None,
            })),
            current_attempt: Arc::new(AtomicU32::new(0)),
        }
    }

    pub async fn get_reconnection_status(&self) -> ReconnectionStatus {
        self.reconnection_status.read().await.clone()
    }

    pub async fn request_resync(&self) -> Result<()> {
        let tx = self.sync_message_tx.read().await;
        if let Some(sender) = tx.as_ref() {
            let request = SyncMessage::state_sync_request();
            sender
                .send(request)
                .map_err(|_| anyhow::anyhow!("Failed to send resync request"))?;
            println!("Sent StateSyncRequest to master");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Not connected to master"))
        }
    }

    pub async fn connect(
        &self,
    ) -> Result<(
        mpsc::UnboundedReceiver<SyncMessage>,
        mpsc::UnboundedSender<SyncMessage>,
    )> {
        let url = format!("ws://{}:{}", self.host, self.port);
        let (tx, rx) = mpsc::unbounded_channel::<SyncMessage>();
        let (send_tx, mut send_rx) = mpsc::unbounded_channel::<SyncMessage>();

        let host = self.host.clone();
        let port = self.port;
        let should_reconnect = self.should_reconnect.clone();
        let max_attempts = self.max_reconnect_attempts;
        let message_tx_for_send = self.message_tx.clone();
        let sync_message_tx_for_store = self.sync_message_tx.clone();

        // Spawn task to handle sending messages (will be connected when WebSocket is ready)
        let send_tx_for_sending = send_tx.clone();
        let (send_ready_tx, mut send_ready_rx) =
            mpsc::unbounded_channel::<futures_util::stream::SplitSink<_, _>>();

        tokio::spawn(async move {
            let mut current_sender: Option<futures_util::stream::SplitSink<_, _>> = None;

            loop {
                tokio::select! {
                    // Receive new WebSocket sender
                    sender = send_ready_rx.recv() => {
                        if let Some(s) = sender {
                            current_sender = Some(s);
                        }
                    }
                    // Receive message to send
                    msg = send_rx.recv() => {
                        if let Some(msg) = msg {
                            if let Some(ref mut sender) = current_sender {
                                let json = match serde_json::to_string(&msg) {
                                    Ok(j) => j,
                                    Err(e) => {
                                        eprintln!("Failed to serialize sync message: {}", e);
                                        continue;
                                    }
                                };
                                if sender.send(Message::Text(json)).await.is_err() {
                                    current_sender = None;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        // Spawn connection task with auto-reconnect
        let reconnection_status_for_task = self.reconnection_status.clone();
        let current_attempt_for_task = self.current_attempt.clone();
        tokio::spawn(async move {
            let mut attempt = 0;

            loop {
                if !should_reconnect.load(Ordering::SeqCst) {
                    // Update status: not reconnecting
                    {
                        let mut status = reconnection_status_for_task.write().await;
                        status.is_reconnecting = false;
                        status.attempt_count = 0;
                        status.last_error = None;
                    }
                    current_attempt_for_task.store(0, Ordering::SeqCst);
                    break;
                }

                if attempt > 0 {
                    // Update status: reconnecting
                    {
                        let mut status = reconnection_status_for_task.write().await;
                        status.is_reconnecting = true;
                        status.attempt_count = attempt;
                        status.max_attempts = max_attempts;
                    }
                    current_attempt_for_task.store(attempt, Ordering::SeqCst);

                    // Exponential backoff: 1s, 2s, 4s, 8s, 16s, max 30s
                    let delay = std::cmp::min(2_u64.pow(attempt - 1), 30);
                    println!(
                        "Reconnecting to master in {} seconds... (attempt {}/{})",
                        delay, attempt, max_attempts
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                }

                if attempt >= max_attempts {
                    eprintln!(
                        "Max reconnection attempts ({}) reached. Stopping reconnection.",
                        max_attempts
                    );
                    {
                        let mut status = reconnection_status_for_task.write().await;
                        status.is_reconnecting = false;
                        status.attempt_count = attempt;
                        status.last_error = Some(format!(
                            "Max reconnection attempts ({}) reached",
                            max_attempts
                        ));
                    }
                    current_attempt_for_task.store(0, Ordering::SeqCst);
                    break;
                }

                let url = format!("ws://{}:{}", host, port);
                match connect_async(&url).await {
                    Ok((ws_stream, _)) => {
                        println!("Connected to master: {}", url);
                        attempt = 0; // Reset attempt counter on successful connection
                                     // Update status: connected successfully
                        {
                            let mut status = reconnection_status_for_task.write().await;
                            status.is_reconnecting = false;
                            status.attempt_count = 0;
                            status.last_error = None;
                        }
                        current_attempt_for_task.store(0, Ordering::SeqCst);

                        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                        let tx_clone = tx.clone();

                        // Store sync message sender for resync requests
                        {
                            let mut sync_tx = sync_message_tx_for_store.write().await;
                            *sync_tx = Some(send_tx_for_sending.clone());
                        }

                        // Send ws_sender to sending task
                        let _ = send_ready_tx.send(ws_sender);

                        // Handle incoming messages
                        let should_reconnect_clone = should_reconnect.clone();
                        let message_tx_for_cleanup = message_tx_for_send.clone();
                        let sync_message_tx_for_cleanup = sync_message_tx_for_store.clone();
                        let reconnection_status_for_incoming = reconnection_status_for_task.clone();
                        tokio::spawn(async move {
                            while let Some(msg) = ws_receiver.next().await {
                                match msg {
                                    Ok(Message::Text(text)) => {
                                        match serde_json::from_str::<SyncMessage>(&text) {
                                            Ok(sync_msg) => {
                                                if tx_clone.send(sync_msg).is_err() {
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to parse sync message: {}", e);
                                            }
                                        }
                                    }
                                    Ok(Message::Ping(_data)) => {
                                        // Pong will be handled by the sending task via ws_sender
                                        // This is handled automatically by tokio-tungstenite
                                    }
                                    Ok(Message::Close(_)) => {
                                        println!("Connection closed by master");
                                        break;
                                    }
                                    Err(e) => {
                                        eprintln!("WebSocket error: {}", e);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            // Connection lost, signal for reconnection
                            should_reconnect_clone.store(true, Ordering::SeqCst);
                            // Clear message sender
                            {
                                let mut tx = message_tx_for_cleanup.write().await;
                                *tx = None;
                            }
                            // Clear sync message sender
                            {
                                let mut sync_tx = sync_message_tx_for_cleanup.write().await;
                                *sync_tx = None;
                            }
                            // Update status: connection lost, will reconnect
                            {
                                let mut status = reconnection_status_for_incoming.write().await;
                                status.is_reconnecting = true;
                                status.attempt_count = 0;
                                status.last_error = Some("Connection lost".to_string());
                            }
                        });

                        // Wait for connection to break
                        // The spawned task above will handle reconnection
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                    Err(e) => {
                        attempt += 1;
                        eprintln!(
                            "Failed to connect to master: {} (attempt {}/{})",
                            e, attempt, max_attempts
                        );
                        // Update status: connection failed
                        {
                            let mut status = reconnection_status_for_task.write().await;
                            status.is_reconnecting = true;
                            status.attempt_count = attempt;
                            status.last_error = Some(format!("{}", e));
                        }
                        current_attempt_for_task.store(attempt, Ordering::SeqCst);
                    }
                }
            }
        });

        Ok((rx, send_tx))
    }

    pub async fn is_connected(&self) -> bool {
        self.ws_stream.read().await.is_some()
    }

    pub async fn disconnect(&self) {
        // Stop reconnection attempts
        self.should_reconnect.store(false, Ordering::SeqCst);

        // Update status: not reconnecting
        {
            let mut status = self.reconnection_status.write().await;
            status.is_reconnecting = false;
            status.attempt_count = 0;
            status.last_error = None;
        }
        self.current_attempt.store(0, Ordering::SeqCst);

        // Clear message sender
        {
            let mut tx = self.message_tx.write().await;
            *tx = None;
        }

        // Clear sync message sender
        {
            let mut sync_tx = self.sync_message_tx.write().await;
            *sync_tx = None;
        }

        let mut stream_lock = self.ws_stream.write().await;
        if let Some(mut stream) = stream_lock.take() {
            let _ = stream.close(None).await;
        }
    }
}
