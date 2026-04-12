use loquitor::audio::{self, Utterance};
use loquitor::tts::{AudioData, AudioFormat};
use bytes::Bytes;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_stale_utterance_is_dropped() {
    let (tx, mut queue) = audio::create_queue(10, 1); // 1 second threshold for testing

    // Enqueue an utterance that's already 2 seconds old
    let old_utterance = Utterance {
        lane_id: "test".into(),
        audio: AudioData {
            bytes: Bytes::new(), // empty — would fail to play, but should be dropped before that
            format: AudioFormat::Mp3,
            sample_rate: 22050,
        },
        enqueued_at: Instant::now() - Duration::from_secs(2),
        text: "stale message".into(),
    };

    tx.send(old_utterance).await.unwrap();
    drop(tx); // Close sender so queue.run() will exit after processing

    // Queue should process (drop the stale item) and exit without error or attempted playback
    queue.run().await;
    // If we get here, the stale utterance was dropped — test passes
}

#[tokio::test]
async fn test_queue_exits_when_sender_dropped() {
    let (tx, mut queue) = audio::create_queue(10, 15);

    // Drop sender immediately — queue should exit cleanly with no utterances
    drop(tx);

    queue.run().await;
    // Test passes if we reach this line (queue.run() returned)
}

#[test]
fn test_create_queue_returns_matching_pair() {
    let (tx, _queue) = audio::create_queue(10, 15);
    // Channel capacity is 10
    assert_eq!(tx.capacity(), 10);
}
