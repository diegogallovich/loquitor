use loquitor::tts::{self, macos::MacOsSayProvider, TtsProvider};

#[tokio::test]
async fn test_macos_say_list_voices() {
    let provider = MacOsSayProvider;
    let voices = provider.list_voices().await.unwrap();
    assert!(!voices.is_empty(), "Should list at least one voice");
    assert!(
        voices.iter().any(|v| v.name == "Samantha" || v.name == "Alex" || v.name == "Karen"),
        "Should include a common macOS voice"
    );
}

#[tokio::test]
async fn test_macos_say_synthesize() {
    let provider = MacOsSayProvider;
    let result = provider.synthesize("hello", &"Samantha".to_string()).await;
    assert!(result.is_ok(), "Synthesis should succeed: {:?}", result.err());
    let audio = result.unwrap();
    assert!(!audio.bytes.is_empty(), "Should produce audio bytes");
    assert_eq!(audio.format, loquitor::tts::AudioFormat::Aiff);
}

#[test]
fn test_create_provider_macos() {
    let provider = tts::create_provider("macos_say", "", "").unwrap();
    assert_eq!(provider.name(), "macOS Say");
}

#[test]
fn test_create_provider_unknown() {
    let result = tts::create_provider("nonexistent", "", "");
    assert!(result.is_err());
}
