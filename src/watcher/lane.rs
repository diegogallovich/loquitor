use super::idle::{self, IdleCfg, IdleState, LineClass, TurnEvent};
use super::parser::strip_ansi;
use crate::audio::LaneId;
use anyhow::Result;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Signal emitted by a LaneWatcher when it detects that Claude has
/// finished its current turn and is waiting for user input. Consumed
/// by the liaison worker (PR5), which calls the summarizer LLM and
/// forwards a one-sentence notification to TTS.
pub struct TurnReady {
    pub lane_id: LaneId,
    /// Full cleaned, ANSI-stripped terminal text for the turn. May be
    /// prefixed with a truncation marker if the underlying buffer
    /// overran `IdleCfg`'s bound and we dropped lines from the front.
    pub turn_text: String,
    pub started_at: Instant,
    pub ended_at: Instant,
    /// True if at least one line was dropped from the front due to the
    /// byte cap being exceeded.
    pub truncated: bool,
}

/// Watches a single log file. Cleans each incoming line, feeds the
/// idle-detector state machine, and accumulates cleaned content into a
/// bounded per-lane turn buffer. On `TurnEvent::TurnEnded` the buffer
/// is flushed as a `TurnReady` event on `turn_tx`.
pub struct LaneWatcher {
    lane_id: LaneId,
    file_path: PathBuf,
    turn_tx: mpsc::Sender<TurnReady>,
    idle_state: IdleState,
    idle_cfg: IdleCfg,
    /// Maximum bytes of accumulated turn text. When exceeded, lines are
    /// dropped from the front until the buffer is back under 80% of the
    /// cap. A banner is added when flushed.
    max_bytes: usize,
    // --- per-turn mutable state ---
    buffer: VecDeque<String>,
    buffer_bytes: usize,
    truncated: bool,
    turn_started: Option<Instant>,
}

impl LaneWatcher {
    pub fn new(
        lane_id: LaneId,
        file_path: PathBuf,
        turn_tx: mpsc::Sender<TurnReady>,
        idle_cfg: IdleCfg,
        max_bytes: usize,
    ) -> Self {
        Self {
            lane_id,
            file_path,
            turn_tx,
            idle_state: IdleState::new(),
            idle_cfg,
            max_bytes,
            buffer: VecDeque::new(),
            buffer_bytes: 0,
            truncated: false,
            turn_started: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            lane = %self.lane_id,
            path = %self.file_path.display(),
            "Lane watcher started"
        );

        // Wait up to 5s for the file to appear.
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
        reader.seek(SeekFrom::End(0)).await?;

        let poll_interval = Duration::from_millis(100);
        let mut leftover: Vec<u8> = Vec::new();
        let mut read_chunk = vec![0u8; 8192];

        loop {
            let mut read_any = false;
            loop {
                match reader.read(&mut read_chunk).await {
                    Ok(0) => break,
                    Ok(n) => {
                        read_any = true;
                        leftover.extend_from_slice(&read_chunk[..n]);

                        while let Some(nl) = leftover.iter().position(|&b| b == b'\n' || b == b'\r')
                        {
                            let line_bytes: Vec<u8> = leftover.drain(..=nl).collect();
                            let raw = String::from_utf8_lossy(&line_bytes);
                            if let Some(event) = self.process_line(&raw) {
                                if self.turn_tx.send(event).await.is_err() {
                                    info!(
                                        lane = %self.lane_id,
                                        "Turn receiver dropped, lane watcher exiting"
                                    );
                                    return Ok(());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(lane = %self.lane_id, error = %e, "Error reading lane file");
                        return Err(e.into());
                    }
                }
            }

            // After draining whatever's available, ask the idle
            // detector if enough quiet time has passed to call the
            // turn ended. This is the *primary* idle signal — it
            // fires regardless of whether the TUI ever emits a clean
            // box-drawing-only prompt frame, which the real Claude
            // Code prompt does NOT (it includes status text like
            // "Opus 4.6 (1M context) · 8% ctx · main").
            if let Some(event) = self.tick(Instant::now()) {
                if self.turn_tx.send(event).await.is_err() {
                    info!(
                        lane = %self.lane_id,
                        "Turn receiver dropped, lane watcher exiting"
                    );
                    return Ok(());
                }
            }

            if !read_any {
                sleep(poll_interval).await;
            }
        }
    }

    /// Time-based check called every poll iteration. Returns a
    /// `TurnReady` when `idle::tick` decides the turn is over (no
    /// content for `quiet_threshold`, OR `turn_max_duration` cap).
    fn tick(&mut self, now: Instant) -> Option<TurnReady> {
        let event = idle::tick(&mut self.idle_state, now, &self.idle_cfg);
        if event == Some(idle::TurnEvent::TurnEnded) {
            self.flush(now)
        } else {
            None
        }
    }

    /// Drain the per-turn buffer into a `TurnReady`. Called from both
    /// the line-driven (`process_line_at`) and the inactivity-driven
    /// (`tick`) paths so the flush logic is single-source-of-truth.
    fn flush(&mut self, now: Instant) -> Option<TurnReady> {
        if self.buffer.is_empty() && !self.truncated && self.turn_started.is_none() {
            // Nothing to flush — defensive against double-fire.
            return None;
        }
        let started_at = self.turn_started.take().unwrap_or(now);
        let truncated = self.truncated;
        self.truncated = false;
        let body: String = std::mem::take(&mut self.buffer).into_iter().collect();
        self.buffer_bytes = 0;
        let turn_text = if truncated {
            format!("[earlier output truncated]\n{body}")
        } else {
            body
        };
        debug!(
            lane = %self.lane_id,
            bytes = turn_text.len(),
            truncated,
            "Turn flushing"
        );
        Some(TurnReady {
            lane_id: self.lane_id.clone(),
            turn_text,
            started_at,
            ended_at: now,
            truncated,
        })
    }

    /// Apply one raw line to the watcher's state and optionally return a
    /// `TurnReady` if this line closed a turn. Delegates to
    /// `process_line_at(raw, Instant::now())` — the separate entry point
    /// lets tests fabricate timestamps without sleeping.
    pub fn process_line(&mut self, raw: &str) -> Option<TurnReady> {
        self.process_line_at(raw, Instant::now())
    }

    /// Test-friendly variant: takes the current `Instant` as a parameter
    /// so unit tests can walk synthetic timelines without real time.
    pub fn process_line_at(&mut self, raw: &str, now: Instant) -> Option<TurnReady> {
        let cleaned = strip_ansi(raw);
        let class = idle::classify(&cleaned);

        let is_content = matches!(class, LineClass::Content);
        let starting_new_turn = is_content && matches!(self.idle_state, IdleState::Idle);

        if starting_new_turn {
            self.turn_started = Some(now);
            // Defensive reset — should already be clean, but if the
            // previous flush failed mid-way we want a fresh slate.
            self.buffer.clear();
            self.buffer_bytes = 0;
            self.truncated = false;
        }

        // Prompt frames are TUI chrome — they don't belong in the text
        // the LLM summarises. Skip them. Blank content lines are
        // preserved because Claude uses them for paragraph breaks.
        if is_content {
            self.append_to_buffer(&cleaned);
        }

        let event = idle::feed(&mut self.idle_state, class, now, &self.idle_cfg);
        if event == Some(TurnEvent::TurnEnded) {
            self.flush(now)
        } else {
            None
        }
    }

    /// Append one cleaned line to the buffer. When the resulting byte
    /// total exceeds `max_bytes`, drop whole lines from the front until
    /// we're back under 80% of the cap. Marks `truncated = true`.
    fn append_to_buffer(&mut self, cleaned: &str) {
        // Append a newline so the flushed text reconstructs readable
        // line breaks for the LLM.
        let line = format!("{cleaned}\n");
        self.buffer_bytes += line.len();
        self.buffer.push_back(line);

        if self.buffer_bytes > self.max_bytes {
            self.truncated = true;
            let target = self.max_bytes * 8 / 10;
            while self.buffer_bytes > target && self.buffer.len() > 1 {
                if let Some(dropped) = self.buffer.pop_front() {
                    self.buffer_bytes -= dropped.len();
                }
            }
        }
    }
}
