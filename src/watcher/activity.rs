//! Claude-activity detector.
//!
//! The fundamental problem with a pure inactivity timer: Claude can
//! pause for many benign reasons mid-turn (tool execution, API
//! round-trips, long thinking phases), and those pauses look identical
//! to "done, waiting for you" from a byte-level perspective.
//!
//! This module adds a **positive** signal: while Claude owns the
//! turn, it continuously redraws a status indicator — a spinner,
//! a "thinking…" line, or a progress counter. The indicator is
//! present during tool execution (the spinner keeps ticking) AND
//! during model thinking. It disappears only when Claude hands the
//! turn back to the user.
//!
//! By tracking when we last saw any activity indicator in the raw
//! byte stream, the LaneWatcher can require BOTH "no new bytes" AND
//! "no activity indicator" before declaring idle. This cuts mid-turn
//! false fires without making the inactivity timer so long that real
//! idle detection feels laggy.
//!
//! ## How to use
//!
//! 1. Watch `~/.config/loquitor/lanes/<current>.log` while using
//!    Claude Code.
//! 2. Note the short natural-language substrings that appear
//!    repeatedly *only while Claude is working*. Ignore stuff that
//!    persists through idle (status bars, the input-box hint line).
//! 3. Add them to `ACTIVITY_INDICATORS` below.
//!
//! Substrings are matched on the RAW bytes (not ANSI-stripped),
//! case-sensitive, as byte-level needles. They only need to appear
//! somewhere in a recent read chunk to count.

/// Substrings that only appear in the lane log while Claude is
/// actively working on a turn — thinking, streaming a response,
/// or executing a tool.
///
/// TODO(diego): populate this. Starter candidates I've observed in
/// live logs are uncommented; add or remove based on what you see in
/// your sessions. Keep each entry short (3–20 bytes) — long strings
/// are expensive to scan for in every read burst and get brittle if
/// Claude shifts a word.
///
/// Guardrails:
///   - Match substrings that appear in the SPINNER/THINKING area
///     only, not in the persistent status bar (context %, model
///     name, git branch).
///   - Prefer lowercase prefixes so capitalised variants don't
///     require duplicates, OR add both if you want to be explicit.
///   - Don't match common English words — a session where Claude
///     writes the word "thinking" in its response would permanently
///     block idle detection.
pub const ACTIVITY_INDICATORS: &[&[u8]] = &[
    // "Spinning" appears in the tool-execution indicator.
    b"Spinning",
    // "thinking with" appears while Claude is in a thinking phase.
    // The trailing space is intentional — avoids matching the word
    // "thinking" embedded in narrative output Claude might produce.
    b"thinking with",
];

/// Return true if any indicator substring is present in `chunk`.
/// Linear scan — fine for the read-burst sizes we see (~8 KB).
pub fn contains_activity_indicator(chunk: &[u8]) -> bool {
    ACTIVITY_INDICATORS
        .iter()
        .any(|needle| contains_needle(chunk, needle))
}

/// Boyer-Moore is overkill here. Naive byte search is
/// trivially fast at these sizes.
fn contains_needle(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_indicator() {
        assert!(contains_activity_indicator(
            b"some prefix Spinning ... some suffix"
        ));
        assert!(contains_activity_indicator(b"thinking with high effort"));
    }

    #[test]
    fn missing_means_false() {
        assert!(!contains_activity_indicator(b"ordinary narrative output"));
        assert!(!contains_activity_indicator(b""));
    }

    #[test]
    fn case_sensitive() {
        assert!(!contains_activity_indicator(b"SPINNING"));
    }
}
