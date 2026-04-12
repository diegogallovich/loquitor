use super::{AudioData, AudioFormat, TtsProvider, Voice, VoiceId};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use std::process::Command;

// NOTE: std::process::Command is blocking. Since the `say` command is fast
// (<100ms for short phrases) and we use a current_thread runtime, this is
// acceptable for v0.1.0. Swap to tokio::process::Command if it becomes a
// problem.
pub struct MacOsSayProvider;

#[async_trait]
impl TtsProvider for MacOsSayProvider {
    fn name(&self) -> &str {
        "macOS Say"
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        let output = Command::new("say")
            .arg("--voice=?")
            .output()
            .context("Failed to run 'say --voice=?'")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let voices: Vec<Voice> = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, '#').collect();
                if parts.len() < 2 {
                    return None;
                }
                let name_lang = parts[0].trim();
                let description = parts[1].trim().to_string();
                let name = name_lang.split_whitespace().next()?.to_string();
                Some(Voice {
                    id: name.clone(),
                    name,
                    description,
                })
            })
            .collect();

        Ok(voices)
    }

    async fn synthesize(&self, text: &str, voice: &VoiceId) -> Result<AudioData> {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!(
            "loquitor-{}-{}.aiff",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let status = Command::new("say")
            .args(["-v", voice, "-o", &temp_file.to_string_lossy(), text])
            .status()
            .context("Failed to run 'say'")?;

        if !status.success() {
            anyhow::bail!("'say' command failed with status: {status}");
        }

        let bytes = std::fs::read(&temp_file)
            .with_context(|| format!("Failed to read output file: {}", temp_file.display()))?;

        let _ = std::fs::remove_file(&temp_file);

        Ok(AudioData {
            bytes: Bytes::from(bytes),
            format: AudioFormat::Aiff,
            sample_rate: 22050,
        })
    }
}
