use crate::audio::player;
use crate::tts::TtsProvider;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Select};

pub async fn test_audio(provider: &dyn TtsProvider, voice: &str) -> Result<bool> {
    let test_phrase = "Hello! I'm Loquitor, and I'll be narrating your agent's thoughts.";

    println!();
    println!("  {}", format!("Speaking: \"{test_phrase}\"").dimmed());

    // Synthesize and play
    match provider.synthesize(test_phrase, &voice.to_string()).await {
        Ok(audio) => {
            // play_audio is blocking — run on blocking thread pool
            let audio_clone = audio.clone();
            let join_result =
                tokio::task::spawn_blocking(move || player::play_audio(&audio_clone)).await;
            match join_result {
                Ok(Ok(())) => println!("  {}", "✓ Audio played successfully".green()),
                Ok(Err(e)) => println!("  {} {}", "✗ Audio failed:".red(), e),
                Err(join_err) => println!("  {} {}", "✗ Audio task panicked:".red(), join_err),
            }
        }
        Err(e) => {
            println!("  {} {}", "✗ Audio failed:".red(), e);
        }
    }
    println!();

    let options = vec![
        "Yes, sounds good!",
        "No, I didn't hear anything",
        "Play it again",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Did you hear the audio?")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => Ok(true),
        1 => {
            println!();
            println!("  {}", "Troubleshooting:".yellow().bold());
            println!("  ┌─────────────────────────────────────────────────┐");
            println!("  │ 1. Check your system volume is turned up         │");
            println!("  │ 2. Make sure the correct output device is set    │");
            println!("  │ 3. Try: say \"test\" in another terminal           │");
            println!("  │    If that works, the issue is with the API key  │");
            println!("  └─────────────────────────────────────────────────┘");
            println!();
            Ok(false)
        }
        2 => Box::pin(test_audio(provider, voice)).await,
        _ => Ok(false),
    }
}
