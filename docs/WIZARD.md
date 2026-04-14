# Wizard Flows

`dialoguer` + `console`. Two entry points:

- `loquitor init` — full first-run setup, walks every sub-flow. Re-runnable; reuses keys via `config::try_load_for_wizard`.
- `loquitor configure <target>` — non-destructive, edit one slice. Targets: `tts`, `liaison`, `voice`, `lane-policy`, `all`. `provider` is a hidden deprecated alias for `tts`.

## `loquitor init`

```
1. Banner       — ASCII logo + tagline + version
2. Pick TTS     — provider, then "Keep existing key" / "Enter new" if matching
3. Pick liaison — provider, key (with TTS-key reuse if same provider), model
4. Pick voice   — list from TTS provider; pre-selects current default
5. Audio test   — synthesise a known phrase, play, confirm
6. Save + summary card + tip section
```

Step 3's API-key prompt offers up to three rows in priority order:

1. **Keep the existing liaison key** — only if a non-empty key for the chosen provider is already in `[liaison]`
2. **Reuse your {provider} TTS key (same account)** — only if `[tts].name == [liaison].name` and `[tts].api_key` is non-empty (and not the same string already shown above)
3. **Enter a new key** — always available

If only #3 applies, the menu is skipped — straight to a `Password` prompt.

Step 3's model prompt shows the provider's curated model list plus a `Custom model id…` row at the bottom. Pre-selection rules:

- `current_model` matches a curated row → that row is default
- `current_model` is non-empty but off-menu → `Custom` is default, the `Input` is pre-filled with the existing id
- otherwise → first (flagship) row is default

## `loquitor configure tts`

Non-destructive: loads existing config, runs the TTS sub-flow, saves only the changed fields. Provider switch resets `voice.pool` to the new default voice (since old pool entries are meaningless under a new provider). Voice-only change preserves the pool and adds the new voice if missing.

## `loquitor configure liaison`

Same shape as the liaison portion of `init`. Still offers TTS-key reuse if the picked liaison provider matches the existing TTS provider.

## `loquitor configure voice`

Skips provider selection entirely — uses the current `[tts]` provider, just re-picks a voice. Pool gets extended (not reset) if the new voice isn't already in it.

## `loquitor configure lane-policy`

One prompt: Shared (every lane uses `voice.default`) vs Per-lane (`lanes.rules[*].voice` wins, default is fallback). Mutates only `voice.mode`.

## `loquitor configure all`

Walks `tts → liaison → voice → lane-policy` in order. Same non-destructive semantics — no defaults overwrite anything you didn't touch.

## Migration from v0.1.0

`config::is_legacy_format()` detects a top-level `[provider]` block. `config::try_load_for_wizard()` reads the legacy block into a `Config` whose `tts` field carries the migrated provider/key/model — `liaison` stays at defaults so `select_liaison` prompts fresh on first run. Result: a v0.1.0 user running `loquitor init` after upgrading sees their OpenAI key already on file, picks "Keep the existing key", and only has to enter a liaison key.

## Failure handling

Each provider's `list_voices` / `synthesize` is wrapped in `anyhow::Result`. Wizard surfaces errors inline with troubleshooting tips, then offers:

- Try again
- Try a different voice
- Switch to macOS Say (free fallback) — TTS only
- Skip for now

The audio test step will recurse on "play it again" so the user can iterate without restarting the wizard.

## Source files

- `src/wizard/mod.rs` — orchestrator (`run_wizard`, `configure_*`)
- `src/wizard/provider.rs` — TTS provider + key sub-flow
- `src/wizard/liaison.rs` — liaison provider + key + model sub-flow
- `src/wizard/voice.rs` — voice picker
- `src/wizard/policy.rs` — Shared / Per-lane toggle
- `src/wizard/test.rs` — audio test + recursion-on-replay
