//! Tests for the liaison worker: the deterministic "Regarding {lane}. "
//! prefix, secret scrubbing decisions, error-fallback shape. A
//! `MockLiaisonProvider` controls what the "LLM" returns.

use anyhow::anyhow;
use async_trait::async_trait;
use loquitor::config::types::Config;
use loquitor::daemon::liaison_worker::{classify_error, handle_turn, should_scrub};
use loquitor::liaison::{LiaisonProvider, TurnContext, TurnSummary};
use loquitor::watcher::lane::TurnReady;
use std::sync::Mutex;
use std::time::Instant;

type FakeResult = Result<TurnSummary, anyhow::Error>;

struct MockLiaisonProvider {
    response: Mutex<Option<FakeResult>>,
    calls: Mutex<Vec<String>>, // records cleaned_log seen per call
}

impl MockLiaisonProvider {
    fn with_success(text: &str) -> Self {
        Self {
            response: Mutex::new(Some(Ok(TurnSummary {
                text: text.to_string(),
            }))),
            calls: Mutex::new(Vec::new()),
        }
    }
    fn with_error(err: anyhow::Error) -> Self {
        Self {
            response: Mutex::new(Some(Err(err))),
            calls: Mutex::new(Vec::new()),
        }
    }
    fn recorded_logs(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl LiaisonProvider for MockLiaisonProvider {
    fn name(&self) -> &str {
        "mock"
    }
    async fn summarize_turn(&self, ctx: &TurnContext<'_>) -> anyhow::Result<TurnSummary> {
        self.calls.lock().unwrap().push(ctx.cleaned_log.to_string());
        self.response
            .lock()
            .unwrap()
            .take()
            .expect("mock response consumed twice")
    }
}

fn fake_turn(lane: &str, text: &str) -> TurnReady {
    TurnReady {
        lane_id: lane.to_string(),
        turn_text: text.to_string(),
        started_at: Instant::now(),
        ended_at: Instant::now(),
        truncated: false,
    }
}

// --- handle_turn: lane prefix + happy path ---

#[tokio::test]
async fn prepends_lane_announcement_deterministically() {
    let cfg = Config::default();
    let provider =
        MockLiaisonProvider::with_success("Finished the refactor. Waiting for you to review.");
    let turn = fake_turn("loquitor", "some terminal output");

    let out = handle_turn(turn, &cfg, &provider).await;
    assert_eq!(out.lane_id, "loquitor");
    assert_eq!(out.lane_name, "loquitor");
    assert_eq!(
        out.text,
        "Regarding loquitor. Finished the refactor. Waiting for you to review."
    );
}

#[tokio::test]
async fn uses_friendly_lane_name_from_rule() {
    use loquitor::config::types::LaneRule;
    let mut cfg = Config::default();
    cfg.lanes.rules.insert(
        "dev-repo".into(),
        LaneRule {
            name: "Project Alpha".into(),
            voice: "nova".into(),
        },
    );
    let provider = MockLiaisonProvider::with_success("Done. Waiting for you.");
    let turn = fake_turn("dev-repo", "x");

    let out = handle_turn(turn, &cfg, &provider).await;
    assert!(
        out.text.starts_with("Regarding Project Alpha. "),
        "friendly name missing: {}",
        out.text
    );
}

#[tokio::test]
async fn scrubber_runs_for_default_cloud_provider() {
    let cfg = Config::default();
    let provider = MockLiaisonProvider::with_success("ok");
    let turn = fake_turn(
        "x",
        "export ANTHROPIC_API_KEY=sk-ant-test-abcdefghijklmnopqrstuvwxyz12 now",
    );

    let _ = handle_turn(turn, &cfg, &provider).await;
    let logs = provider.recorded_logs();
    assert_eq!(logs.len(), 1);
    assert!(
        !logs[0].contains("sk-ant-test"),
        "scrubber should have redacted sk-ant- key: {}",
        logs[0]
    );
    assert!(logs[0].contains("[REDACTED]"));
}

// --- handle_turn failure fallback ---

#[tokio::test]
async fn failure_produces_canned_fallback_with_lane_prefix() {
    let cfg = Config::default();
    let provider = MockLiaisonProvider::with_error(anyhow!("HTTP 401: invalid api key"));
    let turn = fake_turn("my-project", "whatever");

    let out = handle_turn(turn, &cfg, &provider).await;
    assert!(
        out.text
            .starts_with("Regarding my-project. Summary unavailable —"),
        "fallback shape wrong: {}",
        out.text
    );
    assert!(
        out.text.contains("authentication error"),
        "401 should map to 'authentication error': {}",
        out.text
    );
}

#[tokio::test]
async fn timeout_error_classifies_as_timeout() {
    let cfg = Config::default();
    let provider = MockLiaisonProvider::with_error(anyhow!("request timed out after 15 seconds"));
    let turn = fake_turn("lane", "x");

    let out = handle_turn(turn, &cfg, &provider).await;
    assert!(out.text.contains("timeout"), "got: {}", out.text);
}

#[tokio::test]
async fn rate_limit_error_classifies_as_rate_limit() {
    let cfg = Config::default();
    let provider = MockLiaisonProvider::with_error(anyhow!("HTTP 429: rate limit exceeded"));
    let turn = fake_turn("lane", "x");

    let out = handle_turn(turn, &cfg, &provider).await;
    assert!(out.text.contains("rate limit"), "got: {}", out.text);
}

#[tokio::test]
async fn network_error_classifies_as_network() {
    let cfg = Config::default();
    let provider = MockLiaisonProvider::with_error(anyhow!("connection refused by 54.x.x.x"));
    let turn = fake_turn("lane", "x");

    let out = handle_turn(turn, &cfg, &provider).await;
    assert!(out.text.contains("network error"), "got: {}", out.text);
}

// --- classify_error unit tests ---

#[test]
fn classify_authentication_variants() {
    assert_eq!(
        classify_error(&anyhow!("HTTP 401 Unauthorized")),
        "authentication error"
    );
    assert_eq!(
        classify_error(&anyhow!("authentication failed")),
        "authentication error"
    );
}

#[test]
fn classify_5xx_is_provider_outage() {
    assert_eq!(classify_error(&anyhow!("HTTP 503")), "a provider outage");
    assert_eq!(
        classify_error(&anyhow!("received 502 Bad Gateway")),
        "a provider outage"
    );
}

#[test]
fn classify_unknown_falls_back() {
    assert_eq!(
        classify_error(&anyhow!("some weird failure")),
        "an unknown error"
    );
}

// --- should_scrub policy ---

#[test]
fn should_scrub_default_cloud_on() {
    let cfg = Config::default();
    assert!(should_scrub(&cfg));
}

#[test]
fn should_scrub_ollama_always_off() {
    let mut cfg = Config::default();
    cfg.liaison.name = "ollama".into();
    cfg.liaison.scrub_secrets = true;
    assert!(!should_scrub(&cfg), "local provider should skip scrub");
}

#[test]
fn should_scrub_openai_compat_localhost_off() {
    let mut cfg = Config::default();
    cfg.liaison.name = "openai_compat".into();
    cfg.liaison.base_url = "http://localhost:11434/v1".into();
    cfg.liaison.scrub_secrets = true;
    assert!(!should_scrub(&cfg));
}

#[test]
fn should_scrub_openai_compat_remote_respects_flag() {
    let mut cfg = Config::default();
    cfg.liaison.name = "openai_compat".into();
    cfg.liaison.base_url = "https://api.groq.com/openai/v1".into();
    cfg.liaison.scrub_secrets = true;
    assert!(should_scrub(&cfg));

    cfg.liaison.scrub_secrets = false;
    assert!(!should_scrub(&cfg));
}

#[test]
fn should_scrub_cloud_respects_opt_out() {
    let mut cfg = Config::default();
    cfg.liaison.scrub_secrets = false;
    assert!(!should_scrub(&cfg));
}
