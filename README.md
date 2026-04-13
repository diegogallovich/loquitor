# Loquitor

> Let your agents think out loud

Loquitor is a Rust CLI daemon that watches your AI coding agent's terminal output (starting with Claude Code), extracts its narrative thought lines, and speaks them aloud via TTS. It supports multiple concurrent sessions with different voices, so you can distinguish parallel workstreams by ear.

**Full docs and install guide:** [loquitor.reachdiego.com](https://loquitor.reachdiego.com)

## Quick Install

```bash
cargo install loquitor
```

Or download a pre-built binary from the [releases page](https://github.com/diegogallovich/loquitor/releases).

## Getting Started

```bash
loquitor init        # Setup wizard: pick TTS provider, voice, test audio
loquitor enable      # Install shell hook + start daemon
```

Then open a new terminal tab and run `claude` as you normally would. Loquitor will detect the session and start narrating.

## CLI

```
loquitor init        Run the first-time setup wizard
loquitor enable      Install shell hook and start the background daemon
loquitor disable     Remove shell hook and stop the daemon
loquitor status      Show daemon status
loquitor lanes       List active lanes
loquitor lane <id> --name <n> --voice <v>   Rename a lane or change its voice
loquitor voices      List available voices from the configured TTS provider
loquitor test <text> Speak a test phrase
```

## Supported TTS Providers

- **OpenAI TTS** — $15/M chars, simple setup, good quality
- **ElevenLabs** — Best voices, lowest latency, from $5/mo
- **MiniMax** — $60/M chars, multilingual, expressive
- **macOS Say** — Free, offline, built-in (lower quality)

## Requirements

- macOS (primary target for v0.1.0 — Linux support for non-macOS providers coming soon)
- zsh shell
- Rust 1.85+ (if building from source)

## Configuration

Loquitor stores its config at `~/.config/loquitor/config.toml`. The setup wizard populates this file — you rarely need to edit it by hand. See the [config reference](https://loquitor.reachdiego.com/config) for details on lane rules, voice pools, and parsing tuning.

## How It Works

1. `loquitor enable` installs a small zsh hook in `~/.zshrc` that wraps the `claude` command with `script -q`, capturing terminal output to `~/.config/loquitor/lanes/<project>-<timestamp>.log`.
2. The daemon watches that directory with `notify`. New `.log` files spawn per-lane watcher tasks.
3. Each lane watcher tails its log file, strips ANSI escapes, and uses a 6-stage parser to extract narrative thought lines (filtering out tool invocations, code blocks, file paths, etc.).
4. Speakable text is synthesized via the configured TTS provider and queued for playback. A single global audio queue plays utterances serially — lanes never talk over each other. Utterances older than 15 seconds are dropped to keep narration current.

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
