use super::parser::Parser;
use crate::audio::LaneId;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader, SeekFrom};
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
        // Leftover bytes that are either pre-newline or a partial UTF-8 character at
        // the end of the last read. Carry them across reads so we don't split a line
        // or a multi-byte grapheme on an arbitrary buffer boundary.
        let mut leftover: Vec<u8> = Vec::new();
        let mut read_chunk = vec![0u8; 8192];

        loop {
            // Read any available bytes
            let mut read_any = false;
            loop {
                match reader.read(&mut read_chunk).await {
                    Ok(0) => break, // No new data
                    Ok(n) => {
                        read_any = true;
                        leftover.extend_from_slice(&read_chunk[..n]);

                        // Split on LF or CR (either line separator). TUIs like
                        // Claude Code use bare \r (carriage return) to redraw a line
                        // in-place — if we only split on \n we'd concatenate the
                        // redrawn chunks (narrative + status + prompt + box drawing)
                        // into one mega-line that poisons the speakability filter.
                        // Treating \r as a line break on its own splits each redraw
                        // frame into its own logical line.
                        while let Some(nl) = leftover
                            .iter()
                            .position(|&b| b == b'\n' || b == b'\r')
                        {
                            let line_bytes: Vec<u8> = leftover.drain(..=nl).collect();
                            // Lossy UTF-8 conversion so control-sequence bytes from `script`
                            // captures don't kill the watcher — invalid sequences become U+FFFD
                            // which the speakability filter drops cleanly.
                            let line = String::from_utf8_lossy(&line_bytes);
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
