use clap::Parser;

#[derive(Parser)]
#[command(name = "loquitor", version, about = "Let your agents think out loud")]
enum Cli {
    /// Run the first-time setup wizard
    Init,
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
    Test {
        text: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Init => println!("TODO: wizard"),
        Cli::Enable => println!("TODO: enable"),
        Cli::Disable => println!("TODO: disable"),
        Cli::Status => println!("TODO: status"),
        Cli::Lanes => println!("TODO: lanes"),
        Cli::Lane { id, name, voice } => {
            println!("TODO: lane {id} name={name:?} voice={voice:?}")
        }
        Cli::Voices => println!("TODO: voices"),
        Cli::Test { text } => println!("TODO: test {text}"),
    }
}
