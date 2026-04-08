use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub title: String,
    pub description: String,
}
