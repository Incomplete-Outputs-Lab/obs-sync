use anyhow::{Context, Result};
use obws::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneItemTransform {
    pub position_x: f64,
    pub position_y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct OBSCommands;

impl OBSCommands {
    pub async fn get_current_program_scene(client: &Client) -> Result<String> {
        let scene = client
            .scenes()
            .current_program_scene()
            .await
            .context("Failed to get current program scene")?;
        Ok(scene)
    }

    pub async fn get_current_preview_scene(client: &Client) -> Result<Option<String>> {
        match client.scenes().current_preview_scene().await {
            Ok(scene) => Ok(Some(scene)),
            Err(_) => Ok(None),
        }
    }

    pub async fn set_current_program_scene(client: &Client, scene_name: &str) -> Result<()> {
        client
            .scenes()
            .set_current_program_scene(scene_name)
            .await
            .context("Failed to set current program scene")?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn set_scene_item_transform(
        _client: &Client,
        _scene_name: &str,
        _scene_item_id: i64,
        _transform: SceneItemTransform,
    ) -> Result<()> {
        // Transform setting would be implemented based on the actual obws API
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_scene_item_list(_client: &Client, _scene_name: &str) -> Result<Vec<Value>> {
        // Scene item list retrieval would be implemented based on the actual obws API
        Ok(vec![])
    }

    #[allow(dead_code)]
    pub async fn set_input_settings(
        _client: &Client,
        _input_name: &str,
        _settings: &Value,
    ) -> Result<()> {
        // Input settings would be implemented based on the actual obws API
        Ok(())
    }
}
