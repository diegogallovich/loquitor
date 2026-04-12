# Contributing to Loquitor

Thanks for your interest in contributing! Loquitor is a small, focused tool and we aim to keep it that way.

## Development Setup

```bash
git clone https://github.com/diegogallovich/loquitor
cd loquitor
cargo build
cargo test
```

Loquitor requires **Rust 1.85+** (for edition 2024 transitive dependencies) and currently targets **macOS** primarily. Linux builds work for non-macOS TTS providers (OpenAI, ElevenLabs, MiniMax).

## Running Tests

```bash
cargo test              # All tests
cargo test --test parser_test   # One test file
cargo clippy -- -D warnings     # Lints
cargo fmt --check       # Formatting
```

CI runs all four checks on every PR (fmt, clippy, test, build).

## Architecture

The daemon is a single tokio process with a channel-connected pipeline:

```
shell hook → log files → directory watcher → lane watchers → TTS worker → audio queue
```

Each module lives in its own directory under `src/`:

- `config/` — TOML config load/save + types
- `tts/` — Provider trait + 4 implementations (OpenAI, ElevenLabs, MiniMax, macOS Say)
- `audio/` — Playback queue with stale-drop
- `watcher/` — Directory watcher + lane watchers + 6-stage parser
- `shell/` — zsh hook install/remove
- `daemon/` — Lifecycle, IPC server, pipeline orchestrator
- `wizard/` — Interactive setup wizard

## Pull Request Guidelines

1. **One concern per PR.** Separate formatting/refactor PRs from feature/bugfix PRs.
2. **Add tests** for new logic. Parser changes especially — add a test case to `tests/parser_test.rs`.
3. **Run `cargo fmt` and `cargo clippy`** before pushing.
4. **Write descriptive commit messages** using the conventional commits format (`feat:`, `fix:`, `refactor:`, `docs:`, etc.).

## Reporting Issues

- **Bugs:** Include your OS, Rust version, TTS provider, and steps to reproduce.
- **Feature requests:** Describe the problem first, then the proposed solution.
- **Security:** Email the maintainer privately — don't file a public issue.

## Adding a New TTS Provider

1. Create a new file in `src/tts/` (e.g., `my_provider.rs`).
2. Implement the `TtsProvider` trait — see `openai.rs` as a reference.
3. Register your provider in `src/tts/mod.rs` under `create_provider()`.
4. Add it to the wizard's provider list in `src/wizard/provider.rs`.
5. Document the provider in `README.md`.

Keep provider implementations minimal — error propagation via anyhow, no caching, no retry logic (that's the daemon's job).

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
