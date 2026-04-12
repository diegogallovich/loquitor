pub mod provider;
pub mod test;
pub mod voice;

use crate::config::{self, types::Config};
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

    // Step 5: Save config
    let mut cfg = Config::default();
    cfg.provider = provider_config;
    cfg.voice.default = voice_id;
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

    println!("  ┌─────────────────────────────────────────────────┐");
    println!("  │  Provider:  {:<36}│", provider_display.green());
    println!("  │  Model:     {:<36}│", model_display.green());
    println!("  │  Voice:     {:<36}│", voice_display.green());
    println!("  │  Config:    {:<36}│", "~/.config/loquitor/config.toml".dimmed());
    println!("  └─────────────────────────────────────────────────┘");
    println!();
    println!("  Get started:");
    println!("  {}", "$ loquitor enable".purple());
    println!("{}", "  Then open a new terminal tab and run claude.".dimmed());

    // Tip section
    println!();
    println!("{}", "  ─────────────────────────────────────────────────".dimmed());
    println!("{}", "  Loquitor is free and open source.".dimmed());
    println!("{}", "  If it saves you time, consider tipping the creator:".dimmed());
    println!();
    println!("{}", "  SOL/USDC/USDT: [address]".dimmed());
    println!("{}", "  ETH/USDC/USDT: [address]".dimmed());
    println!("{}", "  BTC:           [address]".dimmed());
    println!("{}", "  TON:           [address]".dimmed());
    println!();
}
