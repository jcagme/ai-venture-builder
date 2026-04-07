use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

pub async fn create_issue(
    token: &str,
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
) -> Result<()> {
    let client = Client::new();

    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues");

    let payload = json!({
        "title": title,
        "body": body
    });

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .json(&payload)
        .send()
        .await?;

    let status = res.status();
    let text = res.text().await?;

    if !status.is_success() {
        return Err(anyhow!("GitHub API error (status {status}): {text}",));
    }

    println!("GitHub response: {text}");

    Ok(())
}
