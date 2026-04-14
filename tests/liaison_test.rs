//! Covers the provider-agnostic parts of the liaison layer: prompt
//! rendering, response parsing, and the factory. The actual HTTP
//! calls are exercised end-to-end manually with real API keys.

use loquitor::liaison::prompt::{parse_response, render_user_prompt, SYSTEM_PROMPT};
use loquitor::liaison::{create_provider, TurnContext};

fn ctx(log: &str) -> TurnContext<'_> {
    TurnContext {
        cleaned_log: log,
        max_output_tokens: 120,
    }
}

// --- Prompt rendering ---

#[test]
fn system_prompt_contains_core_contract() {
    // Load-bearing substrings: if any of these drift the summaries
    // will sound different or stop being single-sentence.
    assert!(SYSTEM_PROMPT.contains("ONE spoken sentence"));
    assert!(SYSTEM_PROMPT.contains("under 40 words"));
    assert!(SYSTEM_PROMPT.contains("waiting"));
}

#[test]
fn user_prompt_is_the_cleaned_log() {
    // The user half of the prompt is intentionally unadorned — the
    // LLM sees the turn buffer verbatim. Lane context and the
    // "Regarding X." prefix are added downstream, not here.
    let rendered = render_user_prompt(&ctx("⏺ ran cargo test; 5 passed"));
    assert_eq!(rendered, "⏺ ran cargo test; 5 passed");
}

// --- Response parsing ---

#[test]
fn trims_whitespace() {
    let s = parse_response("   Finished the refactor. Waiting for you to review.   \n\n");
    assert_eq!(s.text, "Finished the refactor. Waiting for you to review.");
}

#[test]
fn passes_plain_text_through() {
    let s = parse_response("Ran 12 tests. Waiting for you to approve the PR.");
    assert_eq!(s.text, "Ran 12 tests. Waiting for you to approve the PR.");
}

#[test]
fn empty_reply_is_empty_summary() {
    // Defensive: if the model returns nothing, we return empty text.
    // The worker's canned-fallback path handles errors; this is for
    // the model-returned-but-blank edge.
    let s = parse_response("");
    assert_eq!(s.text, "");
}

// --- Factory routing ---

#[test]
fn create_provider_builds_anthropic() {
    let p = create_provider("anthropic", "fake-key", "claude-haiku-4-5").unwrap();
    assert_eq!(p.name(), "anthropic");
}

#[test]
fn create_provider_builds_openai() {
    let p = create_provider("openai", "fake", "gpt-4o-mini").unwrap();
    assert_eq!(p.name(), "openai");
}

#[test]
fn create_provider_builds_minimax() {
    let p = create_provider("minimax", "fake", "MiniMax-Text-01").unwrap();
    assert_eq!(p.name(), "minimax");
}

#[test]
fn create_provider_rejects_unknown() {
    let err = create_provider("gemini-pro", "k", "m").err().unwrap();
    assert!(err.to_string().contains("Unknown liaison provider"));
}
