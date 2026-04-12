use super::lane::{LaneMessage, LaneWatcher};
use crate::audio::LaneId;
use crate::config::types::Config;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct DirectoryWatcher {
    lanes_dir: PathBuf,
    config: Config,
    lane_tx: mpsc::Sender<LaneMessage>,
}

impl DirectoryWatcher {
    pub fn new(
        lanes_dir: PathBuf,
        config: Config,
        lane_tx: mpsc::Sender<LaneMessage>,
    ) -> Self {
        Self {
            lanes_dir,
            config,
            lane_tx,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!(dir = %self.lanes_dir.display(), "Directory watcher started");

        std::fs::create_dir_all(&self.lanes_dir)?;

        // Channel for notify events
        let (notify_tx, mut notify_rx) = mpsc::channel::<Event>(100);

        // notify runs its callback on a blocking thread
        let notify_tx_clone = notify_tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    // Use blocking_send since we're in a sync callback on a notify thread
                    let _ = notify_tx_clone.blocking_send(event);
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&self.lanes_dir, RecursiveMode::NonRecursive)?;

        while let Some(event) = notify_rx.recv().await {
            if matches!(event.kind, EventKind::Create(_)) {
                for path in &event.paths {
                    if path.extension().is_some_and(|e| e == "log") {
                        self.spawn_lane_watcher(path.clone());
                    }
                }
            }
        }

        Ok(())
    }

    fn spawn_lane_watcher(&self, path: PathBuf) {
        let lane_id = Self::lane_id_from_path(&path);
        debug!(lane = %lane_id, path = %path.display(), "New lane detected");

        let mut lane_watcher = LaneWatcher::new(
            lane_id.clone(),
            path,
            self.lane_tx.clone(),
            &self.config.parsing.tool_pattern,
            self.config.parsing.speakability_threshold,
            self.config.queue.coalesce_window_ms,
        );

        tokio::spawn(async move {
            if let Err(e) = lane_watcher.run().await {
                warn!(lane = %lane_id, error = %e, "Lane watcher exited with error");
            }
        });
    }

    /// Extract the lane ID from a log filename.
    /// Format: `<dirname>-<timestamp>.log` -> return `<dirname>`
    pub fn lane_id_from_path(path: &Path) -> LaneId {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        if let Some(dash_pos) = stem.rfind('-') {
            stem[..dash_pos].to_string()
        } else {
            stem.to_string()
        }
    }

    /// Resolve the voice name for a given lane ID, based on config rules + fallback.
    pub fn voice_for_lane(&self, lane_id: &str) -> String {
        // Check lane rules - match by directory suffix or rule name
        for (dir, rule) in &self.config.lanes.rules {
            if dir.ends_with(lane_id) || rule.name == lane_id {
                return rule.voice.clone();
            }
        }
        // Fallback to default voice
        self.config.voice.default.clone()
    }
}
