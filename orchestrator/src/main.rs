mod agents;
mod github;
mod models;

use agents::idea_agent::generate_idea;
use github::client::create_issue;

use std::env;

#[tokio::main]
async fn main() {
    let anthropic_key = env::var("ANTHROPIC_API_KEY").expect("Missing ANTHROPIC_API_KEY");
    let github_token = env::var("GITHUB_TOKEN").expect("Missing GITHUB_TOKEN");

    let owner = "jcagme";
    let repo = "ai-venture-builder";

    println!("🚀 Running Idea Agent...\n");

    let idea = match generate_idea(&anthropic_key).await {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to generate idea: {e}");
            std::process::exit(1);
        }
    };

    let pretty = serde_json::to_string_pretty(&idea).expect("Failed to serialize idea");
    println!("💡 Generated Idea: {}\n", idea.title);

    if let Err(e) = create_issue(
        &github_token,
        owner,
        repo,
        &format!("AI-generated Startup Idea: {}", idea.title),
        &pretty,
    )
    .await
    {
        eprintln!("Failed to create GitHub issue: {e}");
        std::process::exit(1);
    }

    println!("✅ GitHub issue created!");
}
