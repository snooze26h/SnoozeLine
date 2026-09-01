use super::{Segment, SegmentData};
use crate::config::{InputData, ModelConfig, SegmentId, TranscriptEntry};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Default)]
pub struct ContextWindowSegment;

impl ContextWindowSegment {
    pub fn new() -> Self {
        Self
    }

    fn get_context_limit_for_model(model_id: &str) -> u64 {
        let model_config = ModelConfig::load();
        u64::from(model_config.get_context_limit(model_id))
    }

    fn format_percentage(percentage: f64) -> String {
        if percentage.fract() == 0.0 {
            format!("{:.0}%", percentage)
        } else {
            format!("{:.1}%", percentage)
        }
    }

    fn format_tokens(tokens: u64) -> String {
        if tokens >= 1000 {
            let thousands = tokens as f64 / 1000.0;
            if thousands.fract() == 0.0 {
                format!("{}k", thousands as u64)
            } else {
                format!("{:.1}k", thousands)
            }
        } else {
            tokens.to_string()
        }
    }
}

impl Segment for ContextWindowSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let (tokens, percentage, context_limit, source) = match input.context_window.as_ref() {
            Some(context_window) => {
                let limit = context_window
                    .context_window_size
                    .filter(|limit| *limit > 0)
                    .unwrap_or_else(|| Self::get_context_limit_for_model(&input.model.id));
                (
                    context_window.current_input_tokens(),
                    context_window.current_percentage(),
                    limit,
                    "native",
                )
            }
            None => {
                let limit = Self::get_context_limit_for_model(&input.model.id);
                let tokens = parse_transcript_usage(&input.transcript_path).map(u64::from);
                let percentage =
                    tokens.map(|value| ((value as f64 / limit as f64) * 100.0).clamp(0.0, 100.0));
                (tokens, percentage, limit, "transcript")
            }
        };

        let percentage_display = percentage
            .map(Self::format_percentage)
            .unwrap_or_else(|| "-".to_string());
        let tokens_display = tokens
            .map(Self::format_tokens)
            .unwrap_or_else(|| "-".to_string());

        let mut metadata = HashMap::new();
        metadata.insert(
            "tokens".to_string(),
            tokens.map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
        metadata.insert(
            "percentage".to_string(),
            percentage.map_or_else(|| "-".to_string(), |value| value.to_string()),
        );
        metadata.insert("limit".to_string(), context_limit.to_string());
        metadata.insert("model".to_string(), input.model.id.clone());
        metadata.insert("source".to_string(), source.to_string());

        Some(SegmentData {
            primary: format!("{} · {} tokens", percentage_display, tokens_display),
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::ContextWindow
    }
}

fn parse_transcript_usage<P: AsRef<Path>>(transcript_path: P) -> Option<u32> {
    try_parse_transcript_file(transcript_path.as_ref())
}

fn try_parse_transcript_file(path: &Path) -> Option<u32> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    if lines.is_empty() {
        return None;
    }

    // Check if the last line is a summary
    let last_line = lines.last()?.trim();
    if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(last_line) {
        if entry.r#type.as_deref() == Some("summary") {
            // Handle summary case: find usage by leafUuid
            if let Some(leaf_uuid) = &entry.leaf_uuid {
                let project_dir = path.parent()?;
                return find_usage_by_leaf_uuid(leaf_uuid, project_dir);
            }
        }
    }

    // Normal case: find the last assistant message in current file
    for line in lines.iter().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if entry.r#type.as_deref() == Some("assistant") {
                if let Some(message) = &entry.message {
                    if message.stop_reason.is_some() {
                        if let Some(raw_usage) = &message.usage {
                            let normalized = raw_usage.clone().normalize();
                            return Some(normalized.display_tokens());
                        }
                    }
                }
            }
        }
    }

    None
}

fn find_usage_by_leaf_uuid(leaf_uuid: &str, project_dir: &Path) -> Option<u32> {
    // Search for the leafUuid across all session files in the project directory
    let entries = fs::read_dir(project_dir).ok()?;

    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        if let Some(usage) = search_uuid_in_file(&path, leaf_uuid) {
            return Some(usage);
        }
    }

    None
}

fn search_uuid_in_file(path: &Path, target_uuid: &str) -> Option<u32> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    // Find the message with target_uuid
    for line in &lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if let Some(uuid) = &entry.uuid {
                if uuid == target_uuid {
                    // Found the target message, check its type
                    if entry.r#type.as_deref() == Some("assistant") {
                        // Direct assistant message with usage
                        if let Some(message) = &entry.message {
                            if message.stop_reason.is_some() {
                                if let Some(raw_usage) = &message.usage {
                                    let normalized = raw_usage.clone().normalize();
                                    return Some(normalized.display_tokens());
                                }
                            }
                        }
                    } else if entry.r#type.as_deref() == Some("user") {
                        // User message, need to find the parent assistant message
                        if let Some(parent_uuid) = &entry.parent_uuid {
                            return find_assistant_message_by_uuid(&lines, parent_uuid);
                        }
                    }
                    break;
                }
            }
        }
    }

    None
}

fn find_assistant_message_by_uuid(lines: &[String], target_uuid: &str) -> Option<u32> {
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if let Some(uuid) = &entry.uuid {
                if uuid == target_uuid && entry.r#type.as_deref() == Some("assistant") {
                    if let Some(message) = &entry.message {
                        if message.stop_reason.is_some() {
                            if let Some(raw_usage) = &message.usage {
                                let normalized = raw_usage.clone().normalize();
                                return Some(normalized.display_tokens());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::ContextWindowSegment;
    use crate::config::{ContextWindow, CurrentContextUsage, InputData, Model, Workspace};
    use crate::core::segments::Segment;

    fn input(transcript_path: String, context_window: Option<ContextWindow>) -> InputData {
        InputData {
            cwd: None,
            model: Model {
                id: "claude-opus-5".to_string(),
                display_name: "Opus 5".to_string(),
            },
            workspace: Workspace {
                current_dir: "/tmp/project".to_string(),
            },
            transcript_path,
            cost: None,
            output_style: None,
            context_window,
            rate_limits: None,
            version: None,
        }
    }

    #[test]
    fn native_context_wins_and_excludes_output() {
        let context = ContextWindow {
            context_window_size: Some(1_000_000),
            used_percentage: Some(25.0),
            current_usage: Some(CurrentContextUsage {
                input_tokens: Some(1_000),
                output_tokens: Some(99_000),
                cache_creation_input_tokens: Some(2_000),
                cache_read_input_tokens: Some(247_000),
            }),
            ..Default::default()
        };
        let data = ContextWindowSegment::new()
            .collect(&input("/missing".to_string(), Some(context)))
            .unwrap();

        assert_eq!(data.primary, "25% · 250k tokens");
        assert_eq!(data.metadata.get("source").unwrap(), "native");
    }

    #[test]
    fn native_null_does_not_reuse_transcript_data() {
        let path = std::env::temp_dir().join(format!(
            "snoozeline-native-null-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"{"type":"assistant","message":{"stop_reason":"end_turn","usage":{"input_tokens":2,"cache_creation_input_tokens":609,"cache_read_input_tokens":874001,"output_tokens":2061}}}"#,
        )
        .unwrap();
        let context = ContextWindow {
            context_window_size: Some(1_000_000),
            ..Default::default()
        };
        let data = ContextWindowSegment::new()
            .collect(&input(path.to_string_lossy().into_owned(), Some(context)))
            .unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(data.primary, "- · - tokens");
        assert_eq!(data.metadata.get("source").unwrap(), "native");
    }
}
