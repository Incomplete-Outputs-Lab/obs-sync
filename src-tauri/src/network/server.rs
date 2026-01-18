use crate::sync::protocol::SyncMessage;
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};

type ClientId = String;
type ClientConnection = WebSocketStream<TcpStream>;

type InitialStateCallback = Arc<
    dyn Fn(ClientId) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub id: String,
    pub ip_address: String,
    pub connected_at: i64,
    pub last_activity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaveStatus {
    pub client_id: String,
    pub is_synced: bool,
    pub desync_details: Vec<serde_json::Value>,
    pub last_report_time: i64,
}

pub struct MasterServer {
    clients: Arc<RwLock<HashMap<ClientId, mpsc::UnboundedSender<Message>>>>,
    client_info: Arc<RwLock<HashMap<ClientId, ClientInfo>>>,
    slave_statuses: Arc<RwLock<HashMap<ClientId, SlaveStatus>>>,
    port: u16,
    shutdown: Arc<AtomicBool>,
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    initial_state_callback: Arc<RwLock<Option<InitialStateCallback>>>,
}

impl MasterServer {
    pub fn new(port: u16) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            client_info: Arc::new(RwLock::new(HashMap::new())),
            slave_statuses: Arc::new(RwLock::new(HashMap::new())),
            port,
            shutdown: Arc::new(AtomicBool::new(false)),
            tasks: Arc::new(RwLock::new(Vec::new())),
            initial_state_callback: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_initial_state_callback<F, Fut>(&self, callback: F)
    where
        F: Fn(ClientId) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let wrapped = Arc::new(move |client_id: ClientId| {
            Box::pin(callback(client_id))
                as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });
        *self.initial_state_callback.write().await = Some(wrapped);
    }

    pub async fn stop(&self) {
        // Signal shutdown
        self.shutdown.store(true, Ordering::SeqCst);

        // Abort all tasks
        let tasks = self.tasks.write().await;
        for task in tasks.iter() {
            task.abort();
        }

        // Clear clients
        self.clients.write().await.clear();
        self.client_info.write().await.clear();
        self.slave_statuses.write().await.clear();

        println!("Master server stopped");
    }

    pub async fn start(&self, mut sync_rx: mpsc::UnboundedReceiver<SyncMessage>) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .context(format!("Failed to bind to {}", addr))?;

        println!("Master server listening on: {}", addr);

        let clients = self.clients.clone();
        let shutdown = self.shutdown.clone();

        // Broadcast sync messages to all connected clients
        let broadcast_task = tokio::spawn(async move {
            while let Some(message) = sync_rx.recv().await {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                let json = match serde_json::to_string(&message) {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("Failed to serialize sync message: {}", e);
                        continue;
                    }
                };

                let clients_lock = clients.read().await;
                for (client_id, tx) in clients_lock.iter() {
                    if let Err(e) = tx.send(Message::Text(json.clone())) {
                        eprintln!("Failed to send message to client {}: {}", client_id, e);
                    }
                }
            }
        });

        // Accept incoming connections
        let clients_for_accept = self.clients.clone();
        let client_info_for_accept = self.client_info.clone();
        let shutdown_for_accept = self.shutdown.clone();
        let callback_for_accept = self.initial_state_callback.clone();
        let accept_task = tokio::spawn(async move {
            loop {
                if shutdown_for_accept.load(Ordering::SeqCst) {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, addr)) => {
                        println!("New connection from: {}", addr);
                        let clients = clients_for_accept.clone();
                        let client_info = client_info_for_accept.clone();
                        let slave_statuses = self.slave_statuses.clone();
                        let callback = callback_for_accept.clone();
                        tokio::spawn(handle_connection(
                            stream,
                            addr.to_string(),
                            clients,
                            client_info,
                            slave_statuses,
                            callback,
                        ));
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                        break;
                    }
                }
            }
        });

        // Store task handles
        let mut tasks = self.tasks.write().await;
        tasks.push(broadcast_task);
        tasks.push(accept_task);

        Ok(())
    }

    pub async fn get_connected_clients_count(&self) -> usize {
        self.clients.read().await.len()
    }

    pub async fn get_connected_clients_info(&self) -> Vec<ClientInfo> {
        let info = self.client_info.read().await;
        info.values().cloned().collect()
    }

    pub async fn get_slave_statuses(&self) -> Vec<SlaveStatus> {
        let statuses = self.slave_statuses.read().await;
        statuses.values().cloned().collect()
    }
}

async fn handle_connection(
    stream: TcpStream,
    client_id: ClientId,
    clients: Arc<RwLock<HashMap<ClientId, mpsc::UnboundedSender<Message>>>>,
    client_info: Arc<RwLock<HashMap<ClientId, ClientInfo>>>,
    slave_statuses: Arc<RwLock<HashMap<ClientId, SlaveStatus>>>,
    callback: Arc<RwLock<Option<InitialStateCallback>>>,
) {
    let peer_addr = stream.peer_addr().ok();
    let ip_address = peer_addr
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let connected_at = chrono::Utc::now().timestamp_millis();

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WebSocket handshake failed for {}: {}", client_id, e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Add client to the list
    clients.write().await.insert(client_id.clone(), tx.clone());

    // Add client info
    {
        let mut info = client_info.write().await;
        info.insert(
            client_id.clone(),
            ClientInfo {
                id: client_id.clone(),
                ip_address: ip_address.clone(),
                connected_at,
                last_activity: connected_at,
            },
        );
    }

    println!("Client connected: {} from {}", client_id, ip_address);

    // Call initial state callback for new client
    let callback_lock = callback.read().await;
    if let Some(cb) = callback_lock.as_ref() {
        let client_id_clone = client_id.clone();
        let future = cb(client_id_clone);
        drop(callback_lock); // Release lock before awaiting
        tokio::spawn(future);
        println!("Triggered initial state sync for client: {}", client_id);
    }

    // Forward messages from tx to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender.send(message).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client (heartbeats, etc.)
    let client_info_for_update = client_info.clone();
    while let Some(msg) = ws_receiver.next().await {
        // Update last activity time
        {
            let mut info = client_info_for_update.write().await;
            if let Some(info_entry) = info.get_mut(&client_id) {
                info_entry.last_activity = chrono::Utc::now().timestamp_millis();
            }
        }

        match msg {
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                // Send pong
                if let Some(tx) = clients.read().await.get(&client_id) {
                    let _ = tx.send(Message::Pong(data));
                }
            }
            Ok(Message::Text(text)) => {
                // Try to parse as SyncMessage to handle StateSyncRequest and StateReport
                if let Ok(sync_msg) = serde_json::from_str::<SyncMessage>(&text) {
                    match sync_msg.message_type {
                        crate::sync::protocol::SyncMessageType::StateSyncRequest => {
                            println!("Received StateSyncRequest from {}", client_id);
                            // Trigger initial state callback
                            let callback_lock = callback.read().await;
                            if let Some(cb) = callback_lock.as_ref() {
                                let client_id_clone = client_id.clone();
                                let future = cb(client_id_clone);
                                drop(callback_lock);
                                tokio::spawn(future);
                            }
                        }
                        crate::sync::protocol::SyncMessageType::StateReport => {
                            // Update slave status
                            let mut statuses = slave_statuses.write().await;
                            if let (Some(is_synced), Some(desync_details)) = (
                                sync_msg.payload.get("is_synced").and_then(|v| v.as_bool()),
                                sync_msg
                                    .payload
                                    .get("desync_details")
                                    .and_then(|v| v.as_array()),
                            ) {
                                statuses.insert(
                                    client_id.clone(),
                                    SlaveStatus {
                                        client_id: client_id.clone(),
                                        is_synced,
                                        desync_details: desync_details.clone(),
                                        last_report_time: chrono::Utc::now().timestamp_millis(),
                                    },
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("WebSocket error for {}: {}", client_id, e);
                break;
            }
            _ => {}
        }
    }

    // Remove client from the list
    clients.write().await.remove(&client_id);
    client_info.write().await.remove(&client_id);
    slave_statuses.write().await.remove(&client_id);
    send_task.abort();
    println!("Client disconnected: {}", client_id);
}
