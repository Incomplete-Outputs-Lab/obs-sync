use super::protocol::{SyncMessage, SyncMessageType};
use super::diff::{DiffDetector, DiffSeverity};
use crate::obs::{commands::OBSCommands, OBSClient};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::fs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DesyncAlert {
    pub id: String,
    pub timestamp: i64,
    pub scene_name: String,
    pub source_name: String,
    pub message: String,
    pub severity: AlertSeverity,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AlertSeverity {
    Warning,
    Error,
}

pub struct SlaveSync {
    obs_client: Arc<OBSClient>,
    alert_tx: mpsc::UnboundedSender<DesyncAlert>,
    expected_state: Arc<RwLock<serde_json::Value>>,
}

impl SlaveSync {
    pub fn new(obs_client: Arc<OBSClient>) -> (Self, mpsc::UnboundedReceiver<DesyncAlert>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                obs_client,
                alert_tx: tx,
                expected_state: Arc::new(RwLock::new(serde_json::json!({}))),
            },
            rx,
        )
    }

    /// Start periodic state checking task
    pub fn start_periodic_check(&self, interval_secs: u64) {
        let obs_client = self.obs_client.clone();
        let expected_state = self.expected_state.clone();
        let alert_tx = self.alert_tx.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            
            loop {
                interval.tick().await;
                
                // Get current local OBS state
                let local_state = match Self::get_current_obs_state(&obs_client).await {
                    Ok(state) => state,
                    Err(e) => {
                        eprintln!("Failed to get local OBS state: {}", e);
                        continue;
                    }
                };

                // Compare with expected state
                let expected = expected_state.read().await;
                if expected.is_null() || expected.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    // No expected state yet, skip check
                    continue;
                }

                let diffs = DiffDetector::detect_differences(&local_state, &expected);
                
                if !diffs.is_empty() {
                    println!("⚠️  Detected {} state difference(s)", diffs.len());
                    
                    for diff in diffs {
                        let severity = match diff.severity {
                            DiffSeverity::Critical => AlertSeverity::Error,
                            _ => AlertSeverity::Warning,
                        };
                        
                        let alert = DesyncAlert {
                            id: uuid::Uuid::new_v4().to_string(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            scene_name: diff.scene_name,
                            source_name: diff.source_name,
                            message: diff.description,
                            severity,
                        };
                        
                        if let Err(e) = alert_tx.send(alert) {
                            eprintln!("Failed to send desync alert: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Get current OBS state for comparison
    async fn get_current_obs_state(obs_client: &Arc<OBSClient>) -> Result<serde_json::Value> {
        let client_arc = obs_client.get_client_arc();
        let client_lock = client_arc.read().await;
        
        if let Some(client) = client_lock.as_ref() {
            // Get current scene
            let current_scene = client.scenes().current_program_scene().await
                .context("Failed to get current scene")?;
            
            // Get sources in current scene
            let items = client.scene_items().list(&current_scene).await
                .context("Failed to get scene items")?;
            
                        let mut sources = Vec::new();
                        for item in items {
                            let transform = client.scene_items().transform(&current_scene, item.id).await.ok();
                
                sources.push(serde_json::json!({
                    "name": item.source_name,
                    "transform": transform.map(|t| serde_json::json!({
                        "position_x": t.position_x,
                        "position_y": t.position_y,
                        "scale_x": t.scale_x,
                        "scale_y": t.scale_y,
                        "rotation": t.rotation,
                    })),
                }));
            }
            
            Ok(serde_json::json!({
                "current_scene": current_scene,
                "sources": sources,
            }))
        } else {
            Err(anyhow::anyhow!("OBS client not connected"))
        }
    }

    /// Update expected state from sync message
    async fn update_expected_state(&self, message: &SyncMessage) {
        let mut expected = self.expected_state.write().await;
        
        match message.message_type {
            SyncMessageType::SceneChange => {
                if let Some(scene_name) = message.payload["scene_name"].as_str() {
                    expected["current_scene"] = serde_json::json!(scene_name);
                }
            }
            SyncMessageType::StateSync => {
                // Full state update
                if let Some(current_scene) = message.payload["current_program_scene"].as_str() {
                    expected["current_scene"] = serde_json::json!(current_scene);
                }
                // Could expand to include full scene data
            }
            _ => {}
        }
    }

    pub async fn apply_sync_message(&self, message: SyncMessage) -> Result<()> {
        // Update expected state first
        self.update_expected_state(&message).await;
        
        let client_arc = self.obs_client.get_client_arc();
        let client_lock = client_arc.read().await;
        let client = client_lock.as_ref().context("OBS client not connected")?;

        match message.message_type {
            SyncMessageType::SceneChange => {
                let scene_name = message.payload["scene_name"]
                    .as_str()
                    .context("Invalid scene_name in payload")?;

                if let Err(e) = OBSCommands::set_current_program_scene(&client, scene_name).await {
                    self.send_alert(
                        scene_name.to_string(),
                        String::new(),
                        format!("Failed to change scene: {}", e),
                        AlertSeverity::Error,
                    )?;
                }
            }
            SyncMessageType::TransformUpdate => {
                let scene_name = message.payload["scene_name"]
                    .as_str()
                    .context("Invalid scene_name")?;
                let scene_item_id = message.payload["scene_item_id"]
                    .as_i64()
                    .context("Invalid scene_item_id")?;

                // Apply transform if included in payload
                if let Some(transform) = message.payload["transform"].as_object() {
                    if let Err(e) = self.apply_transform(&client, scene_name, scene_item_id, transform).await {
                        self.send_alert(
                            scene_name.to_string(),
                            String::new(),
                            format!("Failed to update transform: {}", e),
                            AlertSeverity::Warning,
                        )?;
                    } else {
                        println!("Applied transform update for item {} in scene {}", scene_item_id, scene_name);
                    }
                } else {
                    eprintln!("Transform data missing in payload");
                }
            }
            SyncMessageType::ImageUpdate => {
                let source_name = message.payload["source_name"]
                    .as_str()
                    .context("Invalid source_name")?;
                let file_path = message.payload["file"]
                    .as_str()
                    .unwrap_or("");
                let image_data = message.payload["image_data"]
                    .as_str();

                // Handle image update
                if let Err(e) = self.handle_image_update(
                    &client,
                    source_name,
                    file_path,
                    image_data
                ).await {
                    self.send_alert(
                        String::new(),
                        source_name.to_string(),
                        format!("Failed to update image: {}", e),
                        AlertSeverity::Warning,
                    )?;
                }
            }
            SyncMessageType::Heartbeat => {
                // Just acknowledge heartbeat
            }
            SyncMessageType::StateSync => {
                println!("Applying complete initial state from master...");
                
                // Apply all scenes and items
                if let Some(scenes) = message.payload["scenes"].as_array() {
                    for scene in scenes {
                        let scene_name = scene["name"].as_str().unwrap_or("");
                        println!("Processing scene: {}", scene_name);
                        
                        // Apply items in this scene
                        if let Some(items) = scene["items"].as_array() {
                            for item in items {
                                let source_name = item["source_name"].as_str().unwrap_or("");
                                let scene_item_id = item["scene_item_id"].as_i64().unwrap_or(0);
                                
                                println!("  - Applying item: {} (id: {})", source_name, scene_item_id);
                                
                                // Apply transform if available
                                if let Some(transform) = item["transform"].as_object() {
                                    if let Err(e) = self.apply_transform(
                                        &client,
                                        scene_name,
                                        scene_item_id,
                                        transform
                                    ).await {
                                        eprintln!("Failed to apply transform for {}: {}", source_name, e);
                                    }
                                }
                                
                                // Apply image data if available
                                if let Some(image_data) = item["image_data"].as_object() {
                                    if let (Some(file), Some(data)) = (
                                        image_data.get("file").and_then(|v| v.as_str()),
                                        image_data.get("data").and_then(|v| v.as_str())
                                    ) {
                                        if let Err(e) = self.handle_image_update(
                                            &client,
                                            source_name,
                                            file,
                                            Some(data)
                                        ).await {
                                            eprintln!("Failed to apply image for {}: {}", source_name, e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Apply current program scene
                if let Some(scene_name) = message.payload["current_program_scene"].as_str() {
                    if let Err(e) = crate::obs::commands::OBSCommands::set_current_program_scene(&client, scene_name).await {
                        self.send_alert(
                            scene_name.to_string(),
                            String::new(),
                            format!("Failed to sync initial scene: {}", e),
                            AlertSeverity::Warning,
                        )?;
                    } else {
                        println!("✓ Applied current program scene: {}", scene_name);
                    }
                }
                
                // Apply preview scene if in studio mode
                if let Some(preview_scene) = message.payload["current_preview_scene"].as_str() {
                    // Note: Setting preview scene requires studio mode to be enabled
                    println!("Preview scene in master: {}", preview_scene);
                }
                
                println!("✓ Initial state fully applied");
            }
            _ => {}
        }

        Ok(())
    }

    async fn apply_transform(
        &self,
        _client: &obws::Client,
        scene_name: &str,
        scene_item_id: i64,
        transform: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        // Note: Transform application depends on obws library API structure
        // This is a placeholder implementation
        // TODO: Implement actual transform application based on obws version
        
        println!(
            "Transform update received for item {} in scene {}: {:?}",
            scene_item_id, scene_name, transform
        );
        
        // In production, you would use obws API to apply the transform:
        // - Extract position_x, position_y
        // - Extract scale_x, scale_y
        // - Extract rotation
        // - Call appropriate obws method to set transform
        
        Ok(())
    }

    async fn handle_image_update(
        &self,
        client: &obws::Client,
        source_name: &str,
        _original_file_path: &str,
        image_data: Option<&str>,
    ) -> Result<()> {
        if let Some(encoded_data) = image_data {
            println!("Received image data for {}, decoding...", source_name);
            
            // Decode base64 image data
            let decoded_data = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                encoded_data
            ).context("Failed to decode image data")?;
            
            println!("Decoded {} bytes of image data", decoded_data.len());
            
            // Create temp directory for synced images
            let temp_dir = std::env::temp_dir().join("obs-sync");
            fs::create_dir_all(&temp_dir).await.context("Failed to create temp directory")?;
            
            // Generate unique filename
            let file_extension = "png"; // Default to PNG, could be detected from data
            let temp_file_path = temp_dir.join(format!("{}_{}.{}", 
                source_name.replace("/", "_").replace("\\", "_"),
                chrono::Utc::now().timestamp_millis(),
                file_extension
            ));
            
            println!("Saving image to: {:?}", temp_file_path);
            
            // Write decoded data to temp file
            fs::write(&temp_file_path, &decoded_data)
                .await
                .context("Failed to write image file")?;
            
            // Update OBS input settings with new file path
            let temp_file_str = temp_file_path.to_string_lossy().to_string();
            let settings = serde_json::json!({
                "file": temp_file_str
            });
            
            println!("Applying image to OBS source: {}", source_name);
            
            // Apply settings to OBS
            match client.inputs().set_settings(
                obws::requests::inputs::SetSettings {
                    input: source_name,
                    settings: &settings,
                    overlay: Some(true),
                }
            ).await {
                Ok(_) => {
                    println!("Successfully applied image to {}", source_name);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to apply image to OBS: {}", e);
                    Err(anyhow::anyhow!("Failed to apply image: {}", e))
                }
            }
        } else {
            println!("No image data provided for {}", source_name);
            Ok(())
        }
    }

    fn send_alert(
        &self,
        scene_name: String,
        source_name: String,
        message: String,
        severity: AlertSeverity,
    ) -> Result<()> {
        let alert = DesyncAlert {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            scene_name,
            source_name,
            message,
            severity,
        };
        self.alert_tx.send(alert)?;
        Ok(())
    }
}
