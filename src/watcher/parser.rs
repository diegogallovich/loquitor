use regex::Regex;

pub struct Parser {
    tool_pattern: Regex,
    speakability_threshold: f64,
    in_code_block: bool,
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
        }
    }

    /// Stage 1: Check if a raw line (with ANSI codes still present) has a black/default ⏺.
    /// Returns true if the ⏺ character appears with no color escape or with reset/black/bold-default.
    /// Returns false for any other color (tool calls, file ops, etc.).
    pub fn is_narrative_marker(raw_line: &str) -> bool {
        // Line must contain the ⏺ character (U+23FA) to be a candidate
        let marker_pos = match raw_line.find('⏺') {
            Some(p) => p,
            None => return false,
        };

        // Find the last ANSI escape sequence BEFORE the marker
        let before = &raw_line[..marker_pos];
        let last_escape = before.rfind("\x1b[");

        match last_escape {
            None => true, // No escape — default terminal color, narrative
            Some(pos) => {
                let code = &before[pos + 2..]; // Skip \x1b[
                // Extract the SGR code (everything up to 'm')
                if let Some(m_pos) = code.find('m') {
                    let sgr = &code[..m_pos];
                    // Default/reset: "0" or "0;0" or empty
                    // Black foreground: "30"
                    // Bold (often preserves default color): "1"
                    // Bold + default: "0;1", "1;0"
                    // Bold + black: "1;30"
                    matches!(sgr, "0" | "0;0" | "" | "30" | "1" | "1;30" | "0;1" | "1;0")
                } else {
                    true // Malformed escape — treat as narrative (defensive)
                }
            }
        }
    }

    /// Stage 2: Strip all ANSI escape sequences from a line.
    pub fn strip_ansi(line: &str) -> String {
        let bytes = strip_ansi_escapes::strip(line);
        String::from_utf8_lossy(&bytes).into_owned()
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
            .filter(|c| c.is_alphabetic() || *c == ' ' || *c == '\'' || *c == ',' || *c == '.' || *c == '!' || *c == '?')
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

    /// Run the complete pipeline on a raw line from `script` output.
    /// Returns `Some(speakable_text)` if the line should be spoken, or `None` if it should be skipped.
    pub fn parse_line(&mut self, raw_line: &str) -> Option<String> {
        // Stage 1: Color-aware ⏺ detection
        if !Self::is_narrative_marker(raw_line) {
            return None;
        }

        // Stage 2: Strip ANSI
        let clean = Self::strip_ansi(raw_line);

        // Extract text after ⏺
        let text = clean
            .trim()
            .strip_prefix('⏺')
            .unwrap_or(&clean)
            .trim()
            .to_string();

        if text.is_empty() {
            return None;
        }

        // Stage 5: Tool name safety net (run before speakability — reject tool calls early)
        if self.is_tool_call(&text) {
            return None;
        }

        // Stage 4: Speakability filter
        if !self.is_speakable(&text) {
            return None;
        }

        Some(text)
    }
}
