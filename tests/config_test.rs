use loquitor::config::{
    is_legacy_format, resolve_lane_name, resolve_voice,
    types::{Config, LaneRule, VoiceMode},
};

#[test]
fn test_default_config_serializes_and_deserializes() {
    let config = Config::default();
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.tts.name, "macos_say");
    assert_eq!(deserialized.liaison.name, "anthropic");
    assert_eq!(deserialized.queue.stale_threshold_secs, 15);
    assert_eq!(deserialized.voice.pool.len(), 3);
    assert!(!deserialized.ui.tip_shown);
}

#[test]
fn test_default_config_has_expected_values() {
    let config = Config::default();
    // TTS defaults
    assert_eq!(config.tts.name, "macos_say");
    assert_eq!(config.voice.default, "Samantha");
    assert_eq!(config.voice.mode, VoiceMode::PerLane);

    // Liaison defaults — Anthropic Haiku is the shipped default
    assert_eq!(config.liaison.name, "anthropic");
    assert_eq!(config.liaison.model, "claude-haiku-4-5");
    assert_eq!(config.liaison.max_output_tokens, 120);
    assert_eq!(config.liaison.timeout_secs, 15);
    assert!(
        config.liaison.scrub_secrets,
        "scrubber must default on — privacy default"
    );

    // Daemon + idle detector defaults
    assert_eq!(config.daemon.socket_path, "/tmp/loquitor.sock");
    assert_eq!(config.daemon.pid_file, "/tmp/loquitor.pid");
    assert_eq!(config.daemon.idle_confirm_frames, 3);
    assert_eq!(config.daemon.idle_min_silence_ms, 500);
    assert_eq!(config.daemon.turn_buffer_max_bytes, 262_144);
    assert_eq!(config.daemon.turn_max_duration_secs, 1800);

    assert!(config.lanes.rules.is_empty());
    assert!(config.parsing.preserve_ansi_color_for_idle);
}

/// A partial v0.2.0 config (missing liaison extras, missing new daemon
/// fields) must still deserialize with sane defaults. This is how upgrades
/// from incomplete manually-edited configs stay non-catastrophic.
#[test]
fn test_partial_v020_config_applies_defaults() {
    let partial = r#"
[tts]
name = "openai"
api_key = "sk-test"
model = "tts-1"

[liaison]
name = "anthropic"
api_key = "sk-ant-test"
model = "claude-haiku-4-5"

[voice]
default = "nova"
pool = ["nova", "alloy"]

[lanes]
rules = {}

[queue]
stale_threshold_secs = 15
coalesce_window_ms = 2000

[parsing]

[daemon]
socket_path = "/tmp/loquitor.sock"
pid_file = "/tmp/loquitor.pid"
log_level = "info"

[ui]
tip_shown = false
"#;
    let cfg: Config = toml::from_str(partial).expect("partial v0.2.0 config should deserialize");
    // Missing fields pick up defaults
    assert_eq!(cfg.liaison.max_output_tokens, 120);
    assert_eq!(cfg.liaison.timeout_secs, 15);
    assert!(cfg.liaison.scrub_secrets);
    assert_eq!(cfg.daemon.idle_confirm_frames, 3);
    assert_eq!(cfg.daemon.turn_buffer_max_bytes, 262_144);
    assert!(cfg.parsing.preserve_ansi_color_for_idle);
    // Explicit fields preserved
    assert_eq!(cfg.tts.name, "openai");
    assert_eq!(cfg.voice.mode, VoiceMode::PerLane);
}

#[test]
fn test_is_legacy_format_when_no_config() {
    // is_legacy_format is a filesystem probe on the config path. We can't
    // easily swap it in-test, but we can exercise the "no file" branch.
    let _ = is_legacy_format();
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
    assert_eq!(resolve_lane_name(&cfg, "my-project"), "my-project");
}
