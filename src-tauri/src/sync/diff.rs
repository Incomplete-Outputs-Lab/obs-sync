use serde_json::Value;

#[derive(Debug, Clone)]
pub struct StateDifference {
    pub category: DiffCategory,
    pub scene_name: String,
    pub source_name: String,
    pub description: String,
    pub severity: DiffSeverity,
}

#[derive(Debug, Clone)]
pub enum DiffCategory {
    SceneMismatch,
    SourceMissing,
    TransformMismatch,
    SettingsMismatch,
}

#[derive(Debug, Clone)]
pub enum DiffSeverity {
    Critical, // Scene doesn't match
    Warning,  // Transform or settings differ
    Info,     // Minor differences
}

pub struct DiffDetector;

impl DiffDetector {
    const TRANSFORM_TOLERANCE: f64 = 0.5; // Tolerance for position/scale differences

    pub fn detect_differences(local_state: &Value, expected_state: &Value) -> Vec<StateDifference> {
        let mut diffs = Vec::new();

        // Compare current scene
        let local_scene = local_state
            .get("current_scene")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let expected_scene = expected_state
            .get("current_scene")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if local_scene != expected_scene && !expected_scene.is_empty() {
            diffs.push(StateDifference {
                category: DiffCategory::SceneMismatch,
                scene_name: expected_scene.to_string(),
                source_name: String::new(),
                description: format!(
                    "Current scene mismatch: local='{}', expected='{}'",
                    local_scene, expected_scene
                ),
                severity: DiffSeverity::Critical,
            });
        }

        // Compare sources in current scene
        if let (Some(local_sources), Some(expected_sources)) = (
            local_state.get("sources").and_then(|v| v.as_array()),
            expected_state.get("sources").and_then(|v| v.as_array()),
        ) {
            // Check for missing sources
            for expected_source in expected_sources {
                let expected_name = expected_source
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !local_sources
                    .iter()
                    .any(|s| s.get("name").and_then(|v| v.as_str()).unwrap_or("") == expected_name)
                {
                    diffs.push(StateDifference {
                        category: DiffCategory::SourceMissing,
                        scene_name: local_scene.to_string(),
                        source_name: expected_name.to_string(),
                        description: format!("Source '{}' is missing", expected_name),
                        severity: DiffSeverity::Warning,
                    });
                } else {
                    // Source exists, check transform
                    if let Some(local_source) = local_sources.iter().find(|s| {
                        s.get("name").and_then(|v| v.as_str()).unwrap_or("") == expected_name
                    }) {
                        if let Some(transform_diffs) = Self::compare_transforms(
                            local_source,
                            expected_source,
                            local_scene,
                            expected_name,
                        ) {
                            diffs.extend(transform_diffs);
                        }
                    }
                }
            }
        }

        diffs
    }

    fn compare_transforms(
        local_source: &Value,
        expected_source: &Value,
        scene_name: &str,
        source_name: &str,
    ) -> Option<Vec<StateDifference>> {
        let local_transform = local_source.get("transform")?;
        let expected_transform = expected_source.get("transform")?;

        let mut diffs = Vec::new();

        // Compare position
        let local_x = local_transform
            .get("position_x")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let expected_x = expected_transform
            .get("position_x")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let local_y = local_transform
            .get("position_y")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let expected_y = expected_transform
            .get("position_y")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        if (local_x - expected_x).abs() > Self::TRANSFORM_TOLERANCE
            || (local_y - expected_y).abs() > Self::TRANSFORM_TOLERANCE
        {
            diffs.push(StateDifference {
                category: DiffCategory::TransformMismatch,
                scene_name: scene_name.to_string(),
                source_name: source_name.to_string(),
                description: format!(
                    "Position mismatch: local=({:.1}, {:.1}), expected=({:.1}, {:.1})",
                    local_x, local_y, expected_x, expected_y
                ),
                severity: DiffSeverity::Warning,
            });
        }

        // Compare scale
        let local_scale_x = local_transform
            .get("scale_x")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let expected_scale_x = expected_transform
            .get("scale_x")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let local_scale_y = local_transform
            .get("scale_y")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let expected_scale_y = expected_transform
            .get("scale_y")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        if (local_scale_x - expected_scale_x).abs() > 0.01
            || (local_scale_y - expected_scale_y).abs() > 0.01
        {
            diffs.push(StateDifference {
                category: DiffCategory::TransformMismatch,
                scene_name: scene_name.to_string(),
                source_name: source_name.to_string(),
                description: format!(
                    "Scale mismatch: local=({:.2}, {:.2}), expected=({:.2}, {:.2})",
                    local_scale_x, local_scale_y, expected_scale_x, expected_scale_y
                ),
                severity: DiffSeverity::Warning,
            });
        }

        if diffs.is_empty() {
            None
        } else {
            Some(diffs)
        }
    }

    pub fn is_synced(diffs: &[StateDifference]) -> bool {
        diffs.is_empty()
    }

    pub fn has_critical_diffs(diffs: &[StateDifference]) -> bool {
        diffs
            .iter()
            .any(|d| matches!(d.severity, DiffSeverity::Critical))
    }
}
