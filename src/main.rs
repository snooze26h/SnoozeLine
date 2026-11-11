use snoozeline::cli::Cli;
use snoozeline::config::{Config, InputData};
use snoozeline::core::{collect_all_segments, StatusLineGenerator};
use snoozeline::ui::{MainMenu, MenuResult};
use std::io::{self, IsTerminal, Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_args();

    if let Some(directory) = cli.refresh_git {
        snoozeline::core::segments::git_cache::refresh(&directory);
        return Ok(());
    }

    if cli.config {
        snoozeline::ui::run_configurator()?;
        return Ok(());
    }

    // Handle Claude Code patcher
    if let Some(claude_path) = cli.patch {
        use snoozeline::utils::ClaudeCodePatcher;

        println!("🔧 Claude Code Context Warning Disabler");
        println!("Target file: {}", claude_path);

        // Create backup in same directory
        let backup_path = format!("{}.backup", claude_path);
        std::fs::copy(&claude_path, &backup_path)?;
        println!("📦 Created backup: {}", backup_path);

        // Load and patch
        let mut patcher = ClaudeCodePatcher::new(&claude_path)?;

        println!("\n🔄 Applying patches...");
        let results = patcher.apply_all_patches();
        patcher.save()?;

        ClaudeCodePatcher::print_summary(&results);
        println!("💡 To restore warnings, replace your cli.js with the backup file:");
        println!("   cp {} {}", backup_path, claude_path);

        return Ok(());
    }

    // Load configuration
    let config_result = if cli.preview {
        Config::read()
    } else {
        Config::load()
    };
    let mut config = config_result.unwrap_or_else(|error| {
        eprintln!("snoozeline: invalid config, using defaults: {}", error);
        Config::default()
    });

    // Apply theme override if provided
    if let Some(theme) = cli.theme {
        config = snoozeline::ui::themes::ThemePresets::get_theme(&theme);
    }

    if cli.preview {
        let mut preview = snoozeline::ui::components::preview::PreviewComponent::new();
        preview.update_preview(&config);
        println!("{}", preview.get_preview_cache());
        return Ok(());
    }

    // Check if stdin has data
    if io::stdin().is_terminal() {
        if let Some(result) = MainMenu::run()? {
            match result {
                MenuResult::LaunchConfigurator => {
                    snoozeline::ui::run_configurator()?;
                }
                MenuResult::InitConfig | MenuResult::CheckConfig => {}
                MenuResult::Exit => {}
            }
        }
        return Ok(());
    }

    // Read Claude Code data from stdin
    let stdin = io::stdin();
    const MAX_INPUT_BYTES: u64 = 1024 * 1024;
    let mut bytes = Vec::new();
    stdin
        .lock()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("status line input exceeds 1 MiB".into());
    }
    // 输入可能含会话路径和模型信息；解析失败时不把原始字段写入错误输出。
    let input: InputData = serde_json::from_slice(&bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid status line JSON"))?;

    // Collect segment data
    let segments_data = collect_all_segments(&config, &input);

    // Render statusline
    let generator = StatusLineGenerator::new(config);
    let statusline = generator.generate(segments_data);

    println!("{}", statusline);

    Ok(())
}
