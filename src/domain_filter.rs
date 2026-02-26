use regex::Regex;

pub fn is_valid(domain: &str) -> bool {
    if domain.contains("activitypub-troll") {
        return false
    }
    let domain_regex = Regex::new(r"(?i)^[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,63}$").unwrap();
    if !domain_regex.is_match(domain) {
        return false
    }
    true
}