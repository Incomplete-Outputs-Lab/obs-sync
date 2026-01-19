use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncMessageType {
    SourceUpdate,
    TransformUpdate,
    SceneChange,
    ImageUpdate,
    FilterUpdate,
    Heartbeat,
    StateSync,
    StateSyncRequest, // Slave requests initial state from Master
    StateReport,      // Slave reports its current state to Master
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncTargetType {
    Source,
    Preview,
    Program,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    #[serde(rename = "type")]
    pub message_type: SyncMessageType,
    pub timestamp: i64,
    pub target_type: SyncTargetType,
    pub payload: Value,
}

impl SyncMessage {
    pub fn new(message_type: SyncMessageType, target_type: SyncTargetType, payload: Value) -> Self {
        Self {
            message_type,
            timestamp: chrono::Utc::now().timestamp_millis(),
            target_type,
            payload,
        }
    }

    pub fn state_sync_request() -> Self {
        Self::new(
            SyncMessageType::StateSyncRequest,
            SyncTargetType::Program,
            Value::Object(serde_json::Map::new()),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TransformUpdatePayload {
    pub scene_name: String,
    pub scene_item_id: i64,
    pub transform: TransformData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TransformData {
    pub position_x: f64,
    pub position_y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SceneChangePayload {
    pub scene_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ImageUpdatePayload {
    pub scene_name: String,
    pub source_name: String,
    pub file: String,
    /// Base64 encoded image data
    pub image_data: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct StateSyncPayload {
    pub current_program_scene: String,
    pub current_preview_scene: Option<String>,
    pub scenes: Vec<SceneData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SceneData {
    pub name: String,
    pub items: Vec<SceneItemData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SceneItemData {
    pub source_name: String,
    pub source_type: String,
    /// Base64 encoded image data for image sources
    pub image_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceUpdateAction {
    Created,
    Removed,
    EnabledStateChanged,
    SettingsChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceUpdatePayload {
    pub scene_name: String,
    pub scene_item_id: i64,
    pub source_name: String,
    pub action: SourceUpdateAction,
    pub source_type: Option<String>,
    pub scene_item_enabled: Option<bool>,
    pub transform: Option<TransformData>,
}
