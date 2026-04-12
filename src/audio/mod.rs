pub mod player;

use crate::tts::AudioData;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub type LaneId = String;

pub struct Utterance {
    pub lane_id: LaneId,
    pub audio: AudioData,
    pub enqueued_at: Instant,
    pub text: String,
}

pub struct AudioQueue {
    rx: mpsc::Receiver<Utterance>,
    stale_threshold: Duration,
}

impl AudioQueue {
    pub fn new(rx: mpsc::Receiver<Utterance>, stale_threshold_secs: u64) -> Self {
        Self {
            rx,
            stale_threshold: Duration::from_secs(stale_threshold_secs),
        }
    }

    pub async fn run(&mut self) {
        info!("Audio queue started");
        while let Some(utterance) = self.rx.recv().await {
            let age = utterance.enqueued_at.elapsed();
            if age > self.stale_threshold {
                debug!(
                    lane = %utterance.lane_id,
                    text = %utterance.text,
                    age_secs = age.as_secs(),
                    "Dropping stale utterance"
                );
                continue;
            }

            debug!(
                lane = %utterance.lane_id,
                text = %utterance.text,
                "Playing utterance"
            );

            // Offload blocking playback to a blocking thread so we don't stall the tokio runtime.
            let audio_clone = utterance.audio.clone();
            let result = tokio::task::spawn_blocking(move || player::play_audio(&audio_clone)).await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => warn!(error = %e, "Failed to play audio"),
                Err(join_err) => warn!(error = %join_err, "Playback task panicked"),
            }
        }
        info!("Audio queue stopped");
    }
}

/// Create a channel pair for the audio queue.
/// Returns (sender for TTS worker, AudioQueue to run in its own task).
pub fn create_queue(
    buffer_size: usize,
    stale_threshold_secs: u64,
) -> (mpsc::Sender<Utterance>, AudioQueue) {
    let (tx, rx) = mpsc::channel(buffer_size);
    let queue = AudioQueue::new(rx, stale_threshold_secs);
    (tx, queue)
}
