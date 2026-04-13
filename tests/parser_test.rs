use loquitor::watcher::parser::Parser;

fn make_parser() -> Parser {
    Parser::new(
        r"^(Bash|Read|Edit|Write|Glob|Grep|Agent|Skill|TaskCreate|TaskUpdate|ToolSearch|WebFetch|WebSearch|NotebookEdit)\s*\(",
        0.6,
    )
}

// Regression test for a real Claude Code TUI line captured from `script`.
// Claude Code uses 24-bit truecolor SGR ("38;2;0;0;0" = RGB black) for narrative
// markers and `[1C` (cursor forward 1) instead of literal spaces between words.
#[test]
fn test_claude_code_truecolor_narrative_line() {
    let mut parser = make_parser();
    let raw = "\x1b[?2026l\x1b[?2026h\x1b[6A\x1b[38;2;0;0;0m⏺\x1b[1C\x1b[39mCongrats\x1b[1C—\x1b[1Cthat's\x1b[1Ca\x1b[1Creal\x1b[1Cmilestone.";
    let result = parser.parse_line(raw);
    assert_eq!(
        result,
        Some("Congrats — that's a real milestone.".into()),
        "Should recognize RGB-black ⏺ as narrative and expand [1C into spaces",
    );
}

#[test]
fn test_truecolor_non_black_is_skipped() {
    let mut parser = make_parser();
    // RGB(245, 149, 117) — an orange tool-call color Claude Code uses
    let raw = "\x1b[38;2;245;149;117m⏺\x1b[39m Bash(echo hi)";
    assert_eq!(parser.parse_line(raw), None);
}

#[test]
fn test_256_color_black_is_narrative() {
    let mut parser = make_parser();
    let raw = "\x1b[38;5;0m⏺ The build succeeded.";
    assert_eq!(
        parser.parse_line(raw),
        Some("The build succeeded.".into())
    );
}

#[test]
fn test_narrative_line_is_spoken() {
    let mut parser = make_parser();
    // Bare ⏺ with no color escape — default/narrative
    let result = parser.parse_line("⏺ I'll start by reading the configuration file.");
    assert_eq!(
        result,
        Some("I'll start by reading the configuration file.".into())
    );
}

#[test]
fn test_blue_marker_is_skipped() {
    let mut parser = make_parser();
    // Blue ⏺ (used for tool calls in Claude Code)
    let result = parser.parse_line("\x1b[34m⏺\x1b[0m Bash(cat config.toml)");
    assert_eq!(result, None);
}

#[test]
fn test_green_marker_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("\x1b[32m⏺\x1b[0m Edit(src/main.rs)");
    assert_eq!(result, None);
}

#[test]
fn test_red_marker_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("\x1b[31m⏺\x1b[0m Error output");
    assert_eq!(result, None);
}

#[test]
fn test_tool_call_text_is_skipped_by_regex() {
    let mut parser = make_parser();
    // Even if color were default (no escape), the regex safety net catches tool calls
    let result = parser.parse_line("⏺ Bash(cargo test)");
    assert_eq!(result, None);
}

#[test]
fn test_read_tool_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("⏺ Read(src/main.rs)");
    assert_eq!(result, None);
}

#[test]
fn test_agent_dispatch_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("⏺ Agent(description=\"Research\")");
    assert_eq!(result, None);
}

#[test]
fn test_task_create_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("⏺ TaskCreate(subject=\"Implement feature\")");
    assert_eq!(result, None);
}

#[test]
fn test_box_drawing_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("⏺ ┌──────────┐     ┌──────────┐");
    assert_eq!(result, None);
}

#[test]
fn test_code_block_is_skipped() {
    let mut parser = make_parser();
    assert_eq!(parser.parse_line("⏺ ```rust"), None); // opens
    assert_eq!(parser.parse_line("⏺ fn main() {}"), None); // inside
    assert_eq!(parser.parse_line("⏺ ```"), None); // closes
                                                  // Narrative after the block should be spoken again
    let result = parser.parse_line("⏺ The implementation is complete.");
    assert_eq!(result, Some("The implementation is complete.".into()));
}

#[test]
fn test_file_path_alone_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("⏺ /src/main.rs");
    assert_eq!(result, None);
}

#[test]
fn test_empty_marker_is_skipped() {
    let mut parser = make_parser();
    let result = parser.parse_line("⏺");
    assert_eq!(result, None);
}

#[test]
fn test_reset_escape_before_marker_is_narrative() {
    let mut parser = make_parser();
    // Reset code \x1b[0m before ⏺ means "default color" — narrative
    let result = parser.parse_line("\x1b[0m⏺ The tests are passing now.");
    assert_eq!(result, Some("The tests are passing now.".into()));
}

#[test]
fn test_bold_escape_before_marker_is_narrative() {
    let mut parser = make_parser();
    // Bold (\x1b[1m) without a specific color — narrative
    let result = parser.parse_line("\x1b[1m⏺ Important status update.");
    assert_eq!(result, Some("Important status update.".into()));
}

#[test]
fn test_line_without_marker_is_skipped() {
    let mut parser = make_parser();
    // No ⏺ at all — skip
    let result = parser.parse_line("This is just plain terminal output.");
    assert_eq!(result, None);
}

#[test]
fn test_ansi_stripping() {
    let stripped = Parser::strip_ansi("\x1b[1;34mhello\x1b[0m world");
    assert_eq!(stripped, "hello world");
}

#[test]
fn test_is_speakable_direct() {
    let mut parser = make_parser();
    assert!(parser.is_speakable("This is a normal sentence."));
    assert!(!parser.is_speakable("┌──┐──┐┐┐│"));
    assert!(!parser.is_speakable("$ cargo build"));
    assert!(!parser.is_speakable("/Users/diego/Dev/loquitor/src/main.rs"));
}

#[test]
fn test_multiple_narrative_lines_in_sequence() {
    let mut parser = make_parser();
    let line1 = parser.parse_line("⏺ I see the issue.");
    let line2 = parser.parse_line("⏺ The config file is missing the database URL.");
    let line3 = parser.parse_line("⏺ Let me fix that.");
    assert_eq!(line1, Some("I see the issue.".into()));
    assert_eq!(
        line2,
        Some("The config file is missing the database URL.".into())
    );
    assert_eq!(line3, Some("Let me fix that.".into()));
}
