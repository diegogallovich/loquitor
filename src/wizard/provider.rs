use crate::config::types::ProviderConfig;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Password, Select};

pub fn select_provider() -> Result<ProviderConfig> {
    let providers = vec![
        ("openai", "OpenAI TTS", "$15/M chars, simple setup, good quality"),
        ("elevenlabs", "ElevenLabs", "Best voices, lowest latency, from $5/mo"),
        ("minimax", "MiniMax", "$60/M chars, multilingual, expressive"),
        ("macos_say", "macOS Say", "Free, offline, built-in (lower quality)"),
    ];

    let items: Vec<String> = providers
        .iter()
        .map(|(_, name, desc)| format!("{name} — {desc}"))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your TTS provider")
        .items(&items)
        .default(0)
        .interact()?;

    let (id, _, _) = providers[selection];
    let id = id.to_string();

    let (api_key, model) = if id != "macos_say" {
        let key_url = match id.as_str() {
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

        let model = match id.as_str() {
            "openai" => "tts-1".to_string(),
            "minimax" => "speech-02-turbo".to_string(),
            _ => String::new(),
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
