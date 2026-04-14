//! The worker that turns a finished Claude turn (raw cleaned text)
//! into a short spoken notification. Sits between `LaneWatcher` and
//! the TTS synth stage:
//!
//! ```text
//!  LaneWatcher → TurnReady → LiaisonWorker → SummarizedTurn → TtsWorker
//! ```
//!
//! Summarization runs **concurrently** per turn — when two lanes
//! finish within the same second, their LLM calls fire in parallel.
//! A `FuturesOrdered` preserves FIFO order on the way out so the
//! audio queue always plays them in the order their turns ended.
//!
//! The deterministic announcement `"Regarding {lane_name}. "` is
//! prepended here (not asked of the LLM) so every notification opens
//! with the lane the listener needs to hear.

use crate::config::{self, types::Config};
use crate::liaison::scrub::scrub_text;
use crate::liaison::{LiaisonProvider, TurnContext};
use crate::watcher::lane::TurnReady;
use futures::stream::{FuturesOrdered, StreamExt};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// What the worker emits per turn. The TTS stage consumes this and
/// synthesises `text` as speech. `lane_id` rides along so per-lane
/// voice resolution (`config::resolve_voice`) still works at synth time.
pub struct SummarizedTurn {
    pub lane_id: String,
    pub lane_name: String,
    pub text: String,
    /// When the underlying turn ended — used by the audio queue's
    /// stale-drop check so notifications about long-stale turns don't
    /// play out of context.
    pub ended_at: Instant,
}

/// Run the worker. Reads `TurnReady` off `turn_rx`, spawns a
/// concurrent summarization task per turn, and forwards the resulting
/// `SummarizedTurn` to `summary_tx` in arrival order.
pub async fn run(
    config: Config,
    liaison: Box<dyn LiaisonProvider>,
    mut turn_rx: mpsc::Receiver<TurnReady>,
    summary_tx: mpsc::Sender<SummarizedTurn>,
) {
    info!(provider = %liaison.name(), "Liaison worker started");

    // Arc so spawned summary tasks can share the provider + config
    // without cloning them per turn.
    let liaison: Arc<dyn LiaisonProvider> = Arc::from(liaison);
    let config = Arc::new(config);

    // Bounded because FuturesOrdered doesn't back-pressure on its own
    // and we don't want an unbounded number of in-flight LLM calls if
    // turns arrive faster than audio can play them.
    let mut pending: FuturesOrdered<tokio::task::JoinHandle<SummarizedTurn>> =
        FuturesOrdered::new();

    loop {
        tokio::select! {
            // New turn → kick off an LLM call in a spawned task.
            // `biased` makes the select prefer incoming turns over
            // draining completed ones — small latency win.
            biased;
            maybe_turn = turn_rx.recv() => {
                match maybe_turn {
                    Some(turn) => {
                        let cfg = config.clone();
                        let li = liaison.clone();
                        pending.push_back(tokio::spawn(async move {
                            handle_turn(turn, &cfg, li.as_ref()).await
                        }));
                    }
                    None => break, // senders dropped — drain and exit
                }
            }
            // A summary finished — forward in order.
            Some(join) = pending.next(), if !pending.is_empty() => {
                match join {
                    Ok(summary) => {
                        if summary_tx.send(summary).await.is_err() {
                            info!("Summary receiver dropped, liaison worker exiting");
                            return;
                        }
                    }
                    Err(e) => warn!(error = %e, "Summarization task panicked"),
                }
            }
        }
    }

    // Drain whatever's still mid-flight before exiting cleanly.
    while let Some(join) = pending.next().await {
        if let Ok(summary) = join {
            if summary_tx.send(summary).await.is_err() {
                break;
            }
        }
    }
    info!("Liaison worker exiting");
}

/// One-turn pipeline in a testable shape. Builds the `TurnContext`,
/// optionally scrubs secrets, calls the LLM, and prepends the lane
/// announcement. Returns a `SummarizedTurn` — **never errors**: the
/// failure path produces a canned fallback so the listener still gets
/// the "Claude is waiting" signal.
pub async fn handle_turn(
    turn: TurnReady,
    config: &Config,
    liaison: &dyn LiaisonProvider,
) -> SummarizedTurn {
    let lane_name = config::resolve_lane_name(config, &turn.lane_id);

    let cleaned_log = if should_scrub(config) {
        scrub_text(&turn.turn_text)
    } else {
        turn.turn_text
    };

    let ctx = TurnContext {
        cleaned_log: &cleaned_log,
        max_output_tokens: config.liaison.max_output_tokens,
    };

    let summary_text = match liaison.summarize_turn(&ctx).await {
        Ok(summary) => summary.text,
        Err(e) => {
            warn!(
                lane = %turn.lane_id,
                error = %e,
                "Liaison summarization failed; using canned fallback"
            );
            format!("Summary unavailable — {}.", classify_error(&e))
        }
    };

    // Deterministic lane announcement. Always "Regarding {lane}. " so
    // concurrent lanes don't confuse the listener. Handled here, not
    // in the prompt, so the LLM can't forget or reword it.
    let text = format!("Regarding {lane_name}. {summary_text}");

    SummarizedTurn {
        lane_id: turn.lane_id,
        lane_name,
        text,
        ended_at: turn.ended_at,
    }
}

/// Decide whether to run the secret scrubber over the turn buffer
/// before shipping it to the liaison. Skipped for local providers
/// because no content crosses the machine boundary.
pub fn should_scrub(config: &Config) -> bool {
    match config.liaison.name.as_str() {
        "ollama" => false,
        "openai_compat" => {
            let url = config.liaison.base_url.to_lowercase();
            let is_local = url.contains("127.0.0.1")
                || url.contains("localhost")
                || url.contains(".local");
            if is_local {
                false
            } else {
                config.liaison.scrub_secrets
            }
        }
        _ => config.liaison.scrub_secrets,
    }
}

/// Map an anyhow error to a short human phrase. Matched against the
/// lowercased debug string because `anyhow` doesn't give us typed
/// kinds, and this is pronunciation-friendly enough for TTS.
pub fn classify_error(err: &anyhow::Error) -> &'static str {
    let msg = format!("{err:?}").to_lowercase();
    if msg.contains("401") || msg.contains("unauthorized") || msg.contains("authentication") {
        "authentication error"
    } else if msg.contains("403") || msg.contains("forbidden") {
        "permission denied"
    } else if msg.contains("429") || msg.contains("rate limit") || msg.contains("rate-limit") {
        "rate limit"
    } else if msg.contains("timeout") || msg.contains("timed out") {
        "timeout"
    } else if msg.contains("connection")
        || msg.contains("dns")
        || msg.contains("network")
        || msg.contains("connect error")
    {
        "network error"
    } else if msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("504") {
        "a provider outage"
    } else {
        "an unknown error"
    }
}
