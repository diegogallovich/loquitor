//! Exercises the idle-detector state machine with synthetic line
//! sequences. No real time: each test fabricates `Instant`s to prove
//! the transitions and timing behaviours without sleeps.

use loquitor::watcher::idle::{classify, feed, IdleCfg, IdleState, LineClass, TurnEvent};
use std::time::{Duration, Instant};

fn default_cfg() -> IdleCfg {
    IdleCfg {
        confirm_frames: 3,
        min_silence: Duration::from_millis(500),
        turn_max_duration: Duration::from_secs(1800),
    }
}

// --- classify() ---

#[test]
fn classify_content_line() {
    assert_eq!(
        classify("⏺ I'll start by reading the file."),
        LineClass::Content
    );
    assert_eq!(classify("running cargo test"), LineClass::Content);
}

#[test]
fn classify_blank_line_is_content() {
    // Blank lines must NOT count as prompt frames — Claude uses them
    // between paragraphs and treating them as idle signals would misfire.
    assert_eq!(classify(""), LineClass::Content);
    assert_eq!(classify("   \t  "), LineClass::Content);
}

#[test]
fn classify_pure_box_drawing_line_is_prompt_frame() {
    let frame = "╭────────────────────────────╮";
    assert_eq!(classify(frame), LineClass::PromptFrame(frame.to_string()));
}

#[test]
fn classify_box_drawing_with_inner_spaces_is_prompt_frame() {
    // Prompt frames often have whitespace between corners/bars.
    let frame = "│    │   │    │";
    assert_eq!(classify(frame), LineClass::PromptFrame(frame.to_string()));
}

#[test]
fn classify_mixed_content_is_content() {
    // A box-drawing char plus any non-box text disqualifies a line.
    assert_eq!(classify("╭─ Response ───╮"), LineClass::Content);
}

// --- feed(): state transitions ---

#[test]
fn idle_then_content_starts_collecting() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    assert_eq!(feed(&mut s, LineClass::Content, t0, &default_cfg()), None);
    assert!(matches!(s, IdleState::Collecting { .. }));
}

#[test]
fn idle_then_prompt_frame_stays_idle() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    let res = feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0,
        &default_cfg(),
    );
    assert_eq!(res, None);
    assert!(matches!(s, IdleState::Idle));
}

#[test]
fn collecting_then_content_stays_collecting() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    feed(&mut s, LineClass::Content, t0, &default_cfg());
    let res = feed(
        &mut s,
        LineClass::Content,
        t0 + Duration::from_millis(50),
        &default_cfg(),
    );
    assert_eq!(res, None);
    assert!(matches!(s, IdleState::Collecting { .. }));
}

#[test]
fn collecting_then_prompt_frame_enters_possible_idle() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    feed(&mut s, LineClass::Content, t0, &default_cfg());
    let res = feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(100),
        &default_cfg(),
    );
    assert_eq!(res, None);
    match s {
        IdleState::PossibleIdle { frames, .. } => assert_eq!(frames, 1),
        other => panic!("expected PossibleIdle, got {other:?}"),
    }
}

#[test]
fn possible_idle_then_content_returns_to_collecting() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    feed(&mut s, LineClass::Content, t0, &default_cfg());
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(100),
        &default_cfg(),
    );
    let res = feed(
        &mut s,
        LineClass::Content,
        t0 + Duration::from_millis(200),
        &default_cfg(),
    );
    assert_eq!(res, None);
    assert!(matches!(s, IdleState::Collecting { .. }));
}

#[test]
fn possible_idle_different_frame_resets_counter() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    let cfg = default_cfg();
    feed(&mut s, LineClass::Content, t0, &cfg);
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(100),
        &cfg,
    );
    feed(
        &mut s,
        LineClass::PromptFrame("═".into()),
        t0 + Duration::from_millis(200),
        &cfg,
    );
    match s {
        IdleState::PossibleIdle {
            frames, last_frame, ..
        } => {
            assert_eq!(frames, 1);
            assert_eq!(last_frame, "═");
        }
        other => panic!("expected PossibleIdle, got {other:?}"),
    }
}

// --- feed(): idle emission ---

#[test]
fn three_identical_frames_past_min_silence_emit_turn_ended() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    let cfg = default_cfg(); // confirm_frames=3, min_silence=500ms

    feed(&mut s, LineClass::Content, t0, &cfg);

    // Frame #1 — starts possible-idle at t0+100
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(100),
        &cfg,
    );
    // Frame #2 — below threshold
    let r2 = feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(300),
        &cfg,
    );
    assert_eq!(r2, None);

    // Frame #3 — 700ms after first frame (past min_silence), threshold hit
    let r3 = feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(800),
        &cfg,
    );
    assert_eq!(r3, Some(TurnEvent::TurnEnded));
    assert!(matches!(s, IdleState::Idle));
}

#[test]
fn threshold_hit_but_insufficient_silence_does_not_fire() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    let cfg = default_cfg(); // 3 frames + 500ms

    feed(&mut s, LineClass::Content, t0, &cfg);
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(100),
        &cfg,
    );
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(150),
        &cfg,
    );
    // Third frame arrives only 350ms after the first — min_silence=500
    // hasn't elapsed yet, so no emission.
    let res = feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(450),
        &cfg,
    );
    assert_eq!(res, None);
    match s {
        IdleState::PossibleIdle { frames, .. } => assert_eq!(frames, 3),
        other => panic!("expected PossibleIdle, got {other:?}"),
    }
}

#[test]
fn reset_on_mid_sequence_content_blocks_spurious_idle() {
    // Narrative, frame, narrative, frame, frame, frame → TurnEnded.
    // Simulates the case where Claude flashes a prompt briefly mid-turn
    // (e.g. between a tool call and resumed narrative) and we must NOT
    // fire idle on the flash.
    let mut s = IdleState::new();
    let t0 = Instant::now();
    let cfg = default_cfg();

    feed(&mut s, LineClass::Content, t0, &cfg);
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(100),
        &cfg,
    );
    // Content mid-sequence resets
    let r = feed(
        &mut s,
        LineClass::Content,
        t0 + Duration::from_millis(200),
        &cfg,
    );
    assert_eq!(r, None);
    assert!(matches!(s, IdleState::Collecting { .. }));

    // Now a stable 3-frame run past min_silence closes the turn cleanly.
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(300),
        &cfg,
    );
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(500),
        &cfg,
    );
    let r = feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(900),
        &cfg,
    );
    assert_eq!(r, Some(TurnEvent::TurnEnded));
}

#[test]
fn force_ship_on_turn_max_duration_exceeded() {
    let mut s = IdleState::new();
    let t0 = Instant::now();
    let cfg = IdleCfg {
        confirm_frames: 3,
        min_silence: Duration::from_millis(500),
        turn_max_duration: Duration::from_secs(2),
    };

    feed(&mut s, LineClass::Content, t0, &cfg);

    // A content line 3 seconds later triggers the force-ship path even
    // though we never saw a prompt frame. This is the "Claude hung
    // mid-turn" safety net.
    let r = feed(
        &mut s,
        LineClass::Content,
        t0 + Duration::from_secs(3),
        &cfg,
    );
    assert_eq!(r, Some(TurnEvent::TurnEnded));
    assert!(matches!(s, IdleState::Idle));
}

#[test]
fn exactly_confirm_frames_at_exactly_min_silence_fires() {
    // Boundary: frames == confirm_frames AND elapsed == min_silence
    // should fire. Defends against off-by-one.
    let mut s = IdleState::new();
    let t0 = Instant::now();
    let cfg = IdleCfg {
        confirm_frames: 2,
        min_silence: Duration::from_millis(100),
        turn_max_duration: Duration::from_secs(1800),
    };

    feed(&mut s, LineClass::Content, t0, &cfg);
    feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(10),
        &cfg,
    );
    // 2nd frame at t+110ms: elapsed = 100ms since first frame (which was
    // at t+10ms), frames = 2, both == thresholds.
    let r = feed(
        &mut s,
        LineClass::PromptFrame("─".into()),
        t0 + Duration::from_millis(110),
        &cfg,
    );
    assert_eq!(r, Some(TurnEvent::TurnEnded));
}
