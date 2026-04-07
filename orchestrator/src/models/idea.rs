use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Idea {
    pub title: String,
    pub problem: String,
    pub solution: String,
    pub target_users: String,
    pub monetization: String,
    pub mvp_scope: String,
}
