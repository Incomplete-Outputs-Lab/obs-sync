use crate::network::client::SlaveClient;
use crate::network::server::{ClientInfo, MasterServer, SlaveStatus};
use crate::obs::client::{OBSClient, OBSConnectionConfig, OBSConnectionStatus};
use crate::obs::events::OBSEventHandler;
use crate::sync::master::MasterSync;
use crate::sync::protocol::{SyncMessage, SyncTargetType};
use crate::sync::slave::SlaveSync;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, State};
use tokio::fs;
use tokio::sync::{mpsc, Mutex, RwLock};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub obs: OBSSettings,
    pub master: MasterSettings,
    pub slave: SlaveSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OBSSettings {
    pub host: String,
    pub port: u16,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterSettings {
    pub default_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlaveSettings {
    pub default_host: String,
    pub default_port: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            obs: OBSSettings {
                host: "localhost".to_string(),
                port: 4455,
                password: String::new(),
            },
            master: MasterSettings { default_port: 8080 },
            slave: SlaveSettings {
                default_host: "192.168.1.100".to_string(),
                default_port: 8080,
            },
        }
    }
}

async fn get_config_path(state: &AppState) -> Result<PathBuf, String> {
    let app_handle = state.app_handle.read().await;
    if let Some(handle) = app_handle.as_ref() {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        fs::create_dir_all(&app_data_dir)
            .await
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        Ok(app_data_dir.join("config.json"))
    } else {
        Err("App handle not available".to_string())
    }
}

async fn get_log_dir(state: &AppState) -> Result<PathBuf, String> {
    let app_handle = state.app_handle.read().await;
    if let Some(handle) = app_handle.as_ref() {
        let app_data_dir = handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        let log_dir = app_data_dir.join("logs");
        fs::create_dir_all(&log_dir)
            .await
            .map_err(|e| format!("Failed to create log directory: {}", e))?;
        Ok(log_dir)
    } else {
        Err("App handle not available".to_string())
    }
}

fn get_log_file_path(state: &AppState) -> Result<PathBuf, String> {
    // This is a sync function, so we can't use async here
    // We'll need to get the path differently or make this async
    // For now, return a path that will be resolved async
    Err("Use get_log_file_path_async instead".to_string())
}

async fn get_log_file_path_async(state: &AppState) -> Result<PathBuf, String> {
    let log_dir = get_log_dir(state).await?;
    let date = chrono::Utc::now().format("%Y-%m-%d");
    Ok(log_dir.join(format!("obs-sync-{}.log", date)))
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let config_path = get_config_path(&state).await?;
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&config_path, json)
        .await
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    println!("Settings saved to: {:?}", config_path);
    Ok(())
}

#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let config_path = get_config_path(&state).await?;

    if !config_path.exists() {
        // Return default settings if file doesn't exist
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(&config_path)
        .await
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings file: {}", e))?;

    Ok(settings)
}

#[tauri::command]
pub async fn get_log_file_path(state: State<'_, AppState>) -> Result<String, String> {
    let path = get_log_file_path_async(&state).await?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_log_file(state: State<'_, AppState>) -> Result<(), String> {
    let log_path = get_log_file_path_async(&state).await?;

    if !log_path.exists() {
        return Err("Log file does not exist".to_string());
    }

    let app_handle = state.app_handle.read().await;
    if let Some(handle) = app_handle.as_ref() {
        // Use tauri-plugin-opener to open the file
        tauri_plugin_opener::open(&log_path.to_string_lossy(), None, handle)
            .map_err(|e| format!("Failed to open log file: {}", e))?;
        Ok(())
    } else {
        Err("App handle not available".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMetric {
    pub timestamp: i64,
    pub message_type: String,
    pub latency_ms: f64,
    pub message_size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetrics {
    pub average_latency_ms: f64,
    pub total_messages: usize,
    pub messages_per_second: f64,
    pub total_bytes: usize,
    pub recent_metrics: Vec<SyncMetric>,
}

pub struct PerformanceMonitor {
    metrics: Arc<RwLock<VecDeque<SyncMetric>>>,
    max_metrics: usize,
    send_times: Arc<RwLock<std::collections::HashMap<String, Instant>>>,
}

impl PerformanceMonitor {
    pub fn new(max_metrics: usize) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(VecDeque::with_capacity(max_metrics))),
            max_metrics,
            send_times: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn record_send(&self, message_id: String, message_type: String, size: usize) {
        let mut send_times = self.send_times.write().await;
        send_times.insert(message_id, Instant::now());
    }

    pub async fn record_receive(&self, message_id: String, message_type: String, size: usize) {
        let mut send_times = self.send_times.write().await;
        if let Some(send_time) = send_times.remove(&message_id) {
            let latency = send_time.elapsed().as_secs_f64() * 1000.0; // Convert to milliseconds

            let metric = SyncMetric {
                timestamp: chrono::Utc::now().timestamp_millis(),
                message_type,
                latency_ms: latency,
                message_size_bytes: size,
            };

            let mut metrics = self.metrics.write().await;
            if metrics.len() >= self.max_metrics {
                metrics.pop_front();
            }
            metrics.push_back(metric);
        }
    }

    pub async fn get_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.read().await;
        let recent_metrics: Vec<SyncMetric> = metrics.iter().cloned().collect();

        if recent_metrics.is_empty() {
            return PerformanceMetrics {
                average_latency_ms: 0.0,
                total_messages: 0,
                messages_per_second: 0.0,
                total_bytes: 0,
                recent_metrics: vec![],
            };
        }

        let total_messages = recent_metrics.len();
        let average_latency =
            recent_metrics.iter().map(|m| m.latency_ms).sum::<f64>() / total_messages as f64;

        let total_bytes: usize = recent_metrics.iter().map(|m| m.message_size_bytes).sum();

        // Calculate messages per second (based on time span of recent metrics)
        let messages_per_second = if recent_metrics.len() > 1 {
            let time_span_secs = (recent_metrics.last().unwrap().timestamp
                - recent_metrics.first().unwrap().timestamp)
                as f64
                / 1000.0;
            if time_span_secs > 0.0 {
                total_messages as f64 / time_span_secs
            } else {
                0.0
            }
        } else {
            0.0
        };

        PerformanceMetrics {
            average_latency_ms: average_latency,
            total_messages,
            messages_per_second,
            total_bytes,
            recent_metrics: recent_metrics.into_iter().rev().take(100).collect(), // Last 100 metrics
        }
    }
}

#[derive(Clone)]
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
    // Tauri app handle
    pub app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
    // Performance monitoring
    pub performance_monitor: Arc<PerformanceMonitor>,
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
            app_handle: Arc::new(RwLock::new(None)),
            performance_monitor: Arc::new(PerformanceMonitor::new(1000)), // Keep last 1000 metrics
        }
    }

    pub async fn set_app_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.write().await = Some(handle);
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
pub async fn start_master_server(state: State<'_, AppState>, port: u16) -> Result<(), String> {
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

    // Set up callback to send initial state when new slave connects
    let master_sync_for_callback = master_sync.clone();
    master_server
        .set_initial_state_callback(move |client_id: String| {
            let master_sync_clone = master_sync_for_callback.clone();
            async move {
                println!("Sending initial state to new slave: {}", client_id);
                // Small delay to ensure connection is fully established
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if let Err(e) = master_sync_clone.send_initial_state().await {
                    eprintln!("Failed to send initial state to {}: {}", client_id, e);
                }
            }
        })
        .await;

    master_server
        .start(sync_rx)
        .await
        .map_err(|e| format!("Failed to start master server: {}", e))?;
    *state.master_server.write().await = Some(master_server);

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

    // Connect to master and get sync message receiver and sender
    let (sync_rx, send_tx) = slave_client
        .connect()
        .await
        .map_err(|e| format!("Failed to connect to master: {}", e))?;

    *state.slave_client.write().await = Some(slave_client);

    // Create SlaveSync
    let (slave_sync, alert_rx) = SlaveSync::new(state.obs_client.clone());
    slave_sync.set_state_report_sender(send_tx).await;
    let slave_sync = Arc::new(slave_sync);
    *state.slave_sync.write().await = Some(slave_sync.clone());

    // Start periodic state checking (every 5 seconds)
    slave_sync.start_periodic_check(5);
    println!("Started periodic desync detection (interval: 5s)");

    // Start processing sync messages
    let slave_sync_for_processing = slave_sync.clone();
    tokio::spawn(async move {
        let mut rx = sync_rx;
        let mut first_message = true;
        while let Some(message) = rx.recv().await {
            // First message should be StateSync for initial synchronization
            if first_message {
                println!("Waiting for initial state from master...");
                first_message = false;
            }

            if let Err(e) = slave_sync_for_processing.apply_sync_message(message).await {
                eprintln!("Failed to apply sync message: {}", e);
            }
        }
    });

    // Start processing alerts (forward to frontend via Tauri events)
    let app_handle_lock = state.app_handle.clone();
    tokio::spawn(async move {
        let mut rx = alert_rx;
        while let Some(alert) = rx.recv().await {
            println!("🚨 Desync Alert: {} - {}", alert.scene_name, alert.message);

            // Emit Tauri event to frontend
            if let Some(handle) = app_handle_lock.read().await.as_ref() {
                if let Err(e) = handle.emit("desync-alert", alert.clone()) {
                    eprintln!("Failed to emit desync alert event: {}", e);
                }
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
pub async fn get_slave_reconnection_status(
    state: State<'_, AppState>,
) -> Result<Option<crate::network::client::ReconnectionStatus>, String> {
    if let Some(client) = state.slave_client.read().await.as_ref() {
        Ok(Some(client.get_reconnection_status().await))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn resync_all_slaves(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(master_sync) = state.master_sync.read().await.as_ref() {
        master_sync
            .send_initial_state()
            .await
            .map_err(|e| format!("Failed to resync all slaves: {}", e))?;
        println!("Resync triggered for all slaves");
        Ok(())
    } else {
        Err("Master server is not running".to_string())
    }
}

#[tauri::command]
pub async fn resync_specific_slave(
    state: State<'_, AppState>,
    client_id: String,
) -> Result<(), String> {
    if let Some(master_sync) = state.master_sync.read().await.as_ref() {
        // For now, resync all slaves (we can enhance this later to target specific client)
        // The master server already handles sending to specific clients via the callback
        master_sync
            .send_initial_state()
            .await
            .map_err(|e| format!("Failed to resync slave {}: {}", client_id, e))?;
        println!("Resync triggered for slave: {}", client_id);
        Ok(())
    } else {
        Err("Master server is not running".to_string())
    }
}

#[tauri::command]
pub async fn request_resync_from_master(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(slave_client) = state.slave_client.read().await.as_ref() {
        slave_client
            .request_resync()
            .await
            .map_err(|e| format!("Failed to request resync: {}", e))?;
        println!("Resync requested from master");
        Ok(())
    } else {
        Err("Not connected to master".to_string())
    }
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
pub async fn get_connected_clients_count(state: State<'_, AppState>) -> Result<usize, String> {
    if let Some(server) = state.master_server.read().await.as_ref() {
        Ok(server.get_connected_clients_count().await)
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn get_connected_clients_info(
    state: State<'_, AppState>,
) -> Result<Vec<ClientInfo>, String> {
    if let Some(server) = state.master_server.read().await.as_ref() {
        Ok(server.get_connected_clients_info().await)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn get_slave_statuses(state: State<'_, AppState>) -> Result<Vec<SlaveStatus>, String> {
    if let Some(server) = state.master_server.read().await.as_ref() {
        Ok(server.get_slave_statuses().await)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn get_obs_sources(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let client_arc = state.obs_client.get_client_arc();
    let client_lock = client_arc.read().await;

    if let Some(client) = client_lock.as_ref() {
        let mut sources_map = std::collections::HashMap::new();

        // Get all scenes
        match client.scenes().list().await {
            Ok(scenes) => {
                for scene in scenes.scenes {
                    // Get scene items
                    match client.scene_items().list(&scene.name).await {
                        Ok(items) => {
                            for item in items {
                                // Store source info (avoid duplicates)
                                sources_map.entry(item.source_name.clone()).or_insert_with(|| {
                                    serde_json::json!({
                                        "sourceName": item.source_name,
                                        "sourceType": item.input_kind.clone().unwrap_or_else(|| "unknown".to_string()),
                                        "sourceKind": item.input_kind.clone().unwrap_or_else(|| "unknown".to_string()),
                                    })
                                });
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to get scene items for {}: {}", scene.name, e);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to get scenes: {}", e));
            }
        }

        Ok(sources_map.values().cloned().collect())
    } else {
        Err("OBS is not connected".to_string())
    }
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
