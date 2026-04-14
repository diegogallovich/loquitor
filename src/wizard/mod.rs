pub mod liaison;
pub mod policy;
pub mod provider;
pub mod test;
pub mod voice;

use crate::config::{
    self,
    types::{Config, VoiceConfig},
};
use crate::tts;
use anyhow::Result;
use colored::Colorize;

pub async fn run_wizard() -> Result<()> {
    // Step 1: Banner
    print_banner();

    // Best-effort load of whatever config is already on disk, including
    // migration of legacy `[provider]` → `[tts]` so the sub-wizards can
    // offer "Keep existing key" for anything you already set up.
    let existing = config::try_load_for_wizard();
    let existing_tts = existing.as_ref().map(|c| &c.tts);
    let existing_liaison = existing.as_ref().map(|c| &c.liaison);

    // Step 2: TTS provider selection (+ API key if cloud provider)
    let tts_config = provider::select_provider(existing_tts)?;

    // Step 3: Liaison (summary LLM) provider selection. Pass the
    // freshly-chosen TTS config so the wizard can offer to reuse the
    // same key when both sides pick the same provider (OpenAI TTS +
    // OpenAI liaison = one account, one key).
    let liaison_config = liaison::select_liaison(existing_liaison, Some(&tts_config))?;

    // Instantiate the TTS provider so we can list voices and run the audio test.
    // The liaison provider isn't instantiated here — it'll be exercised end-to-end
    // at the demo step (PR6 turns this into a real synthetic-turn demo).
    let tts_provider =
        tts::create_provider(&tts_config.name, &tts_config.api_key, &tts_config.model)?;

    // Step 4: Voice selection
    let voice_id = voice::select_voice(tts_provider.as_ref(), None).await?;

    // Step 5: Audio test
    let _audio_ok = test::test_audio(tts_provider.as_ref(), &voice_id).await?;

    // Step 6: Save config — pool seeded with just the chosen voice so it stays
    // provider-consistent. The user can add more voices by editing config.toml,
    // or we'll add a `loquitor voices add` command later.
    let default = Config::default();
    let cfg = Config {
        tts: tts_config,
        liaison: liaison_config,
        voice: VoiceConfig {
            default: voice_id.clone(),
            pool: vec![voice_id],
            mode: default.voice.mode,
        },
        ..default
    };
    config::save(&cfg)?;

    // Step 7: Summary + tip
    print_summary(&cfg);

    Ok(())
}

/// Re-pick the TTS provider (and re-enter/keep the API key), then re-pick a
/// voice from the new provider's catalog. Other config sections are preserved.
///
/// Because switching providers invalidates any pool of voice IDs from the old
/// provider, the pool is reset to just the newly-chosen voice. A voice-only
/// change (via `configure_voice`) preserves the pool.
pub async fn configure_tts() -> Result<()> {
    let mut cfg = config::load()?;
    let new_provider = provider::select_provider(Some(&cfg.tts))?;
    let tts_provider = tts::create_provider(
        &new_provider.name,
        &new_provider.api_key,
        &new_provider.model,
    )?;

    let current_default = if cfg.tts.name == new_provider.name {
        Some(cfg.voice.default.as_str())
    } else {
        None
    };
    let new_voice = voice::select_voice(tts_provider.as_ref(), current_default).await?;
    let _ = test::test_audio(tts_provider.as_ref(), &new_voice).await?;

    let provider_changed = cfg.tts.name != new_provider.name;
    cfg.tts = new_provider;
    cfg.voice.default = new_voice.clone();
    cfg.voice.pool = if provider_changed {
        vec![new_voice]
    } else if !cfg.voice.pool.contains(&new_voice) {
        let mut pool = cfg.voice.pool;
        pool.push(new_voice);
        pool
    } else {
        cfg.voice.pool
    };

    config::save(&cfg)?;
    println!();
    println!("{}", "✓ TTS configuration saved.".green().bold());
    Ok(())
}

/// Re-pick the liaison LLM provider. Preserves every other config section.
/// PR2 ships only Anthropic; PR6 expands this with OpenAI, Google, MiniMax,
/// and an OpenAI-compatible adapter.
pub async fn configure_liaison() -> Result<()> {
    let mut cfg = config::load()?;
    let new_liaison = liaison::select_liaison(Some(&cfg.liaison), Some(&cfg.tts))?;
    cfg.liaison = new_liaison;
    config::save(&cfg)?;
    println!();
    println!(
        "{}",
        "✓ Liaison configuration saved. (End-to-end test will arrive with PR6.)"
            .green()
            .bold()
    );
    Ok(())
}

/// Re-pick a voice without touching the provider or API key. The existing
/// pool is preserved and extended if the new voice isn't already in it.
pub async fn configure_voice() -> Result<()> {
    let mut cfg = config::load()?;
    let tts_provider = tts::create_provider(&cfg.tts.name, &cfg.tts.api_key, &cfg.tts.model)?;

    let new_voice =
        voice::select_voice(tts_provider.as_ref(), Some(cfg.voice.default.as_str())).await?;
    let _ = test::test_audio(tts_provider.as_ref(), &new_voice).await?;

    cfg.voice.default = new_voice.clone();
    if !cfg.voice.pool.contains(&new_voice) {
        cfg.voice.pool.push(new_voice);
    }
    config::save(&cfg)?;
    println!();
    println!("{}", "✓ Voice saved.".green().bold());
    Ok(())
}

/// Flip between shared-voice and per-lane modes without touching any other
/// config section. Pure preference toggle.
pub async fn configure_lane_policy() -> Result<()> {
    let mut cfg = config::load()?;
    cfg.voice.mode = policy::select_lane_policy(cfg.voice.mode)?;
    config::save(&cfg)?;
    println!();
    println!("{}", "✓ Lane policy saved.".green().bold());
    Ok(())
}

/// Walk all three configure sub-flows in order. Unlike `run_wizard`, this is
/// non-destructive — each sub-flow loads-mutates-saves rather than rewriting
/// the whole config from defaults.
pub async fn configure_all() -> Result<()> {
    configure_tts().await?;
    configure_liaison().await?;
    configure_lane_policy().await?;
    Ok(())
}

fn print_banner() {
    println!();
    println!("{}", "  ╦   ╔═╗ ╔═╗ ╦ ╦ ╦ ╔╦╗ ╔═╗ ╦═╗".purple());
    println!("{}", "  ║   ║ ║ ║═╗ ║ ║ ║  ║  ║ ║ ╠╦╝".purple());
    println!("{}", "  ╩═╝ ╚═╝ ╚═╝ ╚═╝ ╩  ╩  ╚═╝ ╩╚═".purple());
    println!("{}", "  Let your agents think out loud".dimmed());
    println!("{}", format!("  v{}", env!("CARGO_PKG_VERSION")).dimmed());
    println!();
    println!("{}", "◆ Welcome to Loquitor setup!".cyan());
    println!("  This wizard will configure your TTS provider,");
    println!("  choose a voice, and test your audio output.");
    println!("{}", "  Press Ctrl+C at any time to exit.".dimmed());
    println!();
}

fn print_summary(cfg: &Config) {
    println!();
    println!("{}", "◆ Setup complete!".green().bold());
    println!();

    let provider_display = cfg.tts.name.clone();
    let model_display = if cfg.tts.model.is_empty() {
        "(none)".to_string()
    } else {
        cfg.tts.model.clone()
    };
    let voice_display = cfg.voice.default.clone();
    let config_path_display = config::config_path().to_string_lossy().into_owned();

    println!("  ┌─────────────────────────────────────────────────┐");
    println!("  │  Provider:  {:<36}│", provider_display.green());
    println!("  │  Model:     {:<36}│", model_display.green());
    println!("  │  Voice:     {:<36}│", voice_display.green());
    println!("  │  Config:    {}", config_path_display.dimmed());
    println!("  └─────────────────────────────────────────────────┘");
    println!();
    println!("  Get started:");
    println!("  {}", "$ loquitor enable".purple());
    println!(
        "{}",
        "  Then open a new terminal tab and run claude.".dimmed()
    );

    // Tip section
    println!();
    println!(
        "{}",
        "  ─────────────────────────────────────────────────".dimmed()
    );
    println!("{}", "  Loquitor is free and open source.".dimmed());
    println!(
        "{}",
        "  If it saves you time, consider tipping the creator:".dimmed()
    );
    println!();
    println!("{}", "  Telegram (direct):  @diegogallovich".dimmed());
    println!("{}", "  Ethereum (ETH/USDC/USDT):".dimmed());
    println!(
        "{}",
        "    0xeA284b3EAd48388174d7A67c63DC1a3107FbEA16".dimmed()
    );
    println!("{}", "  Solana (SOL/USDC/USDT):".dimmed());
    println!(
        "{}",
        "    BjykpVzwfBYqwN6oNieCKdTux7Derm9n1dqJtGoHSeQv".dimmed()
    );
    println!("{}", "  TON (TON/USDT):".dimmed());
    println!(
        "{}",
        "    UQA6_sZRQkkHspUssT7ruDwhDba3GuGR5qxVPtk2rDZlrLnc".dimmed()
    );
    println!("{}", "  Tron (TRX/USDT):".dimmed());
    println!("{}", "    TWLftLqDRHJNXNv3UGF5vTALE2iXxhkyvF".dimmed());
    println!("{}", "  Bitcoin:".dimmed());
    println!(
        "{}",
        "    bc1qrsnavtmh97rqvvgusva3c0ytkrvammuhccxpdv".dimmed()
    );
    println!();
}
