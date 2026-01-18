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
        let events = client
            .events()
            .map_err(|e| anyhow::anyhow!("Failed to get event stream: {}", e))?;

        println!("Started OBS event listening");

        // Spawn task to process events
        tokio::spawn(async move {
            tokio::pin!(events);
            while let Some(event) = events.next().await {
                match event {
                    Event::CurrentProgramSceneChanged { name } => {
                        let obs_event = OBSEvent::SceneChanged {
                            scene_name: name,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send SceneChanged event: {}", e);
                            break;
                        }
                    }
                    Event::CurrentPreviewSceneChanged { name } => {
                        let obs_event = OBSEvent::CurrentPreviewSceneChanged {
                            scene_name: name,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send CurrentPreviewSceneChanged event: {}", e);
                            break;
                        }
                    }
                    Event::SceneItemTransformChanged {
                        scene,
                        item_id,
                        ..
                    } => {
                        let obs_event = OBSEvent::SceneItemTransformChanged {
                            scene_name: scene,
                            scene_item_id: item_id as i64,
                        };
                        if let Err(e) = tx.send(obs_event) {
                            eprintln!("Failed to send SceneItemTransformChanged event: {}", e);
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
