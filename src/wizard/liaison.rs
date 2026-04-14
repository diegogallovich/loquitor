//! Liaison (summarizer LLM) sub-wizard. Pick a provider, pick (or
//! keep) the API key, pick (or type) a model. Symmetric with
//! `wizard/provider.rs` for TTS.

use crate::config::types::LiaisonConfig;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};

struct ProviderDef {
    id: &'static str,
    display: &'static str,
    blurb: &'static str,
    key_url: &'static str,
    /// Curated model list for the in-wizard menu. Users can always
    /// enter a custom model id via the "Custom model id…" option, so
    /// this list doesn't need to enumerate every valid model — just
    /// the sensible defaults. Keep recent-flagship → balanced → cheap
    /// for an intuitive top-to-bottom reading order.
    models: &'static [&'static str],
}

const PROVIDERS: &[ProviderDef] = &[
    ProviderDef {
        id: "anthropic",
        display: "Anthropic Claude",
        blurb: "Strongest summaries. Opus is pricier; Haiku is cheap and fast.",
        key_url: "https://console.anthropic.com/settings/keys",
        models: &[
            "claude-opus-4-6",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
        ],
    },
    ProviderDef {
        id: "openai",
        display: "OpenAI",
        blurb: "Wide model range from mini to reasoning models.",
        key_url: "https://platform.openai.com/api-keys",
        models: &[
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "o3-mini",
        ],
    },
    ProviderDef {
        id: "minimax",
        display: "MiniMax",
        blurb: "Reuse your MiniMax TTS key. Chat API is OpenAI-shaped.",
        key_url: "https://www.minimax.io/platform",
        models: &[
            "MiniMax-Text-01",
            "MiniMax-M1",
        ],
    },
];

const CUSTOM_MODEL_LABEL: &str = "Custom model id…";

pub fn select_liaison(current: Option<&LiaisonConfig>) -> Result<LiaisonConfig> {
    let provider_idx = select_provider_index(current)?;
    let provider = &PROVIDERS[provider_idx];

    let api_key = resolve_api_key(provider, current)?;
    let model = select_model(provider, current.map(|c| c.model.as_str()))?;

    Ok(LiaisonConfig {
        name: provider.id.to_string(),
        api_key,
        model,
        base_url: String::new(),
        // Defaults mirror Config::default() + serde defaults. Keep in
        // sync if those change.
        max_output_tokens: 120,
        timeout_secs: 15,
        scrub_secrets: true,
    })
}

fn select_provider_index(current: Option<&LiaisonConfig>) -> Result<usize> {
    let items: Vec<String> = PROVIDERS
        .iter()
        .map(|p| format!("{} — {}", p.display, p.blurb))
        .collect();

    let default_idx = current
        .and_then(|c| PROVIDERS.iter().position(|p| p.id == c.name))
        .unwrap_or(0);

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your summary LLM (liaison)")
        .items(&items)
        .default(default_idx)
        .interact()?;
    Ok(selection)
}

fn resolve_api_key(provider: &ProviderDef, current: Option<&LiaisonConfig>) -> Result<String> {
    let reuse_existing = current
        .filter(|c| c.name == provider.id && !c.api_key.is_empty())
        .is_some();
    if reuse_existing {
        prompt_key_reuse(provider, current.unwrap())
    } else {
        prompt_new_key(provider)
    }
}

/// Model menu with the curated list + a "Custom model id…" escape
/// hatch. If `current_model` is known, it's pre-selected; if it's a
/// non-empty custom string, the "Custom" row is pre-selected and its
/// value is used as the default Input text.
fn select_model(provider: &ProviderDef, current_model: Option<&str>) -> Result<String> {
    let mut items: Vec<String> = provider.models.iter().map(|m| m.to_string()).collect();
    items.push(CUSTOM_MODEL_LABEL.to_string());
    let custom_idx = items.len() - 1;

    let default_idx = match current_model {
        Some(m) if !m.is_empty() => provider
            .models
            .iter()
            .position(|&known| known == m)
            .unwrap_or(custom_idx),
        _ => 0,
    };

    println!();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Model for {}", provider.display))
        .items(&items)
        .default(default_idx)
        .interact()?;

    if choice == custom_idx {
        // Free-form entry. Pre-fill with the current model id if the
        // user was already on a non-standard string so editing is cheap.
        let initial = current_model
            .filter(|m| !m.is_empty() && !provider.models.contains(m))
            .unwrap_or("")
            .to_string();
        let model: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("  Model id")
            .with_initial_text(initial)
            .interact_text()?;
        Ok(model.trim().to_string())
    } else {
        Ok(provider.models[choice].to_string())
    }
}

fn prompt_new_key(provider: &ProviderDef) -> Result<String> {
    println!();
    if !provider.key_url.is_empty() {
        println!(
            "  {} {}",
            "Get one at:".dimmed(),
            provider.key_url.dimmed()
        );
    }
    println!();

    let api_key = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("  API key")
        .interact()?;

    println!();
    println!(
        "{}",
        "  The key is stored locally in ~/.config/loquitor/config.toml".dimmed()
    );
    println!(
        "{}",
        "  and only sent to the provider's API when summarizing a turn.".dimmed()
    );
    println!();

    Ok(api_key)
}

fn prompt_key_reuse(provider: &ProviderDef, current: &LiaisonConfig) -> Result<String> {
    println!();
    println!(
        "  {} {}",
        "Existing API key on file for".dimmed(),
        provider.id.cyan()
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
        prompt_new_key(provider)
    }
}
