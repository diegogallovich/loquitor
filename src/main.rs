use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "loquitor", version, about = "Let your agents think out loud")]
enum Cli {
    /// Run the first-time setup wizard
    Init,
    /// Change one slice of your setup (provider, voice, lane policy) without
    /// overwriting the rest of the config
    Configure {
        #[command(subcommand)]
        target: ConfigureTarget,
    },
    /// Install shell hook and start the background daemon
    Enable,
    /// Remove shell hook and stop the daemon
    Disable,
    /// Show daemon status
    Status,
    /// List active lanes
    Lanes,
    /// Modify a lane's name or voice
    Lane {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        voice: Option<String>,
    },
    /// List available voices from the configured TTS provider
    Voices,
    /// Speak a test phrase
    Test { text: String },
}

#[derive(Subcommand)]
enum ConfigureTarget {
    /// Switch TTS provider, update API key, pick a new voice, and test
    Tts,
    /// Switch the liaison LLM (the layer that summarises each Claude turn
    /// before it's spoken). Prompts for provider, API key, and model.
    Liaison,
    /// Pick a different voice from the current TTS provider
    Voice,
    /// Choose whether every lane shares one voice or each lane gets its own
    LanePolicy,
    /// Walk through TTS + liaison + voice + lane-policy in sequence
    All,
    /// Deprecated alias for `tts` — kept for one release to catch muscle
    /// memory from v0.1.x. Prints a notice and delegates to `tts`.
    #[command(hide = true)]
    Provider,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("loquitor=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli {
        Cli::Init => cmd_init().await?,
        Cli::Configure { target } => cmd_configure(target).await?,
        Cli::Enable => cmd_enable().await?,
        Cli::Disable => cmd_disable()?,
        Cli::Status => cmd_status()?,
        Cli::Lanes => cmd_lanes()?,
        Cli::Lane { id, name, voice } => cmd_lane(id, name, voice)?,
        Cli::Voices => cmd_voices().await?,
        Cli::Test { text } => cmd_test(text).await?,
    }

    Ok(())
}

async fn cmd_init() -> Result<()> {
    loquitor::wizard::run_wizard().await
}

async fn cmd_configure(target: ConfigureTarget) -> Result<()> {
    match target {
        ConfigureTarget::Tts => loquitor::wizard::configure_tts().await,
        ConfigureTarget::Liaison => loquitor::wizard::configure_liaison().await,
        ConfigureTarget::Voice => loquitor::wizard::configure_voice().await,
        ConfigureTarget::LanePolicy => loquitor::wizard::configure_lane_policy().await,
        ConfigureTarget::All => loquitor::wizard::configure_all().await,
        ConfigureTarget::Provider => {
            println!(
                "{}",
                "note: `configure provider` is deprecated — use `configure tts`."
                    .yellow()
            );
            loquitor::wizard::configure_tts().await
        }
    }
}

async fn cmd_enable() -> Result<()> {
    let cfg = loquitor::config::load()?;
    let lanes_dir = loquitor::config::lanes_dir();
    let pid_path = PathBuf::from(&cfg.daemon.pid_file);

    if loquitor::daemon::is_daemon_running(&pid_path) {
        println!("{}", "Daemon is already running.".yellow());
        return Ok(());
    }

    // Install shell hook
    loquitor::shell::install(&lanes_dir.to_string_lossy())?;
    println!("{}", "✓ Shell hook installed.".green());

    // Write PID file
    loquitor::daemon::write_pid_file(&pid_path)?;
    println!("{}", "✓ Daemon started.".green());
    println!("  Loquitor is now listening. Open a new terminal");
    println!("  tab and run {} — I'll start talking.", "claude".cyan());

    // Show tip on first enable
    if !cfg.ui.tip_shown {
        println!();
        println!(
            "{}",
            "  ─────────────────────────────────────────────".dimmed()
        );
        println!("{}", "  Loquitor is free and open source.".dimmed());
        println!("{}", "  Tip the creator: loquitor.reachdiego.com".dimmed());
        let mut cfg_update = cfg.clone();
        cfg_update.ui.tip_shown = true;
        loquitor::config::save(&cfg_update)?;
    }

    // Run the pipeline (this blocks for the lifetime of the daemon)
    loquitor::daemon::pipeline::run(cfg, lanes_dir).await?;

    Ok(())
}

fn cmd_disable() -> Result<()> {
    let cfg = loquitor::config::load()?;
    let pid_path = PathBuf::from(&cfg.daemon.pid_file);

    loquitor::shell::remove()?;
    println!("{}", "✓ Shell hook removed.".green());

    loquitor::daemon::stop_daemon(&pid_path)?;
    println!("{}", "✓ Daemon stopped.".green());
    println!("  Restart your shell or run: {}", "source ~/.zshrc".cyan());

    Ok(())
}

fn cmd_status() -> Result<()> {
    let cfg = loquitor::config::load()?;
    let pid_path = PathBuf::from(&cfg.daemon.pid_file);
    let running = loquitor::daemon::is_daemon_running(&pid_path);
    let hook = loquitor::shell::is_installed();

    println!(
        "  Daemon:   {}",
        if running {
            "running".green()
        } else {
            "stopped".red()
        }
    );
    println!(
        "  Hook:     {}",
        if hook {
            "installed".green()
        } else {
            "not installed".red()
        }
    );
    println!("  Provider: {}", cfg.tts.name.cyan());
    Ok(())
}

fn cmd_lanes() -> Result<()> {
    // TODO: query the daemon via IPC for live lane data.
    // For v0.1.0 we print the column header and a placeholder message.
    println!("  {:<12}{:<9}{:<8}LAST SPOKEN", "LANE", "VOICE", "AGE");
    println!(
        "{}",
        "  (lane enumeration via IPC is planned for a future release)".dimmed()
    );
    Ok(())
}

fn cmd_lane(id: String, name: Option<String>, voice: Option<String>) -> Result<()> {
    // TODO: send an IPC message to the daemon to update the lane rule
    // and persist it to the config.
    println!("  Lane configuration updated (requires running daemon).");
    if let Some(n) = name {
        println!("  Name: {}", n.green());
    }
    if let Some(v) = voice {
        println!("  Voice: {}", v.green());
    }
    let _ = id; // currently unused — future IPC call will send this as lane selector
    Ok(())
}

async fn cmd_voices() -> Result<()> {
    let cfg = loquitor::config::load()?;
    let provider = loquitor::tts::create_provider(
        &cfg.tts.name,
        &cfg.tts.api_key,
        &cfg.tts.model,
    )?;
    let voices = provider.list_voices().await?;
    if voices.is_empty() {
        println!("{}", "  (provider returned no voices)".dimmed());
    } else {
        for v in voices {
            println!("  {} — {}", v.name.cyan(), v.description.dimmed());
        }
    }
    Ok(())
}

async fn cmd_test(text: String) -> Result<()> {
    let cfg = loquitor::config::load()?;
    let provider = loquitor::tts::create_provider(
        &cfg.tts.name,
        &cfg.tts.api_key,
        &cfg.tts.model,
    )?;

    println!("  Speaking: \"{text}\"");
    let audio = provider.synthesize(&text, &cfg.voice.default).await?;

    // Move playback to blocking thread pool so we don't stall the runtime
    let audio_for_player = audio.clone();
    tokio::task::spawn_blocking(move || loquitor::audio::player::play_audio(&audio_for_player))
        .await
        .map_err(|e| anyhow::anyhow!("Playback task panicked: {e}"))??;

    println!("{}", "  ✓ Done".green());
    Ok(())
}
