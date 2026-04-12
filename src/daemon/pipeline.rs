use crate::audio::{self, Utterance};
use crate::config::types::Config;
use crate::tts;
use crate::watcher::directory::DirectoryWatcher;
use crate::watcher::lane::LaneMessage;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Run the full daemon pipeline.
/// This function blocks for the lifetime of the daemon.
pub async fn run(config: Config, lanes_dir: PathBuf) -> Result<()> {
    info!("Starting daemon pipeline");

    let provider = tts::create_provider(
        &config.provider.name,
        &config.provider.api_key,
        &config.provider.model,
    )?;

    // Channels between pipeline stages
    let (lane_tx, mut lane_rx) = mpsc::channel::<LaneMessage>(100);
    let (audio_tx, audio_queue) = audio::create_queue(50, config.queue.stale_threshold_secs);

    // Directory watcher (spawns lane watchers on demand)
    let dir_watcher = DirectoryWatcher::new(lanes_dir, config.clone(), lane_tx);
    let dir_handle = tokio::spawn(async move {
        if let Err(e) = dir_watcher.run().await {
            warn!(error = %e, "Directory watcher error");
        }
    });

    // Audio queue (runs the playback loop)
    let mut audio_queue = audio_queue;
    let audio_handle = tokio::spawn(async move {
        audio_queue.run().await;
    });

    // TTS worker - receives (lane, text) from lane watchers, synthesizes, sends to audio queue
    let default_voice = config.voice.default.clone();
    while let Some(msg) = lane_rx.recv().await {
        // For v0.1.0, all lanes use the default voice.
        // TODO: resolve per-lane voice via DirectoryWatcher::voice_for_lane when exposed.
        let voice = default_voice.clone();

        match provider.synthesize(&msg.text, &voice).await {
            Ok(audio_data) => {
                let utterance = Utterance {
                    lane_id: msg.lane_id,
                    audio: audio_data,
                    enqueued_at: Instant::now(),
                    text: msg.text,
                };
                if audio_tx.send(utterance).await.is_err() {
                    warn!("Audio queue dropped, exiting TTS worker");
                    break;
                }
            }
            Err(e) => {
                warn!(error = %e, text = %msg.text, "TTS synthesis failed");
            }
        }
    }

    // Cleanup - wait for background tasks to complete
    info!("TTS worker exiting, cleaning up");
    dir_handle.abort();
    audio_handle.abort();

    Ok(())
}
