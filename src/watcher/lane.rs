use super::idle::{self, IdleCfg, IdleState};
use super::parser::strip_ansi;
use crate::audio::LaneId;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Legacy message shape kept for the duration of the v0.2.0 pivot.
/// PR4 replaces this with `TurnReady { lane_id, lane_name, turn_text,
/// started_at, ended_at, truncated }` once the turn buffer is in place.
pub struct LaneMessage {
    pub lane_id: LaneId,
    pub text: String,
}

/// Watches a single log file. During the pivot (PR3 → PR4) it just
/// feeds cleaned lines into the idle state machine and logs turn-end
/// events. PR4 adds the per-lane turn buffer and emits `TurnReady` on
/// each `TurnEvent::TurnEnded`.
pub struct LaneWatcher {
    lane_id: LaneId,
    file_path: PathBuf,
    /// Kept during the pivot so `DirectoryWatcher` keeps the same
    /// `mpsc::Sender<LaneMessage>` type it used in v0.1.0. Nothing is
    /// sent here in PR3 — PR4 swaps this for a `Sender<TurnReady>`.
    #[allow(dead_code)]
    tx: mpsc::Sender<LaneMessage>,
    idle_state: IdleState,
    idle_cfg: IdleCfg,
}

impl LaneWatcher {
    pub fn new(
        lane_id: LaneId,
        file_path: PathBuf,
        tx: mpsc::Sender<LaneMessage>,
        idle_cfg: IdleCfg,
    ) -> Self {
        Self {
            lane_id,
            file_path,
            tx,
            idle_state: IdleState::new(),
            idle_cfg,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            lane = %self.lane_id,
            path = %self.file_path.display(),
            "Lane watcher started"
        );

        // Wait up to 5s for the file to appear (the shell hook creates
        // it just before spawning `script(1)`).
        let mut attempts = 0;
        while !self.file_path.exists() {
            if attempts >= 50 {
                anyhow::bail!(
                    "Lane file did not appear within 5 seconds: {}",
                    self.file_path.display()
                );
            }
            sleep(Duration::from_millis(100)).await;
            attempts += 1;
        }

        let file = File::open(&self.file_path).await?;
        let mut reader = BufReader::new(file);

        // Seek to EOF — only process new content written after we start.
        reader.seek(SeekFrom::End(0)).await?;

        let poll_interval = Duration::from_millis(100);
        // Leftover = pre-newline bytes or partial UTF-8 characters
        // carried across reads so we don't split lines / graphemes on
        // an arbitrary buffer boundary.
        let mut leftover: Vec<u8> = Vec::new();
        let mut read_chunk = vec![0u8; 8192];

        loop {
            let mut read_any = false;

            // Drain everything the file has for us right now.
            loop {
                match reader.read(&mut read_chunk).await {
                    Ok(0) => break,
                    Ok(n) => {
                        read_any = true;
                        leftover.extend_from_slice(&read_chunk[..n]);

                        // Split on \n or \r. TUIs like Claude Code use
                        // bare CR to redraw a line in-place; treating
                        // each redraw as its own logical line is what
                        // lets the idle detector see stable prompt
                        // frames.
                        while let Some(nl) =
                            leftover.iter().position(|&b| b == b'\n' || b == b'\r')
                        {
                            let line_bytes: Vec<u8> = leftover.drain(..=nl).collect();
                            // Lossy UTF-8: control bytes from `script`
                            // captures become U+FFFD, which the cleaned
                            // string handles fine.
                            let raw = String::from_utf8_lossy(&line_bytes);
                            self.process_line(&raw);
                        }
                    }
                    Err(e) => {
                        warn!(lane = %self.lane_id, error = %e, "Error reading lane file");
                        return Err(e.into());
                    }
                }
            }

            if !read_any {
                sleep(poll_interval).await;
            }
        }
    }

    /// Apply the v0.2.0 read path to one raw line: clean ANSI, classify,
    /// feed the idle state machine, log any turn-end event. PR4 expands
    /// this to also accumulate cleaned text into a bounded turn buffer
    /// and emit `TurnReady` on `TurnEvent::TurnEnded`.
    fn process_line(&mut self, raw: &str) {
        let cleaned = strip_ansi(raw);
        let class = idle::classify(&cleaned);
        let now = tokio::time::Instant::now().into_std();
        if let Some(event) = idle::feed(&mut self.idle_state, class, now, &self.idle_cfg) {
            // During PR3 we just log. PR4 will flush the (not-yet-
            // existing) turn buffer to the downstream channel.
            info!(
                lane = %self.lane_id,
                event = ?event,
                "Turn end detected (PR3 — buffer + emission arrives in PR4)"
            );
        } else {
            debug!(
                lane = %self.lane_id,
                line_len = cleaned.len(),
                "Line processed"
            );
        }
    }
}
