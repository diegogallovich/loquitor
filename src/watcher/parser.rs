use regex::Regex;

pub struct Parser {
    tool_pattern: Regex,
    speakability_threshold: f64,
    in_code_block: bool,
    /// Tracks whether we're currently *inside* a narrative block.
    /// A narrative block opens when a black/default ⏺ is seen and closes
    /// when `should_exit_narrative` returns `true` for the current line.
    /// While inside the block, continuation lines without a ⏺ marker are
    /// still emitted (subject to the speakability filter) — this is what
    /// makes multi-line prose and bulleted lists speak in full.
    in_narrative_block: bool,
}

impl Parser {
    pub fn new(tool_pattern: &str, speakability_threshold: f64) -> Self {
        Self {
            tool_pattern: Regex::new(tool_pattern).unwrap_or_else(|_| {
                // Fallback pattern if the user's custom pattern is invalid
                Regex::new(r"^(Bash|Read|Edit|Write|Glob|Grep|Agent)\s*\(").unwrap()
            }),
            speakability_threshold,
            in_code_block: false,
            in_narrative_block: false,
        }
    }

    /// Stage 1: Check if a raw line (with ANSI codes still present) has a black/default ⏺.
    /// Returns true if the ⏺ character appears with no color escape or with reset/black/bold-default.
    /// Returns false for any other color (tool calls, file ops, etc.).
    ///
    /// Handles:
    /// - No escape before marker -> narrative
    /// - Basic 8-color black (30), reset (0), default (39), bold (1)
    /// - 24-bit truecolor black `38;2;R;G;B` where R+G+B is near zero
    /// - 256-color black `38;5;0` or `38;5;16`
    pub fn is_narrative_marker(raw_line: &str) -> bool {
        let marker_pos = match raw_line.find('⏺') {
            Some(p) => p,
            None => return false,
        };

        let before = &raw_line[..marker_pos];
        let last_escape = before.rfind("\x1b[");

        match last_escape {
            None => true,
            Some(pos) => {
                let code = &before[pos + 2..];
                if let Some(m_pos) = code.find('m') {
                    Self::sgr_is_narrative_foreground(&code[..m_pos])
                } else {
                    true // Malformed; defensive
                }
            }
        }
    }

    /// Returns true iff the raw line contains a ⏺ marker whose colour is *not*
    /// the narrative (black/default) palette. These are tool-call markers —
    /// seeing one mid-narrative is a strong signal that the narrative block
    /// has ended and a tool call is starting.
    pub fn is_tool_marker(raw_line: &str) -> bool {
        raw_line.contains('⏺') && !Self::is_narrative_marker(raw_line)
    }

    /// Decide if an SGR parameter string (e.g. "38;2;0;0;0" or "1;30") sets the
    /// foreground to black/default/bold (= narrative) or to a real color (= tool noise).
    fn sgr_is_narrative_foreground(sgr: &str) -> bool {
        if sgr.is_empty() {
            return true;
        }
        let params: Vec<&str> = sgr.split(';').collect();
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                // Attribute-only or reset-to-default — color-neutral
                "0" | "1" | "2" | "3" | "4" | "5" | "7" | "8" | "9" | "22" | "23" | "24"
                | "25" | "27" | "28" | "29" | "39" | "49" => i += 1,
                // Basic foreground black
                "30" => i += 1,
                // 24-bit truecolor foreground: "38;2;R;G;B"
                "38" if i + 4 < params.len() && params[i + 1] == "2" => {
                    let r: u16 = params[i + 2].parse().unwrap_or(255);
                    let g: u16 = params[i + 3].parse().unwrap_or(255);
                    let b: u16 = params[i + 4].parse().unwrap_or(255);
                    // Narrative only if the emitted color is near black. Claude Code uses
                    // (0,0,0) for narrative and bright/warm RGB for tool calls, so a tight
                    // threshold reliably separates them.
                    if r + g + b > 30 {
                        return false;
                    }
                    i += 5;
                }
                // 256-color foreground: "38;5;N"
                "38" if i + 2 < params.len() && params[i + 1] == "5" => {
                    let n: u16 = params[i + 2].parse().unwrap_or(255);
                    // 0 and 16 are the "black" slots in the 256-color palette
                    if n != 0 && n != 16 {
                        return false;
                    }
                    i += 3;
                }
                // Any other chromatic foreground => colored line => not narrative
                _ => return false,
            }
        }
        true
    }

    /// Stage 2: Strip all ANSI escape sequences from a line. First converts
    /// cursor-forward sequences (`\x1b[NC`) to N spaces so TUI-rendered text
    /// (which uses cursor positioning instead of literal spaces) is readable
    /// after stripping.
    pub fn strip_ansi(line: &str) -> String {
        let preprocessed = Self::expand_cursor_forward(line);
        let bytes = strip_ansi_escapes::strip(&preprocessed);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Replace `\x1b[NC` (cursor forward N columns) with N literal spaces.
    /// Claude Code and similar TUIs use this pattern to position text without
    /// emitting space characters; when we strip ANSI we'd otherwise get words
    /// glued together. Works on bytes so UTF-8 sequences pass through intact.
    fn expand_cursor_forward(line: &str) -> String {
        let bytes = line.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            // Look for ESC [ <digits> C — cursor forward by N columns
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
                    // Cap expansion to avoid pathological input blowing memory.
                    // Claude Code uses small N (typically 1-3).
                    let n = n.min(256);
                    out.extend(std::iter::repeat_n(b' ', n));
                    i = j + 1;
                    continue;
                }
            }
            // Not a cursor-forward sequence; copy the byte through unchanged.
            // Multi-byte UTF-8 sequences pass intact because we don't interpret
            // them — we only recognize the ESC [ <digits> C pattern.
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    /// Stage 4: Determine if a cleaned line is "speakable" natural language
    /// (skips box-drawing diagrams, code blocks, bare file paths, shell commands).
    /// Mutates `in_code_block` when it encounters ``` fence markers.
    pub fn is_speakable(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Track code-fence state and skip fence markers themselves
        if trimmed.starts_with("```") {
            self.in_code_block = !self.in_code_block;
            return false;
        }
        if self.in_code_block {
            return false;
        }

        // Skip bare file paths (leading /, no spaces, looks like a path)
        if trimmed.starts_with('/') && !trimmed.contains(' ') {
            return false;
        }

        // Skip shell commands
        if trimmed.starts_with('$') || trimmed.starts_with('>') {
            return false;
        }

        // Count "speakable" characters: letters, spaces, common punctuation
        let total = trimmed.chars().count() as f64;
        if total == 0.0 {
            return false;
        }
        let speakable_chars = trimmed
            .chars()
            .filter(|c| {
                c.is_alphabetic()
                    || *c == ' '
                    || *c == '\''
                    || *c == ','
                    || *c == '.'
                    || *c == '!'
                    || *c == '?'
            })
            .count() as f64;

        speakable_chars / total >= self.speakability_threshold
    }

    /// Stage 5: Check if a cleaned line matches a known tool-invocation regex.
    /// Tools look like `Bash(...)`, `Read(...)`, `Edit(...)`, etc.
    pub fn is_tool_call(&self, text: &str) -> bool {
        let trimmed = text.trim();
        // Remove the ⏺ prefix if still present (after ANSI strip, it will be)
        let content = trimmed.strip_prefix('⏺').unwrap_or(trimmed).trim();
        self.tool_pattern.is_match(content)
    }

    /// DESIGN DECISION — Diego to implement.
    ///
    /// Called for every line *while a narrative block is already open*.
    /// Return `true` to CLOSE the block. After closing, lines without a
    /// fresh black ⏺ marker will be skipped until a new narrative block opens.
    ///
    /// Inputs describe the current raw line:
    ///   - `has_tool_marker`   — raw line has a coloured ⏺ (tool call starting)
    ///   - `is_tool_call_text` — cleaned text matches `Bash(...)`, `Read(...)` etc.
    ///     (tool call that somehow lacks a marker; belt-and-suspenders)
    ///   - `is_speakable`      — cleaned text passed the speakability filter
    ///     (letters/punctuation ratio, not a code fence, not a path)
    ///   - `is_empty`          — cleaned text is empty after trimming
    ///
    /// Trade-offs:
    ///   - **Too aggressive** (e.g., exit on any non-speakable line) → you lose
    ///     bulleted lists that have blank lines or ASCII between prose items.
    ///   - **Too lenient** (e.g., never exit) → mild over-speaking: box drawings
    ///     and status bars after Claude finishes talking would get probed by
    ///     `is_speakable`. Usually harmless but not free.
    ///
    /// **My recommendation**: `has_tool_marker || is_tool_call_text`.
    ///   Rationale — those are the only two signals that *reliably* mean
    ///   "Claude just started doing something that isn't prose." Everything
    ///   else (blank lines, code blocks, box drawings) is either transient
    ///   formatting or already silently skipped by the speakability filter.
    ///
    /// But you've seen Claude Code's actual output shapes — your call.
    fn should_exit_narrative(
        has_tool_marker: bool,
        is_tool_call_text: bool,
        is_speakable: bool,
        is_empty: bool,
    ) -> bool {
        // TODO(diego): implement the exit policy. 5-10 lines.
        // While this returns `false`, the narrative block never closes —
        // which fixes the "first line only" bug but may over-speak until
        // you define the exit conditions.
        let _ = (has_tool_marker, is_tool_call_text, is_speakable, is_empty);
        false
    }

    /// Run the complete pipeline on a raw line from `script` output.
    /// Returns `Some(speakable_text)` if the line should be spoken, or `None` if it should be skipped.
    pub fn parse_line(&mut self, raw_line: &str) -> Option<String> {
        // Extract the cleaned text once — downstream checks need it.
        let clean = Self::strip_ansi(raw_line);
        let text = clean
            .trim()
            .strip_prefix('⏺')
            .unwrap_or(&clean)
            .trim()
            .to_string();

        let is_empty = text.is_empty();
        let tool_call_text = !is_empty && self.is_tool_call(&text);
        // is_speakable mutates in_code_block, so only call it once per line.
        let speakable = if is_empty { false } else { self.is_speakable(&text) };

        let has_narrative = Self::is_narrative_marker(raw_line);
        let has_tool_marker = Self::is_tool_marker(raw_line);

        // --- State transitions ---
        // A narrative marker always opens (or re-opens) a block.
        if has_narrative {
            self.in_narrative_block = true;
        } else if self.in_narrative_block
            && Self::should_exit_narrative(has_tool_marker, tool_call_text, speakable, is_empty)
        {
            self.in_narrative_block = false;
        }

        // --- Emission gate ---
        // Outside a narrative block, silence.
        if !self.in_narrative_block {
            return None;
        }

        // Inside a narrative block, emit only truly speakable prose.
        // Empty, tool-call text, and non-speakable lines are kept silent but
        // the block stays open — subsequent speakable lines will still flow.
        if is_empty || tool_call_text || !speakable {
            return None;
        }

        Some(text)
    }
}
