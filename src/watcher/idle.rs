//! Idle detector for Claude Code sessions.
//!
//! Pure state machine: no tasks, no sleeps, no I/O. The LaneWatcher
//! feeds each cleaned (ANSI-stripped) line into `feed()` along with the
//! current `Instant`; on every poll cycle (whether or not a new line
//! arrived) it also calls `tick()` so the inactivity-based path can
//! fire even when Claude has gone completely quiet.
//!
//! Two independent signals can confirm "Claude is waiting for input":
//!
//!   1. **Inactivity** (primary): no new bytes for `quiet_threshold`
//!      seconds while we're in `Collecting`. Robust to whatever the
//!      TUI looks like — a real Claude Code prompt includes a lot more
//!      than just box-drawing chars (model name, ctx %, mode hints,
//!      a "thinking" indicator), so byte-level silence is the only
//!      truly reliable signal.
//!   2. **Stable prompt frame** (secondary): N identical box-drawing
//!      lines in a row, separated by `min_silence`. Kept as a fast
//!      path in case Claude ever does emit a clean prompt-frame
//!      sequence — costs nothing extra.
//!
//! Both paths emit `TurnEvent::TurnEnded` and reset to `Idle`.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct IdleCfg {
    /// Identical-prompt-frame threshold for the secondary signal.
    pub confirm_frames: u32,
    /// Minimum gap between first and last identical frame for the
    /// secondary signal.
    pub min_silence: Duration,
    /// Hard cap on a `Collecting` turn — force-ship after this even
    /// if neither signal fires (Claude hung).
    pub turn_max_duration: Duration,
    /// Primary signal: end the turn if `Collecting` has had no new
    /// content for this long. Default is 3s — long enough to ride
    /// through small "thinking" pauses, short enough to feel snappy.
    pub quiet_threshold: Duration,
}

#[derive(Debug)]
pub enum IdleState {
    Idle,
    Collecting {
        turn_started: Instant,
        last_content_at: Instant,
    },
    PossibleIdle {
        since: Instant,
        frames: u32,
        last_frame: String,
        turn_started: Instant,
        last_content_at: Instant,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineClass {
    /// Cleaned line is whitespace + box-drawing only. Payload is the
    /// trimmed frame — used to compare stability across frames.
    PromptFrame(String),
    /// Any other line (including empty). "Claude is still producing
    /// output" for state-machine purposes.
    Content,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TurnEvent {
    TurnEnded,
}

pub fn classify(cleaned: &str) -> LineClass {
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return LineClass::Content;
    }
    if trimmed.chars().all(is_box_drawing_or_ws) {
        LineClass::PromptFrame(trimmed.to_string())
    } else {
        LineClass::Content
    }
}

fn is_box_drawing_or_ws(c: char) -> bool {
    c.is_whitespace() || matches!(c as u32, 0x2500..=0x257F)
}

/// Per-line state transition. Refreshes the inactivity clock on every
/// content line so `tick()` knows when activity last happened.
pub fn feed(
    state: &mut IdleState,
    class: LineClass,
    now: Instant,
    cfg: &IdleCfg,
) -> Option<TurnEvent> {
    let current = std::mem::replace(state, IdleState::Idle);

    match (current, class) {
        // Idle: a content line opens a new turn. Prompt frames while
        // idle are noise.
        (IdleState::Idle, LineClass::Content) => {
            *state = IdleState::Collecting {
                turn_started: now,
                last_content_at: now,
            };
            None
        }
        (IdleState::Idle, LineClass::PromptFrame(_)) => {
            *state = IdleState::Idle;
            None
        }

        // Collecting: refresh inactivity clock on every line.
        (IdleState::Collecting { turn_started, .. }, LineClass::Content) => {
            if now.saturating_duration_since(turn_started) >= cfg.turn_max_duration {
                *state = IdleState::Idle;
                Some(TurnEvent::TurnEnded)
            } else {
                *state = IdleState::Collecting {
                    turn_started,
                    last_content_at: now,
                };
                None
            }
        }
        (
            IdleState::Collecting {
                turn_started,
                last_content_at: _,
            },
            LineClass::PromptFrame(frame),
        ) => {
            *state = IdleState::PossibleIdle {
                since: now,
                frames: 1,
                last_frame: frame,
                turn_started,
                last_content_at: now,
            };
            None
        }

        // PossibleIdle: content resets to Collecting; same-frame
        // increments; different-frame resets the count.
        (IdleState::PossibleIdle { turn_started, .. }, LineClass::Content) => {
            *state = IdleState::Collecting {
                turn_started,
                last_content_at: now,
            };
            None
        }
        (
            IdleState::PossibleIdle {
                since,
                frames,
                last_frame,
                turn_started,
                last_content_at,
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
                        last_content_at,
                    };
                    None
                }
            } else {
                *state = IdleState::PossibleIdle {
                    since: now,
                    frames: 1,
                    last_frame: new_frame,
                    turn_started,
                    last_content_at,
                };
                None
            }
        }
    }
}

/// Time-based check called from the LaneWatcher's polling loop on
/// every iteration (whether or not new bytes arrived). Fires
/// `TurnEnded` when `Collecting` has been quiet for `quiet_threshold`
/// — the primary signal that Claude is waiting for input. Also
/// catches the force-ship path when `turn_max_duration` is exceeded.
pub fn tick(state: &mut IdleState, now: Instant, cfg: &IdleCfg) -> Option<TurnEvent> {
    let current = std::mem::replace(state, IdleState::Idle);
    match current {
        IdleState::Idle => {
            *state = IdleState::Idle;
            None
        }
        IdleState::Collecting {
            turn_started,
            last_content_at,
        } => {
            let quiet = now.saturating_duration_since(last_content_at);
            let total = now.saturating_duration_since(turn_started);
            if quiet >= cfg.quiet_threshold || total >= cfg.turn_max_duration {
                *state = IdleState::Idle;
                Some(TurnEvent::TurnEnded)
            } else {
                *state = IdleState::Collecting {
                    turn_started,
                    last_content_at,
                };
                None
            }
        }
        IdleState::PossibleIdle {
            since,
            frames,
            last_frame,
            turn_started,
            last_content_at,
        } => {
            // Same fall-through: prolonged silence in PossibleIdle is
            // the same signal as in Collecting — Claude is done.
            let quiet = now.saturating_duration_since(last_content_at);
            if quiet >= cfg.quiet_threshold {
                *state = IdleState::Idle;
                Some(TurnEvent::TurnEnded)
            } else {
                *state = IdleState::PossibleIdle {
                    since,
                    frames,
                    last_frame,
                    turn_started,
                    last_content_at,
                };
                None
            }
        }
    }
}
