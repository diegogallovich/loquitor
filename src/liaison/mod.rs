//! Liaison layer — takes a buffered "turn" of raw Claude Code output and
//! produces a single short natural-language summary that the TTS layer
//! speaks as a smart notification. Mirrors the `tts` module in shape:
//! one trait, several provider implementations, one `create_provider`
//! factory driven by config.

pub mod anthropic;
pub mod minimax;
pub mod openai;
pub mod prompt;
pub mod scrub;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A bounded snapshot of one Claude Code turn — what the user just watched
/// Claude do, in raw-ish form, plus enough metadata for the summary to be
/// contextual. Built by the `LaneWatcher` on idle-detection and consumed
/// exactly once by a `LiaisonProvider`.
pub struct TurnContext<'a> {
    /// Human-readable lane name (from config rule, or cwd basename fallback).
    pub lane_name: &'a str,
    /// Optional hint about the working directory, useful when the lane name
    /// isn't self-explanatory (e.g. "dotfiles" lane working on a subpath).
    pub working_dir_hint: Option<&'a str>,
    /// Post-scrub, ANSI-stripped terminal text. May be prefixed with a
    /// truncation marker if the original turn buffer exceeded its cap.
    pub cleaned_log: &'a str,
    /// Provider-agnostic cap for the summary output. Cheap providers ignore
    /// this and just generate a sentence; stricter billing providers use it
    /// to bound cost per turn.
    pub max_output_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    /// Claude finished and is ready — e.g. a question or a review request.
    #[default]
    Normal,
    /// Claude explicitly needs a decision from the user to continue.
    NeedsAction,
    /// A tool call or the agent itself failed; the summary should flag it.
    Error,
}

/// What a `LiaisonProvider` returns. `text` is the exact string sent to TTS.
pub struct TurnSummary {
    pub text: String,
    pub urgency: Urgency,
}

/// Trait implemented by every LLM backend that can produce turn summaries.
/// Mirrors `crate::tts::TtsProvider` in structure so the daemon can treat
/// both layers uniformly: one provider, one call per unit of work, `Send +
/// Sync` for `tokio::spawn`.
#[async_trait]
pub trait LiaisonProvider: Send + Sync {
    /// Short id used in logs and wizard menus ("anthropic", "openai", …).
    fn name(&self) -> &str;
    /// Summarize one turn. Implementations own the HTTP call; cancellation
    /// on drop is the caller's responsibility via `tokio::time::timeout`.
    async fn summarize_turn(&self, ctx: &TurnContext<'_>) -> Result<TurnSummary>;
}

/// Build a `LiaisonProvider` from config values. Factored out so the wizard,
/// daemon, and CLI all resolve the same way.
pub fn create_provider(
    name: &str,
    api_key: &str,
    model: &str,
) -> Result<Box<dyn LiaisonProvider>> {
    match name {
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(api_key, model))),
        "openai" => Ok(Box::new(openai::OpenAiProvider::new(api_key, model))),
        "minimax" => Ok(Box::new(minimax::MiniMaxProvider::new(api_key, model))),
        other => anyhow::bail!("Unknown liaison provider: {other}"),
    }
}
