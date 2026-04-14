//! OpenAI Chat Completions implementation of `LiaisonProvider`, plus a
//! reusable `with_endpoint` constructor so any OpenAI-compatible
//! endpoint (MiniMax, xAI, Groq, Mistral, DeepSeek, Ollama, …) can
//! share the same request/response handling. The wire protocol is the
//! same across the whole ecosystem — only the base URL and model id
//! change.

use super::prompt::{parse_response, render_user_prompt, SYSTEM_PROMPT};
use super::{LiaisonProvider, TurnContext, TurnSummary};
use anyhow::{Context, Result};
use async_trait::async_trait;

const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    endpoint: String,
    /// Used by `name()` so a MiniMax-backed OpenAiProvider reports as
    /// "minimax" rather than masquerading as OpenAI.
    display_name: &'static str,
}

impl OpenAiProvider {
    /// Standard OpenAI setup — POSTs to /v1/chat/completions on api.openai.com.
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model: if model.is_empty() {
                DEFAULT_MODEL.to_string()
            } else {
                model.to_string()
            },
            endpoint: DEFAULT_ENDPOINT.to_string(),
            display_name: "openai",
        }
    }

    /// Reuse the OpenAI protocol against a different endpoint. Used by
    /// the MiniMax wrapper and (eventually) the generic openai_compat
    /// adapter that PR6 ships for xAI/Groq/Mistral/DeepSeek/Ollama.
    pub fn with_endpoint(
        api_key: &str,
        model: &str,
        endpoint: &str,
        display_name: &'static str,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            endpoint: endpoint.to_string(),
            display_name,
        }
    }
}

#[async_trait]
impl LiaisonProvider for OpenAiProvider {
    fn name(&self) -> &str {
        self.display_name
    }

    async fn summarize_turn(&self, ctx: &TurnContext<'_>) -> Result<TurnSummary> {
        let user_prompt = render_user_prompt(ctx);
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": ctx.max_output_tokens,
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": user_prompt }
            ],
        });

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("Failed to call chat-completions API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("{} liaison error ({status}): {error_body}", self.display_name);
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse chat-completions response")?;

        // The OpenAI response shape: choices[0].message.content.
        // MiniMax's v2 endpoint returns the same shape.
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(parse_response(&text))
    }
}
