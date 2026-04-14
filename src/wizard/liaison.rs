//! Liaison (summarizer LLM) sub-wizard. Pick a provider, pick (or
//! keep) the API key, pick (or type) a model. Symmetric with
//! `wizard/provider.rs` for TTS.

use crate::config::types::{LiaisonConfig, TtsConfig};
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
        blurb: "GPT-5.4 family: pro → nano, plus fallback mini-class models.",
        key_url: "https://platform.openai.com/api-keys",
        // Current as of April 2026. The pro tier is overkill for a
        // one-sentence summary but included for users who want it.
        // gpt-5.4-mini is the recommended balance for this workload.
        models: &[
            "gpt-5.4-pro",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.4-nano",
            "gpt-4o-mini",
        ],
    },
    ProviderDef {
        id: "minimax",
        display: "MiniMax",
        blurb: "M2 series. Reuse your MiniMax TTS key; chat API is OpenAI-shaped.",
        key_url: "https://www.minimax.io/platform",
        // M2.7 is current flagship (April 2026); M2.5 and M2 are still
        // available as cheaper fallbacks.
        models: &[
            "MiniMax-M2.7",
            "MiniMax-M2.5",
            "MiniMax-M2",
        ],
    },
];

const CUSTOM_MODEL_LABEL: &str = "Custom model id…";

pub fn select_liaison(
    current: Option<&LiaisonConfig>,
    tts: Option<&TtsConfig>,
) -> Result<LiaisonConfig> {
    let provider_idx = select_provider_index(current)?;
    let provider = &PROVIDERS[provider_idx];

    let api_key = resolve_api_key(provider, current, tts)?;
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

/// Resolve the API key for the chosen liaison provider. Offers up to
/// three sources in a Select menu, ordered by relevance:
///
///   1. Keep the key already in the liaison config (if it matches the
///      chosen provider and is non-empty).
///   2. Reuse the TTS key, if the TTS provider id matches the liaison
///      provider id and has a non-empty key. Same account, same key —
///      OpenAI TTS and OpenAI Chat are one account in one dashboard.
///   3. Enter a fresh key.
///
/// If only option (3) is available, the prompt goes straight there —
/// no menu needed.
fn resolve_api_key(
    provider: &ProviderDef,
    current: Option<&LiaisonConfig>,
    tts: Option<&TtsConfig>,
) -> Result<String> {
    let mut options: Vec<(String, Option<String>)> = Vec::new();

    if let Some(existing) = current.filter(|c| c.name == provider.id && !c.api_key.is_empty()) {
        options.push((
            format!("Keep the existing {} key", provider.id),
            Some(existing.api_key.clone()),
        ));
    }
    if let Some(tts_cfg) = tts.filter(|t| t.name == provider.id && !t.api_key.is_empty()) {
        // Avoid duplicating the option if the TTS key and the current
        // liaison key happen to be identical (common when configure_liaison
        // is re-run after configure_tts set the same key).
        let already_listed = options.iter().any(|(_, v)| v.as_deref() == Some(&tts_cfg.api_key));
        if !already_listed {
            options.push((
                format!("Reuse your {} TTS key (same account)", provider.id),
                Some(tts_cfg.api_key.clone()),
            ));
        }
    }
    options.push(("Enter a new key".to_string(), None));

    if options.len() == 1 {
        // Only "enter new" — skip the menu.
        return prompt_new_key(provider);
    }

    println!();
    let labels: Vec<String> = options.iter().map(|(l, _)| l.clone()).collect();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("API key for {}", provider.id))
        .items(&labels)
        .default(0)
        .interact()?;

    match &options[choice].1 {
        Some(key) => Ok(key.clone()),
        None => prompt_new_key(provider),
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

