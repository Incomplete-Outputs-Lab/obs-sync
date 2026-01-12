use super::protocol::{SyncMessage, SyncMessageType};
use crate::obs::{commands::OBSCommands, OBSClient};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::mpsc;

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
                let input_name = message.payload["input_name"]
                    .as_str()
                    .context("Invalid input_name")?;

                // Handle image update
                if let Err(e) = self.handle_image_update(&client, input_name).await {
                    self.send_alert(
                        String::new(),
                        input_name.to_string(),
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

    async fn handle_image_update(&self, _client: &obws::Client, _input_name: &str) -> Result<()> {
        // Image update logic would go here
        Ok(())
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
