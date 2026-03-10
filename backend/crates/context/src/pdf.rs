use anyhow::Result;
use base64::Engine;
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

/// Extract text from a .pptx file (PowerPoint Open XML / ZIP container).
pub fn extract_pptx_text(bytes: &[u8]) -> Result<String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    // Collect slide file names first
    let mut slide_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_string();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                slide_names.push(name);
            }
        }
    }

    // Sort by slide number
    slide_names.sort_by(|a, b| {
        let n_a: u32 = a
            .trim_start_matches("ppt/slides/slide")
            .trim_end_matches(".xml")
            .parse()
            .unwrap_or(0);
        let n_b: u32 = b
            .trim_start_matches("ppt/slides/slide")
            .trim_end_matches(".xml")
            .parse()
            .unwrap_or(0);
        n_a.cmp(&n_b)
    });

    let mut text = String::new();
    for name in &slide_names {
        let mut xml = String::new();
        archive.by_name(name)?.read_to_string(&mut xml)?;

        let mut pos = 0;
        while pos < xml.len() {
            if let Some(rel) = xml[pos..].find("<a:t>") {
                let t_start = pos + rel + 5;
                if let Some(end) = xml[t_start..].find("</a:t>") {
                    let content = &xml[t_start..t_start + end];
                    if !content.trim().is_empty() {
                        text.push_str(content);
                        text.push(' ');
                    }
                    pos = t_start + end + 6;
                    continue;
                }
            }
            break;
        }
        text.push('\n');
    }

    Ok(text.trim().to_string())
}

/// Extract text values from an .xlsx file (Excel Open XML / ZIP container).
pub fn extract_xlsx_text(bytes: &[u8]) -> Result<String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    let mut text = String::new();

    // Shared strings contain all text cell values
    if let Ok(mut file) = archive.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        file.read_to_string(&mut xml)?;

        let mut pos = 0;
        while pos < xml.len() {
            // Match <t> or <t xml:space="preserve">
            if let Some(rel) = xml[pos..].find("<t") {
                let tag_start = pos + rel;
                if let Some(tag_end) = xml[tag_start..].find('>') {
                    let content_start = tag_start + tag_end + 1;
                    if let Some(end) = xml[content_start..].find("</t>") {
                        let val = &xml[content_start..content_start + end];
                        if !val.trim().is_empty() {
                            text.push_str(val);
                            text.push('\n');
                        }
                        pos = content_start + end + 4;
                        continue;
                    }
                }
            }
            break;
        }
    }

    Ok(text.trim().to_string())
}

/// Use Gemini Vision to describe an image file for interview context.
pub async fn describe_image_with_gemini(
    bytes: &[u8],
    mime_type: &str,
    gemini_key: &str,
) -> Result<String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);

    let body = serde_json::json!({
        "contents": [{
            "parts": [
                { "inlineData": { "mimeType": mime_type, "data": b64 } },
                { "text": "Describe the content of this image for job interview context. Focus on: visible text, credentials, certifications, achievements, projects, skills, metrics, or any professional information. Be concise and factual (2-4 sentences). If there is no meaningful professional content, say so briefly." }
            ]
        }]
    });

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        gemini_key
    );

    let resp = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Gemini image description error: {}", err));
    }

    let json: serde_json::Value = resp.json().await?;
    let description = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(description)
}
