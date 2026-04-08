pub fn extract_idea_name(title: &str) -> String {
    title
        .split(':')
        .nth(1)
        .unwrap_or(title)
        .trim()
        .replace(' ', "-")
        .to_lowercase()
}
