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
                            let payload = serde_json::json!({
                                "input_name": input_name
                            });
                            let msg = SyncMessage::new(
                                SyncMessageType::ImageUpdate,
                                SyncTargetType::Source,
                                payload,
                            );
                            let _ = message_tx.send(msg);
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
