mod agents;
mod github;
mod models;
mod utils;

use agents::{design_agent::generate_design_doc, idea_agent::generate_idea};
use github::client::{create_branch, create_commit_on_branch, create_issue, create_pr, read_issue};
use utils::shared::extract_idea_name;

use std::env;

#[tokio::main]
async fn main() {
    let anthropic_key = env::var("ANTHROPIC_API_KEY").expect("Missing ANTHROPIC_API_KEY");
    let github_token = env::var("GITHUB_TOKEN").expect("Missing GITHUB_TOKEN");

    let owner = "jcagme";
    let repo = "ai-venture-builder";

    run_design_agent(&anthropic_key, &github_token, owner, repo, 3).await;
}

#[allow(dead_code)]
async fn run_idea_agent(anthropic_key: &str, github_token: &str, owner: &str, repo: &str) {
    println!("🚀 Running Idea Agent...\n");

    let idea = match generate_idea(anthropic_key).await {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to generate idea: {e}");
            std::process::exit(1);
        }
    };

    let pretty = serde_json::to_string_pretty(&idea).expect("Failed to serialize idea");
    println!("💡 Generated Idea: {}\n", idea.title);

    if let Err(e) = create_issue(
        github_token,
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

#[allow(dead_code)]
async fn run_design_agent(
    anthropic_key: &str,
    github_token: &str,
    owner: &str,
    repo: &str,
    idea_id: u64,
) {
    println!("🚀 Starting design phase...");

    let idea = read_issue(github_token, owner, repo, idea_id)
        .await
        .expect("Failed to read issue");

    let idea_name = extract_idea_name(&idea.title);
    let branch = format!("feature-{idea_name}");

    create_branch(github_token, owner, repo, &branch)
        .await
        .expect("Failed to create branch");

    let design_doc = generate_design_doc(anthropic_key, &idea.description)
        .await
        .expect("Failed to generate design");

    create_commit_on_branch(
        github_token,
        owner,
        repo,
        &branch,
        &format!("Initial design doc for {idea_name}"),
        &format!("projects/{idea_name}/design_doc.md"),
        &design_doc,
    )
    .await
    .expect("Failed to create commit");

    create_pr(
        github_token,
        owner,
        repo,
        &format!("Design Doc: {idea_name}"),
        &format!("This PR includes the initial design document for the MVP of {idea_name}."),
        &branch,
        "main",
    )
    .await
    .expect("Failed to create PR");

    println!("✅ Design phase completed!");
}
