pub mod types;

use anyhow::{Context, Result};
use std::path::PathBuf;
use types::{Config, VoiceMode};

/// Return the config directory for Loquitor.
///
/// Prefers XDG conventions over platform-native paths because CLI tools
/// almost universally expect `~/.config/<tool>/` on every platform, while
/// `dirs::config_dir()` returns `~/Library/Application Support/` on macOS.
/// Respects `$XDG_CONFIG_HOME` when set.
pub fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("loquitor");
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("loquitor")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn lanes_dir() -> PathBuf {
    config_dir().join("lanes")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config at {}", path.display()))?;
    Ok(config)
}

/// Peek at the config file without deserialising — returns `true` if it
/// looks like a pre-v0.2.0 config (has a top-level `[provider]` block
/// instead of `[tts]`/`[liaison]`). Used by `loquitor init` to decide
/// whether to offer a fresh-wizard run-through.
pub fn is_legacy_format() -> bool {
    let path = config_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return false;
    };
    let has_legacy_provider = content
        .lines()
        .any(|line| line.trim() == "[provider]");
    let has_new_tts = content.lines().any(|line| line.trim() == "[tts]");
    has_legacy_provider && !has_new_tts
}

/// Best-effort load of any existing config — returns a `Config` whose
/// `tts` (and where possible `liaison`) fields are populated with
/// whatever we could extract, so the init wizard can offer
/// "keep existing key" prompts instead of forcing re-entry.
///
/// Handles three cases:
///   - v0.2.0 config that round-trips cleanly → parsed as-is.
///   - v0.1.0 legacy `[provider]` block → migrated: `provider.*`
///     becomes `tts.*`, the rest of the `Config` falls back to
///     defaults, and `liaison` starts blank.
///   - No config / unreadable / genuinely malformed → `None`; caller
///     treats as first-time setup.
pub fn try_load_for_wizard() -> Option<types::Config> {
    let content = std::fs::read_to_string(config_path()).ok()?;

    // v0.2.0 happy path.
    if let Ok(cfg) = toml::from_str::<types::Config>(&content) {
        return Some(cfg);
    }

    // Legacy migration. We parse loosely via `toml::Value` so unknown
    // fields from v0.1.0 don't force a typed-deserialise failure.
    let value: toml::Value = toml::from_str(&content).ok()?;
    let provider = value.get("provider")?;
    let name = provider.get("name")?.as_str()?.to_string();
    let api_key = provider
        .get("api_key")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let model = provider
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // `liaison` intentionally stays at defaults (anthropic, empty key)
    // so `select_liaison` prompts fresh the first time.
    Some(types::Config {
        tts: types::TtsConfig {
            name,
            api_key,
            model,
        },
        ..types::Config::default()
    })
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path();
    let dir = config_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create config directory at {}", dir.display()))?;
    let content = toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}

/// Resolve the voice a lane should speak with.
/// Under `Shared` mode, always returns `voice.default`.
/// Under `PerLane` mode, returns the matching rule's voice or falls back to default.
pub fn resolve_voice(cfg: &Config, lane_id: &str) -> String {
    if cfg.voice.mode == VoiceMode::Shared {
        return cfg.voice.default.clone();
    }
    cfg.lanes
        .rules
        .get(lane_id)
        .map(|r| r.voice.clone())
        .unwrap_or_else(|| cfg.voice.default.clone())
}

/// Resolve the human-readable name for a lane.
/// Prefers the rule's `name` when non-empty; otherwise uses the lane_id itself
/// (which is the cwd basename, already human-readable in the common case).
pub fn resolve_lane_name(cfg: &Config, lane_id: &str) -> String {
    cfg.lanes
        .rules
        .get(lane_id)
        .map(|r| r.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| lane_id.to_string())
}
