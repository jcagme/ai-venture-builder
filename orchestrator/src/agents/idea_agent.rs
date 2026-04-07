use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{Value, json};

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
    let text = res.text().await?;

    if !status.is_success() {
        return Err(anyhow!(format!(
            "Anthropic API error (status {status}): {text}"
        )));
    }

    // Parse the Anthropic response body JSON and extract the assistant text
    let v: Value = serde_json::from_str(&text).map_err(|e| {
        anyhow!(format!(
            "Failed to parse Anthropic response JSON: {e}: {text}"
        ))
    })?;

    // Expect content -> array -> first element -> text
    let assistant_text = v
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("Anthropic response missing assistant text field"))?;

    // The assistant `text` may contain a prefix like "json\n" or other noise before the
    // actual JSON. We'll extract a clean JSON candidate below.

    // Helper: if the assistant wrapped the JSON in fenced blocks like ```json ... ```
    // or in extra text, try to strip fences and take the substring between the
    // first opening brace and the last closing brace for a cleaner parse.
    fn extract_json(s: &str) -> Option<String> {
        // If fenced code block exists, prefer content inside the first fenced block
        if let Some(start_fence) = s.find("```") {
            if let Some(end_fence) = s.rfind("```") {
                if end_fence > start_fence {
                    let inner = &s[start_fence + 3..end_fence];
                    // Trim optional leading 'json' language tag
                    let inner = inner.trim_start();
                    let inner = inner.strip_prefix("json").unwrap_or(inner);
                    let inner = inner.trim_start();
                    return Some(inner.to_string());
                }
            }
        }

        // Otherwise, take from first '{' or '[' to last '}' or ']' respectively
        let start = s.find('{').or_else(|| s.find('['))?;
        // Pick matching closing brace by searching from the end
        if s[start..].starts_with('{') {
            if let Some(end) = s.rfind('}') {
                return Some(s[start..=end].to_string());
            }
        } else if let Some(end) = s.rfind(']') {
            return Some(s[start..=end].to_string());
        }

        // Fallback: return the substring starting at the first brace
        Some(s[start..].to_string())
    }

    let json_candidate = extract_json(assistant_text)
        .ok_or_else(|| anyhow!("Could not extract JSON substring from assistant text"))?;

    let idea: Idea = serde_json::from_str(&json_candidate).map_err(|e| {
        anyhow!(format!(
            "Failed to parse assistant JSON into Idea: {e}: {json_candidate}"
        ))
    })?;

    Ok(idea)
}
