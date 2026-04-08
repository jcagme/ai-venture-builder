use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

use crate::models::github_issue::GitHubIssue;

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

pub async fn read_issue(
    token: &str,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<GitHubIssue> {
    let client = Client::new();

    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}");

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .send()
        .await?;

    let status = res.status();

    if !status.is_success() {
        let text = res.text().await?;
        return Err(anyhow!(
            "GitHub API error while reading issue #{issue_number} (status {status}): {text}"
        ));
    }

    let json: serde_json::Value = res.json().await?;

    let title = json["title"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing issue title for issue #{issue_number}"))?
        .to_string();
    let description = json["body"].as_str().unwrap_or("").to_string();

    Ok(GitHubIssue { title, description })
}

#[allow(dead_code)]
pub async fn get_issue_comments(
    token: &str,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<Vec<String>> {
    let client = Client::new();

    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}/comments");

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .send()
        .await?;

    let json: serde_json::Value = res.json().await?;

    let mut comments = Vec::new();

    if let Some(arr) = json.as_array() {
        for comment in arr {
            if let Some(body) = comment["body"].as_str() {
                comments.push(body.to_string());
            }
        }
    }

    Ok(comments)
}

pub async fn create_branch(token: &str, owner: &str, repo: &str, branch: &str) -> Result<()> {
    let client = Client::new();
    let repo_url = format!("https://api.github.com/repos/{owner}/{repo}");
    let repo_res = client
        .get(&repo_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .send()
        .await?;

    let repo_json: serde_json::Value = repo_res.json().await?;
    let default_branch = repo_json["default_branch"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing default branch"))?;
    let ref_url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/ref/heads/{default_branch}");
    let ref_res = client
        .get(&ref_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .send()
        .await?;

    let ref_json: serde_json::Value = ref_res.json().await?;

    // Check if branch already exists
    let branch_check_url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/ref/heads/{branch}");

    let branch_check_res = client
        .get(&branch_check_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .send()
        .await?;

    if branch_check_res.status().is_success() {
        println!("Branch '{branch}' already exists");
        return Ok(());
    }

    // Step 2: Create new branch
    let create_url = format!("https://api.github.com/repos/{owner}/{repo}/git/refs");
    let sha = ref_json["object"]["sha"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing SHA"))?;
    let body = json!({
        "ref": format!("refs/heads/{branch}"),
        "sha": sha
    });

    let res = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let text = res.text().await?;
        return Err(anyhow!("Failed to create branch: {text}"));
    }

    Ok(())
}

pub async fn create_commit_on_branch(
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    commit_message: &str,
    file_path: &str,
    file_content: &str,
) -> Result<()> {
    let client = Client::new();

    // Get the commit SHA of the branch
    let ref_url = format!("https://api.github.com/repos/{owner}/{repo}/git/ref/heads/{branch}");
    let ref_res = client
        .get(&ref_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .send()
        .await?;

    let ref_json: serde_json::Value = ref_res.json().await?;
    let commit_sha = ref_json["object"]["sha"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing commit SHA for branch '{branch}'"))?;

    // Create a blob for the file content
    let blob_url = format!("https://api.github.com/repos/{owner}/{repo}/git/blobs");
    let blob_payload = json!({
        "content": file_content,
        "encoding": "utf-8"
    });

    let blob_res = client
        .post(&blob_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .json(&blob_payload)
        .send()
        .await?;

    let blob_json: serde_json::Value = blob_res.json().await?;
    let blob_sha = blob_json["sha"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing blob SHA"))?;

    // Get the tree of the current commit
    let commit_url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/commits/{commit_sha}");
    let commit_res = client
        .get(&commit_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .send()
        .await?;

    let commit_json: serde_json::Value = commit_res.json().await?;
    let tree_sha = commit_json["tree"]["sha"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing tree SHA"))?;

    // Create a new tree with the updated file
    let tree_url = format!("https://api.github.com/repos/{owner}/{repo}/git/trees");
    let tree_payload = json!({
        "base_tree": tree_sha,
        "tree": [
            {
                "path": file_path,
                "mode": "100644",
                "type": "blob",
                "sha": blob_sha
            }
        ]
    });

    let tree_res = client
        .post(&tree_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .json(&tree_payload)
        .send()
        .await?;

    let tree_json: serde_json::Value = tree_res.json().await?;
    let new_tree_sha = tree_json["sha"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing new tree SHA"))?;

    // Create a new commit
    let new_commit_url = format!("https://api.github.com/repos/{owner}/{repo}/git/commits");
    let new_commit_payload = json!({
        "message": commit_message,
        "tree": new_tree_sha,
        "parents": [commit_sha]
    });

    let new_commit_res = client
        .post(&new_commit_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .json(&new_commit_payload)
        .send()
        .await?;

    let new_commit_json: serde_json::Value = new_commit_res.json().await?;
    let new_commit_sha = new_commit_json["sha"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing new commit SHA"))?;

    // Update the branch ref to point to the new commit
    let update_ref_url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/refs/heads/{branch}");
    let update_payload = json!({
        "sha": new_commit_sha,
        "force": false
    });

    let update_res = client
        .patch(&update_ref_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "agent-manager")
        .json(&update_payload)
        .send()
        .await?;

    if !update_res.status().is_success() {
        let text = update_res.text().await?;
        return Err(anyhow!("Failed to update branch ref: {text}"));
    }

    Ok(())
}

pub async fn create_pr(
    token: &str,
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
    head: &str,
    base: &str,
) -> Result<()> {
    let client = Client::new();

    let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls",);

    let payload = json!({
        "title": title,
        "body": body,
        "head": head,
        "base": base
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
        return Err(anyhow!("Failed to create PR: {text}"));
    }

    Ok(())
}
