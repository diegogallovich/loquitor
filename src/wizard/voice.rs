use crate::tts::TtsProvider;
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};

/// Prompt the user to pick a voice from the provider's catalog.
/// When `current` matches a listed voice id, that row is pre-selected —
/// so `configure voice` can show the user their existing pick as the
/// default choice instead of resetting the cursor to position 0.
pub async fn select_voice(provider: &dyn TtsProvider, current: Option<&str>) -> Result<String> {
    let voices = provider.list_voices().await?;

    if voices.is_empty() {
        anyhow::bail!("Provider returned no voices. Check your API key and try again.");
    }

    let items: Vec<String> = voices
        .iter()
        .map(|v| format!("{} — {}", v.name, v.description))
        .collect();

    let default_idx = current
        .and_then(|c| voices.iter().position(|v| v.id == c))
        .unwrap_or(0);

    println!();
    println!("  This is the first voice in your pool. Add more in config.toml later.");
    println!();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your default voice")
        .items(&items)
        .default(default_idx)
        .interact()?;

    Ok(voices[selection].id.clone())
}
