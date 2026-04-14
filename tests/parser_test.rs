//! After the v0.2.0 pivot this module tests only the pure text-cleanup
//! helpers that survived the parser gut: ANSI stripping, cursor-forward
//! expansion, and the colour-aware narrative-marker detector. The per-
//! line speakability / narrative-block / tool-call gating is gone —
//! that logic moved into the liaison LLM layer.

use loquitor::watcher::parser::{is_narrative_marker, is_tool_marker, strip_ansi};

// --- ANSI stripping + cursor-forward expansion ---

#[test]
fn strip_ansi_removes_color_escapes() {
    let out = strip_ansi("\x1b[1;34mhello\x1b[0m world");
    assert_eq!(out, "hello world");
}

#[test]
fn strip_ansi_expands_cursor_forward_to_spaces() {
    // Claude Code emits `ESC[1C` (cursor-forward-1) between words instead
    // of literal spaces in some TUI modes. Without expansion, stripping
    // escapes would glue words together.
    let raw = "Congrats\x1b[1C—\x1b[1Cthat's\x1b[1Ca\x1b[1Cgreat\x1b[1Cmilestone.";
    assert_eq!(strip_ansi(raw), "Congrats — that's a great milestone.");
}

#[test]
fn strip_ansi_passes_through_utf8() {
    // Multi-byte chars (⏺, em-dash, non-Latin) must survive intact.
    let out = strip_ansi("\x1b[38;2;0;0;0m⏺\x1b[39m Søren — 日本");
    assert_eq!(out, "⏺ Søren — 日本");
}

// --- Narrative-marker colour detection ---

#[test]
fn bare_marker_is_narrative() {
    assert!(is_narrative_marker("⏺ I'll start by reading the config."));
}

#[test]
fn reset_escape_before_marker_is_narrative() {
    assert!(is_narrative_marker("\x1b[0m⏺ Tests passing."));
}

#[test]
fn bold_only_before_marker_is_narrative() {
    // Bold (1) with no colour is attribute-only, treated as narrative.
    assert!(is_narrative_marker("\x1b[1m⏺ Important."));
}

#[test]
fn truecolor_black_is_narrative() {
    // Claude Code uses (0,0,0) RGB for narrative ⏺ markers.
    assert!(is_narrative_marker("\x1b[38;2;0;0;0m⏺\x1b[39m Speaks."));
}

#[test]
fn truecolor_non_black_is_not_narrative() {
    // Orange RGB — tool-call colour in Claude Code.
    assert!(!is_narrative_marker("\x1b[38;2;245;149;117m⏺\x1b[39m Bash(x)"));
}

#[test]
fn color_256_black_is_narrative() {
    assert!(is_narrative_marker("\x1b[38;5;0m⏺ Complete."));
}

#[test]
fn color_256_non_black_is_not_narrative() {
    assert!(!is_narrative_marker("\x1b[38;5;33m⏺ Read(x)"));
}

#[test]
fn basic_blue_marker_is_not_narrative() {
    assert!(!is_narrative_marker("\x1b[34m⏺\x1b[0m Bash(x)"));
}

#[test]
fn line_without_marker_is_not_narrative() {
    assert!(!is_narrative_marker("just plain output"));
    assert!(!is_narrative_marker(""));
}

// --- is_tool_marker ---

#[test]
fn blue_marker_is_tool_marker() {
    assert!(is_tool_marker("\x1b[34m⏺\x1b[0m Bash(x)"));
}

#[test]
fn narrative_marker_is_not_tool_marker() {
    assert!(!is_tool_marker("⏺ narrative"));
}

#[test]
fn line_without_marker_is_not_tool_marker() {
    assert!(!is_tool_marker("plain text"));
}
