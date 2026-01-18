use futures_util::StreamExt;
use obws::events::Event;
use obws::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum OBSEvent {
    SceneChanged {
        scene_name: String,
    },
    SceneItemTransformChanged {
        scene_name: String,
        scene_item_id: i64,
    },
    SourceCreated {
        source_name: String,
    },
    SourceDestroyed {
        source_name: String,
    },
    InputSettingsChanged {
        input_name: String,
    },
    CurrentPreviewSceneChanged {
        scene_name: String,
    },
}

pub struct OBSEventHandler {
    event_tx: mpsc::UnboundedSender<OBSEvent>,
}

impl OBSEventHandler {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<OBSEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { event_tx: tx }, rx)
    }

    pub async fn start_listening(&self, client: &Client) -> anyhow::Result<()> {
        let tx = self.event_tx.clone();

        // Get event stream from obws client
        let mut events = client
            .events()
            .map_err(|e| anyhow::anyhow!("Failed to get event stream: {}", e))?;

        println!("Started OBS event listening");

        // Spawn task to process events
        tokio::spawn(async move {
            while let Some(event) = events.next().await {
                match event {
                    Event::CurrentProgramSceneChanged(data) => {
                        let obs_event = OBSEvent::SceneChanged {
                            scene_name: data.scene_name,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send SceneChanged event: {}", e);
                            break;
                        }
                    }
                    Event::CurrentPreviewSceneChanged(data) => {
                        let obs_event = OBSEvent::CurrentPreviewSceneChanged {
                            scene_name: data.scene_name,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send CurrentPreviewSceneChanged event: {}", e);
                            break;
                        }
                    }
                    Event::SceneItemTransformChanged(data) => {
                        let obs_event = OBSEvent::SceneItemTransformChanged {
                            scene_name: data.scene_name,
                            scene_item_id: data.scene_item_id,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send SceneItemTransformChanged event: {}", e);
                            break;
                        }
                    }
                    Event::InputSettingsChanged(data) => {
                        let obs_event = OBSEvent::InputSettingsChanged {
                            input_name: data.input_name,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send InputSettingsChanged event: {}", e);
                            break;
                        }
                    }
                    Event::SourceCreated(data) => {
                        let obs_event = OBSEvent::SourceCreated {
                            source_name: data.source_name,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send SourceCreated event: {}", e);
                            break;
                        }
                    }
                    Event::SourceDestroyed(data) => {
                        let obs_event = OBSEvent::SourceDestroyed {
                            source_name: data.source_name,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send SourceDestroyed event: {}", e);
                            break;
                        }
                    }
                    _ => {
                        // Ignore other events
                    }
                }
            }
            println!("OBS event stream ended");
        });

        Ok(())
    }
}

impl Default for OBSEventHandler {
    fn default() -> Self {
        Self::new().0
    }
}
