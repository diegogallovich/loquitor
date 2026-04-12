use super::parser::Parser;
use crate::audio::LaneId;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Message emitted when the parser yields a speakable utterance.
pub struct LaneMessage {
    pub lane_id: LaneId,
    pub text: String,
}

/// Watches a single log file and emits LaneMessages for speakable content.
/// Tails from end-of-file — only processes lines written after the watcher starts.
pub struct LaneWatcher {
    lane_id: LaneId,
    file_path: PathBuf,
    tx: mpsc::Sender<LaneMessage>,
    parser: Parser,
    coalesce_window: Duration,
}

impl LaneWatcher {
    pub fn new(
        lane_id: LaneId,
        file_path: PathBuf,
        tx: mpsc::Sender<LaneMessage>,
        tool_pattern: &str,
        speakability_threshold: f64,
        coalesce_window_ms: u64,
    ) -> Self {
        Self {
            lane_id,
            file_path,
            tx,
            parser: Parser::new(tool_pattern, speakability_threshold),
            coalesce_window: Duration::from_millis(coalesce_window_ms),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            lane = %self.lane_id,
            path = %self.file_path.display(),
            "Lane watcher started"
        );

        // Wait for file to appear (up to 5 seconds)
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

        // Seek to end — only process new content
        reader.seek(SeekFrom::End(0)).await?;

        let poll_interval = Duration::from_millis(100);
        let mut coalesce_buf: Vec<String> = Vec::new();
        let mut last_content_time: Option<tokio::time::Instant> = None;

        loop {
            // Read any available lines
            let mut read_any = false;
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // No new data
                    Ok(_) => {
                        read_any = true;
                        if let Some(text) = self.parser.parse_line(&line) {
                            coalesce_buf.push(text);
                            last_content_time = Some(tokio::time::Instant::now());
                            debug!(
                                lane = %self.lane_id,
                                buffered = coalesce_buf.len(),
                                "Added to coalesce buffer"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(lane = %self.lane_id, error = %e, "Error reading lane file");
                        return Err(e.into());
                    }
                }
            }

            // Decide whether to flush coalesce buffer
            if !coalesce_buf.is_empty() {
                if let Some(last_time) = last_content_time {
                    let elapsed = last_time.elapsed();
                    if elapsed >= self.coalesce_window {
                        // Quiet window elapsed — flush
                        let combined = coalesce_buf.join(" ");
                        coalesce_buf.clear();
                        last_content_time = None;

                        debug!(
                            lane = %self.lane_id,
                            text = %combined,
                            "Flushing coalesced utterance"
                        );

                        if self
                            .tx
                            .send(LaneMessage {
                                lane_id: self.lane_id.clone(),
                                text: combined,
                            })
                            .await
                            .is_err()
                        {
                            info!(lane = %self.lane_id, "Receiver dropped, lane watcher exiting");
                            return Ok(());
                        }
                    }
                }
            }

            // Sleep before next poll (unless we just read data and might have more)
            if !read_any {
                sleep(poll_interval).await;
            }
        }
    }
}
