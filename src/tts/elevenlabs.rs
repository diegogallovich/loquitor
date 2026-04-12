use super::{AudioData, AudioFormat, TtsProvider, Voice, VoiceId};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

pub struct ElevenLabsProvider {
    client: reqwest::Client,
    api_key: String,
}

impl ElevenLabsProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl TtsProvider for ElevenLabsProvider {
    fn name(&self) -> &str {
        "ElevenLabs"
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        let response = self
            .client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to call ElevenLabs voices API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs voices API error ({status}): {error_body}");
        }

        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse ElevenLabs voices response")?;

        let voices = body["voices"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|v| Voice {
                        id: v["voice_id"].as_str().unwrap_or("").to_string(),
                        name: v["name"].as_str().unwrap_or("").to_string(),
                        description: v["labels"]["description"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                    })
                    .filter(|voice| !voice.id.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Ok(voices)
    }

    async fn synthesize(&self, text: &str, voice: &VoiceId) -> Result<AudioData> {
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice}");
        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_flash_v2_5",
        });

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .query(&[("output_format", "mp3_44100_128")])
            .json(&body)
            .send()
            .await
            .context("Failed to call ElevenLabs TTS API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs TTS error ({status}): {error_body}");
        }

        let bytes = response.bytes().await.context("Failed to read ElevenLabs TTS response")?;

        Ok(AudioData {
            bytes,
            format: AudioFormat::Mp3,
            sample_rate: 44100,
        })
    }

    async fn synthesize_stream(
        &self,
        text: &str,
        voice: &VoiceId,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>> {
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice}/stream");
        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_flash_v2_5",
        });

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .query(&[("output_format", "mp3_44100_128")])
            .json(&body)
            .send()
            .await
            .context("Failed to call ElevenLabs streaming TTS API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs stream error ({status}): {error_body}");
        }

        use futures::StreamExt;
        let stream = response
            .bytes_stream()
            .map(|chunk| chunk.map_err(|e| anyhow::anyhow!("Stream error: {e}")));

        Ok(Box::pin(stream))
    }
}
