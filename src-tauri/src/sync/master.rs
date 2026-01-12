use super::protocol::{SyncMessage, SyncMessageType, SyncTargetType};
use crate::obs::{events::OBSEvent, OBSClient};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub struct MasterSync {
    obs_client: Arc<OBSClient>,
    message_tx: mpsc::UnboundedSender<SyncMessage>,
    active_targets: Arc<RwLock<Vec<SyncTargetType>>>,
}

impl MasterSync {
    pub fn new(obs_client: Arc<OBSClient>) -> (Self, mpsc::UnboundedReceiver<SyncMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                obs_client,
                message_tx: tx,
                active_targets: Arc::new(RwLock::new(vec![
                    SyncTargetType::Program,
                    SyncTargetType::Source,
                ])),
            },
            rx,
        )
    }

    pub async fn set_active_targets(&self, targets: Vec<SyncTargetType>) {
        *self.active_targets.write().await = targets;
    }

    pub async fn start_monitoring(&self, mut obs_event_rx: mpsc::UnboundedReceiver<OBSEvent>) {
        let message_tx = self.message_tx.clone();
        let active_targets = self.active_targets.clone();
        let obs_client = self.obs_client.clone();

        tokio::spawn(async move {
            while let Some(event) = obs_event_rx.recv().await {
                let targets = active_targets.read().await.clone();

                match event {
                    OBSEvent::SceneChanged { scene_name } => {
                        if targets.contains(&SyncTargetType::Program) {
                            let payload = serde_json::json!({
                                "scene_name": scene_name
                            });
                            let msg = SyncMessage::new(
                                SyncMessageType::SceneChange,
                                SyncTargetType::Program,
                                payload,
                            );
                            let _ = message_tx.send(msg);
                        }
                    }
                    OBSEvent::CurrentPreviewSceneChanged { scene_name } => {
                        if targets.contains(&SyncTargetType::Preview) {
                            let payload = serde_json::json!({
                                "scene_name": scene_name
                            });
                            let msg = SyncMessage::new(
                                SyncMessageType::SceneChange,
                                SyncTargetType::Preview,
                                payload,
                            );
                            let _ = message_tx.send(msg);
                        }
                    }
                    OBSEvent::SceneItemTransformChanged {
                        scene_name,
                        scene_item_id,
                    } => {
                        if targets.contains(&SyncTargetType::Source) {
                            let payload = serde_json::json!({
                                "scene_name": scene_name,
                                "scene_item_id": scene_item_id
                            });
                            let msg = SyncMessage::new(
                                SyncMessageType::TransformUpdate,
                                SyncTargetType::Source,
                                payload,
                            );
                            let _ = message_tx.send(msg);
                        }
                    }
                    OBSEvent::InputSettingsChanged { input_name } => {
                        if targets.contains(&SyncTargetType::Source) {
                            let obs_client_clone = obs_client.clone();
                            let message_tx_clone = message_tx.clone();
                            let input_name_clone = input_name.clone();
                            
                            // Spawn task to get image data
                            tokio::spawn(async move {
                                let client_arc = obs_client_clone.get_client_arc();
                                let client_lock = client_arc.read().await;
                                
                                if let Some(client) = client_lock.as_ref() {
                                    // Get input settings
                                    match client.inputs().settings::<serde_json::Value>(&input_name_clone).await {
                                        Ok(settings) => {
                                            let file_path = settings.settings
                                                .get("file")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            
                                            // Read and encode image if file path exists
                                            let image_data = if !file_path.is_empty() {
                                                match tokio::fs::read(file_path).await {
                                                    Ok(data) => {
                                                        let encoded = base64::Engine::encode(
                                                            &base64::engine::general_purpose::STANDARD,
                                                            &data
                                                        );
                                                        println!("Encoded image: {} ({} bytes)", file_path, data.len());
                                                        Some(encoded)
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Failed to read image: {}", e);
                                                        None
                                                    }
                                                }
                                            } else {
                                                None
                                            };
                                            
                                            let payload = serde_json::json!({
                                                "scene_name": "",
                                                "source_name": input_name_clone,
                                                "file": file_path,
                                                "image_data": image_data
                                            });
                                            
                                            let msg = SyncMessage::new(
                                                SyncMessageType::ImageUpdate,
                                                SyncTargetType::Source,
                                                payload,
                                            );
                                            let _ = message_tx_clone.send(msg);
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to get input settings: {}", e);
                                        }
                                    }
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    pub fn send_heartbeat(&self) -> Result<()> {
        self.message_tx.send(SyncMessage::heartbeat())?;
        Ok(())
    }

    /// Read image file and encode to base64
    async fn read_and_encode_image(file_path: &str) -> Option<String> {
        match tokio::fs::read(file_path).await {
            Ok(data) => {
                let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                println!("Encoded image: {} ({} bytes -> {} chars)", file_path, data.len(), encoded.len());
                Some(encoded)
            }
            Err(e) => {
                eprintln!("Failed to read image file {}: {}", file_path, e);
                None
            }
        }
    }

    /// Get image source settings from OBS and encode the file
    pub async fn get_image_data_for_source(
        &self,
        input_name: &str,
    ) -> Option<(String, String)> {
        let client_arc = self.obs_client.get_client_arc();
        let client_lock = client_arc.read().await;
        
        if let Some(client) = client_lock.as_ref() {
            // Get input settings to find the file path
            match client.inputs().settings::<serde_json::Value>(input_name).await {
                Ok(settings) => {
                    // Try to get file path from settings
                    if let Some(file_path) = settings.settings.get("file").and_then(|v| v.as_str()) {
                        println!("Found image file for {}: {}", input_name, file_path);
                        
                        // Read and encode the image
                        if let Some(encoded_data) = Self::read_and_encode_image(file_path).await {
                            return Some((file_path.to_string(), encoded_data));
                        }
                    } else {
                        println!("No file path found in settings for {}", input_name);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to get settings for {}: {}", input_name, e);
                }
            }
        }
        
        None
    }

    /// Send initial state to newly connected slave
    pub async fn send_initial_state(&self) -> Result<()> {
        let client_arc = self.obs_client.get_client_arc();
        let client_lock = client_arc.read().await;
        
        if let Some(client) = client_lock.as_ref() {
            // Get current program scene
            let current_scene = match client.scenes().current_program_scene().await {
                Ok(scene) => scene,
                Err(e) => {
                    eprintln!("Failed to get current scene: {}", e);
                    return Ok(());
                }
            };

            // Get preview scene if in studio mode
            let preview_scene = client.scenes().current_preview_scene().await.ok();

            // Create initial state payload
            let payload = serde_json::json!({
                "current_program_scene": current_scene,
                "current_preview_scene": preview_scene,
                "scenes": []  // TODO: Get all scenes and sources
            });

            let msg = SyncMessage::new(
                SyncMessageType::StateSync,
                SyncTargetType::Program,
                payload,
            );

            self.message_tx.send(msg)?;
            println!("Sent initial state to slave");
        }

        Ok(())
    }
}
