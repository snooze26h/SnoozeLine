use super::{sanitize_text, Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

#[derive(Default)]
pub struct DirectorySegment;

impl DirectorySegment {
    pub fn new() -> Self {
        Self
    }

    /// Extract directory name from path, handling both Unix and Windows separators
    fn extract_directory_name(path: &str) -> String {
        let trimmed = path.trim_end_matches(['/', '\\']);
        if trimmed.is_empty() {
            "root".to_string()
        } else {
            let name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
            let sanitized = sanitize_text(name);
            if sanitized.is_empty() {
                "root".to_string()
            } else {
                sanitized
            }
        }
    }
}

impl Segment for DirectorySegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let current_dir = input.current_dir();

        // Handle cross-platform path separators manually for better compatibility
        let dir_name = Self::extract_directory_name(current_dir);

        // Store the full path in metadata for potential use
        let mut metadata = HashMap::new();
        metadata.insert("full_path".to_string(), current_dir.to_string());

        Some(SegmentData {
            primary: dir_name,
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Directory
    }
}

#[cfg(test)]
mod tests {
    use super::DirectorySegment;

    #[test]
    fn handles_trailing_separators_on_both_platforms() {
        assert_eq!(
            DirectorySegment::extract_directory_name("/Users/name/项目/"),
            "项目"
        );
        assert_eq!(
            DirectorySegment::extract_directory_name(r"C:\Users\name\project\"),
            "project"
        );
    }

    #[test]
    fn handles_roots_and_control_characters() {
        assert_eq!(DirectorySegment::extract_directory_name("/"), "root");
        assert_eq!(
            DirectorySegment::extract_directory_name("/tmp/unsafe\u{1b}[31m"),
            "unsafe[31m"
        );
    }
}
