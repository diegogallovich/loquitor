use super::{AudioData, AudioFormat, TtsProvider, Voice, VoiceId};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;

pub struct MiniMaxProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl MiniMaxProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model: if model.is_empty() {
                "speech-02-turbo".to_string()
            } else {
                model.to_string()
            },
        }
    }
}

/// MiniMax voices per their public documentation.
/// MiniMax doesn't expose a voices API endpoint — this list is static.
const MINIMAX_VOICES: &[(&str, &str, &str)] = &[
    ("male-qn-qingse", "Qingse", "young male, clear"),
    ("female-shaonv", "Shaonv", "young female, bright"),
    ("male-qn-jingying", "Jingying", "male, professional"),
    ("female-yujie", "Yujie", "female, mature"),
    ("male-qn-badao", "Badao", "male, dominant"),
    ("female-chengshu", "Chengshu", "female, wise"),
    ("female-tianmei", "Tianmei", "female, sweet"),
    ("presenter_male", "Presenter Male", "presenter, male"),
    ("presenter_female", "Presenter Female", "presenter, female"),
    ("audiobook_male_1", "Audiobook Male 1", "audiobook narrator, male"),
    ("audiobook_female_1", "Audiobook Female 1", "audiobook narrator, female"),
];

#[async_trait]
impl TtsProvider for MiniMaxProvider {
    fn name(&self) -> &str {
        "MiniMax"
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        Ok(MINIMAX_VOICES
            .iter()
            .map(|(id, name, desc)| Voice {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
            })
            .collect())
    }

    async fn synthesize(&self, text: &str, voice: &VoiceId) -> Result<AudioData> {
        let body = serde_json::json!({
            "model": self.model,
            "text": text,
            "voice_setting": {
                "voice_id": voice,
            },
            "audio_setting": {
                "format": "mp3",
                "sample_rate": 32000,
            },
        });

        let response = self
            .client
            .post("https://api.minimax.io/v1/t2a_v2")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("Failed to call MiniMax TTS API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("MiniMax TTS error ({status}): {error_body}");
        }

        // MiniMax returns JSON with hex-encoded audio in data.audio
        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse MiniMax response")?;

        let hex_audio = json["data"]["audio"]
            .as_str()
            .context("MiniMax response missing data.audio field")?;

        let audio_bytes = hex::decode(hex_audio)
            .context("Failed to hex-decode MiniMax audio")?;

        Ok(AudioData {
            bytes: Bytes::from(audio_bytes),
            format: AudioFormat::Mp3,
            sample_rate: 32000,
        })
    }
}
