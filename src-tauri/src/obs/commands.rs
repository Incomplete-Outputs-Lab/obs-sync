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
}
