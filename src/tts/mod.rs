pub mod macos;
pub mod openai;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

pub type VoiceId = String;

#[derive(Debug, Clone)]
pub struct Voice {
    pub id: VoiceId,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Pcm,
    Flac,
    Aiff,
}

#[derive(Debug, Clone)]
pub struct AudioData {
    pub bytes: Bytes,
    pub format: AudioFormat,
    pub sample_rate: u32,
}

#[async_trait]
pub trait TtsProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn list_voices(&self) -> Result<Vec<Voice>>;
    async fn synthesize(&self, text: &str, voice: &VoiceId) -> Result<AudioData>;

    async fn synthesize_stream(
        &self,
        text: &str,
        voice: &VoiceId,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>> {
        let data = self.synthesize(text, voice).await?;
        Ok(Box::pin(futures::stream::once(async { Ok(data.bytes) })))
    }
}

pub fn create_provider(
    provider_name: &str,
    api_key: &str,
    model: &str,
) -> Result<Box<dyn TtsProvider>> {
    match provider_name {
        "openai" => Ok(Box::new(openai::OpenAiProvider::new(api_key, model))),
        "macos_say" => Ok(Box::new(macos::MacOsSayProvider)),
        other => anyhow::bail!("Unknown TTS provider: {other}"),
    }
}
