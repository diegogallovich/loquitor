use super::idle::IdleCfg;
use super::lane::{LaneWatcher, TurnReady};
use crate::audio::LaneId;
use crate::config::types::Config;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct DirectoryWatcher {
    lanes_dir: PathBuf,
    config: Config,
    turn_tx: mpsc::Sender<TurnReady>,
    /// Tracks which .log paths already have a LaneWatcher spawned, so
    /// duplicate Create events from notify (macOS FSEvents fires one
    /// per `script(1)` open + chmod + first-write) don't spawn 4
    /// watchers per lane — observed in real logs as identical
    /// "Lane watcher started" lines milliseconds apart.
    spawned: Mutex<HashSet<PathBuf>>,
}

impl DirectoryWatcher {
    pub fn new(lanes_dir: PathBuf, config: Config, turn_tx: mpsc::Sender<TurnReady>) -> Self {
        Self {
            lanes_dir,
            config,
            turn_tx,
            spawned: Mutex::new(HashSet::new()),
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
        // Dedupe — see field comment on `spawned`.
        if !self.spawned.lock().unwrap().insert(path.clone()) {
            debug!(path = %path.display(), "Duplicate Create event for known lane file; ignoring");
            return;
        }

        let lane_id = Self::lane_id_from_path(&path);
        debug!(lane = %lane_id, path = %path.display(), "New lane detected");

        let idle_cfg = IdleCfg {
            confirm_frames: self.config.daemon.idle_confirm_frames,
            min_silence: Duration::from_millis(self.config.daemon.idle_min_silence_ms),
            turn_max_duration: Duration::from_secs(self.config.daemon.turn_max_duration_secs),
            quiet_threshold: Duration::from_millis(self.config.daemon.idle_quiet_ms),
        };

        let mut lane_watcher = LaneWatcher::new(
            lane_id.clone(),
            path,
            self.turn_tx.clone(),
            idle_cfg,
            self.config.daemon.turn_buffer_max_bytes,
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
}
