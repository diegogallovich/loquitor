use super::{AudioData, AudioFormat, TtsProvider, Voice, VoiceId};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model: if model.is_empty() {
                "tts-1".to_string()
            } else {
                model.to_string()
            },
        }
    }
}

const OPENAI_VOICES: &[(&str, &str)] = &[
    ("alloy", "neutral, balanced"),
    ("ash", "warm, clear"),
    ("coral", "warm, conversational"),
    ("echo", "clear, steady"),
    ("fable", "expressive, narrative"),
    ("nova", "energetic, bright"),
    ("onyx", "deep, authoritative"),
    ("sage", "calm, thoughtful"),
    ("shimmer", "light, expressive"),
];

#[async_trait]
impl TtsProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "OpenAI TTS"
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        Ok(OPENAI_VOICES
            .iter()
            .map(|(id, desc)| Voice {
                id: id.to_string(),
                name: id.to_string(),
                description: desc.to_string(),
            })
            .collect())
    }

    async fn synthesize(&self, text: &str, voice: &VoiceId) -> Result<AudioData> {
        let body = serde_json::json!({
            "model": self.model,
            "voice": voice,
            "input": text,
            "response_format": "mp3",
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("Failed to call OpenAI TTS API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI TTS API error ({status}): {error_body}");
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read OpenAI TTS response")?;

        Ok(AudioData {
            bytes,
            format: AudioFormat::Mp3,
            sample_rate: 24000,
        })
    }

    async fn synthesize_stream(
        &self,
        text: &str,
        voice: &VoiceId,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>> {
        let body = serde_json::json!({
            "model": self.model,
            "voice": voice,
            "input": text,
            "response_format": "mp3",
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("Failed to call OpenAI TTS API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI TTS API error ({status}): {error_body}");
        }

        use futures::StreamExt;
        let stream = response
            .bytes_stream()
            .map(|chunk| chunk.map_err(|e| anyhow::anyhow!("Stream error: {e}")));

        Ok(Box::pin(stream))
    }
}
