# Repository Guidelines

## Repository Overview

Rust-based Git commit tool that generates bilingual (Chinese/English) commit messages using AI providers (OpenAI/Anthropic). Uses Conventional Commits format with automatic repository detection and interactive prompts.

## Project Structure & Module Organization

The Rust CLI lives under `src/`:

- `src/main.rs` — entry point and command routing
- `src/cli.rs` — clap command and flag definitions
- `src/git.rs` — Git operations via the `git2` crate
- `src/config.rs` — TOML configuration loading
- `src/ui.rs` — interactive prompts via `dialoguer`
- `src/ai/` — AI providers: `mod.rs`, `openai.rs`, `anthropic.rs`

Build artifacts go to `target/`. Configuration files may exist at `./.rust-git-cli.toml`, `~/.config/rust-git-cli/config.toml`, or `~/.rust-git-cli.toml`.

## Build, Test, and Development Commands

```bash
# Development
cargo build                       # Debug build
cargo run -- status               # Check repository status (default command)
cargo run -- diff                 # Show all changes
cargo run -- diff --staged        # Show staged changes only
cargo run -- commit               # Generate AI commit message
cargo run -- commit --debug       # Debug mode - shows AI raw response
cargo run -- commit --dry-run     # Print the message only, no staging, no commit
cargo run -- review               # AI code review of the working tree changes
cargo run -- review --base main   # Review everything on this branch since main
cargo run -- log -n 20            # Show commit history (interactive, supports AI changelog)
cargo run -- log -n 20 --export notes.md   # Write the changelog to a file
cargo run -- config               # Show effective config and which file it came from
cargo run -- config path          # Print the config file in use
cargo run -- config doctor        # Check config, credentials and endpoint reachability
cargo run -- config edit          # Open the config in $EDITOR
cargo run -- config set ai.model deepseek-chat   # Change one key, comments preserved
cargo run -- hook install         # Fill empty `git commit` messages with AI
cargo run -- hook status          # Is the hook installed?
cargo run -- hook uninstall       # Remove it
cargo run -- init                 # Initialize config (~/.config/rust-git-cli/)
cargo run -- init --local         # Initialize config in current directory

# Quality checks (required before commit)
cargo fmt                         # Format code
cargo clippy -- -D warnings       # Lint with warnings as errors
cargo test                        # Run all tests
cargo test test_name              # Run a single test by name

# Release
cargo build --release
cargo install --path .
```

## Architecture

### Command Flow

```
CLI (src/cli.rs) → Main dispatch (src/main.rs) → Command handlers
                                                      ↓
                   Git ops (src/git.rs) ← AI generation (src/ai/) → UI (src/ui.rs)
                                                      ↓
                                          Config (src/config.rs)
```

### Key Component Behaviors

**`src/main.rs`** — `init` and `config` bypass the git repo check (handled before repo validation). `check_and_stage_changes()` prompts before staging unstaged files. Commands: Status (default), Commit, Review, Diff, Log, Init, Config.

**`src/ai/mod.rs`** — `AIClient` enum dispatches to OpenAI/Anthropic providers. `CommitMessage` uses custom deserializers for flexible JSON parsing. `build_prompt()` generates the bilingual prompt with diff context; `generate_changelog()` summarizes selected commits.

**`src/git.rs`** — `GitRepo` wrapper exposing `get_status()`, `get_diff()`, `get_combined_diff()`, `get_branch_info()`, `get_commits()`. Handles unborn branches (new repos without commits).

**`src/config.rs`** — Lookup order: `./.rust-git-cli.toml` → `~/.config/rust-git-cli/config.toml` → `~/.rust-git-cli.toml`. API key priority: CLI arg → config file → env var → interactive prompt. `config set` rewrites one value in place and keeps the surrounding comments; it never re-serializes the whole file. `commit.max_diff_size` caps the diff sent to the provider (review uses twice the value). A parse failure is reported on stderr and falls back to defaults — never silently.

**`src/ui.rs`** — `CommitUI::confirm_commit()` offers Accept/Edit/Regenerate/Cancel. Color-coded diff preview. Clipboard support via `arboard` (used in the `log` command for changelog copy).

### stdout / stderr contract

Progress chatter (`CommitUI::show_info`) goes to **stderr**. Stdout carries only data. The commit hook depends on this: it runs `commit --dry-run`, pipes stdout into the message file, and discards stderr. `--dry-run` also skips the banner when stdout is not a TTY. Do not move informational output back to stdout.

### The commit hook

`hook install` writes `.git/hooks/prepare-commit-msg`. It exits early for `-m`/`-F`/merge/squash messages, when the message file already has real content, and when `VGIT_NO_HOOK` or `VGIT_COMMITTING` is set. The hook is identified by the `# managed-by: rust-git-cli` marker; `hook uninstall` refuses to delete a hook without it.

### Core Data Structures

```rust
// src/ai/mod.rs - Bilingual commit message
CommitMessage {
    commit_type: String,      // feat, fix, docs, etc.
    scope: Option<String>,
    description: String,      // Chinese
    description_en: String,   // English
    body: Option<Vec<String>>,    // Chinese details
    body_en: Option<Vec<String>>, // English details
    breaking_change: Option<String>,
}

// src/ai/mod.rs - Changelog generation
ChangelogSummary { title, title_en, highlights, highlights_en, categories: ChangelogCategories }
ChangelogCategories { features, fixes, improvements, others }  // All Vec<String> with bilingual items

// src/cli.rs - Commands enum with clap derive
Commands::Status | Commit { api_key, model, base_url, auto, show_diff, debug } | Diff { staged } | Log { count, grep, author, since, until, full, api_key, model, base_url, debug } | Init { local, force }
```

## Configuration

Config file example (`~/.config/rust-git-cli/config.toml`):

```toml
[ai]
provider = "openai"           # or "anthropic"
model = "gpt-4"
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.deepseek.com/v1"  # optional, for proxies
max_tokens = 2000

[commit]
max_diff_size = 4000
auto_stage = false
```

Never commit API keys. Prefer environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) or local config files that are gitignored.

## Coding Style & Naming Conventions

Use standard Rust formatting via `cargo fmt` (4-space indentation). Follow Rust naming conventions: `snake_case` for functions/modules, `CamelCase` for types, and lower-case CLI subcommand names. Keep diffs focused and avoid introducing unused dependencies.

## Testing Guidelines

Tests are module-local using `#[cfg(test)]` blocks (currently in `src/ai/openai.rs`) with functions named using a `test_` prefix. Add new unit tests close to the code they validate. Run `cargo test` or `cargo test <name>` to target a specific test.

## Commit & Pull Request Guidelines

Commit messages follow Conventional Commits and are bilingual on one line, e.g. `feat(openai): 新增... Add ...`. Include a scope when it clarifies the area touched.

For pull requests, describe the behavior change, link relevant issues, and include the test commands you ran; for CLI behavior changes, add a short sample command/output snippet.

## CI & Release

GitHub Actions via `.github/workflows/rust.yml`:

- Test matrix: `ubuntu-latest`, `windows-latest`, `macos-latest` × `stable`, `beta`, `nightly` (beta excluded on macOS and Windows, so Linux is the only job that exercises beta)
- Quality gates: `cargo fmt --check`, `cargo clippy -- -D warnings`, docs built with warnings as errors
- Tagged releases build and upload platform binaries
- RustSec audit job on Ubuntu

## Debugging

```bash
cargo run -- commit --debug   # Shows raw AI response and JSON parsing
```
