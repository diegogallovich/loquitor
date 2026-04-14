//! Prompt and response handling shared across every liaison backend.
//! Deliberately minimal: the LLM receives a short system prompt plus
//! the raw cleaned log, and returns plain text. The lane announcement
//! ("Regarding {lane}. ") is prepended later by the liaison worker —
//! it's not the LLM's job to produce it.

use super::{TurnContext, TurnSummary};

pub const SYSTEM_PROMPT: &str =
    "You summarize terminal output from a Claude Code session for a voice assistant. \
Output exactly ONE spoken sentence, present tense, under 40 words. \
Describe what Claude finished doing and what it is waiting on the user for. \
Do not read code verbatim. Do not list file paths unless the path is the point.";

/// Render the user message for one turn. Pure — tests can assert on the
/// substitution without mocking HTTP.
pub fn render_user_prompt(ctx: &TurnContext<'_>) -> String {
    ctx.cleaned_log.to_string()
}

/// Turn a raw LLM reply into a `TurnSummary`. The liaison worker
/// will prepend the lane announcement before this hits TTS, so all we
/// need here is to trim whitespace and hand back a single sentence.
pub fn parse_response(raw: &str) -> TurnSummary {
    TurnSummary {
        text: raw.trim().to_string(),
    }
}
