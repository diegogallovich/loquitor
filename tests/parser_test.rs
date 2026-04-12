use loquitor::watcher::parser::Parser;

fn make_parser() -> Parser {
    Parser::new(
        r"^(Bash|Read|Edit|Write|Glob|Grep|Agent|Skill|TaskCreate|TaskUpdate|ToolSearch|WebFetch|WebSearch|NotebookEdit)\s*\(",
        0.6,
    )
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
