use crate::config::types::VoiceMode;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Select};

/// Prompt the user for a lane voice policy. The currently-selected mode is
/// pre-selected, and a one-line explanation of each option is printed below
/// the list so the choice is self-documenting.
pub fn select_lane_policy(current: VoiceMode) -> Result<VoiceMode> {
    let options = [VoiceMode::Shared, VoiceMode::PerLane];
    let labels = [
        "Shared — every lane uses the default voice",
        "Per-lane — each lane can have its own voice (recommended for multi-session users)",
    ];

    let default_idx = options.iter().position(|m| *m == current).unwrap_or(1); // PerLane is the documented default

    println!();
    println!(
        "{}",
        "  Shared   = one voice for all concurrent Claude sessions".dimmed()
    );
    println!(
        "{}",
        "  Per-lane = lanes.rules[<lane_id>].voice wins; voice.default is the fallback".dimmed()
    );
    println!();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Lane voice policy")
        .items(&labels)
        .default(default_idx)
        .interact()?;

    Ok(options[selection])
}
