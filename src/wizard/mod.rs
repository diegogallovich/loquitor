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

    // Step 2: Provider selection (+ API key if cloud provider)
    let provider_config = provider::select_provider()?;

    // Create provider instance for voice listing and testing
    let tts_provider = tts::create_provider(
        &provider_config.name,
        &provider_config.api_key,
        &provider_config.model,
    )?;

    // Step 3: Voice selection
    let voice_id = voice::select_voice(tts_provider.as_ref()).await?;

    // Step 4: Audio test
    let _audio_ok = test::test_audio(tts_provider.as_ref(), &voice_id).await?;

    // Step 5: Save config — pool seeded with just the chosen voice so it stays
    // provider-consistent. The user can add more voices by editing config.toml,
    // or we'll add a `loquitor voices add` command later.
    let default = Config::default();
    let cfg = Config {
        provider: provider_config,
        voice: VoiceConfig {
            default: voice_id.clone(),
            pool: vec![voice_id],
        },
        ..default
    };
    config::save(&cfg)?;

    // Step 6: Summary + tip
    print_summary(&cfg);

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

    let provider_display = cfg.provider.name.clone();
    let model_display = if cfg.provider.model.is_empty() {
        "(none)".to_string()
    } else {
        cfg.provider.model.clone()
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
