# Loquitor

> Hear when your agents need you.

Loquitor is a Rust CLI daemon that watches your AI coding agent's terminal output (starting with Claude Code), waits until each turn finishes, and speaks one short summary so you know what just shipped and what it's waiting for. Smart notifications, not a running monologue.

It supports multiple concurrent sessions ("lanes") with different voices, so parallel work-streams stay distinguishable by ear.

**Full docs and install guide:** [loquitor.reachdiego.com](https://loquitor.reachdiego.com)

## Quick Install

Pre-built binaries for macOS (Intel + Apple Silicon) and Linux (x86_64 + aarch64) on every release:

→ <https://github.com/diegogallovich/loquitor/releases/latest>

Or build from source:

```bash
git clone https://github.com/diegogallovich/loquitor
cd loquitor && cargo install --path .
```

`cargo install loquitor` and `brew install diegogallovich/tap/loquitor` are tracked for a near-future patch — see [`docs/RELEASE-INFRA.md`](docs/RELEASE-INFRA.md).

## Getting Started

```bash
loquitor init        # Pick TTS + summary LLM, model, voice, key reuse
loquitor enable      # Install shell hook + start daemon
```

Then open a new terminal tab and run `claude` as you normally would. Loquitor detects each session, waits for Claude to finish a turn, and announces what happened in one short sentence prefixed `Regarding {session}.`

## CLI

```
loquitor init                                    Run the first-time setup wizard
loquitor configure <tts | liaison | voice |
                    lane-policy | all>          Re-pick one slice non-destructively
loquitor enable                                  Install shell hook + start daemon
loquitor disable                                 Remove shell hook + stop daemon
loquitor status                                  Show daemon status
loquitor lanes                                   List active lanes
loquitor lane <id> --name <n> --voice <v>        Rename a lane or change its voice
loquitor voices                                  List voices from configured TTS
loquitor test <text>                             Speak a test phrase
```

## Supported Providers

**TTS (the voice that speaks notifications):**
- OpenAI TTS — simple setup, good quality
- ElevenLabs — best voices, lowest latency
- MiniMax — multilingual, expressive
- macOS Say — free, offline, built-in

**Liaison (the LLM that summarises Claude turns):**
- Anthropic Claude — Opus / Sonnet / Haiku
- OpenAI — GPT-5.4 family + legacy mini
- MiniMax — M2 series
- "Custom model id…" escape hatch for any model not in the curated list

## Requirements

- macOS or Linux
- zsh shell
- Rust 1.94+ (only if building from source)

## Configuration

Loquitor stores its config at `~/.config/loquitor/config.toml`. The setup wizard populates it — you rarely need to edit by hand. See [`docs/DESIGN.md`](docs/DESIGN.md) for the schema, and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for how the pieces fit together.

## How It Works

1. `loquitor enable` installs a small zsh hook in `~/.zshrc` that wraps the `claude` command with `script -q`, capturing terminal output to `~/.config/loquitor/lanes/<project>-<timestamp>.log`.
2. The daemon watches that directory with `notify`. Each new `.log` file spawns a per-lane watcher.
3. Each lane watcher tails its log, strips ANSI, and feeds lines into a pure-function idle-detector state machine that fires when Claude's input prompt stabilises (turn ended).
4. On turn-end, the buffered text is scrubbed for secrets (`sk-…`, `ghp_…`, JWT, etc.) and shipped to the configured liaison LLM, which returns one short sentence.
5. The summary is prefixed `Regarding {lane_name}.` and synthesised through your TTS provider.
6. A single global audio queue plays summaries serially across all lanes — they never overlap. Stale summaries (>15s old at dequeue) are dropped to keep notifications current.

Multiple simultaneous turn-ends summarise concurrently (one LLM call per lane, in parallel) and play in turn-end order — preserved via `futures::FuturesOrdered`.

## License

MIT. See [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Tip the Creator

Loquitor is free and open source. If it saves you time, consider tipping.

Easiest path: message me on Telegram at [**@diegogallovich**](https://t.me/diegogallovich) — we can settle up in whatever currency works for both of us.

On-chain addresses (each chain has one wallet that accepts the native token plus supported stables):

| Chain | Address | Accepts |
|---|---|---|
| Ethereum | `0xeA284b3EAd48388174d7A67c63DC1a3107FbEA16` | ETH, USDC, USDT |
| Solana | `BjykpVzwfBYqwN6oNieCKdTux7Derm9n1dqJtGoHSeQv` | SOL, USDC, USDT |
| TON | `UQA6_sZRQkkHspUssT7ruDwhDba3GuGR5qxVPtk2rDZlrLnc` | TON, USDT |
| Tron | `TWLftLqDRHJNXNv3UGF5vTALE2iXxhkyvF` | TRX, USDT |
| Bitcoin | `bc1qrsnavtmh97rqvvgusva3c0ytkrvammuhccxpdv` | BTC |
