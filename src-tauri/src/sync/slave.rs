use super::diff::{DiffDetector, DiffSeverity};
use super::protocol::{
    SourceUpdateAction, SourceUpdatePayload, SyncMessage, SyncMessageType, SyncTargetType,
};
use crate::obs::{commands::OBSCommands, OBSClient};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesyncAlert {
    pub id: String,
    pub timestamp: i64,
    pub scene_name: String,
    pub source_name: String,
    pub message: String,
    pub severity: AlertSeverity,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Warning,
    Error,
}

pub struct SlaveSync {
    obs_client: Arc<OBSClient>,
    alert_tx: mpsc::UnboundedSender<DesyncAlert>,
    expected_state: Arc<RwLock<serde_json::Value>>,
    state_report_tx: Arc<RwLock<Option<mpsc::UnboundedSender<SyncMessage>>>>,
}

impl SlaveSync {
    pub fn new(obs_client: Arc<OBSClient>) -> (Self, mpsc::UnboundedReceiver<DesyncAlert>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                obs_client,
                alert_tx: tx,
                expected_state: Arc::new(RwLock::new(serde_json::json!({}))),
                state_report_tx: Arc::new(RwLock::new(None)),
            },
            rx,
        )
    }

    pub async fn set_state_report_sender(&self, tx: mpsc::UnboundedSender<SyncMessage>) {
        *self.state_report_tx.write().await = Some(tx);
    }

    /// Start periodic state checking task
    pub fn start_periodic_check(&self, interval_secs: u64) {
        let obs_client = self.obs_client.clone();
        let expected_state = self.expected_state.clone();
        let alert_tx = self.alert_tx.clone();
        let state_report_tx = self.state_report_tx.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                // Get current local OBS state
                let local_state = match Self::get_current_obs_state(&obs_client).await {
                    Ok(state) => state,
                    Err(e) => {
                        eprintln!("Failed to get local OBS state: {}", e);
                        continue;
                    }
                };

                // Compare with expected state
                let expected = expected_state.read().await;
                if expected.is_null() || expected.as_object().map(|o| o.is_empty()).unwrap_or(true)
                {
                    // No expected state yet, skip check
                    continue;
                }

                let diffs = DiffDetector::detect_differences(&local_state, &expected);

                // Send state report to Master
                {
                    let tx = state_report_tx.read().await;
                    if let Some(sender) = tx.as_ref() {
                        let desync_details: Vec<serde_json::Value> = diffs
                            .iter()
                            .map(|diff| {
                                serde_json::json!({
                                    "category": format!("{:?}", diff.category),
                                    "scene_name": diff.scene_name,
                                    "source_name": diff.source_name,
                                    "description": diff.description,
                                    "severity": format!("{:?}", diff.severity),
                                })
                            })
                            .collect();

                        let report = SyncMessage::new(
                            SyncMessageType::StateReport,
                            SyncTargetType::Program,
                            serde_json::json!({
                                "is_synced": diffs.is_empty(),
                                "desync_details": desync_details,
                                "current_state": local_state,
                            }),
                        );

                        if let Err(e) = sender.send(report) {
                            eprintln!("Failed to send state report: {}", e);
                        }
                    }
                }

                if !diffs.is_empty() {
                    println!("⚠️  Detected {} state difference(s)", diffs.len());

                    for diff in diffs {
                        let severity = match diff.severity {
                            DiffSeverity::Critical => AlertSeverity::Error,
                            _ => AlertSeverity::Warning,
                        };

                        let alert = DesyncAlert {
                            id: uuid::Uuid::new_v4().to_string(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            scene_name: diff.scene_name,
                            source_name: diff.source_name,
                            message: diff.description,
                            severity,
                        };

                        if let Err(e) = alert_tx.send(alert) {
                            eprintln!("Failed to send desync alert: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Get current OBS state for comparison
    async fn get_current_obs_state(obs_client: &Arc<OBSClient>) -> Result<serde_json::Value> {
        let client_arc = obs_client.get_client_arc();
        let client_lock = client_arc.read().await;

        if let Some(client) = client_lock.as_ref() {
            // Get current scene
            let current_scene = client
                .scenes()
                .current_program_scene()
                .await
                .context("Failed to get current scene")?;

            // Convert CurrentProgramScene to SceneId
            // CurrentProgramScene has a scene_name field that can be converted to SceneId
            let scene_name = format!("{:?}", current_scene);
            let scene_id: obws::requests::scenes::SceneId = scene_name.as_str().into();

            // Get sources in current scene
            let items = client
                .scene_items()
                .list(scene_id)
                .await
                .context("Failed to get scene items")?;

            let mut sources = Vec::new();
            for item in items {
                let transform = client.scene_items().transform(scene_id, item.id).await.ok();

                sources.push(serde_json::json!({
                    "name": item.source_name,
                    "transform": transform.map(|t| serde_json::json!({
                        "position_x": t.position_x,
                        "position_y": t.position_y,
                        "scale_x": t.scale_x,
                        "scale_y": t.scale_y,
                        "rotation": t.rotation,
                    })),
                }));
            }

            Ok(serde_json::json!({
                "current_scene": format!("{:?}", current_scene),
                "sources": sources,
            }))
        } else {
            Err(anyhow::anyhow!("OBS client not connected"))
        }
    }

    /// Update expected state from sync message
    async fn update_expected_state(&self, message: &SyncMessage) {
        let mut expected = self.expected_state.write().await;

        match message.message_type {
            SyncMessageType::SceneChange => {
                if let Some(scene_name) = message.payload["scene_name"].as_str() {
                    expected["current_scene"] = serde_json::json!(scene_name);
                }
            }
            SyncMessageType::StateSync => {
                // Full state update
                if let Some(current_scene) = message.payload["current_program_scene"].as_str() {
                    expected["current_scene"] = serde_json::json!(current_scene);
                }
                // Could expand to include full scene data
            }
            _ => {}
        }
    }

    pub async fn apply_sync_message(&self, message: SyncMessage) -> Result<()> {
        // Update expected state first
        self.update_expected_state(&message).await;

        let client_arc = self.obs_client.get_client_arc();
        let client_lock = client_arc.read().await;
        let client = client_lock.as_ref().context("OBS client not connected")?;

        match message.message_type {
            SyncMessageType::SceneChange => {
                let scene_name = message.payload["scene_name"]
                    .as_str()
                    .context("Invalid scene_name in payload")?;

                if let Err(e) = OBSCommands::set_current_program_scene(client, scene_name).await {
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

                // Apply transform if included in payload
                if let Some(transform) = message.payload["transform"].as_object() {
                    if let Err(e) = self
                        .apply_transform(client, scene_name, scene_item_id, transform)
                        .await
                    {
                        self.send_alert(
                            scene_name.to_string(),
                            String::new(),
                            format!("Failed to update transform: {}", e),
                            AlertSeverity::Warning,
                        )?;
                    } else {
                        println!(
                            "Applied transform update for item {} in scene {}",
                            scene_item_id, scene_name
                        );
                    }
                } else {
                    eprintln!("Transform data missing in payload");
                }
            }
            SyncMessageType::ImageUpdate => {
                let source_name = message.payload["source_name"]
                    .as_str()
                    .context("Invalid source_name")?;
                let file_path = message.payload["file"].as_str().unwrap_or("");
                let image_data = message.payload["image_data"].as_str();

                // Handle image update
                if let Err(e) = self
                    .handle_image_update(client, source_name, file_path, image_data)
                    .await
                {
                    self.send_alert(
                        String::new(),
                        source_name.to_string(),
                        format!("Failed to update image: {}", e),
                        AlertSeverity::Warning,
                    )?;
                }
            }
            SyncMessageType::FilterUpdate => {
                let source_name = message.payload["source_name"]
                    .as_str()
                    .context("Invalid source_name")?;
                let filter_name = message.payload["filter_name"]
                    .as_str()
                    .context("Invalid filter_name")?;

                // Get filter settings from payload
                if let Some(filter_settings) = message.payload["filter_settings"].as_object() {
                    if let Err(e) = self
                        .apply_filter_settings(client, source_name, filter_name, filter_settings)
                        .await
                    {
                        self.send_alert(
                            String::new(),
                            source_name.to_string(),
                            format!("Failed to update filter {}: {}", filter_name, e),
                            AlertSeverity::Warning,
                        )?;
                    } else {
                        println!(
                            "Applied filter update for {} on source {}",
                            filter_name, source_name
                        );
                    }
                } else {
                    eprintln!("Filter settings missing in payload");
                }
            }
            SyncMessageType::SourceUpdate => {
                // Parse SourceUpdatePayload from JSON
                let payload: SourceUpdatePayload = serde_json::from_value(message.payload.clone())
                    .context("Failed to parse SourceUpdatePayload")?;

                match payload.action {
                    SourceUpdateAction::Created => {
                        // Create scene item
                        match OBSCommands::create_scene_item(
                            client,
                            &payload.scene_name,
                            &payload.source_name,
                            payload.scene_item_enabled,
                        )
                        .await
                        {
                            Ok(new_item_id) => {
                                println!(
                                    "Created scene item {} (id: {}) in scene {}",
                                    payload.source_name, new_item_id, payload.scene_name
                                );

                                // Apply transform if provided
                                if let Some(transform) = payload.transform {
                                    let transform_map = serde_json::json!({
                                        "position_x": transform.position_x,
                                        "position_y": transform.position_y,
                                        "rotation": transform.rotation,
                                        "scale_x": transform.scale_x,
                                        "scale_y": transform.scale_y,
                                        "width": transform.width,
                                        "height": transform.height,
                                    });

                                    if let Some(transform_obj) = transform_map.as_object() {
                                        if let Err(e) = self
                                            .apply_transform(
                                                client,
                                                &payload.scene_name,
                                                new_item_id,
                                                transform_obj,
                                            )
                                            .await
                                        {
                                            eprintln!(
                                                "Failed to apply transform for newly created item {}: {}",
                                                new_item_id, e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                self.send_alert(
                                    payload.scene_name.clone(),
                                    payload.source_name.clone(),
                                    format!("Failed to create scene item: {}", e),
                                    AlertSeverity::Warning,
                                )?;
                            }
                        }
                    }
                    SourceUpdateAction::Removed => {
                        // Remove scene item
                        if let Err(e) = OBSCommands::remove_scene_item(
                            client,
                            &payload.scene_name,
                            payload.scene_item_id,
                        )
                        .await
                        {
                            self.send_alert(
                                payload.scene_name.clone(),
                                payload.source_name.clone(),
                                format!("Failed to remove scene item: {}", e),
                                AlertSeverity::Warning,
                            )?;
                        } else {
                            println!(
                                "Removed scene item {} (id: {}) from scene {}",
                                payload.source_name, payload.scene_item_id, payload.scene_name
                            );
                        }
                    }
                    SourceUpdateAction::EnabledStateChanged => {
                        // Update enabled state
                        if let Some(enabled) = payload.scene_item_enabled {
                            if let Err(e) = OBSCommands::set_scene_item_enabled(
                                client,
                                &payload.scene_name,
                                payload.scene_item_id,
                                enabled,
                            )
                            .await
                            {
                                self.send_alert(
                                    payload.scene_name.clone(),
                                    payload.source_name.clone(),
                                    format!("Failed to set scene item enabled state: {}", e),
                                    AlertSeverity::Warning,
                                )?;
                            } else {
                                println!(
                                    "Set scene item {} (id: {}) enabled state to {} in scene {}",
                                    payload.source_name,
                                    payload.scene_item_id,
                                    enabled,
                                    payload.scene_name
                                );
                            }
                        }
                    }
                    SourceUpdateAction::SettingsChanged => {
                        // Settings changed - similar to InputSettingsChanged, this might be handled elsewhere
                        // For now, just log it
                        println!(
                            "Received settings changed for scene item {} (id: {}) in scene {}",
                            payload.source_name, payload.scene_item_id, payload.scene_name
                        );
                    }
                }
            }
            SyncMessageType::Heartbeat => {
                // Just acknowledge heartbeat
            }
            SyncMessageType::StateSync => {
                println!("Applying complete initial state from master...");

                // Apply all scenes and items
                if let Some(scenes) = message.payload["scenes"].as_array() {
                    for scene in scenes {
                        let scene_name = scene["name"].as_str().unwrap_or("");
                        println!("Processing scene: {}", scene_name);

                        // Apply items in this scene
                        if let Some(items) = scene["items"].as_array() {
                            for item in items {
                                let source_name = item["source_name"].as_str().unwrap_or("");
                                let scene_item_id = item["scene_item_id"].as_i64().unwrap_or(0);

                                println!(
                                    "  - Applying item: {} (id: {})",
                                    source_name, scene_item_id
                                );

                                // Apply transform if available
                                if let Some(transform) = item["transform"].as_object() {
                                    if let Err(e) = self
                                        .apply_transform(
                                            client,
                                            scene_name,
                                            scene_item_id,
                                            transform,
                                        )
                                        .await
                                    {
                                        eprintln!(
                                            "Failed to apply transform for {}: {}",
                                            source_name, e
                                        );
                                    }
                                }

                                // Apply image data if available
                                if let Some(image_data) = item["image_data"].as_object() {
                                    if let (Some(file), Some(data)) = (
                                        image_data.get("file").and_then(|v| v.as_str()),
                                        image_data.get("data").and_then(|v| v.as_str()),
                                    ) {
                                        if let Err(e) = self
                                            .handle_image_update(
                                                client,
                                                source_name,
                                                file,
                                                Some(data),
                                            )
                                            .await
                                        {
                                            eprintln!(
                                                "Failed to apply image for {}: {}",
                                                source_name, e
                                            );
                                        }
                                    }
                                }

                                // Apply filters if available
                                if let Some(filters) = item["filters"].as_array() {
                                    for filter in filters {
                                        let filter_name = filter["name"].as_str().unwrap_or("");
                                        let filter_enabled =
                                            filter["enabled"].as_bool().unwrap_or(true);
                                        if let Some(filter_settings) =
                                            filter["settings"].as_object()
                                        {
                                            // Apply filter settings
                                            if let Err(e) = self
                                                .apply_filter_settings(
                                                    client,
                                                    source_name,
                                                    filter_name,
                                                    filter_settings,
                                                )
                                                .await
                                            {
                                                eprintln!(
                                                    "Failed to apply filter {} for {}: {}",
                                                    filter_name, source_name, e
                                                );
                                            } else {
                                                // Set filter enabled state
                                                if let Err(e) = client
                                                    .filters()
                                                    .set_enabled(obws::requests::filters::SetEnabled {
                                                        source: obws::requests::sources::SourceId::Name(source_name),
                                                        filter: filter_name,
                                                        enabled: filter_enabled,
                                                    })
                                                    .await
                                                {
                                                    eprintln!(
                                                        "Failed to set filter {} enabled state for {}: {}",
                                                        filter_name, source_name, e
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Apply current program scene
                if let Some(scene_name) = message.payload["current_program_scene"].as_str() {
                    if let Err(e) = crate::obs::commands::OBSCommands::set_current_program_scene(
                        client, scene_name,
                    )
                    .await
                    {
                        self.send_alert(
                            scene_name.to_string(),
                            String::new(),
                            format!("Failed to sync initial scene: {}", e),
                            AlertSeverity::Warning,
                        )?;
                    } else {
                        println!("✓ Applied current program scene: {}", scene_name);
                    }
                }

                // Apply preview scene if in studio mode
                if let Some(preview_scene) = message.payload["current_preview_scene"].as_str() {
                    // Setting preview scene requires studio mode to be enabled
                    match client
                        .scenes()
                        .set_current_preview_scene(preview_scene)
                        .await
                    {
                        Ok(_) => {
                            println!("✓ Applied current preview scene: {}", preview_scene);
                        }
                        Err(e) => {
                            // Studio mode might not be enabled, log warning but don't fail
                            println!("⚠️  Failed to set preview scene (Studio Mode may not be enabled): {}", e);
                            self.send_alert(
                                preview_scene.to_string(),
                                String::new(),
                                format!("Failed to sync preview scene: {} (Studio Mode may not be enabled)", e),
                                AlertSeverity::Warning,
                            )?;
                        }
                    }
                }

                println!("✓ Initial state fully applied");
            }
            _ => {}
        }

        Ok(())
    }

    async fn apply_transform(
        &self,
        client: &obws::Client,
        scene_name: &str,
        scene_item_id: i64,
        transform: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        // Convert scene_name to SceneId
        let scene_id: obws::requests::scenes::SceneId = scene_name.into();

        // Get current transform to preserve values not in the update
        let current_transform = match client
            .scene_items()
            .transform(scene_id, scene_item_id)
            .await
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to get current transform for item {}: {}",
                    scene_item_id, e
                );
                return Err(anyhow::anyhow!("Failed to get current transform: {}", e));
            }
        };

        // Extract values from JSON payload (convert to f32 to match SceneItemTransform)
        let position_x = transform
            .get("position_x")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(current_transform.position_x);
        let position_y = transform
            .get("position_y")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(current_transform.position_y);
        let scale_x = transform
            .get("scale_x")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(current_transform.scale_x);
        let scale_y = transform
            .get("scale_y")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(current_transform.scale_y);
        let rotation = transform
            .get("rotation")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(current_transform.rotation);

        // Build new transform by updating current transform
        let mut new_transform = current_transform;
        new_transform.position_x = position_x;
        new_transform.position_y = position_y;
        new_transform.scale_x = scale_x;
        new_transform.scale_y = scale_y;
        new_transform.rotation = rotation;

        // Apply the transform using SetTransform
        use obws::requests::scene_items::SetTransform;
        let set_transform = SetTransform {
            scene: scene_id,
            item_id: scene_item_id,
            transform: new_transform.into(),
        };
        client
            .scene_items()
            .set_transform(set_transform)
            .await
            .context("Failed to set transform")?;

        println!(
            "Applied transform for item {} in scene {}: pos=({}, {}), scale=({}, {}), rotation={}",
            scene_item_id, scene_name, position_x, position_y, scale_x, scale_y, rotation
        );

        Ok(())
    }

    async fn handle_image_update(
        &self,
        client: &obws::Client,
        source_name: &str,
        original_file_path: &str,
        image_data: Option<&str>,
    ) -> Result<()> {
        if let Some(encoded_data) = image_data {
            println!("Received image data for {}, decoding...", source_name);

            // Decode base64 image data
            let decoded_data =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded_data)
                    .context("Failed to decode image data")?;

            println!("Decoded {} bytes of image data", decoded_data.len());

            // Extract file extension from original file path
            // Fall back to magic bytes detection if extension cannot be determined
            let file_extension = if !original_file_path.is_empty() {
                std::path::Path::new(original_file_path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or_else(|| Self::detect_image_format(&decoded_data))
            } else {
                Self::detect_image_format(&decoded_data)
            };

            // Create temp directory for synced images
            let temp_dir = std::env::temp_dir().join("obs-sync");
            fs::create_dir_all(&temp_dir)
                .await
                .context("Failed to create temp directory")?;

            // Generate unique filename using original file name if available
            let temp_file_path = if !original_file_path.is_empty() {
                // Extract file name (without path) from original path
                let original_file_name = std::path::Path::new(original_file_path)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or(source_name);

                temp_dir.join(format!(
                    "{}_{}.{}",
                    original_file_name.replace("/", "_").replace("\\", "_"),
                    chrono::Utc::now().timestamp_millis(),
                    file_extension
                ))
            } else {
                temp_dir.join(format!(
                    "{}_{}.{}",
                    source_name.replace("/", "_").replace("\\", "_"),
                    chrono::Utc::now().timestamp_millis(),
                    file_extension
                ))
            };

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
            match client
                .inputs()
                .set_settings(obws::requests::inputs::SetSettings {
                    input: obws::requests::inputs::InputId::Name(source_name),
                    settings: &settings,
                    overlay: Some(true),
                })
                .await
            {
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

    /// Detect image format from magic bytes
    fn detect_image_format(data: &[u8]) -> &'static str {
        if data.len() < 4 {
            return "png"; // Default to PNG if data is too short
        }

        // Check magic bytes for common image formats
        match &data[0..4] {
            [0x89, 0x50, 0x4E, 0x47] => "png", // PNG: 89 50 4E 47
            [0xFF, 0xD8, 0xFF, _] => "jpg",    // JPEG: FF D8 FF
            [0x47, 0x49, 0x46, 0x38] => "gif", // GIF: 47 49 46 38
            [0x42, 0x4D, _, _] => "bmp",       // BMP: 42 4D
            [0x52, 0x49, 0x46, 0x46] => {
                // RIFF (WebP or other)
                if data.len() >= 8 && &data[4..8] == b"WEBP" {
                    "webp"
                } else {
                    "png" // Default fallback
                }
            }
            _ => "png", // Default to PNG if format is unknown
        }
    }

    async fn apply_filter_settings(
        &self,
        client: &obws::Client,
        source_name: &str,
        filter_name: &str,
        filter_settings: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        // Convert JSON map to Value for settings
        let settings: serde_json::Value = serde_json::json!(filter_settings);

        // Apply filter settings using OBS API
        client
            .filters()
            .set_settings(obws::requests::filters::SetSettings {
                source: obws::requests::sources::SourceId::Name(source_name),
                filter: filter_name,
                settings: &settings,
                overlay: Some(true),
            })
            .await
            .context("Failed to set filter settings")?;

        println!(
            "Applied filter settings for {} on source {}",
            filter_name, source_name
        );

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
