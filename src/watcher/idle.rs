//! Idle detector for Claude Code sessions.
//!
//! Pure state machine: no tasks, no sleeps, no I/O. The LaneWatcher feeds
//! each cleaned (ANSI-stripped) line into `feed()` along with the current
//! `Instant`; the machine transitions and optionally returns a
//! `TurnEvent::TurnEnded` signal on the exact line that closes a turn.
//!
//! Detection rule — verified against live Claude Code logs:
//!   - A "prompt frame" is a cleaned line whose non-whitespace chars are
//!     all in the Unicode Box Drawing block (U+2500–U+257F).
//!   - After `confirm_frames` identical prompt frames AND `min_silence`
//!     elapsed since the first of them, the turn is declared ended.
//!   - Any content line while in PossibleIdle resets to Collecting.
//!   - A content line after `turn_max_duration` in Collecting force-ships.

use std::time::{Duration, Instant};

/// Tuning parameters for the detector. Wire these in from
/// `Config::daemon.idle_*` / `turn_max_duration_secs` at spawn time.
#[derive(Debug, Clone, Copy)]
pub struct IdleCfg {
    /// How many *identical* prompt frames in a row confirm an idle state.
    pub confirm_frames: u32,
    /// Minimum wall-clock gap between the first prompt frame and the
    /// emission of TurnEnded. Defends against fast-redraw TUI flashes.
    pub min_silence: Duration,
    /// Maximum age of a Collecting turn. If exceeded, the next content
    /// line forces the turn to ship with whatever has accumulated.
    /// Catches hung sessions (Claude crashed mid-turn).
    pub turn_max_duration: Duration,
}

/// Three-stage lifecycle per lane.
///
/// `Idle` is the initial/post-turn resting state — no active buffering.
/// `Collecting` is "Claude is writing." `PossibleIdle` is "Claude looks
/// done, but we need a few stable frames to confirm before shipping."
#[derive(Debug)]
pub enum IdleState {
    Idle,
    Collecting {
        turn_started: Instant,
    },
    PossibleIdle {
        since: Instant,
        frames: u32,
        last_frame: String,
        turn_started: Instant,
    },
}

impl IdleState {
    pub fn new() -> Self {
        Self::Idle
    }
}

impl Default for IdleState {
    fn default() -> Self {
        Self::new()
    }
}

/// Classification of a single cleaned line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineClass {
    /// Line consists only of whitespace + box-drawing characters.
    /// Payload is the trimmed frame content — used to compare stability
    /// across successive frames.
    PromptFrame(String),
    /// Any other line (including empty). Counts as "Claude is still
    /// producing output" for state-machine purposes.
    Content,
}

/// The one event the detector can emit.
#[derive(Debug, PartialEq, Eq)]
pub enum TurnEvent {
    /// The current turn just ended — the caller should flush its turn
    /// buffer to whatever consumes TurnReady downstream.
    TurnEnded,
}

/// Classify a single cleaned line. Pure function — easy to test.
pub fn classify(cleaned: &str) -> LineClass {
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        // Blank lines count as Content because Claude often emits them
        // mid-turn between paragraphs. Treating them as prompt frames
        // would misfire idle on every paragraph break.
        return LineClass::Content;
    }
    if trimmed.chars().all(is_box_drawing_or_ws) {
        LineClass::PromptFrame(trimmed.to_string())
    } else {
        LineClass::Content
    }
}

fn is_box_drawing_or_ws(c: char) -> bool {
    // Unicode Box Drawing block is U+2500..=U+257F.
    // Whitespace covers spaces, tabs, NBSP — all expected inside a prompt
    // frame (the TUI pads between border chars).
    c.is_whitespace() || matches!(c as u32, 0x2500..=0x257F)
}

/// Pure state transition. Call once per cleaned line. Returns
/// `Some(TurnEnded)` exactly on the transition that closes a turn —
/// never on subsequent lines until a new Collecting period is opened.
pub fn feed(
    state: &mut IdleState,
    class: LineClass,
    now: Instant,
    cfg: &IdleCfg,
) -> Option<TurnEvent> {
    // Take ownership of the current state so we can destructure its
    // owned fields (String frames) without clones.
    let current = std::mem::replace(state, IdleState::Idle);

    match (current, class) {
        // Idle: waiting for Claude to start talking. A content line
        // starts a new turn; prompt frames while idle are noise
        // (someone scrolling the log, detector restart, etc.).
        (IdleState::Idle, LineClass::Content) => {
            *state = IdleState::Collecting { turn_started: now };
            None
        }
        (IdleState::Idle, LineClass::PromptFrame(_)) => {
            *state = IdleState::Idle;
            None
        }

        // Collecting: normal "Claude is emitting output" path.
        // Content keeps the turn going; a force-ship fires if we've
        // been collecting for longer than turn_max_duration.
        (IdleState::Collecting { turn_started }, LineClass::Content) => {
            if now.saturating_duration_since(turn_started) >= cfg.turn_max_duration {
                *state = IdleState::Idle;
                Some(TurnEvent::TurnEnded)
            } else {
                *state = IdleState::Collecting { turn_started };
                None
            }
        }
        (IdleState::Collecting { turn_started }, LineClass::PromptFrame(frame)) => {
            // First prompt-frame sighting — might be a brief flash before
            // Claude resumes. Enter PossibleIdle and start counting.
            *state = IdleState::PossibleIdle {
                since: now,
                frames: 1,
                last_frame: frame,
                turn_started,
            };
            None
        }

        // PossibleIdle: we've seen 1+ prompt frame. Content resets to
        // Collecting (Claude resumed). Same-frame increments; different-
        // frame resets the count with the new frame as the baseline.
        (IdleState::PossibleIdle { turn_started, .. }, LineClass::Content) => {
            *state = IdleState::Collecting { turn_started };
            None
        }
        (
            IdleState::PossibleIdle {
                since,
                frames,
                last_frame,
                turn_started,
            },
            LineClass::PromptFrame(new_frame),
        ) => {
            if new_frame == last_frame {
                let next_frames = frames + 1;
                if next_frames >= cfg.confirm_frames
                    && now.saturating_duration_since(since) >= cfg.min_silence
                {
                    *state = IdleState::Idle;
                    Some(TurnEvent::TurnEnded)
                } else {
                    *state = IdleState::PossibleIdle {
                        since,
                        frames: next_frames,
                        last_frame,
                        turn_started,
                    };
                    None
                }
            } else {
                // Frame content shifted — reset the stability counter
                // but stay in PossibleIdle (we're still seeing frames).
                *state = IdleState::PossibleIdle {
                    since: now,
                    frames: 1,
                    last_frame: new_frame,
                    turn_started,
                };
                None
            }
        }
    }
}
