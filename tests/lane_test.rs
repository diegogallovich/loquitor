//! Integration tests for `LaneWatcher::process_line_at` — the turn
//! buffer logic without any real file I/O or clock. Drives the watcher
//! through synthetic line sequences + fabricated `Instant`s.

use loquitor::watcher::idle::IdleCfg;
use loquitor::watcher::lane::{LaneWatcher, TurnReady};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn make_watcher(max_bytes: usize) -> LaneWatcher {
    // Fast idle cfg so tests don't need real sleeps: 2 confirm frames
    // (minimum that actually reaches the `frames >= confirm_frames`
    // check — see idle.rs for why 1 wouldn't fire), zero min_silence.
    let idle_cfg = IdleCfg {
        confirm_frames: 2,
        min_silence: Duration::ZERO,
        turn_max_duration: Duration::from_secs(3600),
    };
    let (tx, _rx) = mpsc::channel::<TurnReady>(16);
    LaneWatcher::new(
        "test-lane".into(),
        "/tmp/nowhere.log".into(),
        tx,
        idle_cfg,
        max_bytes,
    )
}

/// Drive a sequence of (line, Instant-offset-ms) pairs through the
/// watcher and return every TurnReady that fires.
fn drive(watcher: &mut LaneWatcher, origin: Instant, seq: &[(&str, u64)]) -> Vec<TurnReady> {
    let mut out = Vec::new();
    for (line, offset_ms) in seq {
        if let Some(ready) =
            watcher.process_line_at(line, origin + Duration::from_millis(*offset_ms))
        {
            out.push(ready);
        }
    }
    out
}

#[test]
fn buffers_content_and_flushes_on_idle() {
    let mut w = make_watcher(1024);
    let t0 = Instant::now();
    let ready = drive(
        &mut w,
        t0,
        &[
            ("Here's my plan.\n", 0),
            ("I'll read the config first.\n", 10),
            ("Then run the tests.\n", 20),
            ("╭─────╮", 30),
            ("╭─────╮", 50),
        ],
    );

    assert_eq!(ready.len(), 1, "exactly one turn should fire");
    let t = &ready[0];
    assert_eq!(t.lane_id, "test-lane");
    assert!(!t.truncated);
    assert!(t.turn_text.contains("Here's my plan."));
    assert!(t.turn_text.contains("read the config"));
    assert!(t.turn_text.contains("Then run the tests."));
    // Prompt frames must NOT be in the flushed text
    assert!(!t.turn_text.contains("╭"));
}

#[test]
fn does_not_flush_without_idle_confirmation() {
    let mut w = make_watcher(1024);
    let t0 = Instant::now();
    let ready = drive(
        &mut w,
        t0,
        &[
            ("First.\n", 0),
            ("Second.\n", 10),
            ("╭──╮", 20), // only 1 frame — below confirm_frames=2
        ],
    );
    assert!(ready.is_empty(), "should not flush on a single frame");
}

#[test]
fn different_frames_reset_the_counter() {
    // Two DIFFERENT prompt frames after content should NOT confirm —
    // stability requires the frames to be identical.
    let mut w = make_watcher(1024);
    let t0 = Instant::now();
    let ready = drive(
        &mut w,
        t0,
        &[
            ("Working.\n", 0),
            ("╭─────╮", 10),
            ("═════", 20), // different frame — reset, frames=1
            ("═════", 30), // frames=2, threshold hits
        ],
    );
    assert_eq!(ready.len(), 1);
}

#[test]
fn frame_flash_midturn_does_not_fire() {
    // Content → frame → content → frame → frame → TurnEnded.
    // Ensures a brief prompt flash between narrative chunks doesn't
    // prematurely close the turn.
    let mut w = make_watcher(1024);
    let t0 = Instant::now();
    let ready = drive(
        &mut w,
        t0,
        &[
            ("Part one.\n", 0),
            ("╭──╮", 10),
            ("Part two.\n", 20), // resets to Collecting
            ("╭──╮", 30),
            ("╭──╮", 40),
        ],
    );
    assert_eq!(ready.len(), 1);
    let t = &ready[0];
    assert!(t.turn_text.contains("Part one."));
    assert!(t.turn_text.contains("Part two."));
}

#[test]
fn buffer_front_truncates_on_overflow_and_sets_truncated_flag() {
    // 100-byte cap. Feed ~200 bytes of content — front should drop.
    let mut w = make_watcher(100);
    let t0 = Instant::now();
    let filler: String = "x".repeat(40); // each line ~41 bytes after newline
    let _ = drive(
        &mut w,
        t0,
        &[
            (&filler, 0),
            (&filler, 10),
            (&filler, 20),
            (&filler, 30),
            (&filler, 40),
            ("╭─╮", 50),
            ("╭─╮", 60),
        ],
    );
    // We can't easily capture the TurnReady from drive() because of
    // &str lifetimes with `filler` — instead, run the pattern inline
    // for an assertion version.

    // Re-run with captured output
    let mut w2 = make_watcher(100);
    let t1 = Instant::now();
    let filler2 = "x".repeat(40);
    let mut ready = Vec::new();
    for (line, off) in [
        (filler2.as_str(), 0u64),
        (&filler2, 10),
        (&filler2, 20),
        (&filler2, 30),
        (&filler2, 40),
        ("╭─╮", 50),
        ("╭─╮", 60),
    ] {
        if let Some(r) = w2.process_line_at(line, t1 + Duration::from_millis(off)) {
            ready.push(r);
        }
    }
    assert_eq!(ready.len(), 1);
    let t = &ready[0];
    assert!(t.truncated, "overflow should set truncated=true");
    assert!(
        t.turn_text.starts_with("[earlier output truncated]"),
        "truncation banner missing: {:?}",
        &t.turn_text[..t.turn_text.len().min(50)]
    );
}

#[test]
fn sequential_turns_buffer_clean_between_flushes() {
    let mut w = make_watcher(1024);
    let t0 = Instant::now();

    let r1 = drive(
        &mut w,
        t0,
        &[("First turn line.\n", 0), ("╭─╮", 10), ("╭─╮", 20)],
    );
    assert_eq!(r1.len(), 1);
    assert!(r1[0].turn_text.contains("First turn line."));

    // Second turn — first turn's content must not leak in.
    let r2 = drive(
        &mut w,
        t0,
        &[("Second turn content.\n", 100), ("╭─╮", 110), ("╭─╮", 120)],
    );
    assert_eq!(r2.len(), 1);
    assert!(r2[0].turn_text.contains("Second turn content."));
    assert!(
        !r2[0].turn_text.contains("First turn line."),
        "previous turn contents leaked: {}",
        r2[0].turn_text
    );
    assert!(!r2[0].truncated);
}

#[test]
fn timestamps_span_the_turn_lifetime() {
    let mut w = make_watcher(1024);
    let t0 = Instant::now();
    let ready = drive(
        &mut w,
        t0,
        &[
            ("Start.\n", 100),
            ("More.\n", 250),
            ("╭─╮", 400),
            ("╭─╮", 500),
        ],
    );
    assert_eq!(ready.len(), 1);
    let t = &ready[0];
    // started_at should be at the first content line (t0+100ms);
    // ended_at at the confirming frame (t0+500ms).
    assert_eq!(t.started_at, t0 + Duration::from_millis(100));
    assert_eq!(t.ended_at, t0 + Duration::from_millis(500));
}

#[test]
fn ansi_is_stripped_from_buffered_text() {
    let mut w = make_watcher(1024);
    let t0 = Instant::now();
    let ready = drive(
        &mut w,
        t0,
        &[
            ("\x1b[1;34mHello\x1b[0m world\n", 0),
            ("╭─╮", 10),
            ("╭─╮", 20),
        ],
    );
    assert_eq!(ready.len(), 1);
    assert!(ready[0].turn_text.contains("Hello world"));
    assert!(!ready[0].turn_text.contains("\x1b"));
}
