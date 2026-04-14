use super::liaison_worker::{self, SummarizedTurn};
use crate::audio::{self, Utterance};
use crate::config::{self, types::Config};
use crate::liaison;
use crate::tts;
use crate::watcher::directory::DirectoryWatcher;
use crate::watcher::lane::TurnReady;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Run the full v0.2.0 daemon pipeline:
///
/// ```text
///   DirectoryWatcher  →  LaneWatcher(per lane)  →  TurnReady
///                                                       ↓
///                                                LiaisonWorker  →  SummarizedTurn
///                                                       ↓
///                                                 TTS synth  →  Utterance
///                                                       ↓
///                                                 AudioQueue  →  speaker
/// ```
pub async fn run(config: Config, lanes_dir: PathBuf) -> Result<()> {
    info!("Starting daemon pipeline");

    let tts_provider =
        tts::create_provider(&config.tts.name, &config.tts.api_key, &config.tts.model)?;
    let liaison_provider = liaison::create_provider(
        &config.liaison.name,
        &config.liaison.api_key,
        &config.liaison.model,
    )?;

    // Channels between stages
    let (turn_tx, turn_rx) = mpsc::channel::<TurnReady>(16);
    let (summary_tx, mut summary_rx) = mpsc::channel::<SummarizedTurn>(16);
    let (audio_tx, audio_queue) = audio::create_queue(50, config.queue.stale_threshold_secs);

    // Directory watcher — discovers new .log files and spawns LaneWatchers
    let dir_watcher = DirectoryWatcher::new(lanes_dir, config.clone(), turn_tx);
    let dir_handle = tokio::spawn(async move {
        if let Err(e) = dir_watcher.run().await {
            warn!(error = %e, "Directory watcher error");
        }
    });

    // Audio queue — serial playback worker
    let mut audio_queue = audio_queue;
    let audio_handle = tokio::spawn(async move {
        audio_queue.run().await;
    });

    // Liaison worker — consumes TurnReady, produces SummarizedTurn
    let liaison_config = config.clone();
    let liaison_handle = tokio::spawn(async move {
        liaison_worker::run(liaison_config, liaison_provider, turn_rx, summary_tx).await;
    });

    // TTS worker (this task) — consumes SummarizedTurn, synthesises
    // speech, and pushes onto the audio queue. Single sequential call
    // per summary so rate-limit and cost pressure stay predictable.
    while let Some(s) = summary_rx.recv().await {
        let voice = config::resolve_voice(&config, &s.lane_id);
        match tts_provider.synthesize(&s.text, &voice).await {
            Ok(audio) => {
                let utter = Utterance {
                    lane_id: s.lane_id,
                    audio,
                    enqueued_at: Instant::now(),
                    text: s.text,
                };
                if audio_tx.send(utter).await.is_err() {
                    warn!("Audio queue dropped, exiting TTS worker");
                    break;
                }
            }
            Err(e) => warn!(error = %e, text = %s.text, "TTS synthesis failed"),
        }
    }

    info!("TTS worker exiting, cleaning up");
    dir_handle.abort();
    audio_handle.abort();
    liaison_handle.abort();

    Ok(())
}
