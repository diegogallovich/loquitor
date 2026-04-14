# Loquitor — Claude Code orientation

Smart-notification daemon for AI coding agents. Ships v0.2.0+: live-speech model from v0.1.0 was deleted, replaced by an idle-detected, LLM-summarised, lane-aware notification pipeline.

## Where to look

| For | Read |
|---|---|
| User-facing pitch + install | [`README.md`](README.md) |
| Pipeline + state machines + traits | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| CLI surface, config schema, providers | [`docs/DESIGN.md`](docs/DESIGN.md) |
| init / configure flows | [`docs/WIZARD.md`](docs/WIZARD.md) |
| What's left for `cargo install` + brew | [`docs/RELEASE-INFRA.md`](docs/RELEASE-INFRA.md) |
| Web app conventions | [`web/CLAUDE.md`](web/CLAUDE.md) → [`web/AGENTS.md`](web/AGENTS.md) |

Project-management context (status, decisions, in-flight work) lives at `~/Documents/Work/Open Source/Loquitor/` — only useful for PM-shaped sessions, not for coding work.

## Tech stack

- **Rust** edition 2021, MSRV ~1.94 (some transitive deps need it)
- **tokio** (`current_thread` flavour), `reqwest` for HTTP, `rodio` for playback, `notify` for FS events, `dialoguer` for the wizard
- **Next.js 16** + standalone Docker output for the web app at `/web` (see `web/AGENTS.md` — the version is breaking enough that training-era Next.js patterns may not apply)

## Build / test commands

```bash
cargo fmt                          # required to pass CI
cargo clippy --all-targets -- -D warnings
cargo test                         # all unit + integration tests
cargo build --release              # used by release.yml on tag push

# Web
cd web && npm run build
```

## Conventions

- **Tests** live in `tests/` (integration) — module-internal unit tests are inline only when no other consumer needs the helpers
- **CI rejects unformatted code** — `cargo fmt` before pushing, every time
- **Pure-function state machines** for anything timing-sensitive (`src/watcher/idle.rs` is the model — `feed(state, class, now, cfg) -> Option<Event>` so tests can fabricate `Instant`s)
- **No silent failures** — `anyhow::Result` propagates, the daemon's TTS / liaison errors degrade to `tracing::warn!` plus a canned spoken fallback ("Regarding X. Summary unavailable — {reason}.")
- **Provider trait pattern**: `TtsProvider` and `LiaisonProvider` are sibling async traits; new backends are one file in `src/tts/` or `src/liaison/` plus a row in `create_provider`
- **Secrets**: regex-scrubbed by `src/liaison/scrub.rs` before any cloud LLM call. Local providers (ollama, `openai_compat` against localhost) skip the scrub

## Repo layout

```
src/
├── main.rs                  # clap CLI entry
├── audio/                   # AudioQueue + rodio playback
├── config/                  # Config types + load/save + resolve_voice/_lane_name
├── daemon/
│   ├── mod.rs               # PID file + signal handling
│   ├── pipeline.rs          # Wires the full pipeline together
│   └── liaison_worker.rs    # TurnReady → SummarizedTurn (concurrent, FIFO out)
├── liaison/                 # LiaisonProvider trait + Anthropic / OpenAI / MiniMax
├── shell/                   # zshrc hook install/strip
├── tts/                     # TtsProvider trait + 4 backends
├── watcher/
│   ├── directory.rs         # notify-driven; spawns LaneWatchers
│   ├── lane.rs              # Per-lane tail + turn buffer + TurnReady emission
│   ├── idle.rs              # Pure-function idle detector state machine
│   └── parser.rs            # ANSI strip + cursor-forward expansion + ⏺ detection
└── wizard/                  # init + configure sub-flows
tests/                       # Integration tests, one file per module
web/                         # Next.js 16 landing page (loquitor.reachdiego.com)
.github/workflows/
├── ci.yml                   # fmt + clippy + test on every push
└── release.yml              # cross-platform binary build + GH Release on v* tags
```

## Working with this codebase

- v0.2.0+ deleted the per-line speakability parser entirely. If you're tempted to add per-line speech filtering back, read `docs/ARCHITECTURE.md` first — it explains the design pivot
- The web app may be on a Next.js version newer than your training cutoff. Check `web/node_modules/next/dist/docs/` before writing components
- The release pipeline builds binaries but does **not** publish to crates.io or update a Homebrew tap yet — `docs/RELEASE-INFRA.md` tracks both
