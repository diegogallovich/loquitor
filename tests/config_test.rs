use loquitor::config::{
    resolve_lane_name, resolve_voice,
    types::{Config, LaneRule, VoiceMode},
};

#[test]
fn test_default_config_serializes_and_deserializes() {
    let config = Config::default();
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.provider.name, "macos_say");
    assert_eq!(deserialized.queue.stale_threshold_secs, 15);
    assert_eq!(deserialized.voice.pool.len(), 3);
    assert!(!deserialized.ui.tip_shown);
}

#[test]
fn test_default_config_has_expected_values() {
    let config = Config::default();
    assert_eq!(config.provider.name, "macos_say");
    assert_eq!(config.parsing.debounce_ms, 500);
    assert_eq!(config.parsing.speakability_threshold, 0.6);
    assert_eq!(config.daemon.socket_path, "/tmp/loquitor.sock");
    assert_eq!(config.daemon.pid_file, "/tmp/loquitor.pid");
    assert_eq!(config.voice.default, "Samantha");
    assert!(config.lanes.rules.is_empty());
    // New fields introduced by the configure/lane-policy work
    assert_eq!(config.voice.mode, VoiceMode::PerLane);
    assert!(config.daemon.announce_lane_on_switch);
    assert_eq!(config.daemon.lane_intro_template, "Regarding {name}.");
}

/// Configs written before the configure/lane-policy feature had no
/// `voice.mode`, `daemon.announce_lane_on_switch`, or `daemon.lane_intro_template`.
/// They must still deserialize, and the missing fields must pick up sane defaults.
#[test]
fn test_legacy_config_missing_new_fields_deserializes() {
    let legacy = r#"
[provider]
name = "openai"
api_key = "sk-test"
model = "tts-1"

[voice]
default = "nova"
pool = ["nova", "alloy"]

[lanes]
rules = {}

[queue]
stale_threshold_secs = 15
coalesce_window_ms = 2000

[parsing]
debounce_ms = 500
speakability_threshold = 0.6
tool_pattern = "^(Bash)\\s*\\("

[daemon]
socket_path = "/tmp/loquitor.sock"
pid_file = "/tmp/loquitor.pid"
log_level = "info"

[ui]
tip_shown = false
"#;
    let cfg: Config = toml::from_str(legacy).expect("legacy config should still deserialize");
    assert_eq!(cfg.voice.mode, VoiceMode::PerLane);
    assert!(cfg.daemon.announce_lane_on_switch);
    assert_eq!(cfg.daemon.lane_intro_template, "Regarding {name}.");
}

#[test]
fn test_resolve_voice_shared_mode_always_returns_default() {
    let mut cfg = Config::default();
    cfg.voice.mode = VoiceMode::Shared;
    cfg.voice.default = "nova".into();
    cfg.lanes.rules.insert(
        "my-project".into(),
        LaneRule {
            name: "My Project".into(),
            voice: "alloy".into(),
        },
    );
    // Shared mode ignores the rule entirely
    assert_eq!(resolve_voice(&cfg, "my-project"), "nova");
    assert_eq!(resolve_voice(&cfg, "unknown-lane"), "nova");
}

#[test]
fn test_resolve_voice_per_lane_prefers_rule_voice() {
    let mut cfg = Config::default();
    cfg.voice.mode = VoiceMode::PerLane;
    cfg.voice.default = "nova".into();
    cfg.lanes.rules.insert(
        "my-project".into(),
        LaneRule {
            name: "My Project".into(),
            voice: "alloy".into(),
        },
    );
    assert_eq!(resolve_voice(&cfg, "my-project"), "alloy");
    // Lanes without a rule fall back to default
    assert_eq!(resolve_voice(&cfg, "other-repo"), "nova");
}

#[test]
fn test_resolve_lane_name_uses_rule_name_when_present() {
    let mut cfg = Config::default();
    cfg.lanes.rules.insert(
        "my-project".into(),
        LaneRule {
            name: "My Project".into(),
            voice: "nova".into(),
        },
    );
    assert_eq!(resolve_lane_name(&cfg, "my-project"), "My Project");
}

#[test]
fn test_resolve_lane_name_falls_back_to_lane_id() {
    let cfg = Config::default();
    assert_eq!(resolve_lane_name(&cfg, "some-cwd-name"), "some-cwd-name");
}

#[test]
fn test_resolve_lane_name_ignores_empty_rule_name() {
    let mut cfg = Config::default();
    cfg.lanes.rules.insert(
        "my-project".into(),
        LaneRule {
            name: String::new(),
            voice: "nova".into(),
        },
    );
    // Empty name should not mask the lane_id
    assert_eq!(resolve_lane_name(&cfg, "my-project"), "my-project");
}
