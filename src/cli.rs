use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "snoozeline")]
#[command(version, about = "Reliable, customizable Claude Code status line")]
pub struct Cli {
    /// Enter TUI configuration mode
    #[arg(short = 'c', long = "config")]
    pub config: bool,

    /// Set theme
    #[arg(short = 't', long = "theme")]
    pub theme: Option<String>,

    /// Preview the current layout with sample data, without writing files
    #[arg(long, conflicts_with_all = ["config", "patch", "refresh_git"])]
    pub preview: bool,

    /// Internal worker for background Git status refreshes
    #[arg(long, hide = true, conflicts_with_all = ["config", "theme", "patch", "preview"])]
    pub refresh_git: Option<std::path::PathBuf>,

    /// Patch Claude Code cli.js to disable context warnings
    #[arg(long = "patch")]
    pub patch: Option<String>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
