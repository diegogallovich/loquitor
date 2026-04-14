//! Low-level text cleanup used by the lane watcher to feed the idle
//! detector and (eventually) the turn buffer. Before the v0.2.0 pivot
//! this module also handled per-line speakability gating and narrative-
//! block tracking; those responsibilities have moved — speakability is
//! now the summarizer LLM's job, and narrative-block state is subsumed
//! by the turn-buffer lifecycle owned by the idle detector.
//!
//! What remains is the pure, context-free work of turning a raw PTY
//! line (ANSI escapes + cursor-forward sequences + stray multibyte
//! garbage) into a clean string the rest of the pipeline can reason
//! about uniformly.

/// Strip every ANSI escape sequence from a line and return the plain
/// text. First converts `\x1b[NC` (cursor-forward-N) into N literal
/// spaces so TUI text that uses cursor positioning doesn't get glued
/// back together after escape removal.
pub fn strip_ansi(line: &str) -> String {
    let preprocessed = expand_cursor_forward(line);
    let bytes = strip_ansi_escapes::strip(&preprocessed);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Replace `\x1b[NC` (cursor forward N columns) with N literal spaces.
/// Claude Code and similar TUIs use this pattern to position text
/// without emitting literal spaces; stripping escapes without
/// expanding would concatenate whole words.
fn expand_cursor_forward(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // Look for ESC [ <digits> C.
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            let digit_start = i + 2;
            let mut j = digit_start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > digit_start && j < bytes.len() && bytes[j] == b'C' {
                let n: usize = std::str::from_utf8(&bytes[digit_start..j])
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                // Cap expansion so pathological input can't blow memory.
                // Claude Code emits small N in practice (1–3).
                let n = n.min(256);
                out.extend(std::iter::repeat_n(b' ', n));
                i = j + 1;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Returns true iff the raw line contains a ⏺ marker whose foreground
/// colour is black/default (the "narrative" colour in Claude Code).
/// Tool-call ⏺ markers use warm/bright RGB and are excluded.
pub fn is_narrative_marker(raw_line: &str) -> bool {
    let Some(marker_pos) = raw_line.find('⏺') else {
        return false;
    };
    let before = &raw_line[..marker_pos];
    match before.rfind("\x1b[") {
        None => true,
        Some(pos) => {
            let code = &before[pos + 2..];
            if let Some(m_pos) = code.find('m') {
                sgr_is_narrative_foreground(&code[..m_pos])
            } else {
                true // malformed escape; defensive true
            }
        }
    }
}

/// Returns true iff the raw line contains a ⏺ marker with a non-
/// narrative (tool-call) foreground colour.
pub fn is_tool_marker(raw_line: &str) -> bool {
    raw_line.contains('⏺') && !is_narrative_marker(raw_line)
}

/// Decide if an SGR parameter string (e.g. "38;2;0;0;0" or "1;30")
/// sets the foreground to black/default/attributes-only. Returning
/// true means "narrative colour"; false means "tool-call colour".
fn sgr_is_narrative_foreground(sgr: &str) -> bool {
    if sgr.is_empty() {
        return true;
    }
    let params: Vec<&str> = sgr.split(';').collect();
    let mut i = 0;
    while i < params.len() {
        match params[i] {
            // Attribute-only or reset-to-default — color-neutral
            "0" | "1" | "2" | "3" | "4" | "5" | "7" | "8" | "9" | "22" | "23" | "24" | "25"
            | "27" | "28" | "29" | "39" | "49" => i += 1,
            // Basic foreground black
            "30" => i += 1,
            // 24-bit truecolor foreground: "38;2;R;G;B"
            "38" if i + 4 < params.len() && params[i + 1] == "2" => {
                let r: u16 = params[i + 2].parse().unwrap_or(255);
                let g: u16 = params[i + 3].parse().unwrap_or(255);
                let b: u16 = params[i + 4].parse().unwrap_or(255);
                // Claude Code uses (0,0,0) for narrative and bright/warm
                // RGB for tool calls. A tight threshold reliably splits
                // them — near-black is narrative, anything else is noise.
                if r + g + b > 30 {
                    return false;
                }
                i += 5;
            }
            // 256-colour foreground: "38;5;N"
            "38" if i + 2 < params.len() && params[i + 1] == "5" => {
                let n: u16 = params[i + 2].parse().unwrap_or(255);
                if n != 0 && n != 16 {
                    return false;
                }
                i += 3;
            }
            _ => return false,
        }
    }
    true
}
