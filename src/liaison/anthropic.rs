//! Anthropic Messages API implementation of `LiaisonProvider`.
//! Matches the TTS-provider pattern: `reqwest::Client`, bearer-ish auth
//! header, explicit error bodies on non-2xx, one call per summarize.

use super::prompt::{parse_response, render_user_prompt, SYSTEM_PROMPT};
use super::{LiaisonProvider, TurnContext, TurnSummary};
use anyhow::{Context, Result};
use async_trait::async_trait;

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-haiku-4-5";

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model: if model.is_empty() {
                DEFAULT_MODEL.to_string()
            } else {
                model.to_string()
            },
        }
    }
}

#[async_trait]
impl LiaisonProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn summarize_turn(&self, ctx: &TurnContext<'_>) -> Result<TurnSummary> {
        let user_prompt = render_user_prompt(ctx);
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": ctx.max_output_tokens,
            "system": SYSTEM_PROMPT,
            "messages": [{ "role": "user", "content": user_prompt }],
        });

        let response = self
            .client
            .post(ENDPOINT)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await
            .context("Failed to call Anthropic Messages API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic liaison error ({status}): {error_body}");
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        // Anthropic's Messages API returns `content: [{ type: "text", text: "…" }]`.
        // We expect exactly one text block for our short-summary prompt; if the
        // model decides to emit multiple blocks, concatenate them so
        // `parse_response` can still find the JSON object inside.
        let text = json["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        let _ = ctx; // lane context is no longer used here — liaison_worker prepends the announcement
        Ok(parse_response(&text))
    }
}
