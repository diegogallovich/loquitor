//! The worker that turns a finished Claude turn (raw cleaned text)
//! into a short spoken notification. Sits between `LaneWatcher` and
//! the TTS synth stage:
//!
//! ```text
//!  LaneWatcher → TurnReady → LiaisonWorker → SummarizedTurn → TtsWorker
//! ```
//!
//! The worker is a single serial task. Parallelising LLM calls is
//! possible but the downstream audio queue is serial anyway, so
//! concurrent summaries would just queue up. A single task keeps the
//! behaviour predictable and rate-limit-friendly.

use crate::config::{self, types::Config};
use crate::liaison::scrub::scrub_text;
use crate::liaison::{LiaisonProvider, TurnContext};
use crate::watcher::lane::TurnReady;
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
    /// The Instant at which the underlying turn ended. Used downstream
    /// by the AudioQueue's stale-drop check so notifications about
    /// turns that completed long ago (say, during a daemon pause)
    /// don't play out-of-context.
    pub ended_at: Instant,
}

/// Run the worker loop. Exits when `turn_rx` closes (all senders
/// dropped) or when `summary_tx`'s receiver is gone.
pub async fn run(
    config: Config,
    liaison: Box<dyn LiaisonProvider>,
    mut turn_rx: mpsc::Receiver<TurnReady>,
    summary_tx: mpsc::Sender<SummarizedTurn>,
) {
    info!(
        provider = %liaison.name(),
        "Liaison worker started"
    );
    while let Some(turn) = turn_rx.recv().await {
        let summary = handle_turn(turn, &config, liaison.as_ref()).await;
        if summary_tx.send(summary).await.is_err() {
            info!("Summary receiver dropped, liaison worker exiting");
            break;
        }
    }
    info!("Liaison worker exiting");
}

/// One-turn pipeline in a testable shape. Pure(-ish): the only I/O is
/// the LLM call inside `liaison.summarize_turn`, which tests stub out
/// via a mock provider.
pub async fn handle_turn(
    turn: TurnReady,
    config: &Config,
    liaison: &dyn LiaisonProvider,
) -> SummarizedTurn {
    let lane_name = config::resolve_lane_name(config, &turn.lane_id);

    // Apply the secret scrubber for any cloud provider. Local providers
    // (ollama, localhost openai_compat) skip it: the content never leaves
    // the machine, and scrubbing can degrade summary quality.
    let cleaned_log = if should_scrub(config) {
        scrub_text(&turn.turn_text)
    } else {
        turn.turn_text
    };

    let ctx = TurnContext {
        lane_name: &lane_name,
        working_dir_hint: None,
        cleaned_log: &cleaned_log,
        max_output_tokens: config.liaison.max_output_tokens,
    };

    let text = match liaison.summarize_turn(&ctx).await {
        Ok(summary) => summary.text,
        Err(e) => {
            warn!(
                lane = %turn.lane_id,
                error = %e,
                "Liaison summarization failed; using canned fallback"
            );
            canned_fallback(&lane_name, &e)
        }
    };

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

/// Produce a short human-readable notification for the failure path.
/// The *exact* error goes to the debug log; the spoken version stays
/// terse so the listener still gets the "Claude is waiting" signal.
fn canned_fallback(lane_name: &str, err: &anyhow::Error) -> String {
    let reason = classify_error(err);
    format!("Claude is waiting in {lane_name}. Summary unavailable — {reason}.")
}

/// Map a rust/HTTP error into one of a handful of human-readable
/// phrases. Matched against lowercased debug/display strings of the
/// error because we don't have structured error kinds from `anyhow`.
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
