use loquitor::liaison::scrub::{scrub, scrub_text};

#[test]
fn redacts_openai_style_key() {
    let out = scrub_text(
        "curl -H 'Authorization: Bearer sk-proj-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789' x.com",
    );
    assert!(
        !out.contains("sk-proj"),
        "OpenAI-style sk- key leaked: {out}"
    );
    assert!(out.contains("[REDACTED]"));
}

#[test]
fn redacts_anthropic_specific_prefix_before_generic() {
    // sk-ant- should hit the specific pattern first; both patterns remove it.
    let out = scrub_text("export ANTHROPIC_API_KEY=sk-ant-api03-abcdef1234567890ABCDEFGHIJ");
    assert!(!out.contains("sk-ant"), "Anthropic key survived: {out}");
}

#[test]
fn redacts_github_tokens() {
    let inputs = [
        "token ghp_abcdefghijklmnopqrstuvwxyz0123456789 active",
        "GHO_abcdefghijklmnopqrstuvwxyz0123456789", // case-insensitive not required: they're case-specific
        "ghs_abcdefghijklmnopqrstuvwxyz0123456789",
    ];
    for raw in inputs {
        let out = scrub_text(raw);
        // Only lowercase-prefix tokens are in-scope (ghp_/gho_/ghs_/ghu_/ghr_).
        if raw.starts_with("GHO_") {
            // Uppercase token should NOT be redacted — it's not a real GitHub shape.
            assert!(
                out.contains("GHO_"),
                "should not touch uppercase variant: {out}"
            );
        } else {
            assert!(!out.contains("abcdefghij"), "token survived: {out}");
            assert!(out.contains("[REDACTED]"));
        }
    }
}

#[test]
fn redacts_aws_access_key() {
    let out = scrub_text("AKIAIOSFODNN7EXAMPLE is a key");
    assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(out.contains("[REDACTED]"));
}

#[test]
fn redacts_google_api_key() {
    // AIza + 35 url-safe chars = 39 total
    let out = scrub_text("key=AIzaSyA-abcdefghijklmnopqrstuvwxyz01234567 end");
    assert!(!out.contains("AIzaSy"), "Google key leaked: {out}");
}

#[test]
fn redacts_jwt() {
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.sflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let out = scrub_text(&format!("cookie: session={jwt}"));
    assert!(!out.contains("eyJhbGci"), "JWT leaked: {out}");
}

#[test]
fn redacts_bearer_tokens() {
    let out = scrub_text("Authorization: Bearer abcdefghijklmnopqrstuvwxyz12345");
    assert!(!out.contains("abcdefghij"));
}

#[test]
fn preserves_ordinary_prose() {
    let input = "In marketing-site: finished the pricing-page refactor and 3 tests pass. Waiting for you to approve the PR.";
    let out = scrub_text(input);
    assert_eq!(input, out, "ordinary prose should not trigger any pattern");
}

#[test]
fn scrub_result_counts_every_match() {
    let input = "sk-aaaaaaaaaaaaaaaaaaaaaaa1 and sk-bbbbbbbbbbbbbbbbbbbbbb2 and ghp_1234567890123456789012345678901 are three";
    let res = scrub(input);
    assert_eq!(
        res.redaction_count, 3,
        "expected 3 redactions (2 sk + 1 ghp), got {}: {}",
        res.redaction_count, res.text
    );
    assert!(!res.text.contains("sk-a"));
    assert!(!res.text.contains("ghp_1"));
}

#[test]
fn zero_matches_returns_unchanged() {
    let input = "just some harmless English prose with no secrets";
    let res = scrub(input);
    assert_eq!(res.redaction_count, 0);
    assert_eq!(res.text, input);
}
