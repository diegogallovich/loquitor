# Release Infrastructure TODO

## Why this exists

When v0.2.0 shipped (smart-notification pivot, tag `v0.2.0`, April 2026), the website at [loquitor.reachdiego.com](https://loquitor.reachdiego.com) had to drop the `cargo install loquitor` and `brew install diegogallovich/tap/loquitor` rows from the install section because **neither install method actually works today**. The crate name on crates.io is unclaimed and there's no `diegogallovich/homebrew-tap` repository.

Today's `release.yml` only does the binary-build half of a release: cross-compiles for `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, packages each as a `.tar.gz`, and attaches them all to a GitHub Release on every `v*` tag push. That's solid — but it's not what most users expect from a Rust CLI.

This doc captures what's left so a future session (or future me) can finish the release pipeline in one focused sitting.

## What works today (don't redo)

- `cargo build --release` cross-platform via `release.yml`
- Binaries published to <https://github.com/diegogallovich/loquitor/releases> on every `v*` tag
- Auto-generated release notes via `softprops/action-gh-release@v2` with `generate_release_notes: true`
- CI workflow (`ci.yml`) runs `cargo fmt --check` + `cargo clippy` + `cargo test` on every push
- Website (`loquitor.reachdiego.com`) redeploys via Railway on every push to `main` that touches `/web/`

## What's missing

### 1. crates.io publish

**Goal:** `cargo install loquitor` works.

Steps (sequential — order matters):

1. Generate a publish-scoped API token at <https://crates.io/me> — name it `loquitor-release` for clarity
2. Locally: `cargo login <token>` then `cd ~/Dev/Open\ Source/loquitor && cargo publish` to claim the name (one-time first publish must come from a logged-in machine, not CI)
3. Add the same token as a GitHub repo secret named `CARGO_REGISTRY_TOKEN` (`gh secret set CARGO_REGISTRY_TOKEN -R diegogallovich/loquitor`)
4. Append a `publish-crate` job to `release.yml` that runs `cargo publish --token ${{ secrets.CARGO_REGISTRY_TOKEN }}` after the `release` job succeeds. Skip on dry runs / re-tags.

Sanity checks before first publish:

- `Cargo.toml` already has `description`, `license = "MIT"`, `repository`, `keywords`, `categories` — required by crates.io
- `README.md` exists at the crate root — referenced as the package's long description
- No path-only dependencies in `[dependencies]` — all are crates.io versions
- `cargo package --list` to dry-run what gets uploaded; check no `.env`, no test fixtures, no API keys

### 2. Homebrew tap

**Goal:** `brew install diegogallovich/tap/loquitor` works.

Steps:

1. Create a new public GitHub repo: `diegogallovich/homebrew-tap` (any personal-tap repo named `homebrew-*` is auto-discovered by `brew tap`)
2. Initialise with a `Formula/` directory and a starter `Formula/loquitor.rb`. The formula should declare separate `on_macos` / `on_linux` blocks, each with a `url` to the matching `loquitor-<target>.tar.gz` from the GitHub Release and an `sha256` hash of that artifact
3. Generate a fine-grained PAT with **Contents: write** scope on the `homebrew-tap` repo only; store as `HOMEBREW_TAP_TOKEN` in the loquitor repo's secrets
4. Add a `bump-homebrew` job to `release.yml` using [`mislav/bump-homebrew-formula-action`](https://github.com/mislav/bump-homebrew-formula-action) — it computes the new sha256s, edits `Formula/loquitor.rb`, opens-and-merges a PR (or pushes directly) on the tap repo

Reference: this is the same pattern `gh`, `bat`, `eza`, etc. use — copy any of their tap repos as a template.

### 3. Re-enable the install rows on the website

Once #1 and #2 are live and verified working from a clean machine:

1. Edit `web/src/app/page.tsx` — restore the `cargo install loquitor` and `brew install diegogallovich/tap/loquitor` rows in the "Quick Install" section (they live in `git log -- web/src/app/page.tsx`'s pre-v0.2.0 history, easy to copy back)
2. Drop the "cargo install loquitor and brew tap install arrive in a near-future patch" note
3. Move "Or build from source" back to a secondary position
4. Push to `main` → Railway redeploys

### 4. Worth considering when the time comes

- **CHANGELOG.md** — `generate_release_notes: true` is OK but a curated `CHANGELOG.md` reads better in `cargo install` listings and on the Homebrew formula's `desc` block. Keepachangelog format is the safe choice.
- **MSRV pinning** — set `rust-version = "1.94"` in `Cargo.toml` so crates.io shows the minimum supported Rust version and `cargo install` errors out cleanly on too-old toolchains.
- **Channel split** — eventually want `v0.x.y-beta` releases for testing the next pivot before mainline. `release.yml`'s `tags: ["v*"]` filter already accepts these; the formula-bump job can skip pre-releases via an `if:` guard.

## When to do this

No hard deadline. Priority signal: any of these would justify scheduling it:

- A user asks how to install (other than from source) → time to ship `cargo install` at minimum
- The download URL on the homepage gets > 50 hits/week → friction-reduction has compounding value
- v0.3.0 pivots again → don't want to be doing this *and* a feature pivot in the same week
