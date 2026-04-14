//! Shared prompt assembly + response parsing for liaison providers.
//! Every provider uses the same system/user prompt pair so the spoken
//! output stays consistent regardless of which LLM backs it.

use super::{TurnContext, TurnSummary, Urgency};

pub const SYSTEM_PROMPT: &str = "You summarize terminal output from Claude Code sessions for a voice assistant. \
Output exactly ONE spoken sentence, present tense, under 40 words. \
Start with \"In {lane_name}: \" (or \"Error in {lane_name}: \" on failures). \
End with a prompt like \"Waiting for you to …\". \
Do not read code verbatim. Do not list file paths unless the path is the point. \
Respond with JSON only, in the form: {\"urgency\": \"normal\"|\"needs_action\"|\"error\", \"text\": \"…\"}";

/// Render the user-facing half of the prompt. Kept pure so tests can
/// assert on the substitution without mocking HTTP.
pub fn render_user_prompt(ctx: &TurnContext<'_>) -> String {
    let hint = ctx.working_dir_hint.unwrap_or("(unknown)");
    format!(
        "Lane: {lane}\nProject hint: {hint}\n--- Terminal output ---\n{log}\n--- End ---",
        lane = ctx.lane_name,
        hint = hint,
        log = ctx.cleaned_log,
    )
}

/// Parse the LLM's JSON-shaped response. Robust to:
///   - stray prose before/after the JSON object (some models add a
///     chat wrapper even when told not to)
///   - missing `urgency` (defaults to Normal)
///   - malformed JSON (falls back to treating the whole reply as `text`)
///
/// Never returns an error — the caller always gets a speakable summary.
pub fn parse_response(lane_name: &str, raw: &str) -> TurnSummary {
    if let Some(json) = extract_json_object(raw) {
        if let Ok(parsed) = serde_json::from_str::<RawResponse>(json) {
            return TurnSummary {
                text: ensure_prefixed(lane_name, parsed.text.trim(), parsed.urgency),
                urgency: parsed.urgency,
            };
        }
    }

    // Fallback: speak whatever the model gave us verbatim. Trim to be polite,
    // but don't reject — a degraded notification beats silence.
    TurnSummary {
        text: ensure_prefixed(lane_name, raw.trim(), Urgency::Normal),
        urgency: Urgency::Normal,
    }
}

/// Ensure every summary opens with "In {lane}: " (or "Error in {lane}: ")
/// even if the model forgot. Belt-and-suspenders — the prompt already asks
/// for this, but a malformed response shouldn't lose the lane context that
/// the listener depends on.
fn ensure_prefixed(lane: &str, text: &str, urgency: Urgency) -> String {
    let expected = match urgency {
        Urgency::Error => format!("Error in {lane}:"),
        _ => format!("In {lane}:"),
    };
    if text.starts_with(&expected) {
        text.to_string()
    } else {
        format!("{expected} {text}")
    }
}

/// Locate the first `{…}` JSON object in a string. Models sometimes prefix
/// their JSON with a chat preamble ("Here's the summary:") — we don't want
/// to reject those responses, just strip the wrapping.
fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(&s[start..=end])
    } else {
        None
    }
}

#[derive(serde::Deserialize)]
struct RawResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    urgency: Urgency,
}
