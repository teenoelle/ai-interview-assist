use anyhow::Result;
use lopdf::Document;

pub fn extract_pdf_text(bytes: &[u8]) -> Result<String> {
    let doc = Document::load_mem(bytes)?;
    let mut text = String::new();

    let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    for page_num in pages {
        match doc.extract_text(&[page_num]) {
            Ok(page_text) => {
                text.push_str(&page_text);
                text.push('\n');
            }
            Err(e) => {
                tracing::warn!("Failed to extract text from page {}: {}", page_num, e);
            }
        }
    }

    // Normalize whitespace
    let normalized: String = text
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(normalized)
}
