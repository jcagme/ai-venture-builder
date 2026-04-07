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

    let idea = generate_idea(&anthropic_key)
        .await
        .expect("Failed to generate idea");

    println!("💡 Generated Idea:\n{idea}\n");

    create_issue(
        &github_token,
        owner,
        repo,
        "AI-generated Startup Idea",
        &idea,
    )
    .await
    .expect("Failed to create GitHub issue");

    println!("✅ GitHub issue created!");
}
