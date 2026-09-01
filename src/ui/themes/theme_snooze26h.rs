use crate::config::{
    AnsiColor, ColorConfig, IconConfig, SegmentConfig, SegmentId, TextStyleConfig,
};
use std::collections::HashMap;

fn segment(
    id: SegmentId,
    enabled: bool,
    plain_icon: &str,
    nerd_font_icon: &str,
    color: u8,
    options: HashMap<String, serde_json::Value>,
) -> SegmentConfig {
    SegmentConfig {
        id,
        enabled,
        icon: IconConfig {
            plain: plain_icon.to_string(),
            nerd_font: nerd_font_icon.to_string(),
        },
        colors: ColorConfig {
            icon: Some(AnsiColor::Color16 { c16: color }),
            text: Some(AnsiColor::Color16 { c16: color }),
            background: None,
        },
        styles: TextStyleConfig { text_bold: true },
        options,
    }
}

pub fn model_segment() -> SegmentConfig {
    segment(SegmentId::Model, true, "🤖", "\u{e26d}", 14, HashMap::new())
}

pub fn directory_segment() -> SegmentConfig {
    segment(
        SegmentId::Directory,
        true,
        "📁",
        "\u{f024b}",
        12,
        HashMap::new(),
    )
}

pub fn context_window_segment() -> SegmentConfig {
    segment(
        SegmentId::ContextWindow,
        true,
        "⚡️",
        "\u{f49b}",
        13,
        HashMap::new(),
    )
}

pub fn usage_segment() -> SegmentConfig {
    let mut options = HashMap::new();
    options.insert("timeout".to_string(), serde_json::Value::Number(2.into()));
    options.insert(
        "api_base_url".to_string(),
        serde_json::Value::String("https://api.anthropic.com".to_string()),
    );
    options.insert(
        "cache_duration".to_string(),
        serde_json::Value::Number(180.into()),
    );

    segment(SegmentId::Usage, true, "📊", "\u{f0a9e}", 11, options)
}

pub fn git_segment() -> SegmentConfig {
    let mut options = HashMap::new();
    options.insert("show_sha".to_string(), serde_json::Value::Bool(false));
    segment(SegmentId::Git, true, "🌿", "\u{f02a2}", 10, options)
}

pub fn cost_segment() -> SegmentConfig {
    segment(SegmentId::Cost, false, "💰", "\u{eec1}", 3, HashMap::new())
}

pub fn session_segment() -> SegmentConfig {
    segment(
        SegmentId::Session,
        false,
        "⏱️",
        "\u{f19bb}",
        2,
        HashMap::new(),
    )
}

pub fn output_style_segment() -> SegmentConfig {
    segment(
        SegmentId::OutputStyle,
        true,
        "🎯",
        "\u{f135}",
        7,
        HashMap::new(),
    )
}
