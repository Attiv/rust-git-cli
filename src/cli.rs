use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, help = "Path to check (default: current directory)")]
    pub path: Option<PathBuf>,

    #[arg(short, long, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check repository status (default)
    Status,

    /// Generate commit message using AI
    Commit {
        #[arg(long, help = "API key for AI service (or set OPENAI_API_KEY env var)")]
        api_key: Option<String>,

        #[arg(long, help = "AI model to use (overrides config)")]
        model: Option<String>,

        #[arg(long, help = "Custom API base URL (e.g., https://api.openai.com/v1)")]
        base_url: Option<String>,

        #[arg(long, help = "Auto-commit without confirmation")]
        auto: bool,

        #[arg(
            long,
            help = "Print the generated message without committing or staging"
        )]
        dry_run: bool,

        #[arg(long, help = "Show diff in the prompt")]
        show_diff: bool,

        #[arg(long, help = "Debug mode - show AI raw response")]
        debug: bool,
    },

    /// Review code changes using AI
    Review {
        #[arg(long, help = "API key for AI service (or set OPENAI_API_KEY env var)")]
        api_key: Option<String>,

        #[arg(long, help = "AI model to use (overrides config)")]
        model: Option<String>,

        #[arg(long, help = "Custom API base URL")]
        base_url: Option<String>,

        #[arg(long, help = "Review staged changes only")]
        staged: bool,

        #[arg(long, help = "Review unstaged changes only")]
        unstaged: bool,

        #[arg(
            long,
            help = "Review everything that differs from this ref, e.g. --base main"
        )]
        base: Option<String>,

        #[arg(long, help = "Debug mode - show AI raw response")]
        debug: bool,
    },

    /// Show git diff
    Diff {
        #[arg(long, help = "Show staged changes only")]
        staged: bool,
    },

    /// Initialize configuration file
    Init {
        #[arg(long, help = "Create config in current directory instead of home")]
        local: bool,

        #[arg(long, help = "Force overwrite existing config")]
        force: bool,
    },

    /// Inspect and edit the configuration file
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,

        #[arg(long, help = "Show secret values unmasked")]
        show_secrets: bool,
    },

    /// Install or remove the git commit hook
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },

    /// Show git commit log (changelog)
    Log {
        #[arg(
            short = 'n',
            long,
            default_value = "10",
            help = "Number of commits to show"
        )]
        count: usize,

        #[arg(long, help = "Show only commits containing this text")]
        grep: Option<String>,

        #[arg(long, help = "Show commits by specific author")]
        author: Option<String>,

        #[arg(
            long,
            help = "Show commits since date (e.g., '2024-01-01' or '1 week ago')"
        )]
        since: Option<String>,

        #[arg(long, help = "Show commits until date")]
        until: Option<String>,

        #[arg(long, help = "Show full commit message")]
        full: bool,

        #[arg(long, value_name = "FILE", help = "Write the changelog to a file")]
        export: Option<PathBuf>,

        #[arg(long, help = "API key for AI service (or set OPENAI_API_KEY env var)")]
        api_key: Option<String>,

        #[arg(long, help = "AI model to use (overrides config)")]
        model: Option<String>,

        #[arg(long, help = "Custom API base URL")]
        base_url: Option<String>,

        #[arg(long, help = "Debug mode - show AI raw response")]
        debug: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Print the path of the config file currently in use
    Path,

    /// Print the effective configuration (default)
    Show,

    /// Open the config file in $EDITOR
    Edit,

    /// Open the config file with the system default application
    Open,

    /// Set a single key, e.g. `config set ai.model gpt-4o`
    Set {
        /// Dotted key, e.g. ai.model or commit.auto_stage
        key: String,

        /// New value (numbers and booleans are written unquoted)
        value: String,
    },

    /// Check config, credentials and endpoint reachability
    Doctor {
        /// Skip the network probe
        #[arg(long)]
        offline: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum HookAction {
    /// Install a prepare-commit-msg hook that fills in the message with AI
    Install {
        /// Overwrite an existing hook
        #[arg(long)]
        force: bool,
    },

    /// Remove the hook installed by this tool
    Uninstall,

    /// Report whether the hook is installed
    Status,
}
