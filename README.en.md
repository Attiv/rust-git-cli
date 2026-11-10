# Rust Git CLI

[中文](README.md) | [日本語](README.ja.md) | [Deutsch](README.de.md) | English

An intelligent Git commit tool that generates bilingual (Chinese/English) commit messages using AI.

## Features

- **AI-Powered** - Supports OpenAI, Anthropic, and custom endpoints (e.g., DeepSeek)
- **Bilingual Commits** - Automatically generates Chinese/English commit messages following Conventional Commits
- **Smart Staging** - Detects unstaged changes and prompts for confirmation
- **Interactive UI** - Colored output, diff preview, commit confirmation
- **AI Changelog** - Interactively select commits, generate AI changelog summary, copy to clipboard
- **AI Review** - Generate an AI code review report for current changes (risks/suggestions/tests)
- **Config Management** - `config` inspects, locates, edits and rewrites the config; `config doctor` checks the key and endpoint
- **Scriptable** - `commit --dry-run` prints the message without committing, `log --export` writes it to a file, `hook install` brings AI to plain `git commit`

## Installation

```bash
# Build from source
git clone https://github.com/duanyongcheng/rust-git-cli.git
cd rust-git-cli
cargo build --release

# Install to system
cargo install --path .
```

**Requirements**: Rust 1.70+, Git 2.0+

## Quick Start

### 1. Initialize Configuration

```bash
rust-git-cli init                 # Create global config (~/.config/rust-git-cli/config.toml)
rust-git-cli init --local         # Create project config (.rust-git-cli.toml)
```

### 2. Set API Key

```bash
# Recommended: environment variables (api_key_env holds a NAME, not the key)
export OPENAI_API_KEY="your-api-key"
# or
export ANTHROPIC_API_KEY="your-api-key"

# You can also store it in the config file (mode 600; `config` masks it)
rust-git-cli config set ai.api_key "your-api-key"
```

> `api_key_env` takes a variable name such as `OPENAI_API_KEY`, not the key itself. `config set` refuses the key there and `config doctor` points it out.

### 3. Usage

```bash
rust-git-cli                      # Check repository status (default)
rust-git-cli commit               # Generate AI commit message
rust-git-cli commit --show-diff   # Preview diff before generation
rust-git-cli commit --dry-run     # Print the message only, stage nothing, commit nothing
rust-git-cli commit --debug       # Debug mode
rust-git-cli review               # Review current changes with AI
rust-git-cli review --staged      # Review staged changes only
rust-git-cli review --base main   # Review the whole branch against main
rust-git-cli config doctor        # Check config, key and endpoint
```

The name is long; a short alias helps:

```bash
alias vgit=rust-git-cli     # add to ~/.zshrc or ~/.bashrc
# or leave your shell config alone: ln -sf "$(command -v rust-git-cli)" ~/.local/bin/vgit
```

## Commands

| Command | Description |
|---------|-------------|
| `status` | Check repository status (default) |
| `commit` | Generate and execute AI commit |
| `review` | Review current changes with AI |
| `diff` | Show code changes |
| `log` | Show commit history, supports AI changelog generation and file export |
| `config` | Inspect, locate, edit and rewrite the config; `doctor` self-checks |
| `hook` | Install/remove a `prepare-commit-msg` hook so `git commit` uses AI |
| `init` | Initialize config file |

### Global Options

```bash
rust-git-cli [OPTIONS] [COMMAND]

Options:
  -p, --path <PATH>    Path to check (default: current directory)
  -v, --verbose        Verbose output
  -h, --help           Print help
  -V, --version        Print version
```

### commit Options

```bash
rust-git-cli commit [OPTIONS]

Options:
  --api-key <KEY>      Specify API key temporarily
  --model <MODEL>      Specify AI model (e.g., gpt-4, deepseek-v3)
  --base-url <URL>     Custom API endpoint
  --auto               Skip confirmation and commit directly
  --dry-run            Print the generated message without staging or committing
  --show-diff          Preview diff before generation
  --debug              Show raw AI response
```

### log Options

```bash
rust-git-cli log [OPTIONS]

Options:
  -n, --count <N>      Number of commits to show (default: 10)
  --grep <PATTERN>     Filter by content
  --author <NAME>      Filter by author
  --since <DATE>       Start date (e.g., "2024-01-01" or "1 week ago")
  --until <DATE>       End date
  --full               Show full commit message
  --export <FILE>      Write the changelog to a file (plain commit list when there is no AI summary)
  --api-key <KEY>      Specify API key temporarily (for changelog generation)
  --model <MODEL>      Specify AI model
  --base-url <URL>     Custom API endpoint
  --debug              Show raw AI response
```

### diff Options

```bash
rust-git-cli diff [OPTIONS]

Options:
  --staged             Show only staged changes
```

### review Options

```bash
rust-git-cli review [OPTIONS]

Options:
  --api-key <KEY>      Specify API key temporarily
  --model <MODEL>      Specify AI model
  --base-url <URL>     Custom API endpoint
  --staged             Review staged changes only
  --unstaged           Review unstaged changes only
  --base <REF>         Compare against a ref, reviewing the whole branch (committed but unmerged)
  --debug              Show raw AI response
```

### config Command

Config management works outside a repository too.

```bash
rust-git-cli config                  # Show effective values and which file they came from
rust-git-cli config path             # Print the file in use, plus shadowed candidates
rust-git-cli config edit             # Open with $VISUAL / $EDITOR
rust-git-cli config open             # Open with the system default app
rust-git-cli config set ai.model deepseek-chat   # Rewrite one key, comments preserved
rust-git-cli config doctor           # Checks parsing, key source, endpoint, diff budget
rust-git-cli config doctor --offline # Skip the network probe
```

`config show` masks `api_key` (use `--show-secrets` to reveal it) and checks whether the variable named by `api_key_env` actually exists in this shell. `config set` validates before writing: a key in `api_key_env`, a misspelled provider or an unknown format is rejected instead of failing silently at run time.

### hook Command

```bash
rust-git-cli hook install     # Write .git/hooks/prepare-commit-msg
rust-git-cli hook status      # Show current state
rust-git-cli hook uninstall   # Remove it (only its own file)
```

After installing, a plain `git commit` with an empty message calls `rust-git-cli commit --dry-run` and fills in the result. Messages from `-m` / `-F` / merge / squash are left alone, `VGIT_NO_HOOK=1` bypasses it, and a hook written by someone else is never touched.

### init Options

```bash
rust-git-cli init [OPTIONS]

Options:
  --local              Create config in current directory instead of home
  --force              Force overwrite existing config
```

## Configuration

Config file lookup order:
1. `./.rust-git-cli.toml` (project-level)
2. `~/.config/rust-git-cli/config.toml` (user-level)
3. `~/.rust-git-cli.toml` (user-level fallback)

### Example Configuration

```toml
[ai]
provider = "openai"                      # openai (covers any compatible endpoint) or anthropic
model = "gpt-4"                          # Model name
api_key_env = "OPENAI_API_KEY"           # Variable NAME, not the key itself
# api_key = "sk-..."                     # Direct setting (not recommended, but works)
# base_url = "https://api.deepseek.com/v1"  # Custom endpoint
max_tokens = 2000                        # Starting cap; doubles when a response is truncated

[commit]
format = "conventional"                  # conventional | plain | english
include_emoji = false                    # Prefix the subject with the commit type's emoji
max_diff_size = 4000                     # Max diff characters sent to AI (review uses twice this)
auto_stage = false                       # Auto-stage all changes
```

### API Key Priority

1. Command line argument `--api-key`
2. Config file `api_key`
3. Environment variable (specified by `api_key_env`)
4. Interactive input

## Commit Message Format

Generated commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
feat(auth): 添加用户认证功能
Add user authentication feature

实现了JWT令牌验证
Implement JWT token validation
添加了用户登录接口
Add user login endpoint
```

### Commit Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation |
| `style` | Code formatting |
| `refactor` | Code refactoring |
| `test` | Testing |
| `chore` | Build/tooling |
| `perf` | Performance |

### Output Formats

`commit.format` controls rendering; all three keep the bilingual body:

| Value | Output |
|-------|--------|
| `conventional` | `feat(auth): 添加用户认证功能` + `Add user authentication feature` |
| `plain` | `添加用户认证功能` + `Add user authentication feature` (no type prefix) |
| `english` | `feat(auth): Add user authentication feature` (English only) |

`commit.include_emoji = true` prefixes the subject: `feat` ✨, `fix` 🐛, `docs` 📝, `style` 💄, `refactor` ♻️, `perf` ⚡️, `test` ✅, `build` 📦, `ci` 👷, `chore` 🔧, `revert` ⏪. Unknown types get nothing rather than a generic decoration on every line.

## Workflow

```bash
# 1. Check status
$ rust-git-cli status

# 2. View changes
$ rust-git-cli diff

# 3. Generate commit
$ rust-git-cli commit

# When unstaged changes are detected:
# Unstaged changes detected:
# ──────────────────────────────────────────────────
#   M src/main.rs
#   ? src/new_file.rs
# ──────────────────────────────────────────────────
# Do you want to stage all changes (git add .)? (Y/n)

# After AI generation, choose action:
# - Accept and commit: Accept and commit
# - Edit message: Edit before commit
# - Regenerate: Generate again
# - Cancel: Cancel

# 4. Generate Changelog
$ rust-git-cli log -n 20

# Interactively select commits (Space to select, Enter to confirm)
# After selection, generate AI changelog summary
# Supports copying to clipboard
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| API connection failed | Check network, verify API key, use `--debug` for details |
| JSON parsing error | Use `--debug` to view raw response, try different model |
| Config not working | Run `config doctor` first; a parse failure now warns instead of silently using defaults |
| "API key not set" although you set one | The key is probably in `api_key_env`, which takes a variable NAME. Use `config set ai.api_key <key>` |
| Commit failed | Ensure Git user is configured (`git config user.name/email`) |

```bash
# Debug mode
rust-git-cli commit --debug

# View help
rust-git-cli --help
rust-git-cli commit --help
```

## Development

```bash
cargo build                       # Build
cargo test                        # Test
cargo fmt                         # Format
cargo clippy -- -D warnings       # Lint
```

### Project Structure

```
src/
├── main.rs          # Entry point and command dispatch
├── cli.rs           # CLI definitions (clap)
├── config.rs        # Configuration management
├── git.rs           # Git operations (git2)
├── ui.rs            # Interactive UI (dialoguer)
└── ai/
    ├── mod.rs       # AI client abstraction
    ├── openai.rs    # OpenAI implementation
    └── anthropic.rs # Anthropic implementation
```

## License

MIT License

## Credits

- [git2-rs](https://github.com/rust-lang/git2-rs) - Git operations
- [clap](https://github.com/clap-rs/clap) - Command line parsing
- [dialoguer](https://github.com/console-rs/dialoguer) - Interactive UI
- [colored](https://github.com/colored-rs/colored) - Colored output
