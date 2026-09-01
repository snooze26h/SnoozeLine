use super::{sanitize_text, Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;

#[derive(Default)]
pub struct OutputStyleSegment;

impl OutputStyleSegment {
    pub fn new() -> Self {
        Self
    }
}

impl Segment for OutputStyleSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let output_style = input.output_style.as_ref()?;

        // Primary display: style name
        let primary = sanitize_text(&output_style.name);

        let mut metadata = HashMap::new();
        metadata.insert("style_name".to_string(), output_style.name.clone());

        Some(SegmentData {
            primary,
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::OutputStyle
    }
}
