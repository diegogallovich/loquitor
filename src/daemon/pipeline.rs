use crate::audio::{self, Utterance};
use crate::config::{self, types::Config};
use crate::tts::{self, TtsProvider};
use crate::watcher::directory::DirectoryWatcher;
use crate::watcher::lane::LaneMessage;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Signal that the audio queue is gone; the TTS worker loop should exit.
#[derive(Debug)]
pub struct SendFailed;

/// Handle one incoming LaneMessage: synthesize it via the TTS provider and
/// push to the audio queue. Returns `Err(SendFailed)` iff the audio queue
/// receiver has been dropped.
///
/// NOTE (v0.2.0 pivot): the lane-switch "Regarding X" announcement that
/// used to live here has been removed. Under the new design, the liaison
/// LLM produces a single summary per turn that already opens with
/// "In {lane_name}: …", so the announcement is redundant. This function
/// is a shim during the pivot — PR5 replaces it entirely with a worker
/// that reads pre-summarised `SummarizedTurn` events instead of raw
/// `LaneMessage` lines.
pub async fn handle_lane_message(
    msg: LaneMessage,
    provider: &dyn TtsProvider,
    config: &Config,
    audio_tx: &mpsc::Sender<Utterance>,
) -> Result<(), SendFailed> {
    let voice = config::resolve_voice(config, &msg.lane_id);

    match provider.synthesize(&msg.text, &voice).await {
        Ok(audio) => {
            let utter = Utterance {
                lane_id: msg.lane_id,
                audio,
                enqueued_at: Instant::now(),
                text: msg.text,
            };
            if audio_tx.send(utter).await.is_err() {
                return Err(SendFailed);
            }
        }
        Err(e) => warn!(error = %e, text = %msg.text, "TTS synthesis failed"),
    }
    Ok(())
}

/// Run the full daemon pipeline.
/// This function blocks for the lifetime of the daemon.
pub async fn run(config: Config, lanes_dir: PathBuf) -> Result<()> {
    info!("Starting daemon pipeline");

    let provider = tts::create_provider(
        &config.tts.name,
        &config.tts.api_key,
        &config.tts.model,
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

    // TTS worker — serialises synthesis and emits utterances into the audio queue.
    while let Some(msg) = lane_rx.recv().await {
        if handle_lane_message(msg, provider.as_ref(), &config, &audio_tx)
            .await
            .is_err()
        {
            warn!("Audio queue dropped, exiting TTS worker");
            break;
        }
    }

    // Cleanup - wait for background tasks to complete
    info!("TTS worker exiting, cleaning up");
    dir_handle.abort();
    audio_handle.abort();

    Ok(())
}
