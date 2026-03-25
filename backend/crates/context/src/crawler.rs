use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use tokio::task::JoinSet;
use url::Url;

/// If `url` is a GitHub user profile (e.g. github.com/alice), return "alice". Repo links return None.
pub fn extract_github_username(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/');
    let stripped = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")).unwrap_or(url);
    let stripped = stripped.strip_prefix("www.").unwrap_or(stripped);
    let path = stripped.strip_prefix("github.com/")?;
    // Exactly one non-empty segment → user profile; two segments → org/repo → skip
    if path.is_empty() || path.contains('/') { return None; }
    Some(path.to_string())
}

/// Fetch a GitHub user's public repos and build a plain-text portfolio summary.
pub async fn fetch_github_portfolio(username: &str) -> String {
    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("InterviewAssist/1.0")
        .build()
    {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let url = format!("https://api.github.com/users/{}/repos?sort=updated&per_page=30", username);
    let resp = match client.get(&url).header("Accept", "application/vnd.github.v3+json").send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return String::new(),
    };
    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    let repos = match json.as_array() {
        Some(a) => a,
        None => return String::new(),
    };

    let mut languages: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut lines: Vec<String> = Vec::new();
    for repo in repos.iter().take(15) {
        let fork = repo["fork"].as_bool().unwrap_or(false);
        if fork { continue; }
        let name = repo["name"].as_str().unwrap_or("");
        let desc = repo["description"].as_str().unwrap_or("").trim();
        let lang = repo["language"].as_str().unwrap_or("");
        let stars = repo["stargazers_count"].as_u64().unwrap_or(0);
        if !lang.is_empty() { languages.insert(lang.to_string()); }
        let star_str = if stars > 0 { format!(" ★{}", stars) } else { String::new() };
        let lang_str = if !lang.is_empty() { format!(" ({})", lang) } else { String::new() };
        let desc_str = if !desc.is_empty() { format!(": {}", desc) } else { String::new() };
        lines.push(format!("- {}{}{}{}", name, lang_str, desc_str, star_str));
    }
    if lines.is_empty() { return String::new(); }

    let mut out = format!("GitHub Portfolio: github.com/{}\n", username);
    if !languages.is_empty() {
        let mut lang_list: Vec<String> = languages.into_iter().collect();
        lang_list.sort();
        out.push_str(&format!("Active languages: {}\n", lang_list.join(", ")));
    }
    out.push_str("Public repositories:\n");
    out.push_str(&lines.join("\n"));
    out
}

const CRAWL_CONCURRENCY: usize = 8;
const SKIP_EXTENSIONS: &[&str] = &[".pdf", ".jpg", ".jpeg", ".png", ".gif", ".zip", ".css", ".js", ".woff", ".woff2", ".svg", ".ico"];

async fn fetch_page(client: Client, url: String, base_host: String) -> (String, Vec<String>) {
    if SKIP_EXTENSIONS.iter().any(|ext| url.to_lowercase().ends_with(ext)) {
        return (String::new(), vec![]);
    }
    let resp = match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return (String::new(), vec![]),
    };
    let is_html = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/html"))
        .unwrap_or(false);
    if !is_html { return (String::new(), vec![]); }

    let html = match resp.text().await {
        Ok(t) => t,
        Err(_) => return (String::new(), vec![]),
    };
    let doc = Html::parse_document(&html);
    let base = match Url::parse(&url) {
        Ok(u) => u,
        Err(_) => return (String::new(), vec![]),
    };

    let text_sel = Selector::parse("p, h1, h2, h3, h4, h5, li, article, section").unwrap();
    let page_text: Vec<String> = doc.select(&text_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty() && t.len() > 20)
        .collect();

    let content = if page_text.is_empty() {
        String::new()
    } else {
        format!("\n\n--- Page: {} ---\n{}", url, page_text.join("\n"))
    };

    let link_sel = Selector::parse("a[href]").unwrap();
    let links: Vec<String> = doc.select(&link_sel)
        .filter_map(|el| {
            let href = el.value().attr("href")?;
            let linked = Url::parse(href).ok().or_else(|| base.join(href).ok())?;
            if linked.host_str().unwrap_or("") != base_host { return None; }
            Some(linked.to_string())
        })
        .collect();

    (content, links)
}

pub async fn crawl_website(start_url: &str, max_pages: usize) -> Result<String> {
    let base_url = Url::parse(start_url)?;
    let base_host = base_url.host_str().unwrap_or("").to_string();

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("Mozilla/5.0 (compatible; InterviewAssist/1.0)")
        .build()?;

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut collected_text = String::new();

    queue.push_back(start_url.to_string());
    // Prioritise high-value paths — trimmed to the most commonly useful ones
    let priority_paths = [
        "/about", "/about-us", "/company",
        "/mission", "/values", "/culture",
        "/products", "/services", "/solutions",
        "/team", "/leadership",
        "/careers", "/jobs",
        "/blog", "/news",
        "/case-studies", "/customers",
    ];
    for path in &priority_paths {
        if let Ok(u) = base_url.join(path) {
            let s = u.to_string();
            if !queue.contains(&s) { queue.push_front(s); }
        }
    }

    let mut in_flight: JoinSet<(String, Vec<String>)> = JoinSet::new();

    loop {
        // Dispatch up to CRAWL_CONCURRENCY concurrent fetches
        while in_flight.len() < CRAWL_CONCURRENCY && !queue.is_empty() && visited.len() < max_pages {
            let url = queue.pop_front().unwrap();
            if visited.contains(&url) { continue; }
            visited.insert(url.clone());
            tracing::debug!("Crawling: {}", url);
            let c = client.clone();
            let h = base_host.clone();
            in_flight.spawn(async move { fetch_page(c, url, h).await });
        }

        if in_flight.is_empty() { break; }

        if let Some(Ok((content, links))) = in_flight.join_next().await {
            if !content.is_empty() { collected_text.push_str(&content); }
            for link in links {
                if !visited.contains(&link) { queue.push_back(link); }
            }
        }
    }

    if collected_text.len() > 100_000 {
        let safe_end = collected_text.char_indices().map(|(i, _)| i)
            .take_while(|&i| i <= 100_000).last().unwrap_or(0);
        collected_text.truncate(safe_end);
        collected_text.push_str("\n\n[Content truncated]");
    }

    Ok(collected_text)
}
