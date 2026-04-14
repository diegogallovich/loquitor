//! Regex-based secret scrubber applied to turn-buffer text before it is
//! shipped to a cloud LLM. The goal is harm reduction, not perfection —
//! users who need a hard privacy guarantee should route the liaison
//! through a local provider (`ollama` or an `openai_compat` pointed at
//! `127.0.0.1`), which bypasses the scrubber entirely.

use regex::Regex;
use std::sync::LazyLock;

const REDACTED: &str = "[REDACTED]";

/// Each pattern captures one well-known secret shape emitted by common
/// tooling. The list is intentionally short — false positives (mangling
/// legitimate content) degrade summary quality, so we only include
/// patterns that are both high-precision and high-value.
///
/// Ordering matters only for debugging symmetry: more specific prefixes
/// come before more generic ones (e.g. `sk-ant-` before `sk-`).
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let raw = [
        // Anthropic API keys — specific prefix first so it's never shadowed
        // by the generic `sk-` below.
        r"sk-ant-[A-Za-z0-9_\-]{20,}",
        // OpenAI / MiniMax / other `sk-`-prefixed platform keys.
        r"sk-[A-Za-z0-9_\-]{20,}",
        // GitHub personal / OAuth / server tokens (ghp_, gho_, ghs_, ghu_, ghr_).
        r"gh[pousr]_[A-Za-z0-9]{30,}",
        // AWS access key id.
        r"AKIA[0-9A-Z]{16}",
        // Google API key (AIza…) — length-bounded so we don't catch
        // legitimate 30-char identifiers.
        r"AIza[0-9A-Za-z_\-]{35}",
        // Google OAuth access tokens.
        r"ya29\.[0-9A-Za-z_\-]{20,}",
        // Generic JWT — three base64url segments joined by dots.
        r"eyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+",
        // Bearer tokens in HTTP-like contexts. We require both the literal
        // "Bearer " prefix and a substantial token body to avoid redacting
        // prose that happens to contain the word.
        r"Bearer\s+[A-Za-z0-9_\.\-]{20,}",
    ];
    raw.iter()
        .map(|p| Regex::new(p).expect("scrub pattern must compile"))
        .collect()
});

pub struct ScrubResult {
    pub text: String,
    pub redaction_count: usize,
}

/// Replace every match of every pattern with `[REDACTED]`. Returns the
/// scrubbed text and the total number of matches across all patterns —
/// used for observability logging (we never log the matched content).
pub fn scrub(input: &str) -> ScrubResult {
    let mut text = input.to_string();
    let mut total = 0usize;
    for pat in PATTERNS.iter() {
        let count = pat.find_iter(&text).count();
        if count > 0 {
            total += count;
            text = pat.replace_all(&text, REDACTED).into_owned();
        }
    }
    ScrubResult {
        text,
        redaction_count: total,
    }
}

/// Convenience helper when the count is not interesting.
pub fn scrub_text(input: &str) -> String {
    scrub(input).text
}
