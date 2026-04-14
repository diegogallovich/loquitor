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

/// Substitute `{name}` in the lane-intro template with a sanitized lane name.
/// Control characters in the name are replaced with spaces so TUI garbage
/// (e.g., stray escape bytes that slipped past the parser) can't poison the
/// TTS request body.
pub fn render_intro(template: &str, name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    template.replace("{name}", clean.trim())
}

/// Handle one incoming LaneMessage:
///   1. If the lane differs from the last one seen AND announcements are enabled,
///      synthesize the lane-intro and push it to the audio queue.
///   2. Synthesize the message itself and push it.
///
/// Updates `last_lane` *before* any synthesis, so a failed main-synth doesn't
/// cause a duplicate announcement on the next utterance from the same lane.
///
/// Returns `Err(SendFailed)` iff the audio queue receiver has been dropped —
/// the caller should exit the worker loop.
pub async fn handle_lane_message(
    msg: LaneMessage,
    last_lane: &mut Option<String>,
    provider: &dyn TtsProvider,
    config: &Config,
    audio_tx: &mpsc::Sender<Utterance>,
) -> Result<(), SendFailed> {
    let voice = config::resolve_voice(config, &msg.lane_id);

    let should_announce = config.daemon.announce_lane_on_switch
        && last_lane.as_deref() != Some(msg.lane_id.as_str());
    *last_lane = Some(msg.lane_id.clone());

    if should_announce {
        let lane_name = config::resolve_lane_name(config, &msg.lane_id);
        let prefix_text = render_intro(&config.daemon.lane_intro_template, &lane_name);
        match provider.synthesize(&prefix_text, &voice).await {
            Ok(audio) => {
                let utter = Utterance {
                    lane_id: msg.lane_id.clone(),
                    audio,
                    enqueued_at: Instant::now(),
                    text: prefix_text,
                };
                if audio_tx.send(utter).await.is_err() {
                    return Err(SendFailed);
                }
            }
            Err(e) => warn!(error = %e, lane = %msg.lane_id, "Lane intro synth failed; skipping"),
        }
    }

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

/// Signal that the audio queue is gone; the TTS worker loop should exit.
#[derive(Debug)]
pub struct SendFailed;

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

    // TTS worker — serializes synthesis and emits utterances (plus lane-intro
    // announcements on lane switches) into the audio queue.
    let mut last_lane: Option<String> = None;
    while let Some(msg) = lane_rx.recv().await {
        if handle_lane_message(msg, &mut last_lane, provider.as_ref(), &config, &audio_tx)
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
