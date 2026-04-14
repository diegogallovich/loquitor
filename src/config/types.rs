use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub provider: ProviderConfig,
    pub voice: VoiceConfig,
    pub lanes: LanesConfig,
    pub queue: QueueConfig,
    pub parsing: ParsingConfig,
    pub daemon: DaemonConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub model: String,
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
    pub debounce_ms: u64,
    pub speakability_threshold: f64,
    pub tool_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub pid_file: String,
    pub log_level: String,
    /// Speak `lane_intro_template` before any utterance whose lane differs from
    /// the previous one played. Keeps the listener oriented when concurrent
    /// Claude sessions interleave in the same audio queue.
    #[serde(default = "default_true")]
    pub announce_lane_on_switch: bool,
    /// Template for the lane-switch announcement. `{name}` is replaced with the
    /// lane's friendly name (from `lanes.rules`) or its ID (cwd basename).
    #[serde(default = "default_intro_template")]
    pub lane_intro_template: String,
}

fn default_true() -> bool {
    true
}

fn default_intro_template() -> String {
    "Regarding {name}.".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub tip_shown: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig {
                name: "macos_say".into(),
                api_key: String::new(),
                model: String::new(),
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
                debounce_ms: 500,
                speakability_threshold: 0.6,
                tool_pattern: r"^(Bash|Read|Edit|Write|Glob|Grep|Agent|Skill|TaskCreate|TaskUpdate|ToolSearch|WebFetch|WebSearch|NotebookEdit)\s*\(".into(),
            },
            daemon: DaemonConfig {
                socket_path: "/tmp/loquitor.sock".into(),
                pid_file: "/tmp/loquitor.pid".into(),
                log_level: "info".into(),
                announce_lane_on_switch: true,
                lane_intro_template: "Regarding {name}.".into(),
            },
            ui: UiConfig { tip_shown: false },
        }
    }
}
