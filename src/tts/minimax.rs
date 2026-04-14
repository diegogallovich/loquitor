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
/// MiniMax doesn't expose a voices API endpoint — this list is curated.
/// English-native voices are listed first because English is Loquitor's
/// dominant use case (narrating Claude Code sessions).
const MINIMAX_VOICES: &[(&str, &str, &str)] = &[
    // --- English-native voices (recommended for English agent output) ---
    (
        "English_Graceful_Lady",
        "Graceful Lady (English)",
        "English, warm and measured",
    ),
    (
        "English_Insightful_Speaker",
        "Insightful Speaker (English)",
        "English, confident male",
    ),
    (
        "English_radiant_girl",
        "Radiant Girl (English)",
        "English, bright female",
    ),
    (
        "English_Persuasive_Man",
        "Persuasive Man (English)",
        "English, authoritative male",
    ),
    (
        "English_Lucky_Robot",
        "Lucky Robot (English)",
        "English, quirky robotic",
    ),
    // --- Chinese-native voices (use only for Chinese text) ---
    ("male-qn-qingse", "Qingse (中文)", "Chinese, young male"),
    ("female-shaonv", "Shaonv (中文)", "Chinese, young female"),
    (
        "male-qn-jingying",
        "Jingying (中文)",
        "Chinese, male professional",
    ),
    ("female-yujie", "Yujie (中文)", "Chinese, female mature"),
    ("male-qn-badao", "Badao (中文)", "Chinese, male dominant"),
    ("female-chengshu", "Chengshu (中文)", "Chinese, female wise"),
    ("female-tianmei", "Tianmei (中文)", "Chinese, female sweet"),
    (
        "presenter_male",
        "Presenter Male (中文)",
        "Chinese, presenter",
    ),
    (
        "presenter_female",
        "Presenter Female (中文)",
        "Chinese, presenter",
    ),
    (
        "audiobook_male_1",
        "Audiobook Male (中文)",
        "Chinese, audiobook narrator",
    ),
    (
        "audiobook_female_1",
        "Audiobook Female (中文)",
        "Chinese, audiobook narrator",
    ),
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
        // `language_boost: "auto"` lets MiniMax detect the input language and
        // synthesize natively instead of phonetically approximating through
        // Chinese syllables. Critical for English text on any of their voices.
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
            "language_boost": "auto",
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

        let audio_bytes = hex::decode(hex_audio).context("Failed to hex-decode MiniMax audio")?;

        Ok(AudioData {
            bytes: Bytes::from(audio_bytes),
            format: AudioFormat::Mp3,
            sample_rate: 32000,
        })
    }
}
