use crate::obs::client::{OBSClient, OBSConnectionConfig, OBSConnectionStatus};
use crate::obs::events::OBSEventHandler;
use crate::sync::master::MasterSync;
use crate::sync::slave::SlaveSync;
use crate::sync::protocol::{SyncMessage, SyncTargetType};
use crate::network::server::MasterServer;
use crate::network::client::SlaveClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::{mpsc, RwLock, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppMode {
    Master,
    Slave,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    pub host: String,
    pub port: u16,
}

pub struct AppState {
    pub obs_client: Arc<OBSClient>,
    pub mode: Arc<RwLock<Option<AppMode>>>,
    pub network_port: Arc<RwLock<u16>>,
    // Master mode components
    pub master_server: Arc<RwLock<Option<Arc<MasterServer>>>>,
    pub master_sync: Arc<RwLock<Option<Arc<MasterSync>>>>,
    pub obs_event_handler: Arc<RwLock<Option<Arc<OBSEventHandler>>>>,
    // Slave mode components
    pub slave_client: Arc<RwLock<Option<Arc<SlaveClient>>>>,
    pub slave_sync: Arc<RwLock<Option<Arc<SlaveSync>>>>,
    // Message channels
    pub sync_message_tx: Arc<Mutex<Option<mpsc::UnboundedSender<SyncMessage>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            obs_client: Arc::new(OBSClient::new()),
            mode: Arc::new(RwLock::new(None)),
            network_port: Arc::new(RwLock::new(8080)),
            master_server: Arc::new(RwLock::new(None)),
            master_sync: Arc::new(RwLock::new(None)),
            obs_event_handler: Arc::new(RwLock::new(None)),
            slave_client: Arc::new(RwLock::new(None)),
            slave_sync: Arc::new(RwLock::new(None)),
            sync_message_tx: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
pub async fn connect_obs(
    state: State<'_, AppState>,
    config: OBSConnectionConfig,
) -> Result<(), String> {
    state
        .obs_client
        .connect(config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect_obs(state: State<'_, AppState>) -> Result<(), String> {
    state
        .obs_client
        .disconnect()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_obs_status(state: State<'_, AppState>) -> Result<OBSConnectionStatus, String> {
    Ok(state.obs_client.get_status().await)
}

#[tauri::command]
pub async fn set_app_mode(state: State<'_, AppState>, mode: AppMode) -> Result<(), String> {
    *state.mode.write().await = Some(mode);
    Ok(())
}

#[tauri::command]
pub async fn get_app_mode(state: State<'_, AppState>) -> Result<Option<AppMode>, String> {
    Ok(state.mode.read().await.clone())
}

#[tauri::command]
pub async fn start_master_server(
    state: State<'_, AppState>,
    port: u16,
) -> Result<(), String> {
    // Check if OBS is connected
    if !state.obs_client.is_connected().await {
        return Err("OBS is not connected".to_string());
    }

    // Update port
    *state.network_port.write().await = port;

    // Create MasterSync
    let (master_sync, sync_rx) = MasterSync::new(state.obs_client.clone());
    let master_sync = Arc::new(master_sync);
    *state.master_sync.write().await = Some(master_sync.clone());

    // Create and start MasterServer
    let master_server = Arc::new(MasterServer::new(port));
    master_server
        .start(sync_rx)
        .await
        .map_err(|e| format!("Failed to start master server: {}", e))?;
    *state.master_server.write().await = Some(master_server);

    // Send initial state to any connecting slaves
    // Note: In a real implementation, you'd want to detect new client connections
    // and send the initial state only to them. For now, we send it periodically.
    let master_sync_for_state = master_sync.clone();
    tokio::spawn(async move {
        // Wait a bit for slaves to connect
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        // Send initial state
        if let Err(e) = master_sync_for_state.send_initial_state().await {
            eprintln!("Failed to send initial state: {}", e);
        }
    });

    // Create OBS event handler
    let (event_handler, event_rx) = OBSEventHandler::new();
    let event_handler = Arc::new(event_handler);
    
    // Start listening to OBS events
    let client_arc = state.obs_client.get_client_arc();
    let client_lock = client_arc.read().await;
    if let Some(obs_client) = client_lock.as_ref() {
        event_handler
            .start_listening(obs_client)
            .await
            .map_err(|e| format!("Failed to start OBS event listener: {}", e))?;
    }
    drop(client_lock);
    
    // Start monitoring OBS events
    master_sync.start_monitoring(event_rx).await;
    
    // Store event handler
    *state.obs_event_handler.write().await = Some(event_handler);

    println!("Master server started on port {}", port);
    Ok(())
}

#[tauri::command]
pub async fn stop_master_server(state: State<'_, AppState>) -> Result<(), String> {
    // Stop master server if running
    if let Some(server) = state.master_server.write().await.take() {
        server.stop().await;
    }
    
    // Clear master components
    *state.master_sync.write().await = None;
    *state.obs_event_handler.write().await = None;
    *state.sync_message_tx.lock().await = None;

    println!("Master server stopped");
    Ok(())
}

#[tauri::command]
pub async fn connect_to_master(
    state: State<'_, AppState>,
    config: NetworkConfig,
) -> Result<(), String> {
    // Check if OBS is connected
    if !state.obs_client.is_connected().await {
        return Err("OBS is not connected".to_string());
    }

    println!("Connecting to master at {}:{}", config.host, config.port);

    // Create SlaveClient
    let slave_client = Arc::new(SlaveClient::new(config.host.clone(), config.port));
    
    // Connect to master and get sync message receiver
    let sync_rx = slave_client
        .connect()
        .await
        .map_err(|e| format!("Failed to connect to master: {}", e))?;
    
    *state.slave_client.write().await = Some(slave_client);

    // Create SlaveSync
    let (slave_sync, _alert_rx) = SlaveSync::new(state.obs_client.clone());
    let slave_sync = Arc::new(slave_sync);
    *state.slave_sync.write().await = Some(slave_sync.clone());

    // Start processing sync messages
    tokio::spawn(async move {
        let mut rx = sync_rx;
        let mut first_message = true;
        while let Some(message) = rx.recv().await {
            // First message should be StateSync for initial synchronization
            if first_message {
                println!("Waiting for initial state from master...");
                first_message = false;
            }
            
            if let Err(e) = slave_sync.apply_sync_message(message).await {
                eprintln!("Failed to apply sync message: {}", e);
            }
        }
    });

    println!("Connected to master at {}:{}", config.host, config.port);
    println!("Note: Initial state will be synchronized from master...");
    Ok(())
}

#[tauri::command]
pub async fn disconnect_from_master(state: State<'_, AppState>) -> Result<(), String> {
    // Disconnect slave client
    if let Some(client) = state.slave_client.write().await.take() {
        client.disconnect().await;
    }

    // Clear slave components
    *state.slave_sync.write().await = None;

    println!("Disconnected from master");
    Ok(())
}

#[tauri::command]
pub async fn set_sync_targets(
    state: State<'_, AppState>,
    targets: Vec<SyncTargetType>,
) -> Result<(), String> {
    println!("Setting sync targets: {:?}", targets);
    
    // Update targets for master mode
    if let Some(master_sync) = state.master_sync.read().await.as_ref() {
        master_sync.set_active_targets(targets).await;
    } else {
        // Just log the targets if not in master mode (slave mode doesn't need to set targets)
        println!("Sync targets set (not in master mode)");
    }
    
    Ok(())
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
