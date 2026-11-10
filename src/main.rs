mod ai;
mod cli;
mod config;
mod git;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use log::debug;
use std::env;
use std::process::Command;

use crate::cli::{Args, Commands, ConfigAction, HookAction};
use crate::config::{mask_secret, Config};
use crate::git::{GitRepo, LogOptions};
use crate::ui::{CommitAction, CommitUI};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments before touching the logger: `--debug` is what decides
    // whether the `debug!` calls scattered through the AI clients are shown.
    let args = Args::parse();
    init_logging(args.debug());

    let path = args
        .path
        .clone()
        .unwrap_or_else(|| env::current_dir().unwrap());

    // Handle init command first (doesn't need git repo)
    if let Some(Commands::Init { local, force }) = &args.command {
        return handle_init_command(*local, *force);
    }

    // Config commands also work outside a repository
    if let Some(Commands::Config {
        action,
        show_secrets,
    }) = &args.command
    {
        return handle_config_command(action, *show_secrets).await;
    }

    // Check if it's a git repository
    let repo = match GitRepo::open(&path) {
        Ok(repo) => repo,
        Err(_) => {
            println!("{} {}", "Checking:".bold(), path.display());
            println!();
            println!("{} ✗", "Not a Git repository".red().bold());
            println!();
            println!(
                "{}",
                "Tip: Run 'git init' to initialize a Git repository".yellow()
            );
            return Ok(());
        }
    };

    // Handle commands
    match args.command {
        Some(Commands::Commit {
            api_key,
            model,
            base_url,
            auto,
            dry_run,
            show_diff,
            debug,
        }) => {
            handle_commit_command(
                repo, api_key, model, base_url, auto, dry_run, show_diff, debug,
            )
            .await?;
        }
        Some(Commands::Review {
            api_key,
            model,
            base_url,
            staged,
            unstaged,
            base,
            debug,
        }) => {
            handle_review_command(
                repo, api_key, model, base_url, staged, unstaged, base, debug,
            )
            .await?;
        }
        Some(Commands::Diff { staged }) => {
            handle_diff_command(repo, staged)?;
        }
        Some(Commands::Log {
            count,
            grep,
            author,
            since,
            until,
            full,
            export,
            api_key,
            model,
            base_url,
            debug,
        }) => {
            handle_log_command(
                repo, count, grep, author, since, until, full, export, api_key, model, base_url,
                debug,
            )
            .await?;
        }
        Some(Commands::Hook { action }) => {
            handle_hook_command(repo, action)?;
        }
        Some(Commands::Status) | None => {
            handle_status_command(repo, args.path.clone(), args.verbose)?;
        }
        Some(Commands::Config { .. }) => {
            // Already handled above
            unreachable!()
        }
        Some(Commands::Init { .. }) => {
            // Already handled above
            unreachable!()
        }
    }

    Ok(())
}

/// Routing for the `debug!` calls in the AI clients.
///
/// `RUST_LOG` still wins, so anyone with that habit configured keeps working.
/// Otherwise `--debug` is the only switch: the previous `log_level` config key
/// could not be set correctly through `config set` (it is a top-level key, but
/// `set` writes into the current section) and defaulted to `warn`, which
/// silently swallowed every `--debug` line.
fn init_logging(debug: bool) {
    let mut builder = env_logger::Builder::new();

    match env::var("RUST_LOG") {
        Ok(spec) if !spec.trim().is_empty() => {
            builder.parse_filters(&spec);
        }
        _ => {
            builder.filter_level(if debug {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Warn
            });
        }
    }

    builder.format_timestamp(None).init();
}

fn handle_init_command(local: bool, force: bool) -> Result<()> {
    match Config::init(local, force) {
        Ok(path) => {
            println!(
                "{} Configuration file created at: {}",
                "✓".green().bold(),
                path.display()
            );
            println!();
            println!("{}", "Next steps:".bold());
            println!("  1. Edit the config file to set your API provider and model");
            println!("  2. Set your API key either:");
            println!("     - In the environment variable (recommended)");
            println!("     - Directly in the config file (not recommended)");
            println!();
            println!("  Example:");
            println!("  export OPENAI_API_KEY=\"your-api-key\"");
            println!("  # or");
            println!("  export ANTHROPIC_API_KEY=\"your-api-key\"");
            Ok(())
        }
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            Err(e)
        }
    }
}

async fn handle_config_command(action: &Option<ConfigAction>, show_secrets: bool) -> Result<()> {
    match action {
        None | Some(ConfigAction::Show) => show_config(show_secrets),
        Some(ConfigAction::Path) => show_config_path(),
        Some(ConfigAction::Edit) => edit_config(),
        Some(ConfigAction::Open) => open_config(),
        Some(ConfigAction::Set { key, value }) => set_config(key, value),
        Some(ConfigAction::Doctor { offline }) => doctor(*offline).await,
    }
}

/// Every check that has caused "why is my config not working" at least once.
async fn doctor(offline: bool) -> Result<()> {
    let mut problems = 0;
    let mut warnings = 0;

    let ok = |label: &str, detail: String| {
        println!("  {} {:<24} {}", "✓".green(), label, detail);
    };

    println!("{}", "Configuration".bold());
    println!();

    let (config, source) = match Config::load_with_source() {
        Ok(pair) => pair,
        Err(e) => {
            println!("  {} {:<24} {}", "✗".red(), "parse", e);
            println!();
            println!(
                "{}",
                "Fix the file before anything else — every command is silently using defaults."
                    .yellow()
            );
            return Ok(());
        }
    };

    match &source {
        Some(p) => ok("file", p.display().to_string()),
        None => {
            problems += 1;
            println!(
                "  {} {:<24} none found, using built-in defaults",
                "✗".red(),
                "file"
            );
            println!(
                "    {} {}",
                "run:".dimmed(),
                format!(
                    "vgit init  (would create {})",
                    Config::default_path()?.display()
                )
                .dimmed()
            );
        }
    }

    ok(
        "provider / model",
        format!("{} / {}", config.ai.provider, config.ai.model),
    );

    if !matches!(
        config.ai.provider.to_lowercase().as_str(),
        "openai" | "anthropic"
    ) {
        problems += 1;
        println!(
            "  {} {:<24} `{}` is not supported",
            "✗".red(),
            "provider",
            config.ai.provider
        );
    }

    // Credential resolution, mirroring Config::get_api_key
    let env_name = config.ai.api_key_env.as_deref();
    // A pasted key in the name field used to fail as "not set in this shell",
    // which points the reader at the wrong thing entirely.
    let env_name_is_key = env_name.is_some_and(|n| !crate::config::is_env_var_name(n));

    let from_env = env_name
        .filter(|_| !env_name_is_key)
        .and_then(|n| env::var(n).ok())
        .filter(|v| !v.is_empty());

    if config.ai.api_key.is_some() {
        ok("api key", "from config file (plain text)".to_string());
        warnings += 1;
        println!(
            "  {} {:<24} {}",
            "!".yellow(),
            "",
            "plain text in the config file; an env var is safer".yellow()
        );
    } else if from_env.is_some() {
        ok(
            "api key",
            format!("from environment ({})", env_name.unwrap_or_default()),
        );
    } else if env_name_is_key {
        problems += 1;
        println!(
            "  {} {:<24} ai.api_key_env holds `{}` — that is the key, not a variable name",
            "✗".red(),
            "api key",
            mask_secret(env_name.unwrap_or_default())
        );
        println!(
            "    {} {}",
            "fix:".dimmed(),
            "vgit config set ai.api_key <key>".dimmed()
        );
        println!(
            "    {} {}",
            "or:".dimmed(),
            "export OPENAI_API_KEY=… then vgit config set ai.api_key_env OPENAI_API_KEY".dimmed()
        );
    } else {
        problems += 1;
        let hint = match env_name {
            Some(name) => format!("`{}` is not set in this shell", name),
            None => "neither api_key nor api_key_env is configured".to_string(),
        };
        println!("  {} {:<24} {}", "✗".red(), "api key", hint);
    }

    let format = config.commit.format.trim().to_lowercase();
    if crate::ai::FORMATS.contains(&format.as_str()) {
        ok(
            "commit format",
            format!(
                "{}{}",
                format,
                if config.commit.include_emoji {
                    ", with emoji"
                } else {
                    ""
                }
            ),
        );
    } else {
        problems += 1;
        println!(
            "  {} {:<24} `{}` is unknown (valid: {})",
            "✗".red(),
            "commit format",
            config.commit.format,
            crate::ai::FORMATS.join(", ")
        );
    }

    let diff_limit = config.commit.max_diff_size;
    ok(
        "diff budget",
        format!(
            "{} chars for commit, {} for review",
            diff_limit,
            diff_limit * 2
        ),
    );

    // Working tree size vs the budget
    if let Ok(cwd) = env::current_dir() {
        if let Ok(repo) = GitRepo::open(&cwd) {
            if let Ok(diff) = repo.get_combined_diff() {
                if !diff.is_empty() {
                    let size = diff.len();
                    if size > diff_limit {
                        println!(
                            "  {} {:<24} {}",
                            "!".yellow(),
                            "working tree",
                            format!(
                                "{} chars of diff, {} will be cut before sending",
                                size,
                                size - diff_limit
                            )
                            .yellow()
                        );
                    } else {
                        ok(
                            "working tree",
                            format!("{} chars of diff, fits the budget", size),
                        );
                    }
                }
            }
        }
    }

    println!();

    if offline {
        println!("{}", "Skipped the network probe (--offline).".dimmed());
        return report_doctor(problems, warnings);
    }

    let Some(key) = config.get_api_key() else {
        println!("{}", "Skipped the network probe: no API key.".dimmed());
        return report_doctor(problems, warnings);
    };

    let interactive = std::io::IsTerminal::is_terminal(&std::io::stdout());

    if interactive {
        print!("  {} {:<24} ", "...".dimmed(), "endpoint");
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    let probe = ai::create_client(
        &config.ai.provider,
        key,
        config.ai.model.clone(),
        config.ai.base_url.clone(),
        config.ai.max_tokens,
        config.commit.max_diff_size,
    );

    let prefix = if interactive { "\r" } else { "  " };

    match probe {
        Ok(client) => match client.ping().await {
            Ok(detail) => {
                println!("{}{} {:<24} {}", prefix, "✓".green(), "endpoint", detail);
            }
            Err(e) => {
                problems += 1;
                println!("{}{} {:<24} {}", prefix, "✗".red(), "endpoint", e);
            }
        },
        Err(e) => {
            problems += 1;
            println!("{}{} {:<24} {}", prefix, "✗".red(), "endpoint", e);
        }
    }

    println!();
    report_doctor(problems, warnings)
}

/// Problems block usage; warnings are advice. Keeping them apart stops a
/// perfectly working setup from reporting "1 problem to fix".
fn report_doctor(problems: usize, warnings: usize) -> Result<()> {
    if problems == 0 && warnings == 0 {
        println!("{}", "No problems found.".green().bold());
    } else if problems == 0 {
        println!(
            "{} {} warning(s), nothing blocking.",
            "!".yellow().bold(),
            warnings
        );
    } else {
        println!(
            "{} {} problem(s) to fix{}. Run `vgit config edit`.",
            "!".yellow().bold(),
            problems,
            if warnings > 0 {
                format!(" (plus {} warning(s))", warnings)
            } else {
                String::new()
            }
        );
    }
    Ok(())
}

fn show_config_path() -> Result<()> {
    match Config::resolve_path() {
        Some(path) => {
            println!("{} {}", "In use:".bold(), path.display().to_string().cyan());

            let shadowed: Vec<_> = Config::candidate_paths()
                .into_iter()
                .filter(|p| *p != path && p.exists())
                .collect();

            if !shadowed.is_empty() {
                println!();
                println!("{}", "Ignored — a higher-priority file wins:".yellow());
                for p in shadowed {
                    println!("  {} {}", "-".dimmed(), p.display().to_string().dimmed());
                }
            }
        }
        None => {
            println!(
                "{}",
                "No config file found — built-in defaults are in effect.".yellow()
            );
            println!();
            println!("{}", "Searched, in order:".bold());
            for p in Config::candidate_paths() {
                println!("  {} {}", "-".dimmed(), p.display());
            }
            println!();
            println!(
                "{} {}",
                "Would be created at:".bold(),
                Config::default_path()?.display()
            );
            println!("{}", "Run `init` to create it.".cyan());
        }
    }

    Ok(())
}

fn show_config(show_secrets: bool) -> Result<()> {
    let (config, source) = Config::load_with_source()?;

    match &source {
        Some(p) => println!(
            "{} {}",
            "Config file:".bold(),
            p.display().to_string().cyan()
        ),
        None => println!(
            "{}",
            "Config file: none found, showing built-in defaults".yellow()
        ),
    }
    println!();

    let row = |key: &str, value: String| {
        println!("  {:<22} {}", key, value);
    };
    let unset = "(not set)".dimmed().to_string();

    row("ai.provider", config.ai.provider.clone());
    row("ai.model", config.ai.model.clone());
    row(
        "ai.base_url",
        match &config.ai.base_url {
            Some(v) => v.clone(),
            None => unset.clone(),
        },
    );
    row("ai.max_tokens", config.ai.max_tokens.to_string());
    row(
        "ai.api_key_env",
        match &config.ai.api_key_env {
            Some(name) if !crate::config::is_env_var_name(name) => format!(
                "{} {}",
                mask_secret(name),
                "not a variable name — looks like the key itself".red()
            ),
            Some(name) => {
                let present = env::var(name).map(|v| !v.is_empty()).unwrap_or(false);
                let state = if present {
                    "set".green().to_string()
                } else if config.ai.api_key.is_some() {
                    // Harmless: api_key takes precedence, so the env var's
                    // absence says nothing about whether the setup works.
                    "unused, ai.api_key wins".dimmed().to_string()
                } else {
                    "missing in env".red().to_string()
                };
                format!("{} ({})", name, state)
            }
            None => unset.clone(),
        },
    );
    row(
        "ai.api_key",
        match &config.ai.api_key {
            Some(k) => {
                if show_secrets {
                    k.clone()
                } else {
                    mask_secret(k)
                }
            }
            None => unset.clone(),
        },
    );

    row("commit.format", config.commit.format.clone());
    row(
        "commit.include_emoji",
        config.commit.include_emoji.to_string(),
    );
    row(
        "commit.max_diff_size",
        config.commit.max_diff_size.to_string(),
    );
    row("commit.auto_stage", config.commit.auto_stage.to_string());

    println!();

    if !crate::ai::FORMATS.contains(&config.commit.format.trim().to_lowercase().as_str()) {
        println!(
            "{} {}",
            "!".yellow().bold(),
            format!(
                "commit.format `{}` is unknown; falling back to conventional (valid: {}).",
                config.commit.format,
                crate::ai::FORMATS.join(", ")
            )
            .yellow()
        );
    }

    if config.ai.api_key.is_some() && !show_secrets {
        println!(
            "{}",
            "The API key is stored in plain text (file mode 600). To move it out:".yellow()
        );
        println!(
            "{}",
            "  export OPENAI_API_KEY=...   then   vgit config set ai.api_key_env OPENAI_API_KEY"
                .dimmed()
        );
    }

    println!(
        "{}",
        "Edit with `config edit`, locate with `config path`.".dimmed()
    );

    Ok(())
}

fn ensure_config_file() -> Result<std::path::PathBuf> {
    if let Some(path) = Config::resolve_path() {
        return Ok(path);
    }

    println!(
        "{}",
        "No config file yet — creating the default one first.".yellow()
    );
    Config::init(false, false)
}

fn edit_config() -> Result<()> {
    let path = ensure_config_file()?;

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or("vi");
    let extra: Vec<&str> = parts.collect();

    println!(
        "{} {} {}",
        "Opening".bold(),
        path.display(),
        format!("with {}", editor).dimmed()
    );

    let status = Command::new(program)
        .args(&extra)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to launch editor `{}`", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with status {}", status);
    }

    match Config::load_with_source() {
        Ok(_) => println!("{}", "✓ Config still parses correctly.".green()),
        Err(e) => println!("{} {}", "✗ Config has problems:".red().bold(), e),
    }

    Ok(())
}

fn open_config() -> Result<()> {
    let path = ensure_config_file()?;

    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(&path).status();

    #[cfg(target_os = "windows")]
    let status = Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(&path)
        .status();

    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("xdg-open").arg(&path).status();

    let status = status.context("Failed to launch the system file opener")?;

    if !status.success() {
        anyhow::bail!("System opener exited with status {}", status);
    }

    println!("{} {}", "Opened".green().bold(), path.display());
    Ok(())
}

fn set_config(key: &str, value: &str) -> Result<()> {
    // Catch the mistakes that would otherwise silently resolve to nothing.
    Config::validate_set(key, value)?;

    let path = ensure_config_file()?;
    Config::set_key_value(&path, key, value)?;

    // Never echo a secret back into a terminal log or a shared screen.
    let shown = if key.contains("api_key") && !key.ends_with("api_key_env") {
        mask_secret(value)
    } else {
        value.to_string()
    };

    println!("{} {} = {}", "✓".green().bold(), key, shown.cyan());
    println!("{}", format!("  written to {}", path.display()).dimmed());

    if let Err(e) = Config::load_with_source() {
        println!("{} {}", "✗ Config no longer parses:".red().bold(), e);
    }

    Ok(())
}

const HOOK_MARKER: &str = "# managed-by: rust-git-cli";

const HOOK_SCRIPT: &str = r#"#!/bin/sh
# managed-by: rust-git-cli
# Fills an empty commit message with an AI-generated one.
# Bypass with VGIT_NO_HOOK=1. Remove with `vgit hook uninstall`.

[ -n "$VGIT_NO_HOOK" ] && exit 0
[ -n "$VGIT_COMMITTING" ] && exit 0

MSG_FILE="$1"
SOURCE="$2"

# Leave messages that came from -m, -F, a merge or a squash alone.
case "$SOURCE" in
  message|merge|squash|commit) exit 0 ;;
esac

[ -f "$MSG_FILE" ] || exit 0

# Only act when the file holds nothing but comments and blank lines.
if grep -qvE '^[[:space:]]*(#|$)' "$MSG_FILE" 2>/dev/null; then
  exit 0
fi

command -v rust-git-cli >/dev/null 2>&1 || exit 0

MSG=$(VGIT_COMMITTING=1 rust-git-cli commit --dry-run 2>/dev/null) || exit 0

[ -n "$MSG" ] || exit 0

printf '%s\n' "$MSG" > "$MSG_FILE"
"#;

fn handle_hook_command(repo: GitRepo, action: HookAction) -> Result<()> {
    let hook_path = repo.git_dir().join("hooks").join("prepare-commit-msg");

    match action {
        HookAction::Status => {
            if hook_is_ours(&hook_path) {
                println!("{} prepare-commit-msg installed", "✓".green().bold());
            } else if hook_path.exists() {
                println!(
                    "{} a prepare-commit-msg hook exists but was not created by this tool",
                    "!".yellow().bold()
                );
            } else {
                println!("{}", "No prepare-commit-msg hook.".yellow());
            }
            println!("  {}", hook_path.display());
            Ok(())
        }
        HookAction::Install { force } => {
            if hook_path.exists() && !hook_is_ours(&hook_path) && !force {
                anyhow::bail!(
                    "{} already exists and was not created by this tool; re-run with --force to replace it",
                    hook_path.display()
                );
            }

            if let Some(parent) = hook_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&hook_path, HOOK_SCRIPT)
                .with_context(|| format!("Failed to write {}", hook_path.display()))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&hook_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&hook_path, perms)?;
            }

            println!("{} Hook installed", "✓".green().bold());
            println!("  {}", hook_path.display());
            println!();
            println!(
                "{}",
                "An empty `git commit` message will now be filled by AI.".dimmed()
            );
            println!(
                "{}",
                "Bypass with VGIT_NO_HOOK=1, remove with `vgit hook uninstall`.".dimmed()
            );
            Ok(())
        }
        HookAction::Uninstall => {
            if !hook_path.exists() {
                println!("{}", "Nothing to remove.".yellow());
                return Ok(());
            }

            if !hook_is_ours(&hook_path) {
                anyhow::bail!(
                    "{} was not created by this tool; refusing to delete it",
                    hook_path.display()
                );
            }

            std::fs::remove_file(&hook_path)?;
            println!("{} Hook removed", "✓".green().bold());
            Ok(())
        }
    }
}

fn hook_is_ours(path: &std::path::Path) -> bool {
    std::fs::read_to_string(path)
        .map(|s| s.contains(HOOK_MARKER))
        .unwrap_or(false)
}

fn write_export(
    path: &std::path::Path,
    ai_summary: Option<&str>,
    commits: &[crate::git::CommitInfo],
) -> Result<()> {
    use std::fmt::Write as _;

    let mut out = String::from("# Changelog\n\n");

    if let Some(summary) = ai_summary {
        out.push_str("## AI summary\n\n");
        out.push_str(summary.trim());
        out.push_str("\n\n");
    }

    out.push_str("## Commits\n\n");
    for commit in commits {
        let _ = writeln!(
            out,
            "- `{}` {} — {} ({})",
            commit.short_id,
            commit.time.format("%Y-%m-%d"),
            commit.summary,
            commit.author
        );
    }

    std::fs::write(path, out).with_context(|| format!("Failed to write {}", path.display()))?;

    println!(
        "\n{} wrote {} ({} commits)",
        "✓".green().bold(),
        path.display(),
        commits.len()
    );

    Ok(())
}

fn handle_status_command(
    repo: GitRepo,
    path: Option<std::path::PathBuf>,
    verbose: bool,
) -> Result<()> {
    let shown_path = path.unwrap_or_else(|| env::current_dir().unwrap_or_default());
    println!("{} {}", "Checking:".bold(), shown_path.display());
    println!();
    println!("{} ✓", "Git repository detected".green().bold());

    let status = repo.get_status()?;

    if status.is_clean {
        println!("{} ✓", "Working tree clean".green().bold());
        println!("All changes have been committed.");
    } else {
        println!("{} ✗", "Uncommitted changes detected".yellow().bold());
        println!();

        if !status.modified_files.is_empty() {
            println!("{}:", "Modified files".yellow());
            for file in &status.modified_files {
                println!("  {} {}", "M".yellow(), file);
            }
            println!();
        }

        if !status.new_files.is_empty() {
            println!("{}:", "New files".green());
            for file in &status.new_files {
                println!("  {} {}", "A".green(), file);
            }
            println!();
        }

        if !status.deleted_files.is_empty() {
            println!("{}:", "Deleted files".red());
            for file in &status.deleted_files {
                println!("  {} {}", "D".red(), file);
            }
            println!();
        }

        if !status.renamed_files.is_empty() {
            println!("{}:", "Renamed files".blue());
            for file in &status.renamed_files {
                println!("  {} {}", "R".blue(), file);
            }
            println!();
        }

        println!(
            "{}: {}",
            "Total uncommitted changes".bold(),
            status.total_changes().to_string().yellow()
        );

        if verbose {
            println!();
            println!("{}", "Tip: Use 'git add .' to stage all changes".cyan());
            println!(
                "{}",
                "     Use 'git commit -m \"message\"' to commit staged changes".cyan()
            );
            println!(
                "{}",
                "     Use 'rust-git-cli commit' to generate AI commit message".cyan()
            );
        }
    }

    // Show branch info
    let branch_info = repo.get_branch_info()?;
    println!();

    if let Some(name) = branch_info.name {
        if name == "unborn" {
            println!(
                "{}: No commits yet (unborn branch)",
                "Branch".yellow().bold()
            );
        } else {
            println!("{}: {}", "Current branch".bold(), name.cyan());

            if let Some(tracking) = branch_info.tracking_info {
                println!("{}: {}", "Tracking".bold(), tracking.upstream.cyan());

                if tracking.ahead > 0 || tracking.behind > 0 {
                    let mut status_parts = Vec::new();
                    if tracking.ahead > 0 {
                        status_parts.push(format!("{} ahead", tracking.ahead).green().to_string());
                    }
                    if tracking.behind > 0 {
                        status_parts
                            .push(format!("{} behind", tracking.behind).yellow().to_string());
                    }
                    println!("{}: {}", "Status".bold(), status_parts.join(", "));
                }
            }
        }
    } else if branch_info.is_detached {
        println!("{}: detached", "HEAD state".yellow().bold());
    }

    Ok(())
}

fn handle_diff_command(repo: GitRepo, staged: bool) -> Result<()> {
    let diff = if staged {
        println!("{}", "Showing staged changes:".bold().green());
        repo.get_diff(true)?
    } else {
        println!("{}", "Showing all changes:".bold().green());
        repo.get_combined_diff()?
    };

    if diff.is_empty() {
        println!("{}", "No changes to show".yellow());
    } else {
        println!("{}", diff);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_review_command(
    repo: GitRepo,
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    staged: bool,
    unstaged: bool,
    base: Option<String>,
    debug: bool,
) -> Result<()> {
    if staged && unstaged {
        anyhow::bail!("Cannot use both --staged and --unstaged");
    }

    if base.is_some() && (staged || unstaged) {
        anyhow::bail!("--base reviews the whole branch; drop --staged/--unstaged");
    }

    let config = Config::load_or_warn();

    let diff_scope = if let Some(ref base_ref) = base {
        format!("vs {}", base_ref)
    } else if staged {
        "staged".to_string()
    } else if unstaged {
        "unstaged".to_string()
    } else {
        "all".to_string()
    };

    let (diff, file_count) = if let Some(ref base_ref) = base {
        repo.get_diff_against_base(base_ref)?
    } else if staged {
        (repo.get_diff(true)?, repo.get_status()?.total_changes())
    } else if unstaged {
        (repo.get_diff(false)?, repo.get_status()?.total_changes())
    } else {
        (
            repo.get_combined_diff()?,
            repo.get_status()?.total_changes(),
        )
    };

    if diff.is_empty() {
        CommitUI::show_info("No changes to review");
        return Ok(());
    }

    let added_lines = diff
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count();
    let removed_lines = diff
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .count();

    let branch_info = repo.get_branch_info()?;

    let context = ai::ReviewContext {
        branch_name: branch_info.name,
        file_count,
        added_lines,
        removed_lines,
        diff_scope,
    };

    let api_key = api_key
        .or_else(|| config.get_api_key())
        .or_else(|| CommitUI::get_api_key(&config.ai.provider).ok())
        .context("No API key provided")?;

    let final_model = model.unwrap_or(config.ai.model.clone());
    let final_base_url = base_url.or(config.ai.base_url.clone());
    let client = ai::create_client(
        &config.ai.provider,
        api_key,
        final_model,
        final_base_url,
        config.ai.max_tokens,
        config.commit.max_diff_size,
    )?;

    CommitUI::show_info("Generating AI code review...");

    let report = client
        .generate_review_report(&diff, &context, debug)
        .await?;

    println!("\n{}", "=".repeat(60));
    println!("{}", "AI Code Review".bold().green());
    println!("{}\n", "=".repeat(60));
    println!("{}", report.format_display());

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_log_command(
    repo: GitRepo,
    count: usize,
    grep: Option<String>,
    author: Option<String>,
    since: Option<String>,
    until: Option<String>,
    full: bool,
    export: Option<std::path::PathBuf>,
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    debug: bool,
) -> Result<()> {
    let options = LogOptions {
        count,
        grep,
        author,
        since: since.clone(),
        until: until.clone(),
    };

    let commits = repo.get_commits(&options)?;

    if commits.is_empty() {
        println!("{}", "No commits found".yellow());
        return Ok(());
    }

    let is_interactive = std::io::IsTerminal::is_terminal(&std::io::stdin())
        && std::io::IsTerminal::is_terminal(&std::io::stderr());

    let format_item = |commit: &crate::git::CommitInfo| {
        let date_str = commit.time.format("%Y-%m-%d %H:%M").to_string();
        format!(
            "{} {} - {} ({})",
            commit.short_id, date_str, commit.summary, commit.author
        )
    };

    let print_commit = |commit: &crate::git::CommitInfo| {
        let date_str = commit.time.format("%Y-%m-%d %H:%M").to_string();

        println!(
            "{} {} - {} ({})",
            commit.short_id.yellow(),
            date_str.dimmed(),
            commit.summary.bold(),
            commit.author.cyan()
        );

        if full && commit.message.lines().count() > 1 {
            let body: String = commit
                .message
                .lines()
                .skip(1)
                .filter(|line| !line.trim().is_empty())
                .map(|line| format!("    {}", line))
                .collect::<Vec<_>>()
                .join("\n");

            if !body.is_empty() {
                println!("{}", body.dimmed());
            }
            println!();
        }
    };

    // Store selected commits for AI changelog generation
    let selected_commits: Vec<crate::git::CommitInfo>;

    if is_interactive {
        use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};

        let items: Vec<String> = commits.iter().map(format_item).collect();
        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Select commits ({} available, Space to select, Enter to confirm)",
                commits.len()
            ))
            .items(&items)
            .interact()?;

        if selections.is_empty() {
            println!("{}", "No commits selected".yellow());
            return Ok(());
        }

        // Collect selected commits
        selected_commits = selections
            .iter()
            .filter_map(|&idx| commits.get(idx).cloned())
            .collect();

        println!(
            "\n{} ({} selected)\n",
            "Selected Commits".bold().green(),
            selected_commits.len()
        );

        for commit in &selected_commits {
            print_commit(commit);
        }

        // Ask what to do next
        let actions = vec!["Generate AI Changelog Summary", "Just print (done)"];

        let action = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&actions)
            .default(0)
            .interact()?;

        if action == 1 {
            if let Some(path) = &export {
                write_export(path, None, &selected_commits)?;
            }
            return Ok(());
        }

        // Generate AI changelog
        let config = Config::load_or_warn();

        let api_key = api_key
            .or_else(|| config.get_api_key())
            .or_else(|| CommitUI::get_api_key(&config.ai.provider).ok());

        let api_key = match api_key {
            Some(key) => key,
            None => {
                println!(
                    "{}",
                    "No API key provided. Cannot generate AI changelog.".red()
                );
                if let Some(path) = &export {
                    write_export(path, None, &selected_commits)?;
                }
                return Ok(());
            }
        };

        let final_model = model.unwrap_or(config.ai.model.clone());
        let final_base_url = base_url.or(config.ai.base_url.clone());

        let client = ai::create_client(
            &config.ai.provider,
            api_key,
            final_model,
            final_base_url,
            config.ai.max_tokens,
            config.commit.max_diff_size,
        )?;

        // Build date range string
        let date_range = if !selected_commits.is_empty() {
            let first_date = selected_commits
                .last()
                .map(|c| c.time.format("%Y-%m-%d").to_string());
            let last_date = selected_commits
                .first()
                .map(|c| c.time.format("%Y-%m-%d").to_string());
            match (first_date, last_date) {
                (Some(first), Some(last)) if first != last => Some(format!("{} ~ {}", first, last)),
                (Some(first), _) => Some(first),
                _ => None,
            }
        } else {
            None
        };

        let context = ai::ChangelogContext {
            total_commits: selected_commits.len(),
            date_range,
        };

        CommitUI::show_info("Generating AI changelog summary...");

        let changelog = client
            .generate_changelog(&selected_commits, &context, debug)
            .await?;

        let changelog_text = changelog.format_display();

        println!("\n{}", "=".repeat(60));
        println!("{}", "AI Generated Changelog Summary".bold().green());
        println!("{}\n", "=".repeat(60));
        println!("{}", changelog_text);

        if let Some(path) = &export {
            write_export(path, Some(&changelog_text), &selected_commits)?;
        }

        // Ask if user wants to copy to clipboard
        let copy_actions = vec!["Copy to clipboard", "Done"];

        let copy_action = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&copy_actions)
            .default(0)
            .interact()?;

        if copy_action == 0 {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(&changelog_text) {
                    Ok(_) => {
                        CommitUI::show_success("Changelog copied to clipboard!");
                    }
                    Err(e) => {
                        println!(
                            "{} Failed to copy to clipboard: {}",
                            "Error:".red().bold(),
                            e
                        );
                    }
                },
                Err(e) => {
                    println!(
                        "{} Failed to access clipboard: {}",
                        "Error:".red().bold(),
                        e
                    );
                }
            }
        }

        return Ok(());
    }

    // Non-interactive mode
    println!(
        "{} ({} commits)\n",
        "Changelog".bold().green(),
        commits.len()
    );

    for commit in &commits {
        print_commit(commit);
    }

    if let Some(path) = &export {
        write_export(path, None, &commits)?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_commit_command(
    repo: GitRepo,
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    auto: bool,
    dry_run: bool,
    show_diff: bool,
    debug: bool,
) -> Result<()> {
    // Load config
    let config = Config::load_or_warn();

    // Check for changes
    let status = repo.get_status()?;
    if status.is_clean {
        CommitUI::show_info("No changes to commit");
        return Ok(());
    }

    // A dry run must not touch the index, so the staging prompt is skipped.
    if !dry_run {
        check_and_stage_changes(config.commit.auto_stage)?;
    }

    // Get diff - this should now include staged changes
    let diff = repo.get_combined_diff()?;

    // Debug: Check if we're getting the staged diff correctly
    debug!("Combined diff length: {}", diff.len());
    if diff.len() > 100 {
        debug!("First 100 chars of diff: {}", &diff[..100]);
    }

    if diff.is_empty() {
        CommitUI::show_info("No changes detected");
        return Ok(());
    }

    // Show diff preview if requested
    if show_diff && !CommitUI::show_diff_preview(&diff, 30)? {
        CommitUI::show_info("Commit generation cancelled");
        return Ok(());
    }

    // Get API key
    let api_key = api_key
        .or_else(|| config.get_api_key())
        .or_else(|| CommitUI::get_api_key(&config.ai.provider).ok())
        .context("No API key provided")?;

    // Count changes for context
    let added_lines = diff.lines().filter(|l| l.starts_with('+')).count();
    let removed_lines = diff.lines().filter(|l| l.starts_with('-')).count();

    // Get branch info
    let branch_info = repo.get_branch_info()?;

    // Create context
    let context = ai::CommitContext {
        branch_name: branch_info.name,
        file_count: status.total_changes(),
        added_lines,
        removed_lines,
    };

    // Create AI client
    // Use model from CLI if provided, otherwise use config
    let final_model = model.unwrap_or(config.ai.model.clone());
    // Use base_url from CLI if provided, otherwise use config
    let final_base_url = base_url.or(config.ai.base_url.clone());
    let client = ai::create_client(
        &config.ai.provider,
        api_key,
        final_model,
        final_base_url,
        config.ai.max_tokens,
        config.commit.max_diff_size,
    )?;

    CommitUI::show_info("Generating commit message with AI...");

    // Generate commit message
    let commit_message = client
        .generate_commit_message(&diff, &context, debug)
        .await?;

    // Handle user action
    let formatted = commit_message.render(&config.commit.format, config.commit.include_emoji);

    if dry_run {
        // Interactive callers get a header; the commit hook pipes stdout and
        // expects nothing but the message itself.
        if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
            println!("\n{}", "=".repeat(60));
            println!(
                "{}",
                "Generated message (dry run, nothing committed)"
                    .bold()
                    .green()
            );
            println!("{}\n", "=".repeat(60));
        }
        println!("{}", formatted);
        return Ok(());
    }

    let action = if auto {
        CommitAction::Accept
    } else {
        CommitUI::confirm_commit(&formatted)?
    };

    match action {
        CommitAction::Accept => {
            execute_commit(&formatted)?;
            CommitUI::show_success("Changes committed successfully!");
        }
        CommitAction::Edit(edited_message) => {
            execute_commit(&edited_message)?;
            CommitUI::show_success("Changes committed with edited message!");
        }
        CommitAction::Regenerate => {
            CommitUI::show_info("Please run the command again to regenerate");
        }
        CommitAction::Cancel => {
            CommitUI::show_info("Commit cancelled");
        }
    }

    Ok(())
}

fn check_and_stage_changes(auto_stage: bool) -> Result<()> {
    use crate::ui::CommitUI;
    use dialoguer::{theme::ColorfulTheme, Confirm};

    // Check if there are unstaged changes
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to check git status")?;

    if status_output.status.success() {
        let status = String::from_utf8_lossy(&status_output.stdout);
        let has_unstaged = status.lines().any(|line| {
            line.starts_with(" M")
                || line.starts_with("??")
                || line.starts_with(" D")
                || line.starts_with(" A")
        });

        if has_unstaged {
            println!("\n{}", "Unstaged changes detected:".yellow());
            println!("{}", "─".repeat(50));

            // Show unstaged files
            for line in status.lines() {
                if line.starts_with(" M") {
                    println!("  {} {}", "M".yellow(), &line[3..]);
                } else if line.starts_with("??") {
                    println!("  {} {}", "?".red(), &line[3..]);
                } else if line.starts_with(" D") {
                    println!("  {} {}", "D".red(), &line[3..]);
                }
            }
            println!("{}", "─".repeat(50));

            if auto_stage {
                CommitUI::show_info("commit.auto_stage is on, staging everything");
                stage_all()?;
                return Ok(());
            }

            let should_stage = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Do you want to stage all changes (git add .)?")
                .default(true)
                .interact()?;

            if should_stage {
                stage_all()?;
            } else {
                CommitUI::show_info("Proceeding with only currently staged changes");
            }
        }
    }

    Ok(())
}

fn stage_all() -> Result<()> {
    use crate::ui::CommitUI;

    let add_output = Command::new("git")
        .args(["add", "."])
        .output()
        .context("Failed to execute git add command")?;

    if !add_output.status.success() {
        let error = String::from_utf8_lossy(&add_output.stderr);
        anyhow::bail!("Failed to stage changes: {}", error.trim());
    }

    CommitUI::show_info("All changes staged successfully");
    Ok(())
}

fn execute_commit(message: &str) -> Result<()> {
    // Execute git commit
    let commit_output = Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .context("Failed to execute git commit command")?;

    if !commit_output.status.success() {
        let error = String::from_utf8_lossy(&commit_output.stderr);

        // Check for common git errors and provide helpful messages
        let error_msg = if error.contains("nothing to commit") {
            "No changes to commit. All changes may already be committed."
        } else if error.contains("Please tell me who you are") {
            "Git user not configured. Please run:\n  git config --global user.email \"you@example.com\"\n  git config --global user.name \"Your Name\""
        } else {
            error.trim()
        };

        anyhow::bail!("Git commit failed: {}", error_msg);
    }

    Ok(())
}
