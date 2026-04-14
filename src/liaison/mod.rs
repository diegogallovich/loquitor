//! Liaison layer — takes a buffered "turn" of raw Claude Code output
//! and produces a single short natural-language summary that the TTS
//! layer speaks as a smart notification. Mirrors `crate::tts` in shape:
//! one trait, several provider implementations, one `create_provider`
//! factory driven by config.

pub mod anthropic;
pub mod minimax;
pub mod openai;
pub mod prompt;
pub mod scrub;

use anyhow::Result;
use async_trait::async_trait;

/// A bounded snapshot of one Claude Code turn — what the user just
/// watched Claude do, in cleaned form. Built by the `LaneWatcher` on
/// idle detection and consumed once by a `LiaisonProvider`.
pub struct TurnContext<'a> {
    /// Post-scrub, ANSI-stripped terminal text. May be prefixed with a
    /// truncation marker if the original turn buffer exceeded its cap.
    pub cleaned_log: &'a str,
    /// Soft cap on the summary length. Cheap providers tend to ignore
    /// it; stricter billing providers use it to bound per-turn cost.
    pub max_output_tokens: u32,
}

/// What the liaison returns. A plain string that the TTS layer speaks.
/// The lane announcement is prepended by the liaison worker — not the
/// LLM and not the provider.
pub struct TurnSummary {
    pub text: String,
}

/// Trait implemented by every LLM backend that can produce turn
/// summaries. Mirrors `crate::tts::TtsProvider` in structure so the
/// daemon can treat both layers uniformly.
#[async_trait]
pub trait LiaisonProvider: Send + Sync {
    /// Short id used in logs and wizard menus ("anthropic", "openai", …).
    fn name(&self) -> &str;
    /// Summarize one turn. Implementations own the HTTP call.
    async fn summarize_turn(&self, ctx: &TurnContext<'_>) -> Result<TurnSummary>;
}

/// Build a `LiaisonProvider` from config values. Factored out so the
/// wizard, daemon, and CLI all resolve the same way.
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
