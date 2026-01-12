use super::protocol::{SyncMessage, SyncMessageType};
use crate::obs::{commands::OBSCommands, OBSClient};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct DesyncAlert {
    pub id: String,
    pub timestamp: i64,
    pub scene_name: String,
    pub source_name: String,
    pub message: String,
    pub severity: AlertSeverity,
}

#[derive(Debug, Clone)]
pub enum AlertSeverity {
    Warning,
    Error,
}

pub struct SlaveSync {
    obs_client: Arc<OBSClient>,
    alert_tx: mpsc::UnboundedSender<DesyncAlert>,
}

impl SlaveSync {
    pub fn new(obs_client: Arc<OBSClient>) -> (Self, mpsc::UnboundedReceiver<DesyncAlert>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                obs_client,
                alert_tx: tx,
            },
            rx,
        )
    }

    pub async fn apply_sync_message(&self, message: SyncMessage) -> Result<()> {
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

                // Note: Transform data would need to be included in the payload
                // This is a simplified version
                if let Err(e) = self.handle_transform_update(&client, scene_name, scene_item_id).await {
                    self.send_alert(
                        scene_name.to_string(),
                        String::new(),
                        format!("Failed to update transform: {}", e),
                        AlertSeverity::Warning,
                    )?;
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
                // Apply initial state from master
                if let Some(scene_name) = message.payload["current_program_scene"].as_str() {
                    if let Err(e) = crate::obs::commands::OBSCommands::set_current_program_scene(&client, scene_name).await {
                        self.send_alert(
                            scene_name.to_string(),
                            String::new(),
                            format!("Failed to sync initial scene: {}", e),
                            AlertSeverity::Warning,
                        )?;
                    } else {
                        println!("Applied initial state: scene = {}", scene_name);
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_transform_update(
        &self,
        _client: &obws::Client,
        _scene_name: &str,
        _scene_item_id: i64,
    ) -> Result<()> {
        // Transform update logic would go here
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
