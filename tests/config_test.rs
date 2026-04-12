use loquitor::config::types::Config;

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
}
