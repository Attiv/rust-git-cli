use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;

static WARN_ONCE: Once = Once::new();
static WARN_ENV_ONCE: Once = Once::new();

/// Every key `config set` may write.
///
/// Explicit on purpose: `set` edits the file line by line, so a typo would
/// otherwise land in whatever section happened to be current and then be
/// ignored by serde — the exact failure that made `log_level` unfixable.
pub const KNOWN_KEYS: &[&str] = &[
    "ai.provider",
    "ai.model",
    "ai.api_key",
    "ai.api_key_env",
    "ai.base_url",
    "ai.max_tokens",
    "commit.format",
    "commit.include_emoji",
    "commit.max_diff_size",
    "commit.auto_stage",
];

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub ai: AIConfig,
    #[serde(default)]
    pub commit: CommitConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub provider: String,
    pub model: String,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 {
    2000
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommitConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub include_emoji: bool,
    #[serde(default = "default_max_diff_size")]
    pub max_diff_size: usize,
    #[serde(default)]
    pub auto_stage: bool,
}

fn default_format() -> String {
    "conventional".to_string()
}

fn default_max_diff_size() -> usize {
    4000
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4.1".to_string(),
            api_key_env: None,
            api_key: None,
            base_url: None,
            max_tokens: default_max_tokens(),
        }
    }
}

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            format: default_format(),
            include_emoji: false,
            max_diff_size: default_max_diff_size(),
            auto_stage: false,
        }
    }
}

impl Config {
    /// Config file locations in lookup order. The first one that exists wins.
    pub fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = vec![PathBuf::from(".rust-git-cli.toml")];

        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".config/rust-git-cli/config.toml"));
            paths.push(home.join(".rust-git-cli.toml"));
        }

        paths
    }

    /// Where `init` writes by default when `--local` is not used.
    pub fn default_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|p| p.join(".config/rust-git-cli/config.toml"))
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))
    }

    /// The file actually being read, if any.
    pub fn resolve_path() -> Option<PathBuf> {
        Self::candidate_paths()
            .into_iter()
            .find(|p| p.exists() && p.is_file())
    }

    /// Best-effort load for command paths that must keep working even with a
    /// broken config. A parse failure is reported once instead of being
    /// swallowed silently, so "my settings have no effect" stops being a
    /// mystery.
    pub fn load_or_warn() -> Self {
        match Self::load_with_source() {
            Ok((config, _)) => config,
            Err(e) => {
                WARN_ONCE.call_once(|| {
                    eprintln!("{} {}", "warning:".yellow().bold(), e);
                    eprintln!(
                        "{}",
                        "Falling back to built-in defaults. Run `config path` to find the file, `config edit` to fix it."
                            .dimmed()
                    );
                });
                Self::default()
            }
        }
    }

    pub fn load_with_source() -> Result<(Self, Option<PathBuf>)> {
        if let Some(path) = Self::resolve_path() {
            let content = fs::read_to_string(&path)
                .context(format!("Failed to read config from {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .context(format!("Failed to parse config from {}", path.display()))?;
            return Ok((config, Some(path)));
        }

        Ok((Self::default(), None))
    }

    pub fn get_api_key(&self) -> Option<String> {
        // First check if api_key is directly set
        if let Some(key) = &self.ai.api_key {
            return Some(key.clone());
        }

        // Then check environment variable if configured
        if let Some(env_var) = &self.ai.api_key_env {
            // A key pasted into the variable-name field is a common mistake and
            // used to fail with nothing but "not set in this shell".
            if !is_env_var_name(env_var) {
                WARN_ENV_ONCE.call_once(|| {
                    eprintln!(
                        "{} ai.api_key_env holds `{}`, which is not an environment variable name.",
                        "warning:".yellow().bold(),
                        mask_secret(env_var)
                    );
                    eprintln!(
                        "{}",
                        "  That field takes a NAME such as OPENAI_API_KEY. Move the key to ai.api_key with `config set ai.api_key <key>`."
                            .dimmed()
                    );
                });
                return None;
            }
            return std::env::var(env_var).ok();
        }

        None
    }

    /// Reject writes that would silently break the config later.
    ///
    /// These are the mistakes that otherwise surface as "my setting has no
    /// effect": a key in the variable-name field, a misspelled format, an
    /// unknown provider.
    pub fn validate_set(key: &str, value: &str) -> Result<()> {
        let (section, field) = match key.split_once('.') {
            Some((s, f)) => (s, f),
            None => ("", key),
        };

        match (section, field) {
            ("ai", "api_key_env") => {
                if !is_env_var_name(value.trim()) {
                    anyhow::bail!(
                        "ai.api_key_env takes the NAME of an environment variable, not the key itself.\n\
                         \n\
                         For a key you want to store in the file:  vgit config set ai.api_key <key>\n\
                         For an environment variable:              vgit config set ai.api_key_env OPENAI_API_KEY"
                    );
                }
            }
            ("ai", "provider") => {
                let v = value.trim().to_lowercase();
                if v != "openai" && v != "anthropic" {
                    anyhow::bail!(
                        "`{}` is not a supported provider. Use \"openai\" (covers any OpenAI-compatible endpoint) or \"anthropic\".",
                        value.trim()
                    );
                }
            }
            ("commit", "format") => {
                let v = value.trim().to_lowercase();
                if !crate::ai::FORMATS.contains(&v.as_str()) {
                    anyhow::bail!(
                        "`{}` is not a known format. Use one of: {}.",
                        value.trim(),
                        crate::ai::FORMATS.join(", ")
                    );
                }
            }
            _ => {
                if !KNOWN_KEYS.contains(&key) {
                    anyhow::bail!(
                        "`{}` is not a known setting.\n\
                         \n\
                         Known settings:\n  {}\n\
                         \n\
                         Run `vgit config edit` to add anything else by hand.",
                        key.trim(),
                        KNOWN_KEYS.join("\n  ")
                    );
                }
            }
        }

        Ok(())
    }

    pub fn init(local: bool, force: bool) -> Result<PathBuf> {
        let path = if local {
            PathBuf::from(".rust-git-cli.toml")
        } else {
            Self::default_path()?
        };

        // Check if file already exists
        if path.exists() && !force {
            anyhow::bail!(
                "Config file already exists at {:?}. Use --force to overwrite.",
                path
            );
        }

        // Create default config with helpful comments
        let config_content = r#"# Rust Git CLI Configuration File
# This file configures the rust-git-cli tool for AI-powered commit message generation

[ai]
# AI provider: "openai" or "anthropic"
provider = "openai"

# Model to use for generation
# OpenAI: "gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"
# Anthropic: "claude-3-opus", "claude-3-sonnet", "claude-3-haiku"
model = "gpt-4"

# Environment variable containing the API key.
# This is a NAME such as OPENAI_API_KEY — never the key itself.
# For OpenAI: typically "OPENAI_API_KEY"
# For Anthropic: typically "ANTHROPIC_API_KEY"
api_key_env = "OPENAI_API_KEY"

# Direct API key (not recommended for security reasons)
# Uncomment and set your API key here if you prefer not to use environment variables
# api_key = "your-api-key-here"

# Custom API endpoint (optional - uncomment and modify if using a proxy or alternative API)
# For OpenAI-compatible APIs (e.g., Azure OpenAI, local LLMs, proxies):
# base_url = "https://api.openai.com/v1"
# For Anthropic-compatible APIs:
# base_url = "https://api.anthropic.com"
# Examples:
# base_url = "https://your-proxy.com/v1"  # For API proxies
# base_url = "http://localhost:8080/v1"   # For local LLMs

# Maximum tokens for AI response (default: 2000)
# This is the initial max_tokens value. If the response is truncated,
# it will automatically double up to 4000 tokens.
max_tokens = 2000

[commit]
# Commit message format:
#   "conventional" - type(scope): 中文描述, then the English line
#   "plain"        - no type prefix, still bilingual
#   "english"      - keeps the type prefix, English line only
format = "conventional"

# Prefix the subject with the emoji for its commit type (feat ✨, fix 🐛, ...)
include_emoji = false

# Maximum diff size in characters to send to AI
max_diff_size = 4000

# Whether to automatically stage all changes before committing
auto_stage = false
"#;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create directory {:?}", parent))?;
            }
        }

        // Write config file
        fs::write(&path, config_content)
            .context(format!("Failed to write config to {:?}", path))?;

        // Set appropriate file permissions on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600); // Read/write for owner only
            fs::set_permissions(&path, permissions)
                .context("Failed to set config file permissions")?;
        }

        Ok(path)
    }

    /// Rewrite a single key in place, keeping comments and layout intact.
    /// Only the value after `=` on the matching line is replaced.
    pub fn set_key_value(path: &Path, key: &str, raw_value: &str) -> Result<()> {
        let (section, field) = match key.split_once('.') {
            Some((s, f)) => (Some(s.to_string()), f.to_string()),
            None => (None, key.to_string()),
        };

        let formatted = format_value(raw_value);
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        // Locate the matching assignment within the right section. A
        // commented-out assignment counts as a candidate: the generated
        // template ships every optional key in `# key = ...` form, so ignoring
        // those would make `set` append a second assignment and the file would
        // then fail to parse with "duplicate key".
        let mut current: Option<String> = None;
        let mut active: Option<usize> = None;
        let mut commented: Option<usize> = None;

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if let Some(name) = section_name(trimmed) {
                current = Some(name);
                continue;
            }

            let same_section = match (&current, &section) {
                (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                (None, None) => true,
                _ => false,
            };
            if !same_section {
                continue;
            }

            let is_comment = trimmed.starts_with('#');
            let body = if is_comment {
                trimmed.trim_start_matches('#').trim_start()
            } else {
                trimmed
            };

            if let Some((k, _)) = body.split_once('=') {
                if k.trim() == field {
                    if is_comment {
                        commented.get_or_insert(idx);
                    } else {
                        active = Some(idx);
                        break;
                    }
                }
            }
        }

        match active.or(commented) {
            Some(idx) => {
                let line = lines[idx].clone();
                let indent = &line[..line.len() - line.trim_start().len()];
                let trimmed = line.trim();
                let body = if trimmed.starts_with('#') {
                    trimmed.trim_start_matches('#').trim_start()
                } else {
                    trimmed
                };

                let (head, trailing_comment) = split_inline_comment(body);
                let Some(eq) = head.find('=') else {
                    unreachable!("line was matched on the presence of '='")
                };

                // Keep whatever separation and comment followed the old value.
                let gap = &head[head.trim_end().len()..];
                let gap = if trailing_comment.is_empty() {
                    ""
                } else if gap.is_empty() {
                    "  "
                } else {
                    gap
                };

                lines[idx] = format!(
                    "{}{} = {}{}{}",
                    indent,
                    head[..eq].trim_end(),
                    formatted,
                    gap,
                    trailing_comment
                );
            }
            None => {
                let new_line = format!("{} = {}", field, formatted);
                match find_section(&lines, section.as_deref()) {
                    Some((_, end)) => {
                        // Insert after the last meaningful line of the section.
                        let mut insert_at = end;
                        while insert_at > 0 && lines[insert_at - 1].trim().is_empty() {
                            insert_at -= 1;
                        }
                        lines.insert(insert_at, new_line);
                    }
                    None => {
                        if let Some(name) = &section {
                            lines.push(String::new());
                            lines.push(format!("[{}]", name));
                        }
                        lines.push(new_line);
                    }
                }
            }
        }

        let mut out = lines.join("\n");
        out.push('\n');
        fs::write(path, out).with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}

/// Whether `name` is a plausible environment variable name.
///
/// Deliberately strict: `sk-ant-…` and `sk-…` are not names, and accepting
/// them is what produced a config that looked correct but resolved to nothing.
pub fn is_env_var_name(name: &str) -> bool {
    let name = name.trim();
    let mut chars = name.chars();

    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Show enough of a secret to recognize it, never enough to use it.
pub fn mask_secret(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();

    if chars.len() <= 8 {
        return "*".repeat(chars.len().max(1));
    }

    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{}…{}", head, tail)
}

/// Split a TOML line into `(code, trailing comment)`.
///
/// A `#` inside a quoted value is not a comment, so quotes are tracked. The
/// first part keeps the whitespace that separated it from the comment, which
/// lets callers preserve the original layout.
fn split_inline_comment(line: &str) -> (&str, &str) {
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '#' if !in_string => return (&line[..idx], &line[idx..]),
            _ => {}
        }
    }

    (line, "")
}

/// `[ai]` -> Some("ai"), anything else -> None.
fn section_name(trimmed: &str) -> Option<String> {
    if trimmed.starts_with('[') && !trimmed.starts_with("[[") && trimmed.ends_with(']') {
        Some(
            trimmed
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string(),
        )
    } else {
        None
    }
}

/// Byte range of a section body: (header line, exclusive end).
fn find_section(lines: &[String], name: Option<&str>) -> Option<(usize, usize)> {
    let name = name?;

    let start = lines
        .iter()
        .position(|l| section_name(l.trim()).is_some_and(|n| n.eq_ignore_ascii_case(name)))?;

    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| section_name(l.trim()).is_some())
        .map(|(i, _)| i)
        .unwrap_or(lines.len());

    Some((start, end))
}

/// Numbers and booleans go in bare; everything else gets quoted.
fn format_value(raw: &str) -> String {
    let trimmed = raw.trim();

    if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
        return trimmed.to_lowercase();
    }

    if trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok() {
        return trimmed.to_string();
    }

    let already_quoted = (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''));

    if already_quoted {
        return trimmed.to_string();
    }

    let escaped = trimmed.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const BASE: &str = r#"# top of file
[ai]
# which model to talk to
provider = "openai"
model = "gpt-4"

[commit]
format = "conventional"
include_emoji = false
max_diff_size = 4000
auto_stage = false
"#;

    fn temp_config(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rgc-test-{}-{}", std::process::id(), name));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn format_value_leaves_scalars_bare() {
        assert_eq!(format_value("true"), "true");
        assert_eq!(format_value("FALSE"), "false");
        assert_eq!(format_value("4000"), "4000");
        assert_eq!(format_value("1.5"), "1.5");
    }

    #[test]
    fn format_value_quotes_strings() {
        assert_eq!(format_value("gpt-4"), "\"gpt-4\"");
        assert_eq!(format_value("\"already\""), "\"already\"");
        assert_eq!(format_value("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn set_replaces_value_and_keeps_comments() {
        let path = temp_config("replace", BASE);

        Config::set_key_value(&path, "ai.model", "deepseek-chat").unwrap();

        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("# top of file"));
        assert!(out.contains("# which model to talk to"));
        assert!(out.contains("model = \"deepseek-chat\""));
        assert!(out.contains("max_diff_size = 4000"));

        let parsed: Config = toml::from_str(&out).unwrap();
        assert_eq!(parsed.ai.model, "deepseek-chat");
    }

    #[test]
    fn set_uncomments_the_placeholder_instead_of_appending_a_duplicate() {
        // The generated template ships optional keys commented out. Appending a
        // second `api_key` next to the comment made the file unparseable with
        // "duplicate key", which is how a working config got broken in practice.
        let content = r#"[ai]
provider = "openai"
model = "gpt-4"
# api_key = "your-api-key-here"
"#;
        let path = temp_config("uncomment", content);

        Config::set_key_value(&path, "ai.api_key", "sk-test").unwrap();

        let out = fs::read_to_string(&path).unwrap();
        assert_eq!(out.matches("api_key =").count(), 1, "got:\n{}", out);
        assert!(out.contains("api_key = \"sk-test\""));

        let parsed: Config = toml::from_str(&out).unwrap();
        assert_eq!(parsed.ai.api_key.as_deref(), Some("sk-test"));
    }

    #[test]
    fn set_keeps_a_trailing_comment_on_the_line_it_rewrites() {
        let content = r#"[ai]
provider = "openai"
model = "gpt-4"
base_url = "http://127.0.0.1:8045/v1"   # For local LLMs
"#;
        let path = temp_config("trailing", content);

        Config::set_key_value(&path, "ai.base_url", "http://127.0.0.1:9000/v1").unwrap();

        let out = fs::read_to_string(&path).unwrap();
        assert!(
            out.contains("base_url = \"http://127.0.0.1:9000/v1\"   # For local LLMs"),
            "got:\n{}",
            out
        );

        let parsed: Config = toml::from_str(&out).unwrap();
        assert_eq!(
            parsed.ai.base_url.as_deref(),
            Some("http://127.0.0.1:9000/v1")
        );
    }

    #[test]
    fn a_hash_inside_a_value_is_not_a_comment() {
        let (code, comment) = split_inline_comment(r#"model = "a# b"   # real comment"#);
        assert_eq!(code, r#"model = "a# b"   "#);
        assert_eq!(comment, "# real comment");

        let (code, comment) = split_inline_comment(r#"model = "a# b""#);
        assert_eq!(code, r#"model = "a# b""#);
        assert!(comment.is_empty());
    }

    #[test]
    fn set_inserts_missing_key_into_existing_section() {
        let path = temp_config("insert", BASE);

        Config::set_key_value(&path, "ai.max_tokens", "8000").unwrap();

        let parsed: Config = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed.ai.max_tokens, 8000);
        assert_eq!(parsed.ai.model, "gpt-4");
        assert_eq!(parsed.commit.max_diff_size, 4000);
    }

    #[test]
    fn set_creates_missing_section() {
        let path = temp_config("newsec", "[ai]\nprovider = \"openai\"\nmodel = \"gpt-4\"\n");

        Config::set_key_value(&path, "logging.level", "debug").unwrap();

        let out = fs::read_to_string(&path).unwrap();
        assert!(out.contains("[logging]"));
        assert!(out.contains("level = \"debug\""));
    }

    #[test]
    fn section_name_only_matches_tables() {
        assert_eq!(section_name("[ai]"), Some("ai".to_string()));
        assert_eq!(section_name("[[bin]]"), None);
        assert_eq!(section_name("key = 1"), None);
    }

    #[test]
    fn env_var_names_are_identified() {
        assert!(is_env_var_name("OPENAI_API_KEY"));
        assert!(is_env_var_name("_private"));
        assert!(is_env_var_name("key123"));

        // The exact mistake that produced a config which resolved to nothing.
        assert!(!is_env_var_name("sk-fe772ccdac404901aebaf30531ea669d"));
        assert!(!is_env_var_name("sk-ant-api03-xxxx"));
        assert!(!is_env_var_name("123KEY"));
        assert!(!is_env_var_name(""));
        assert!(!is_env_var_name("MY-KEY"));
    }

    #[test]
    fn validate_set_rejects_a_key_in_the_env_field() {
        let err = Config::validate_set("ai.api_key_env", "sk-fe772ccdac404901aebaf30531ea669d")
            .unwrap_err()
            .to_string();

        assert!(err.contains("NAME"));
        // The secret must never be echoed back in the error.
        assert!(!err.contains("fe772ccd"));
    }

    #[test]
    fn validate_set_accepts_correct_values() {
        assert!(Config::validate_set("ai.api_key_env", "OPENAI_API_KEY").is_ok());
        assert!(Config::validate_set("ai.api_key", "sk-anything-at-all").is_ok());
        assert!(Config::validate_set("ai.provider", "Anthropic").is_ok());
        assert!(Config::validate_set("commit.format", "plain").is_ok());
        assert!(Config::validate_set("commit.auto_stage", "true").is_ok());
    }

    #[test]
    fn every_known_key_is_accepted() {
        // Guards against the list and the validators drifting apart: a key may
        // still be rejected for a bad value, but never as "unknown".
        for key in KNOWN_KEYS {
            if let Err(e) = Config::validate_set(key, "x") {
                assert!(
                    !e.to_string().contains("not a known setting"),
                    "{} is listed in KNOWN_KEYS but validate_set calls it unknown",
                    key
                );
            }
        }
    }

    #[test]
    fn validate_set_rejects_unknown_keys() {
        // `log_level` used to be a real field, but `set` wrote it into the
        // wrong section, so it never took effect. It must not be accepted now.
        let err = Config::validate_set("log_level", "debug")
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a known setting"));
        assert!(err.contains("commit.max_diff_size"));

        // A typo must fail instead of being written into the current section.
        let err = Config::validate_set("commit.maxdiffsize", "5000")
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a known setting"));
    }

    #[test]
    fn a_config_that_still_contains_log_level_parses() {
        // Files written by older versions must not start failing to load.
        let toml = r#"
log_level = "debug"

[ai]
provider = "openai"
model = "gpt-4.1"

[commit]
format = "plain"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ai.model, "gpt-4.1");
        assert_eq!(config.commit.format, "plain");
    }

    #[test]
    fn validate_set_rejects_unknown_provider_and_format() {
        let err = Config::validate_set("ai.provider", "gemini").unwrap_err();
        assert!(err.to_string().contains("not a supported provider"));

        let err = Config::validate_set("commit.format", "coventional").unwrap_err();
        assert!(err.to_string().contains("conventional, plain, english"));
    }

    #[test]
    fn mask_keeps_only_the_edges() {
        assert_eq!(mask_secret("12345678"), "********");
        assert_eq!(
            mask_secret("sk-fe772ccdac404901aebaf30531ea669d"),
            "sk-f…669d"
        );
    }
}
