# Loquitor Design (v0.2.0+)

> Hear when your agents need you.

Smart-notification daemon for Claude Code (and any future AI coding agent that emits a tail-able terminal log). Stays silent while Claude is working. When a turn ends, an LLM summarises what happened, the daemon prepends `Regarding {session}.`, and TTS speaks the result through your speakers.

## Why notifications, not narration

v0.1.0 spoke every narrative line live. After one session of dogfooding it became clear: a running monologue is noise. What you actually want is **one signal per turn-end** that tells you whether the agent needs you.

That's what every layer is now optimised for: detect the turn boundary, summarise once, speak once.

## Core UX

```bash
$ loquitor init      # one-time: pick TTS, pick liaison LLM, pick voice, test
$ loquitor enable    # install shell hook + start the background daemon
```

After that, `claude` in any terminal tab is auto-detected. When Claude finishes a turn and is waiting for input, you hear:

> *"Regarding loquitor. Refactored the parser and ran the tests; 12 pass. Waiting for you to review the diff."*

If something goes wrong with the LLM call (network, auth, rate limit) you still hear the lane signal:

> *"Regarding loquitor. Summary unavailable — authentication error."*

## CLI

```
loquitor init                                       First-run setup wizard
loquitor configure <tts | liaison | voice |
                    lane-policy | all>             Re-pick one slice non-destructively
loquitor enable                                     Install shell hook + start daemon
loquitor disable                                    Remove shell hook + stop daemon
loquitor status                                     PID, hook state, provider summary
loquitor lanes                                      List active lanes (planned IPC)
loquitor lane <id> --name <n> --voice <v>           Rename or revoice a lane (planned)
loquitor voices                                     List voices from configured TTS
loquitor test <text>                                Speak a test phrase via TTS
```

`configure provider` is kept as a hidden deprecated alias for `configure tts` (one-release grace period).

## Shell hook

`loquitor enable` writes (between markers, idempotent) into `~/.zshrc`:

```zsh
# --- Loquitor Shell Hook (managed by loquitor) ---
__loquitor_hook() {
  if [[ "$1" == "claude" ]]; then
    local lane_dir="$HOME/.config/loquitor/lanes"
    local logfile="$lane_dir/$(basename "$PWD")-$(date +%s).log"
    mkdir -p "$lane_dir"
    script -q "$logfile" claude "${@:2}"
  else
    command "$@"
  fi
}
alias claude='__loquitor_hook claude'
# --- End Loquitor Hook ---
```

The hook bakes the cwd basename into the log filename, which becomes the lane id. To work in a different "session" you `cd` and re-launch `claude` — lanes are immutable after spawn.

## Providers

### TTS (the voice that speaks notifications)

| id | endpoint shape | auth | streaming | notes |
|---|---|---|---|---|
| `openai` | POST /v1/audio/speech | Bearer | chunked | `tts-1` default |
| `elevenlabs` | POST /v1/text-to-speech/{voice} | `xi-api-key` | SSE | best voice quality |
| `minimax` | POST /v1/t2a_v2 | Bearer | hex-encoded JSON | multilingual, set `language_boost: "auto"` for English |
| `macos_say` | shells out to `say(1)` | — | no | free, offline, lower quality |

### Liaison (the LLM that summarises Claude turns)

| id | endpoint shape | curated models in wizard |
|---|---|---|
| `anthropic` | POST /v1/messages, `anthropic-version: 2023-06-01` | `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5` |
| `openai` | POST /v1/chat/completions | `gpt-5.4-pro`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-4o-mini` |
| `minimax` | POST /v1/text/chatcompletion_v2 | `MiniMax-M2.7`, `MiniMax-M2.5`, `MiniMax-M2` |

Each row in the model picker also shows a "Custom model id…" escape hatch so a user can type any model name (preview models, fine-tunes, models that ship after the curated list goes stale).

`OpenAiProvider::with_endpoint(...)` is the reusable adapter shape — MiniMax already delegates to it. Adding xAI / Groq / Mistral / DeepSeek / Ollama is one match arm in `liaison::create_provider` plus a row in `wizard/liaison.rs`.

## Configuration

Location: `~/.config/loquitor/config.toml`. The wizard manages it; manual edits are valid but rarely needed.

```toml
[tts]
name = "openai"            # openai | elevenlabs | minimax | macos_say
api_key = "sk-..."
model = "tts-1"

[liaison]
name = "anthropic"         # anthropic | openai | minimax
api_key = "sk-ant-..."
model = "claude-haiku-4-5"
base_url = ""              # only meaningful for openai_compat (future)
max_output_tokens = 120
timeout_secs = 15
scrub_secrets = true       # forced on for cloud providers anyway

[voice]
default = "nova"
pool = ["nova", "alloy"]
mode = "per_lane"          # Shared | PerLane

[lanes.rules]
# Optional. lane_id → friendly name + voice override
# [lanes.rules.loquitor]
# name = "Loquitor Dev"
# voice = "English_Graceful_Lady"

[queue]
stale_threshold_secs = 15
coalesce_window_ms = 2000

[parsing]
preserve_ansi_color_for_idle = true

[daemon]
socket_path = "/tmp/loquitor.sock"
pid_file = "/tmp/loquitor.pid"
log_level = "info"
idle_confirm_frames = 3
idle_min_silence_ms = 500
turn_buffer_max_bytes = 262144     # 256 KB
turn_max_duration_secs = 1800      # 30 min force-ship

[ui]
tip_shown = true
```

**Legacy migration**: `config::is_legacy_format()` detects a top-level `[provider]` block (v0.1.0 shape). `config::try_load_for_wizard()` migrates the legacy `[provider]` into `[tts]` so `loquitor init` can offer "Keep the existing key" instead of forcing re-entry.

## Repo structure

```
loquitor/
├── src/                    # See CLAUDE.md for the full layout
├── tests/                  # Integration tests, one file per module
├── web/                    # Next.js 16 landing page
├── docs/                   # ← you are here
├── .github/workflows/
│   ├── ci.yml              # fmt + clippy + test on every push
│   └── release.yml         # cross-platform binary build on v* tags
├── Cargo.toml
├── README.md
├── CLAUDE.md               # Claude Code orientation
├── CONTRIBUTING.md
└── LICENSE                 # MIT
```

## Tip integration

Surfaced once at end of `loquitor init` and once on the first `loquitor enable` (`ui.tip_shown` flag prevents repeats). Same on-chain addresses as the README and the website.
