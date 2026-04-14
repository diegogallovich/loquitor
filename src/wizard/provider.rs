use crate::config::types::ProviderConfig;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Password, Select};

const PROVIDERS: &[(&str, &str, &str)] = &[
    (
        "openai",
        "OpenAI TTS",
        "$15/M chars, simple setup, good quality",
    ),
    (
        "elevenlabs",
        "ElevenLabs",
        "Best voices, lowest latency, from $5/mo",
    ),
    (
        "minimax",
        "MiniMax",
        "$60/M chars, multilingual, expressive",
    ),
    (
        "macos_say",
        "macOS Say",
        "Free, offline, built-in (lower quality)",
    ),
];

/// Prompt the user to pick a TTS provider and (for cloud providers) an API key.
///
/// When `current` is `Some`, the current provider is pre-selected in the list.
/// If the user re-selects the same provider and already has a non-empty API key,
/// they're offered "keep existing key / enter new key" rather than being forced
/// to retype it — useful for `loquitor configure` flows.
pub fn select_provider(current: Option<&ProviderConfig>) -> Result<ProviderConfig> {
    let items: Vec<String> = PROVIDERS
        .iter()
        .map(|(_, name, desc)| format!("{name} — {desc}"))
        .collect();

    let default_idx = current
        .and_then(|c| PROVIDERS.iter().position(|(id, _, _)| *id == c.name))
        .unwrap_or(0);

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your TTS provider")
        .items(&items)
        .default(default_idx)
        .interact()?;

    let (id, _, _) = PROVIDERS[selection];
    let id = id.to_string();

    let (api_key, model) = if id != "macos_say" {
        let reuse_existing = current
            .filter(|c| c.name == id && !c.api_key.is_empty())
            .is_some();

        let api_key = if reuse_existing {
            prompt_key_reuse(&id, current.unwrap())?
        } else {
            prompt_new_key(&id)?
        };

        // Preserve the existing model when the provider is unchanged; otherwise
        // seed the provider's conventional default.
        let model = if current.map(|c| c.name.as_str()) == Some(id.as_str()) {
            current.map(|c| c.model.clone()).unwrap_or_default()
        } else {
            default_model_for(&id)
        };

        (api_key, model)
    } else {
        (String::new(), String::new())
    };

    Ok(ProviderConfig {
        name: id,
        api_key,
        model,
    })
}

fn prompt_new_key(provider_id: &str) -> Result<String> {
    let key_url = match provider_id {
        "openai" => "https://platform.openai.com/api-keys",
        "elevenlabs" => "https://elevenlabs.io/app/settings/api-keys",
        "minimax" => "https://www.minimax.io/platform",
        _ => "",
    };

    println!();
    println!("  {} {}", "Get one at:".dimmed(), key_url.dimmed());
    println!();

    let api_key = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("  API key")
        .interact()?;

    println!();
    println!(
        "{}",
        "  Your key is stored locally in ~/.config/loquitor/config.toml".dimmed()
    );
    println!(
        "{}",
        "  and never sent anywhere except the provider's API.".dimmed()
    );
    println!();

    Ok(api_key)
}

/// Offered when the picked provider matches the current one and a key is
/// already on file — saves the user from retyping a known-good key.
fn prompt_key_reuse(provider_id: &str, current: &ProviderConfig) -> Result<String> {
    println!();
    println!(
        "  {} {}",
        "Existing API key on file for".dimmed(),
        provider_id.cyan()
    );
    println!();

    let options = ["Keep the existing key", "Enter a new key"];
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("API key")
        .items(&options)
        .default(0)
        .interact()?;

    if choice == 0 {
        Ok(current.api_key.clone())
    } else {
        prompt_new_key(provider_id)
    }
}

fn default_model_for(provider_id: &str) -> String {
    match provider_id {
        "openai" => "tts-1".into(),
        "minimax" => "speech-02-turbo".into(),
        _ => String::new(),
    }
}
