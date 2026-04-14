//! Covers the provider-agnostic parts of the liaison layer: prompt
//! rendering, response parsing (including fallback paths for malformed
//! model output), and the factory. The Anthropic HTTP call itself is
//! verified end-to-end manually with a real API key in PR5.

use loquitor::liaison::prompt::{parse_response, render_user_prompt, SYSTEM_PROMPT};
use loquitor::liaison::{create_provider, TurnContext, Urgency};

fn ctx<'a>(lane: &'a str, log: &'a str) -> TurnContext<'a> {
    TurnContext {
        lane_name: lane,
        working_dir_hint: Some("/Users/diego/Dev/project"),
        cleaned_log: log,
        max_output_tokens: 120,
    }
}

// --- Prompt rendering ---

#[test]
fn system_prompt_contains_core_contract() {
    // These substrings are load-bearing — the summary format depends on them.
    assert!(SYSTEM_PROMPT.contains("ONE spoken sentence"));
    assert!(SYSTEM_PROMPT.contains("urgency"));
    assert!(SYSTEM_PROMPT.contains("Waiting for you to"));
}

#[test]
fn user_prompt_substitutes_lane_and_log() {
    let c = ctx("marketing-site", "⏺ ran cargo test; 5 passed");
    let rendered = render_user_prompt(&c);
    assert!(rendered.contains("Lane: marketing-site"));
    assert!(rendered.contains("⏺ ran cargo test; 5 passed"));
    assert!(rendered.contains("/Users/diego/Dev/project"));
}

#[test]
fn user_prompt_falls_back_when_hint_is_missing() {
    let c = TurnContext {
        lane_name: "foo",
        working_dir_hint: None,
        cleaned_log: "stuff",
        max_output_tokens: 120,
    };
    let rendered = render_user_prompt(&c);
    assert!(rendered.contains("Project hint: (unknown)"));
}

// --- Response parsing ---

#[test]
fn parses_well_formed_json() {
    let raw = r#"{"urgency": "needs_action", "text": "In acme: finished the refactor. Waiting for you to review the diff."}"#;
    let summary = parse_response("acme", raw);
    assert_eq!(summary.urgency, Urgency::NeedsAction);
    assert_eq!(
        summary.text,
        "In acme: finished the refactor. Waiting for you to review the diff."
    );
}

#[test]
fn parses_json_wrapped_in_chat_preamble() {
    // Some models add "Here's your summary:" before emitting JSON.
    // We strip the prose before parsing.
    let raw = r#"Here's the result: {"urgency": "normal", "text": "In foo: done. Waiting for you to approve."}"#;
    let summary = parse_response("foo", raw);
    assert_eq!(summary.urgency, Urgency::Normal);
    assert!(summary.text.starts_with("In foo:"));
}

#[test]
fn urgency_error_prefix_normalized() {
    // If the model says urgency=error but forgot to prefix "Error in", we add it.
    let raw = r#"{"urgency": "error", "text": "tests crashed unexpectedly. Waiting for you to investigate."}"#;
    let summary = parse_response("ci", raw);
    assert_eq!(summary.urgency, Urgency::Error);
    assert!(
        summary.text.starts_with("Error in ci:"),
        "error prefix missing: {}",
        summary.text
    );
}

#[test]
fn missing_urgency_defaults_to_normal() {
    let raw = r#"{"text": "In bar: progress made. Waiting for you to continue."}"#;
    let summary = parse_response("bar", raw);
    assert_eq!(summary.urgency, Urgency::Normal);
}

#[test]
fn malformed_json_falls_back_to_raw_text() {
    // If JSON parsing fails, we still produce a speakable summary rather
    // than erroring out — a degraded notification is better than silence.
    let raw = "Claude finished and is waiting for tests.";
    let summary = parse_response("sbx", raw);
    assert_eq!(summary.urgency, Urgency::Normal);
    assert!(
        summary.text.starts_with("In sbx:"),
        "lane prefix should be added on fallback: {}",
        summary.text
    );
    assert!(summary.text.contains("Claude finished"));
}

#[test]
fn already_prefixed_text_is_not_double_prefixed() {
    let raw = r#"{"urgency": "normal", "text": "In lane-x: all good. Waiting for you to proceed."}"#;
    let summary = parse_response("lane-x", raw);
    assert_eq!(summary.text.matches("In lane-x:").count(), 1);
}

// --- Factory ---

#[test]
fn create_provider_builds_anthropic() {
    let p = create_provider("anthropic", "fake-key", "claude-haiku-4-5").unwrap();
    assert_eq!(p.name(), "anthropic");
}

#[test]
fn create_provider_rejects_unknown() {
    // Using `.err().unwrap()` because `Box<dyn LiaisonProvider>` doesn't
    // implement Debug (required by `.unwrap_err()`).
    let err = create_provider("gemini-pro", "k", "m").err().unwrap();
    assert!(err.to_string().contains("Unknown liaison provider"));
}
