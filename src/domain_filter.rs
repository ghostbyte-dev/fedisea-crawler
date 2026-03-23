pub fn is_valid(domain: &str) -> bool {

    //TODO: improve is valid function
    if domain.contains("activitypub-troll") {
        return false;
    }

    if domain.len() > 253 {
        return false;
    }

    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    parts.iter().all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}
