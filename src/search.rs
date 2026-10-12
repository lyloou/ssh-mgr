use crate::config::model::Server;

pub fn filter<'a>(servers: &'a [Server], query: &str) -> Vec<&'a Server> {
    let query = query.to_lowercase();
    let terms: Vec<_> = query.split_whitespace().collect();
    if terms.is_empty() {
        return servers.iter().collect();
    }
    servers
        .iter()
        .filter(|server| {
            let haystack = server.searchable_text().to_lowercase();
            terms.iter().all(|term| subsequence(term, &haystack))
        })
        .collect()
}

fn subsequence(needle: &str, haystack: &str) -> bool {
    let mut chars = haystack.chars();
    needle
        .chars()
        .all(|wanted| chars.any(|actual| actual == wanted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::Server;
    use std::path::PathBuf;

    #[test]
    fn matches_fields_by_letter() {
        let server = Server {
            name: "prod-api".into(),
            host: "10.0.0.1".into(),
            port: 22,
            user: "ubuntu".into(),
            description: "primary".into(),
            tags: vec![],
            password: None,
            group: "production".into(),
            source: PathBuf::new(),
        };
        assert_eq!(filter(&[server], "prd api").len(), 1);
    }
}
