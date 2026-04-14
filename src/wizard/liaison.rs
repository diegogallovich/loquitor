//! Liaison (summarizer LLM) sub-wizard. Mirrors the structure of
//! `wizard/provider.rs` (TTS) so the flow feels symmetric: pick a
//! provider, enter/reuse an API key, pick a model. The actual list of
//! providers grows in PR6 to cover OpenAI, Google, MiniMax, and the
//! OpenAI-compatible adapter; PR2 ships only Anthropic.

use crate::config::types::LiaisonConfig;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Password, Select};

/// Each tuple is (id, display name, one-line description, default model,
/// default base_url). Only the `anthropic` row is live in PR2; the rest
/// ship with PR6 and will be added to this array in that PR.
const PROVIDERS: &[(&str, &str, &str, &str, &str)] = &[(
    "anthropic",
    "Anthropic (Claude Haiku 4.5)",
    "Fast, cheap (~$3/yr at 50 turns/day), recommended default",
    "claude-haiku-4-5",
    "",
)];

pub fn select_liaison(current: Option<&LiaisonConfig>) -> Result<LiaisonConfig> {
    let items: Vec<String> = PROVIDERS
        .iter()
        .map(|(_, name, desc, _, _)| format!("{name} — {desc}"))
        .collect();

    let default_idx = current
        .and_then(|c| PROVIDERS.iter().position(|(id, _, _, _, _)| *id == c.name))
        .unwrap_or(0);

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your summary LLM (liaison)")
        .items(&items)
        .default(default_idx)
        .interact()?;

    let (id, _, _, default_model, default_base_url) = PROVIDERS[selection];

    let reuse_existing = current
        .filter(|c| c.name == id && !c.api_key.is_empty())
        .is_some();

    let api_key = if reuse_existing {
        prompt_key_reuse(id, current.unwrap())?
    } else {
        prompt_new_key(id)?
    };

    Ok(LiaisonConfig {
        name: id.to_string(),
        api_key,
        model: default_model.to_string(),
        base_url: default_base_url.to_string(),
        // Defaults match `LiaisonConfig::default` / the serde defaults —
        // keep them in sync when new providers are added in PR6.
        max_output_tokens: 120,
        timeout_secs: 15,
        scrub_secrets: true,
    })
}

fn prompt_new_key(provider_id: &str) -> Result<String> {
    let key_url = match provider_id {
        "anthropic" => "https://console.anthropic.com/settings/keys",
        _ => "",
    };

    println!();
    if !key_url.is_empty() {
        println!("  {} {}", "Get one at:".dimmed(), key_url.dimmed());
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

fn prompt_key_reuse(provider_id: &str, current: &LiaisonConfig) -> Result<String> {
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
