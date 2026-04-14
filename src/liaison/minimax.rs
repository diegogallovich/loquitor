//! MiniMax liaison backend. Their chat API is OpenAI-compatible in
//! shape (same `messages`/`choices`/`message.content`), so we delegate
//! the actual HTTP call to `OpenAiProvider::with_endpoint` and only
//! carry the MiniMax-specific endpoint + default model.
//!
//! Diego already uses MiniMax for TTS, so letting users point the
//! liaison at the same account removes one API-key setup step.

use super::openai::OpenAiProvider;
use super::{LiaisonProvider, TurnContext, TurnSummary};
use anyhow::Result;
use async_trait::async_trait;

const ENDPOINT: &str = "https://api.minimax.io/v1/text/chatcompletion_v2";
const DEFAULT_MODEL: &str = "MiniMax-Text-01";

pub struct MiniMaxProvider {
    inner: OpenAiProvider,
}

impl MiniMaxProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        let model = if model.is_empty() {
            DEFAULT_MODEL
        } else {
            model
        };
        Self {
            inner: OpenAiProvider::with_endpoint(api_key, model, ENDPOINT, "minimax"),
        }
    }
}

#[async_trait]
impl LiaisonProvider for MiniMaxProvider {
    fn name(&self) -> &str {
        "minimax"
    }

    async fn summarize_turn(&self, ctx: &TurnContext<'_>) -> Result<TurnSummary> {
        self.inner.summarize_turn(ctx).await
    }
}
