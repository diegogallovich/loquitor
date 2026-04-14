# Loquitor Architecture (v0.2.0+)

Single-process daemon with channel-connected async stages. Replaces the v0.1.0 per-line speech model with idle-detected, LLM-summarised, lane-aware notifications.

## Pipeline

```
loquitor enable
  ├─ installs zsh hook in ~/.zshrc
  └─ starts background daemon

User opens a terminal tab → runs `claude`
  └─ shell hook wraps the command with `script -q`
     and writes ~/.config/loquitor/lanes/<cwd-basename>-<unix-ts>.log

Daemon (single tokio process, current_thread flavour):

    DirectoryWatcher (notify) — watches lanes_dir for new .log files
            │
            │ spawns one LaneWatcher per file
            ▼
    LaneWatcher (per session)
      • tails raw bytes, splits on \n AND \r (TUI redraws)
      • strip_ansi → cleaned line
      • idle::classify → LineClass::PromptFrame | Content
      • idle::feed (state machine) → maybe TurnEvent::TurnEnded
      • appends content to bounded turn buffer (256 KB cap, front-truncates)
            │
            │ on TurnEnded: emits TurnReady { lane_id, turn_text,
            │                                 started_at, ended_at,
            │                                 truncated }
            ▼  mpsc<TurnReady>  (cap 16)
    LiaisonWorker (single task)
      • spawns a tokio task per TurnReady (concurrent LLM calls)
      • futures::FuturesOrdered preserves arrival order on the way out
      • per-turn: scrub secrets → call LiaisonProvider::summarize_turn
      • on error: canned fallback "Summary unavailable — {reason}"
      • prepends "Regarding {lane_name}. " deterministically
            │
            ▼  mpsc<SummarizedTurn>  (cap 16)
    TTS worker (the daemon's main task)
      • config::resolve_voice(cfg, lane_id) → voice id
      • TtsProvider::synthesize → AudioData
            │
            ▼  mpsc<Utterance>  (cap 50)
    AudioQueue
      • serial playback via spawn_blocking + rodio
      • stale-drop: skip if Utterance.enqueued_at > 15s old at dequeue
      • lanes never overlap
            │
            ▼
        speaker
```

## Idle detector (`src/watcher/idle.rs`)

Pure state machine — no tasks, no sleeps, no I/O. Tests fabricate `Instant`s to walk synthetic timelines.

```rust
pub enum IdleState {
    Idle,
    Collecting     { turn_started: Instant },
    PossibleIdle   { since: Instant, frames: u32, last_frame: String, turn_started: Instant },
}

pub enum LineClass { PromptFrame(String), Content }

pub enum TurnEvent { TurnEnded }

pub fn classify(cleaned: &str) -> LineClass;
pub fn feed(state: &mut IdleState, class: LineClass, now: Instant, cfg: &IdleCfg) -> Option<TurnEvent>;
```

**Detection rule** (verified against live Claude Code logs):

- `is_prompt_frame` ≡ cleaned line's non-whitespace chars are all in U+2500–U+257F (Unicode Box Drawing).
- `Idle → Collecting` on first `Content`.
- `Collecting → PossibleIdle` on first `PromptFrame`.
- `PossibleIdle → Idle (emit TurnEnded)` once `confirm_frames` *identical* prompt frames in a row AND `min_silence` elapsed since the first.
- Any `Content` while in `PossibleIdle` returns to `Collecting` (Claude resumed mid-flash).
- Force-ship: `Collecting` past `turn_max_duration` (default 30 min) emits `TurnEnded` on the next content line — catches a hung session.

Defaults are wired from `[daemon]`: `idle_confirm_frames = 3`, `idle_min_silence_ms = 500`, `turn_max_duration_secs = 1800`.

## Turn buffer (`src/watcher/lane.rs`)

`VecDeque<String>`, bounded by `daemon.turn_buffer_max_bytes` (default 256 KB). Append-on-content; prompt frames are skipped (they're TUI chrome, not narrative). On overflow, front-pop whole lines until back under 80% of the cap, set `truncated = true`, prepend `[earlier output truncated]` on flush.

`process_line(raw)` and `process_line_at(raw, now)` are both public — the `_at` variant lets integration tests drive the watcher with synthetic timestamps without sleeps.

## LiaisonProvider trait (`src/liaison/`)

```rust
#[async_trait]
pub trait LiaisonProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn summarize_turn(&self, ctx: &TurnContext<'_>) -> Result<TurnSummary>;
}

pub struct TurnContext<'a> {
    pub cleaned_log: &'a str,        // post-scrub
    pub max_output_tokens: u32,
}
pub struct TurnSummary { pub text: String }
```

Backends: `anthropic`, `openai`, `minimax`. `OpenAiProvider::with_endpoint(...)` is the reusable shape — MiniMax delegates to it with a different URL. Adding xAI / Groq / Mistral / DeepSeek / Ollama via an `openai_compat` adapter is one match arm in `create_provider`.

The shared prompt (`src/liaison/prompt.rs`) is a short system-message asking for one present-tense sentence under 40 words. The lane announcement (`Regarding {lane}.`) is **not** in the prompt — `liaison_worker::handle_turn` prepends it deterministically so the LLM can't forget or reword.

## Secret scrubber (`src/liaison/scrub.rs`)

Regex pass over the turn buffer before any cloud LLM call:

- `sk-[A-Za-z0-9_-]{20,}`, `sk-ant-…`
- `gh[pousr]_[A-Za-z0-9]{30,}`
- `AKIA[0-9A-Z]{16}`, `AIza[0-9A-Za-z_-]{35}`, `ya29\.…`
- `eyJ…\.…\.…` (JWT)
- `Bearer [A-Za-z0-9_.-]{20,}`

Skipped for local providers (`ollama`, or `openai_compat` with `base_url` resolving to localhost / 127.0.0.1 / *.local). Always-on for everything else; users can flip `liaison.scrub_secrets = false` in config to opt out (default on).

## TTS provider trait (`src/tts/`)

```rust
#[async_trait]
pub trait TtsProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn list_voices(&self) -> Result<Vec<Voice>>;
    async fn synthesize(&self, text: &str, voice: &VoiceId) -> Result<AudioData>;
    async fn synthesize_stream(&self, text: &str, voice: &VoiceId)
        -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>>;
}
```

Backends: `openai`, `elevenlabs`, `minimax`, `macos_say`. MiniMax is the odd one — its TTS response is hex-encoded inside `data.audio` JSON, decoded inside the provider.

## Audio queue (`src/audio/mod.rs`)

```rust
pub struct Utterance {
    pub lane_id: LaneId,
    pub audio: AudioData,
    pub enqueued_at: Instant,
    pub text: String,
}
```

Single global serial queue. Stale-drop check (`enqueued_at + stale_threshold < now`) at dequeue keeps notifications current during bursts. Playback is offloaded to `spawn_blocking` so rodio's blocking I/O doesn't stall tokio.

## Lane identity & voice resolution

- `lane_id` ≡ basename of cwd at hook time (parsed from log filename via `lane_id_from_path`).
- `config::resolve_voice(cfg, lane_id)`:
  - if `voice.mode == Shared` → `voice.default`
  - else (`PerLane`, the default) → `lanes.rules[lane_id].voice` if present, else `voice.default`
- `config::resolve_lane_name(cfg, lane_id)`:
  - `lanes.rules[lane_id].name` if present and non-empty, else the lane_id itself
- Both helpers are pure — covered by `tests/config_test.rs`.

## Daemon lifecycle

- `loquitor enable`: writes PID to `daemon.pid_file`, installs the zsh hook, then runs `pipeline::run` in the foreground (it owns the tokio runtime).
- `loquitor disable`: SIGTERM via PID file, strips the hook from `~/.zshrc`.
- IPC (`src/daemon/ipc.rs`) is a Unix-socket skeleton — wired up but not used by any subcommand yet; reserved for future runtime introspection / lane control.
