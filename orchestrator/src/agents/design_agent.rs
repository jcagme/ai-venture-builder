use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

pub async fn generate_design_doc(api_key: &str, idea_markdown: &str) -> Result<String> {
    let client = Client::new();

    let prompt = format!(
        r#"
You are a senior software architect.

Given the following product idea:

{idea_markdown}

Generate a detailed design document in markdown for a minimum viable product (MVP).

The design MUST include:

# Overview

# Tech Stack
- Programming language
- Frameworks

# Database Design
- SQL vs NoSQL decision
- Schema or structure

# Architecture
- High-level components

# Deployment Plan
- Containerized vs not
- CI/CD

# Infrastructure
- Cloud provider

# Task Breakdown
- 5-8 engineering tasks for the MVP

Be specific and practical but frugal with tokens.
"#
    );

    let mut messages = vec![json!({
        "role": "user",
        "content": prompt
    })];

    let mut full_content = String::new();

    loop {
        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 2000,
            "messages": messages
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

        let content_blocks = json["content"]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid Anthropic response format: missing content array"))?;

        let mut chunk = String::new();
        for block in content_blocks {
            if block["type"].as_str() == Some("text") {
                if let Some(text) = block["text"].as_str() {
                    chunk.push_str(text);
                }
            }
        }

        if chunk.is_empty() {
            return Err(anyhow!(
                "Invalid Anthropic response format: no text content returned"
            ));
        }

        full_content.push_str(&chunk);
        println!("\nPartial Design Doc:\n{full_content}");
        if json["stop_reason"].as_str() != Some("max_tokens") {
            break;
        }

        // Append assistant reply and a continuation prompt to the conversation
        messages.push(json!({
            "role": "assistant",
            "content": chunk
        }));
        messages.push(json!({
            "role": "user",
            "content": "Continue exactly where you left off."
        }));
    }

    println!("Generated Design Doc:\n{full_content}");
    Ok(full_content)
}
