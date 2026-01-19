use anyhow::{Context, Result};
use obws::Client;

pub struct OBSCommands;

impl OBSCommands {
    pub async fn set_current_program_scene(client: &Client, scene_name: &str) -> Result<()> {
        client
            .scenes()
            .set_current_program_scene(scene_name)
            .await
            .context("Failed to set current program scene")?;
        Ok(())
    }

    pub async fn create_scene_item(
        client: &Client,
        scene_name: &str,
        source_name: &str,
        scene_item_enabled: Option<bool>,
    ) -> Result<i64> {
        let scene_id: obws::requests::scenes::SceneId =
            obws::requests::scenes::SceneId::Name(scene_name);
        let source_id: obws::requests::sources::SourceId =
            obws::requests::sources::SourceId::Name(source_name);

        use obws::requests::scene_items::CreateSceneItem;
        let item_id = client
            .scene_items()
            .create(CreateSceneItem {
                scene: scene_id,
                source: source_id,
                enabled: scene_item_enabled,
            })
            .await
            .context("Failed to create scene item")?;

        Ok(item_id)
    }

    pub async fn remove_scene_item(
        client: &Client,
        scene_name: &str,
        scene_item_id: i64,
    ) -> Result<()> {
        let scene_id: obws::requests::scenes::SceneId =
            obws::requests::scenes::SceneId::Name(scene_name);

        client
            .scene_items()
            .remove(scene_id, scene_item_id)
            .await
            .context("Failed to remove scene item")?;

        Ok(())
    }

    pub async fn set_scene_item_enabled(
        client: &Client,
        scene_name: &str,
        scene_item_id: i64,
        enabled: bool,
    ) -> Result<()> {
        let scene_id: obws::requests::scenes::SceneId =
            obws::requests::scenes::SceneId::Name(scene_name);

        use obws::requests::scene_items::SetEnabled;
        client
            .scene_items()
            .set_enabled(SetEnabled {
                scene: scene_id,
                item_id: scene_item_id,
                enabled,
            })
            .await
            .context("Failed to set scene item enabled state")?;

        Ok(())
    }
}
