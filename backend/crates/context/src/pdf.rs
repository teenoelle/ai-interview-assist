use anyhow::Result;
use lopdf::Document;
use std::io::Read;

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

/// Extract plain text from a .docx file (Word Open XML / ZIP container).
pub fn extract_docx_text(bytes: &[u8]) -> Result<String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    let mut xml = String::new();
    archive.by_name("word/document.xml")?.read_to_string(&mut xml)?;

    let mut result = String::new();
    let mut search_from = 0;

    while search_from < xml.len() {
        let Some(rel_tag_start) = xml[search_from..].find('<') else { break };
        let tag_start = search_from + rel_tag_start;
        let Some(rel_tag_end) = xml[tag_start..].find('>') else { break };
        let tag_end = tag_start + rel_tag_end;
        let tag = &xml[tag_start + 1..tag_end];

        if tag == "w:p" || tag.starts_with("w:p ") {
            if !result.is_empty() {
                result.push('\n');
            }
        } else if tag == "w:t" || tag.starts_with("w:t ") {
            let content_start = tag_end + 1;
            if let Some(close) = xml[content_start..].find("</w:t>") {
                result.push_str(&xml[content_start..content_start + close]);
                search_from = content_start + close + 6;
                continue;
            }
        }
        search_from = tag_end + 1;
    }

    Ok(result.trim().to_string())
}
