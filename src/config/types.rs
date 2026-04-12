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
            },
            ui: UiConfig { tip_shown: false },
        }
    }
}
