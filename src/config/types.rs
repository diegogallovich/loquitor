use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Text-to-speech provider — the voice layer that speaks the final
    /// summary notification.
    pub tts: TtsConfig,
    /// Liaison LLM — the layer that reads the turn buffer and produces
    /// the short sentence to be spoken. Configured separately because
    /// users routinely pair a cheap LLM (Haiku) with a premium TTS voice
    /// (ElevenLabs) or vice versa.
    pub liaison: LiaisonConfig,
    pub voice: VoiceConfig,
    pub lanes: LanesConfig,
    pub queue: QueueConfig,
    pub parsing: ParsingConfig,
    pub daemon: DaemonConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub name: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiaisonConfig {
    /// Provider id: `anthropic`, `openai`, `google`, `minimax`, `openai_compat`.
    pub name: String,
    pub api_key: String,
    pub model: String,
    /// Only meaningful for `openai_compat` — the base URL of the
    /// OpenAI-compatible endpoint (xAI, Groq, Mistral, DeepSeek, Ollama,
    /// self-hosted, etc.). Ignored by native providers.
    #[serde(default)]
    pub base_url: String,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    #[serde(default = "default_liaison_timeout_secs")]
    pub timeout_secs: u64,
    /// Run the regex secret scrubber before shipping the turn buffer.
    /// Forced on for every non-local provider regardless of this setting
    /// — the flag is only read when resolving local endpoints.
    #[serde(default = "default_true")]
    pub scrub_secrets: bool,
}

fn default_max_output_tokens() -> u32 {
    120
}
fn default_liaison_timeout_secs() -> u64 {
    15
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub default: String,
    pub pool: Vec<String>,
    #[serde(default)]
    pub mode: VoiceMode,
}

/// Controls how a lane resolves its voice.
/// `Shared` — every lane speaks with `voice.default`; `lanes.rules` voices are ignored.
/// `PerLane` — `lanes.rules[<lane_id>].voice` wins; falls back to `voice.default` when no rule matches.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VoiceMode {
    Shared,
    #[default]
    PerLane,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanesConfig {
    #[serde(default)]
    pub rules: HashMap<String, LaneRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneRule {
    pub name: String,
    pub voice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub stale_threshold_secs: u64,
    pub coalesce_window_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingConfig {
    /// Preserve ANSI colour codes when feeding lines into the idle detector.
    /// The detector uses box-drawing-only content to signal idle, so colour
    /// matters only if the user customises the detection heuristic.
    #[serde(default = "default_true")]
    pub preserve_ansi_color_for_idle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub pid_file: String,
    pub log_level: String,
    /// Number of identical consecutive prompt frames required to confirm
    /// that Claude is idle (secondary signal — only fires when Claude
    /// emits a clean box-drawing-only prompt frame).
    #[serde(default = "default_idle_confirm_frames")]
    pub idle_confirm_frames: u32,
    /// Minimum ms of quiet (no non-prompt output) between the first
    /// detected prompt frame and the idle emission.
    #[serde(default = "default_idle_min_silence_ms")]
    pub idle_min_silence_ms: u64,
    /// **Primary** idle signal: end the turn if no new bytes have hit
    /// the lane log for this many ms. Robust to whatever the TUI looks
    /// like — Claude's input prompt has too much status text to match
    /// the box-drawing classifier reliably.
    #[serde(default = "default_idle_quiet_ms")]
    pub idle_quiet_ms: u64,
    /// Upper bound on per-lane turn buffer size. When exceeded, the buffer
    /// front-truncates and the turn is marked `truncated`.
    #[serde(default = "default_turn_buffer_max_bytes")]
    pub turn_buffer_max_bytes: usize,
    /// Hard ceiling on how long a single turn can collect before being
    /// force-shipped (e.g. Claude hung). Default 30 min.
    #[serde(default = "default_turn_max_duration_secs")]
    pub turn_max_duration_secs: u64,
}

fn default_idle_confirm_frames() -> u32 {
    3
}
fn default_idle_min_silence_ms() -> u64 {
    500
}
fn default_idle_quiet_ms() -> u64 {
    // Bumped from 3s → 5s because 3s catches mid-turn pauses (tool
    // calls, network round-trips, Claude's own thinking gaps) and
    // misfires. 5s still feels snappy to the listener, rides through
    // most transient silences. Users can tune via [daemon].idle_quiet_ms.
    5000
}
fn default_turn_buffer_max_bytes() -> usize {
    262144
}
fn default_turn_max_duration_secs() -> u64 {
    1800
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub tip_shown: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tts: TtsConfig {
                name: "macos_say".into(),
                api_key: String::new(),
                model: String::new(),
            },
            liaison: LiaisonConfig {
                name: "anthropic".into(),
                api_key: String::new(),
                model: "claude-haiku-4-5".into(),
                base_url: String::new(),
                max_output_tokens: default_max_output_tokens(),
                timeout_secs: default_liaison_timeout_secs(),
                scrub_secrets: true,
            },
            voice: VoiceConfig {
                default: "Samantha".into(),
                pool: vec!["Samantha".into(), "Daniel".into(), "Karen".into()],
                mode: VoiceMode::PerLane,
            },
            lanes: LanesConfig {
                rules: HashMap::new(),
            },
            queue: QueueConfig {
                stale_threshold_secs: 15,
                coalesce_window_ms: 2000,
            },
            parsing: ParsingConfig {
                preserve_ansi_color_for_idle: true,
            },
            daemon: DaemonConfig {
                socket_path: "/tmp/loquitor.sock".into(),
                pid_file: "/tmp/loquitor.pid".into(),
                log_level: "info".into(),
                idle_confirm_frames: default_idle_confirm_frames(),
                idle_min_silence_ms: default_idle_min_silence_ms(),
                idle_quiet_ms: default_idle_quiet_ms(),
                turn_buffer_max_bytes: default_turn_buffer_max_bytes(),
                turn_max_duration_secs: default_turn_max_duration_secs(),
            },
            ui: UiConfig { tip_shown: false },
        }
    }
}
