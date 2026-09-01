// Legacy defaults - now using ui/themes/presets.rs for configuration
// This file kept for backward compatibility

use super::types::Config;

impl Default for Config {
    fn default() -> Self {
        // Use the theme presets as the source of truth
        crate::ui::themes::ThemePresets::get_snooze26h()
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::config::SegmentId;

    #[test]
    fn snooze26h_is_the_default_theme() {
        let config = Config::default();

        assert_eq!(config.theme, "snooze26h");
        assert_eq!(
            config
                .segments
                .iter()
                .map(|segment| segment.id)
                .collect::<Vec<_>>(),
            vec![
                SegmentId::Model,
                SegmentId::Directory,
                SegmentId::ContextWindow,
                SegmentId::Usage,
                SegmentId::Git,
                SegmentId::Cost,
                SegmentId::Session,
                SegmentId::OutputStyle,
            ]
        );
    }
}
