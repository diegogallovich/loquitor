use crate::audio;
use crate::config::types::Config;
use crate::watcher::directory::DirectoryWatcher;
use crate::watcher::lane::TurnReady;
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Run the full daemon pipeline.
///
/// PR4 state: the pipeline reads `TurnReady` events from the lane
/// watchers and logs them. PR5 wires in the liaison worker between
/// `turn_rx` and the TTS layer so that every turn end produces exactly
/// one spoken notification.
pub async fn run(config: Config, lanes_dir: PathBuf) -> Result<()> {
    info!("Starting daemon pipeline");

    // Channels.  `turn_tx` is what every LaneWatcher publishes on; in
    // PR5 we'll split this into `turn_rx -> liaison -> summary_tx ->
    // TTS worker -> audio_tx -> AudioQueue`. For now turn_rx feeds a
    // passthrough logger and audio_tx is unused.
    let (turn_tx, mut turn_rx) = mpsc::channel::<TurnReady>(16);
    let (_audio_tx, audio_queue) = audio::create_queue(50, config.queue.stale_threshold_secs);

    let dir_watcher = DirectoryWatcher::new(lanes_dir, config.clone(), turn_tx);
    let dir_handle = tokio::spawn(async move {
        if let Err(e) = dir_watcher.run().await {
            warn!(error = %e, "Directory watcher error");
        }
    });

    let mut audio_queue = audio_queue;
    let audio_handle = tokio::spawn(async move {
        audio_queue.run().await;
    });

    // Passthrough logger — replaced by the liaison worker in PR5.
    while let Some(turn) = turn_rx.recv().await {
        info!(
            lane = %turn.lane_id,
            bytes = turn.turn_text.len(),
            truncated = turn.truncated,
            duration_ms = turn.ended_at.duration_since(turn.started_at).as_millis() as u64,
            "Turn received (PR4 — liaison worker arrives in PR5)"
        );
    }

    info!("Turn consumer exited, cleaning up");
    dir_handle.abort();
    audio_handle.abort();

    Ok(())
}
