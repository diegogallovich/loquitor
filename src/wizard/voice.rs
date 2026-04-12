use crate::tts::TtsProvider;
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};

pub async fn select_voice(provider: &dyn TtsProvider) -> Result<String> {
    let voices = provider.list_voices().await?;

    if voices.is_empty() {
        anyhow::bail!("Provider returned no voices. Check your API key and try again.");
    }

    let items: Vec<String> = voices
        .iter()
        .map(|v| format!("{} — {}", v.name, v.description))
        .collect();

    println!();
    println!("  This is the first voice in your pool. Add more in config.toml later.");
    println!();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your default voice")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(voices[selection].id.clone())
}
