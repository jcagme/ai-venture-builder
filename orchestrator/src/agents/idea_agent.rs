use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

use crate::models::idea::Idea;

pub async fn generate_idea(api_key: &str) -> Result<Idea> {
    let client = Client::new();

    let prompt = r#"
You are a startup advisor meant to help entrepreneurs come up with new SaaS startup ideas. Your task is to generate ONE unique and innovative SaaS idea that has the potential to succeed in the market based on gaps or market trends.

You need to keep context so you don't recommend similar ideas in the future. Each idea should be distinct and not overlap with previous ideas.

When generating the idea, consider the following aspects:
- title: A catchy and descriptive name for the startup.
- problem: A clear and concise statement of the problem that the startup aims to solve.
- solution: A brief description of the product or service that addresses the problem.
- target_users: The specific group of people or businesses that would benefit from the solution.
- monetization: How the startup plans to make money (e.g., subscription, freemium, one-time purchase).
- mvp_scope: A minimal set of features that would allow the startup to launch and validate the idea with early users.

Respond ONLY in valid JSON.
"#;

    let body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 800,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await?;

    let status = res.status();
    let json: serde_json::Value = res.json().await?;

    if !status.is_success() {
        return Err(anyhow!("Anthropic API error (status {status}): {json}"));
    }

    let content = json["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("Invalid Anthropic response format"))?;

    let cleaned = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let idea: Idea = serde_json::from_str(cleaned)?;

    Ok(idea)
}
