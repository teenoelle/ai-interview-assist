use anyhow::Result;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use url::Url;

pub async fn crawl_website(start_url: &str, max_pages: usize) -> Result<String> {
    let base_url = Url::parse(start_url)?;
    let base_host = base_url.host_str().unwrap_or("").to_string();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("Mozilla/5.0 (compatible; InterviewAssist/1.0)")
        .build()?;

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut collected_text = String::new();

    // Seed with the start URL first, then prioritise high-value paths
    queue.push_back(start_url.to_string());
    let priority_paths = [
        "/about", "/about-us", "/company", "/who-we-are",
        "/mission", "/values", "/culture",
        "/products", "/services", "/solutions", "/how-it-works",
        "/team", "/leadership", "/people", "/our-team",
        "/careers", "/jobs", "/work-with-us",
        "/blog", "/news", "/newsroom", "/press",
        "/case-studies", "/customers", "/clients",
        "/engineering", "/tech",
        "/investors", "/investor-relations",
        "/partners",
    ];
    for path in &priority_paths {
        if let Ok(u) = base_url.join(path) {
            let s = u.to_string();
            if !queue.contains(&s) {
                queue.push_front(s);
            }
        }
    }

    let skip_extensions = [".pdf", ".jpg", ".jpeg", ".png", ".gif", ".zip", ".css", ".js"];

    while let Some(url) = queue.pop_front() {
        if visited.len() >= max_pages {
            break;
        }
        if visited.contains(&url) {
            continue;
        }

        // Skip binary/asset files
        if skip_extensions.iter().any(|ext| url.to_lowercase().ends_with(ext)) {
            continue;
        }

        visited.insert(url.clone());
        tracing::debug!("Crawling: {}", url);

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to fetch {}: {}", url, e);
                continue;
            }
        };

        if !resp.status().is_success() {
            continue;
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if !content_type.contains("text/html") {
            continue;
        }

        let html = match resp.text().await {
            Ok(t) => t,
            Err(_) => continue,
        };

        let doc = Html::parse_document(&html);

        // Extract text
        let text_selector = Selector::parse("p, h1, h2, h3, h4, h5, li, article, section").unwrap();
        let page_text: Vec<String> = doc
            .select(&text_selector)
            .map(|e| e.text().collect::<String>().trim().to_string())
            .filter(|t| !t.is_empty() && t.len() > 20)
            .collect();

        if !page_text.is_empty() {
            collected_text.push_str(&format!("\n\n--- Page: {} ---\n", url));
            collected_text.push_str(&page_text.join("\n"));
        }

        // Find links
        let link_selector = Selector::parse("a[href]").unwrap();
        for element in doc.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                let linked = match Url::parse(href) {
                    Ok(u) => u,
                    Err(_) => match base_url.join(href) {
                        Ok(u) => u,
                        Err(_) => continue,
                    },
                };

                // Same-origin only
                if linked.host_str().unwrap_or("") != base_host {
                    continue;
                }

                let link_str = linked.as_str().to_string();
                if !visited.contains(&link_str) {
                    queue.push_back(link_str);
                }
            }
        }
    }

    // Truncate to reasonable size (100KB)
    if collected_text.len() > 100_000 {
        collected_text.truncate(100_000);
        collected_text.push_str("\n\n[Content truncated]");
    }

    Ok(collected_text)
}
