use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

pub async fn generate_idea(api_key: &str) -> Result<String> {
    let client = Client::new();

    let prompt = r#"
You are a startup advisor.

Generate ONE SaaS idea with:
- title
- problem
- solution
- target_users
- monetization
- mvp_scope

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
    let text = res.text().await?;

    if !status.is_success() {
        return Err(anyhow!(format!(
            "Anthropic API error (status {status}): {text}"
        )));
    }

    Ok(text)
}
