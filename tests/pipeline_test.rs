use async_trait::async_trait;
use bytes::Bytes;
use loquitor::config::types::{Config, LaneRule};
use loquitor::daemon::pipeline::{handle_lane_message, render_intro};
use loquitor::tts::{AudioData, AudioFormat, TtsProvider, Voice, VoiceId};
use loquitor::watcher::lane::LaneMessage;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

type CallLog = Arc<Mutex<Vec<(String, String)>>>;

/// Records every `synthesize` call for test assertions. Returns a 1-byte
/// AudioData so the utterance flows through the audio channel without
/// triggering stale drops during playback.
struct RecordingProvider {
    calls: CallLog,
}

impl RecordingProvider {
    fn new() -> (Self, CallLog) {
        let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                calls: calls.clone(),
            },
            calls,
        )
    }
}

#[async_trait]
impl TtsProvider for RecordingProvider {
    fn name(&self) -> &str {
        "recording"
    }
    async fn list_voices(&self) -> anyhow::Result<Vec<Voice>> {
        Ok(vec![])
    }
    async fn synthesize(&self, text: &str, voice: &VoiceId) -> anyhow::Result<AudioData> {
        self.calls
            .lock()
            .unwrap()
            .push((text.to_string(), voice.clone()));
        Ok(AudioData {
            bytes: Bytes::from_static(b"\x00"),
            format: AudioFormat::Mp3,
            sample_rate: 22050,
        })
    }
}

/// Drive `handle_lane_message` over a sequence of lane IDs and return
/// (synth_calls, utterances_queued).
async fn run_sequence(
    config: &Config,
    messages: &[(&str, &str)],
) -> (Vec<(String, String)>, usize) {
    let (provider, calls) = RecordingProvider::new();
    let provider: Box<dyn TtsProvider> = Box::new(provider);
    let (audio_tx, mut audio_rx) = mpsc::channel(64);
    let mut last_lane: Option<String> = None;

    for (lane_id, text) in messages {
        handle_lane_message(
            LaneMessage {
                lane_id: (*lane_id).into(),
                text: (*text).into(),
            },
            &mut last_lane,
            provider.as_ref(),
            config,
            &audio_tx,
        )
        .await
        .expect("queue should be alive");
    }
    drop(audio_tx);

    let mut queued = 0;
    while audio_rx.recv().await.is_some() {
        queued += 1;
    }
    let recorded = calls.lock().unwrap().clone();
    (recorded, queued)
}

#[tokio::test]
async fn announces_on_every_lane_switch_not_just_first_time() {
    let cfg = Config::default();
    // Sequence: A, A, B, A — switches at positions 0, 2, 3
    let seq = [
        ("lane-a", "hello from A"),
        ("lane-a", "more from A"),
        ("lane-b", "B speaks"),
        ("lane-a", "A speaks again"),
    ];
    let (calls, queued) = run_sequence(&cfg, &seq).await;

    // 4 messages + 3 announcements (at switch points 0, 2, 3) = 7 synth calls,
    // and 7 utterances should hit the audio queue.
    assert_eq!(
        calls.len(),
        7,
        "expected 4 messages + 3 switch announcements; got {calls:?}"
    );
    assert_eq!(queued, 7);

    // Announcement texts land in the call log immediately before their message.
    assert_eq!(calls[0].0, "Regarding lane-a.");
    assert_eq!(calls[1].0, "hello from A");
    assert_eq!(calls[2].0, "more from A"); // no switch -> no announcement
    assert_eq!(calls[3].0, "Regarding lane-b.");
    assert_eq!(calls[4].0, "B speaks");
    assert_eq!(calls[5].0, "Regarding lane-a.");
    assert_eq!(calls[6].0, "A speaks again");
}

#[tokio::test]
async fn zero_announcements_when_feature_disabled() {
    let mut cfg = Config::default();
    cfg.daemon.announce_lane_on_switch = false;

    let seq = [
        ("lane-a", "hello"),
        ("lane-b", "world"),
        ("lane-a", "again"),
    ];
    let (calls, queued) = run_sequence(&cfg, &seq).await;

    assert_eq!(
        calls.len(),
        3,
        "no announcements expected; got {:?}",
        calls.iter().map(|(t, _)| t).collect::<Vec<_>>()
    );
    assert_eq!(queued, 3);
    assert_eq!(calls[0].0, "hello");
    assert_eq!(calls[1].0, "world");
    assert_eq!(calls[2].0, "again");
}

#[tokio::test]
async fn uses_rule_name_over_lane_id_when_rule_present() {
    let mut cfg = Config::default();
    cfg.lanes.rules.insert(
        "dev-repo".into(),
        LaneRule {
            name: "Project Alpha".into(),
            voice: "nova".into(),
        },
    );

    let seq = [("dev-repo", "hi")];
    let (calls, _) = run_sequence(&cfg, &seq).await;

    assert_eq!(calls[0].0, "Regarding Project Alpha.");
}

#[tokio::test]
async fn prefix_synthesizes_in_the_lane_voice_not_the_default() {
    // Per-lane mode with a rule: the announcement should speak in the
    // rule's voice (not voice.default) so it matches the utterance that follows.
    let mut cfg = Config::default();
    cfg.voice.default = "nova".into();
    cfg.lanes.rules.insert(
        "dev-repo".into(),
        LaneRule {
            name: "Project".into(),
            voice: "onyx".into(),
        },
    );

    let seq = [("dev-repo", "hi")];
    let (calls, _) = run_sequence(&cfg, &seq).await;

    assert_eq!(calls[0].0, "Regarding Project.");
    assert_eq!(calls[0].1, "onyx", "prefix should use lane's rule voice");
    assert_eq!(calls[1].1, "onyx", "utterance should match");
}

#[test]
fn render_intro_substitutes_name() {
    assert_eq!(render_intro("Regarding {name}.", "alpha"), "Regarding alpha.");
}

#[test]
fn render_intro_strips_control_characters() {
    // Embedded \r and \x1b should become spaces, then trim() removes edge
    // whitespace introduced by the substitution.
    assert_eq!(
        render_intro("Regarding {name}.", "alpha\r\x1bbeta"),
        "Regarding alpha  beta."
    );
}

#[test]
fn render_intro_without_placeholder_returns_literal() {
    // Missing {name} → template passes through verbatim. User config quirk, not ours to fix.
    assert_eq!(render_intro("Hello world.", "alpha"), "Hello world.");
}
