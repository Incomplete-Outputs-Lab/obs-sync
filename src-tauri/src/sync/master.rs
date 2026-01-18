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
                            // Get the full transform data
                            let obs_client_clone = obs_client.clone();
                            let message_tx_clone = message_tx.clone();
                            let scene_name_clone = scene_name.clone();

                            tokio::spawn(async move {
                                let client_arc = obs_client_clone.get_client_arc();
                                let client_lock = client_arc.read().await;

                                if let Some(client) = client_lock.as_ref() {
                                    let scene_id: obws::requests::scenes::SceneId =
                                        obws::requests::scenes::SceneId::Name(&scene_name_clone);
                                    match client
                                        .scene_items()
                                        .transform(scene_id, scene_item_id)
                                        .await
                                    {
                                        Ok(transform) => {
                                            let payload = serde_json::json!({
                                                "scene_name": scene_name_clone,
                                                "scene_item_id": scene_item_id,
                                                "transform": {
                                                    "position_x": transform.position_x,
                                                    "position_y": transform.position_y,
                                                    "rotation": transform.rotation,
                                                    "scale_x": transform.scale_x,
                                                    "scale_y": transform.scale_y,
                                                    "width": transform.width,
                                                    "height": transform.height,
                                                }
                                            });

                                            let msg = SyncMessage::new(
                                                SyncMessageType::TransformUpdate,
                                                SyncTargetType::Source,
                                                payload,
                                            );
                                            let _ = message_tx_clone.send(msg);
                                            println!(
                                                "Sent transform update for scene item {} in {}",
                                                scene_item_id, scene_name_clone
                                            );
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Failed to get transform for item {}: {}",
                                                scene_item_id, e
                                            );
                                        }
                                    }
                                }
                            });
                        }
                    }
                    OBSEvent::SceneItemFilterChanged {
                        scene_name,
                        scene_item_id,
                        filter_name,
                    } => {
                        if targets.contains(&SyncTargetType::Source) {
                            // Get filter settings and send update
                            let obs_client_clone = obs_client.clone();
                            let message_tx_clone = message_tx.clone();
                            let scene_name_clone = scene_name.clone();
                            let filter_name_clone = filter_name.clone();

                            tokio::spawn(async move {
                                let client_arc = obs_client_clone.get_client_arc();
                                let client_lock = client_arc.read().await;

                                if let Some(client) = client_lock.as_ref() {
                                    let (resolved_scene_name, resolved_scene_item_id, source_name) =
                                        if !scene_name_clone.is_empty() && scene_item_id > 0 {
                                            // scene_name and scene_item_id are already provided
                                            // Get scene items to find source name
                                            match client
                                                .scene_items()
                                                .list(obws::requests::scenes::SceneId::Name(
                                                    &scene_name_clone,
                                                ))
                                                .await
                                            {
                                                Ok(items) => {
                                                    if let Some(item) = items
                                                        .iter()
                                                        .find(|i| i.id as i64 == scene_item_id)
                                                    {
                                                        (
                                                            Some(scene_name_clone.clone()),
                                                            Some(scene_item_id),
                                                            Some(item.source_name.clone()),
                                                        )
                                                    } else {
                                                        (None, None, None)
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "Failed to get scene items for {}: {}",
                                                        scene_name_clone, e
                                                    );
                                                    (None, None, None)
                                                }
                                            }
                                        } else {
                                            // Need to search all scenes to find the source
                                            match client.scenes().list().await {
                                                Ok(scenes) => {
                                                    let mut found = None;
                                                    'outer: for scene in scenes.scenes {
                                                        let scene_id: obws::requests::scenes::SceneId = scene.id.clone().into();
                                                        match client
                                                            .scene_items()
                                                            .list(scene_id.clone())
                                                            .await
                                                        {
                                                            Ok(items) => {
                                                                for item in items {
                                                                    // Check if this source has the filter
                                                                    match client.filters().list(obws::requests::sources::SourceId::Name(&item.source_name)).await {
                                                Ok(filters) => {
                                                    if filters.iter().any(|f| f.name == filter_name_clone) {
                                                        found = Some((format!("{:?}", scene.id), item.id as i64, item.source_name.clone()));
                                                        break 'outer;
                                                    }
                                                }
                                                Err(_) => continue,
                                            }
                                                                }
                                                            }
                                                            Err(_) => continue,
                                                        }
                                                    }
                                                    if let Some((s, id, src)) = found {
                                                        (Some(s), Some(id), Some(src))
                                                    } else {
                                                        (None, None, None)
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("Failed to get scenes list for filter resolution: {}", e);
                                                    (None, None, None)
                                                }
                                            }
                                        };

                                    if let (Some(scene), Some(item_id), Some(source)) =
                                        (resolved_scene_name, resolved_scene_item_id, source_name)
                                    {
                                        // Get filter settings
                                        match client
                                            .filters()
                                            .list(obws::requests::sources::SourceId::Name(&source))
                                            .await
                                        {
                                            Ok(filters) => {
                                                if let Some(filter) = filters
                                                    .iter()
                                                    .find(|f| f.name == filter_name_clone)
                                                {
                                                    let payload = serde_json::json!({
                                                        "scene_name": scene,
                                                        "scene_item_id": item_id,
                                                        "source_name": source,
                                                        "filter_name": filter_name_clone,
                                                        "filter_settings": filter.settings
                                                    });

                                                    let msg = SyncMessage::new(
                                                        SyncMessageType::FilterUpdate,
                                                        SyncTargetType::Source,
                                                        payload,
                                                    );
                                                    let _ = message_tx_clone.send(msg);
                                                    println!(
                                                        "Sent filter update for {} on source {} in scene {} (item: {})",
                                                        filter_name_clone, source, scene, item_id
                                                    );
                                                } else {
                                                    eprintln!(
                                                        "Filter {} not found on source {}",
                                                        filter_name_clone, source
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!(
                                                    "Failed to get filter list for {}: {}",
                                                    source, e
                                                );
                                            }
                                        }
                                    } else {
                                        eprintln!("Could not resolve scene_name/scene_item_id for filter {} on source", filter_name_clone);
                                    }
                                }
                            });
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
                                    // Get input settings first to check if it's an image source
                                    match client
                                        .inputs()
                                        .settings::<serde_json::Value>(
                                            obws::requests::inputs::InputId::Name(
                                                &input_name_clone,
                                            ),
                                        )
                                        .await
                                    {
                                        Ok(settings) => {
                                            let file_path = settings
                                                .settings
                                                .get("file")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");

                                            // Only process if it has a file path (likely an image source)
                                            if file_path.is_empty() {
                                                println!(
                                                    "Skipping InputSettingsChanged for {} - no file path found",
                                                    input_name_clone
                                                );
                                                return;
                                            }

                                            println!(
                                                "Processing InputSettingsChanged for {} (file: {})",
                                                input_name_clone, file_path
                                            );

                                            // Read and encode image if file path exists
                                            let image_data = if !file_path.is_empty() {
                                                match tokio::fs::read(file_path).await {
                                                    Ok(data) => {
                                                        let encoded = base64::Engine::encode(
                                                            &base64::engine::general_purpose::STANDARD,
                                                            &data
                                                        );
                                                        println!(
                                                            "Encoded image: {} ({} bytes)",
                                                            file_path,
                                                            data.len()
                                                        );
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
                }
            }
        });
    }

    /// Read image file and encode to base64
    async fn read_and_encode_image(file_path: &str) -> Option<String> {
        match tokio::fs::read(file_path).await {
            Ok(data) => {
                let encoded =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                println!(
                    "Encoded image: {} ({} bytes -> {} chars)",
                    file_path,
                    data.len(),
                    encoded.len()
                );
                Some(encoded)
            }
            Err(e) => {
                eprintln!("Failed to read image file {}: {}", file_path, e);
                None
            }
        }
    }

    /// Get image source settings from OBS and encode the file
    pub async fn get_image_data_for_source(&self, input_name: &str) -> Option<(String, String)> {
        let client_arc = self.obs_client.get_client_arc();
        let client_lock = client_arc.read().await;

        if let Some(client) = client_lock.as_ref() {
            // Get input settings to find the file path
            match client
                .inputs()
                .settings::<serde_json::Value>(obws::requests::inputs::InputId::Name(input_name))
                .await
            {
                Ok(settings) => {
                    // Try to get file path from settings
                    if let Some(file_path) = settings.settings.get("file").and_then(|v| v.as_str())
                    {
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
        println!("Collecting full OBS state for new slave...");
        let client_arc = self.obs_client.get_client_arc();
        let client_lock = client_arc.read().await;

        if let Some(client) = client_lock.as_ref() {
            // Get current program scene
            let current_program_scene = match client.scenes().current_program_scene().await {
                Ok(scene) => scene,
                Err(e) => {
                    eprintln!("Failed to get current scene: {}", e);
                    return Ok(());
                }
            };

            // Get preview scene if in studio mode
            let current_preview_scene = client.scenes().current_preview_scene().await.ok();

            // Get all scenes
            let scenes_list = match client.scenes().list().await {
                Ok(scenes) => scenes,
                Err(e) => {
                    eprintln!("Failed to get scenes list: {}", e);
                    return Ok(());
                }
            };

            let mut scenes_data = Vec::new();

            // For each scene, get all items
            for scene in scenes_list.scenes {
                let scene_id: obws::requests::scenes::SceneId = scene.id.clone().into();
                println!("Processing scene: {:?}", scene.id);

                match client.scene_items().list(scene_id.clone()).await {
                    Ok(items) => {
                        let mut scene_items_data = Vec::new();

                        for item in items {
                            println!("  - Item: {} (id: {})", item.source_name, item.id);

                            // Get transform for this item
                            let transform = match client
                                .scene_items()
                                .transform(scene_id.clone(), item.id)
                                .await
                            {
                                Ok(t) => Some(serde_json::json!({
                                    "position_x": t.position_x,
                                    "position_y": t.position_y,
                                    "rotation": t.rotation,
                                    "scale_x": t.scale_x,
                                    "scale_y": t.scale_y,
                                    "width": t.width,
                                    "height": t.height,
                                })),
                                Err(e) => {
                                    eprintln!(
                                        "Failed to get transform for {}: {}",
                                        item.source_name, e
                                    );
                                    None
                                }
                            };

                            // Get source type from item
                            let source_type = item
                                .input_kind
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string());

                            // If it's an image source, get the image data
                            let image_data = if source_type.contains("image") {
                                self.get_image_data_for_source(&item.source_name).await.map(
                                    |(path, data)| {
                                        serde_json::json!({
                                            "file": path,
                                            "data": data
                                        })
                                    },
                                )
                            } else {
                                None
                            };

                            // Get filters for this source
                            let mut filters_data = Vec::new();
                            match client
                                .filters()
                                .list(obws::requests::sources::SourceId::Name(&item.source_name))
                                .await
                            {
                                Ok(filters) => {
                                    for filter in filters {
                                        filters_data.push(serde_json::json!({
                                            "name": filter.name,
                                            "enabled": filter.enabled,
                                            "settings": filter.settings
                                        }));
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Failed to get filters for source {}: {}",
                                        item.source_name, e
                                    );
                                }
                            }

                            scene_items_data.push(serde_json::json!({
                                "source_name": item.source_name,
                                "scene_item_id": item.id,
                                "source_type": source_type,
                                "transform": transform,
                                "image_data": image_data,
                                "filters": filters_data,
                            }));
                        }

                        // Use scene.id for name (SceneId doesn't implement Display)
                        let scene_name = format!("{:?}", scene.id);
                        scenes_data.push(serde_json::json!({
                            "name": scene_name.clone(),
                            "items": scene_items_data,
                        }));
                    }
                    Err(e) => {
                        eprintln!("Failed to get items for scene {:?}: {}", scene.id, e);
                    }
                }
            }

            // Create comprehensive initial state payload
            let payload = serde_json::json!({
                "current_program_scene": current_program_scene,
                "current_preview_scene": current_preview_scene,
                "scenes": scenes_data,
            });

            let msg =
                SyncMessage::new(SyncMessageType::StateSync, SyncTargetType::Program, payload);

            self.message_tx.send(msg)?;
            println!(
                "✓ Sent complete initial state to slave ({} scenes)",
                scenes_data.len()
            );
        }

        Ok(())
    }
}
