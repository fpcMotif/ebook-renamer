#![allow(dead_code)]

use anyhow::{anyhow, Result};
use lopdf::Document;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read as IoRead;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub year: Option<String>,
    pub publisher: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<Vec<String>>,
}

impl Default for ExtractedMetadata {
    fn default() -> Self {
        ExtractedMetadata {
            title: None,
            author: None,
            year: None,
            publisher: None,
            subject: None,
            keywords: None,
        }
    }
}

impl ExtractedMetadata {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.author.is_none()
            && self.year.is_none()
            && self.publisher.is_none()
            && self.subject.is_none()
            && self.keywords.is_none()
    }
}

pub struct MetadataExtractor;

impl MetadataExtractor {
    pub fn extract_from_file(path: &Path) -> Result<ExtractedMetadata> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match extension.as_deref() {
            Some("pdf") => Self::extract_pdf_metadata(path),
            Some("epub") => Self::extract_epub_metadata(path),
            _ => Ok(ExtractedMetadata::default()),
        }
    }

    fn extract_pdf_metadata(path: &Path) -> Result<ExtractedMetadata> {
        let doc = Document::load(path)?;
        let mut metadata = ExtractedMetadata::default();

        if let Ok(info_ref) = doc.trailer.get(b"Info") {
            if let lopdf::Object::Reference(object_id) = info_ref {
                if let Ok(lopdf::Object::Dictionary(dict)) = doc.get_object(*object_id) {
                    if let Some(title) = dict
                        .get(b"Title")
                        .ok()
                        .and_then(|o| Self::extract_pdf_string(o))
                    {
                        metadata.title = Some(title);
                    }

                    if let Some(author) = dict
                        .get(b"Author")
                        .ok()
                        .and_then(|o| Self::extract_pdf_string(o))
                    {
                        metadata.author = Some(author);
                    }

                    if let Some(date) = dict
                        .get(b"CreationDate")
                        .ok()
                        .and_then(|o| Self::extract_pdf_string(o))
                    {
                        if let Some(year) = Self::extract_year_from_pdf_date(&date) {
                            metadata.year = Some(year);
                        }
                    }

                    if let Some(subject) = dict
                        .get(b"Subject")
                        .ok()
                        .and_then(|o| Self::extract_pdf_string(o))
                    {
                        metadata.subject = Some(subject);
                    }

                    if let Some(publisher) = dict
                        .get(b"Producer")
                        .ok()
                        .and_then(|o| Self::extract_pdf_string(o))
                    {
                        metadata.publisher = Some(publisher);
                    }
                }
            }
        }

        Ok(metadata)
    }

    fn extract_pdf_string(obj: &lopdf::Object) -> Option<String> {
        match obj {
            lopdf::Object::String(bytes, _) => {
                if bytes.starts_with(&[0xFE, 0xFF]) {
                    let utf16_bytes: Vec<u16> = bytes[2..]
                        .chunks(2)
                        .filter_map(|chunk| {
                            if chunk.len() == 2 {
                                Some(u16::from_be_bytes([chunk[0], chunk[1]]))
                            } else {
                                None
                            }
                        })
                        .collect();
                    String::from_utf16(&utf16_bytes).ok()
                } else {
                    String::from_utf8(bytes.clone())
                        .ok()
                        .or_else(|| {
                            bytes
                                .iter()
                                .map(|&b| b as char)
                                .collect::<String>()
                                .into()
                        })
                }
            }
            lopdf::Object::Reference(_) => None,
            _ => None,
        }
    }

    fn extract_year_from_pdf_date(date: &str) -> Option<String> {
        if date.starts_with("D:") && date.len() >= 6 {
            let year_part = &date[2..6];
            if year_part.chars().all(|c| c.is_ascii_digit()) {
                return Some(year_part.to_string());
            }
        } else if date.len() >= 4 && date[..4].chars().all(|c| c.is_ascii_digit()) {
            return Some(date[..4].to_string());
        }
        None
    }

    fn extract_year_from_date(date_str: &str) -> Option<String> {
        if let Some(dash_idx) = date_str.find('-') {
            let year_part = &date_str[..dash_idx];
            if year_part.len() >= 4 {
                let year: &str = &year_part[..4];
                if year.chars().all(|c| c.is_ascii_digit()) {
                    return Some(year.to_string());
                }
            }
        }
        None
    }

    fn extract_epub_metadata(path: &Path) -> Result<ExtractedMetadata> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let opf_path = Self::find_opf_path(&mut archive)?;

        if let Some(opf_path) = opf_path {
            let mut opf_file = archive.by_name(&opf_path)?;
            let mut opf_content = String::new();
            opf_file.read_to_string(&mut opf_content)?;
            return Ok(Self::parse_opf_metadata(&opf_content));
        }

        Ok(ExtractedMetadata::default())
    }

    fn find_opf_path(archive: &mut zip::ZipArchive<fs::File>) -> Result<Option<String>> {
        if let Ok(mut container) = archive.by_name("META-INF/container.xml") {
            let mut content = String::new();
            container.read_to_string(&mut content)?;

            if let Some(start) = content.find("full-path=\"") {
                let rest = &content[start + 11..];
                if let Some(end) = rest.find('"') {
                    return Ok(Some(rest[..end].to_string()));
                }
            }
        }

        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                if file.name().ends_with(".opf") {
                    return Ok(Some(file.name().to_string()));
                }
            }
        }

        Ok(None)
    }

    fn parse_opf_metadata(content: &str) -> ExtractedMetadata {
        let mut metadata = ExtractedMetadata::default();

        if let Some(title_match) = Self::extract_opf_tag(content, "<dc:title>") {
            metadata.title = Some(title_match);
        } else if let Some(title_match) = Self::extract_opf_tag(content, "<title>") {
            metadata.title = Some(title_match);
        }

        if let Some(author_match) = Self::extract_opf_tag(content, "<dc:creator>") {
            metadata.author = Some(author_match);
        } else if let Some(author_match) = Self::extract_opf_tag(content, "<creator>") {
            metadata.author = Some(author_match);
        }

        if let Some(date_match) = Self::extract_opf_tag(content, "<dc:date>") {
            if date_match.len() >= 4 {
                metadata.year = Some(date_match[..4].to_string());
            }
        }

        if let Some(publisher_match) = Self::extract_opf_tag(content, "<dc:publisher>") {
            metadata.publisher = Some(publisher_match);
        }

        if let Some(subject_match) = Self::extract_opf_tag(content, "<dc:subject>") {
            metadata.subject = Some(subject_match);
        }

        metadata
    }

    fn extract_opf_tag(content: &str, tag: &str) -> Option<String> {
        if let Some(start) = content.find(tag) {
            let adjusted_tag = tag.trim_start_matches('<').trim_end_matches('>');
            let closing_tag = format!("</{}>", adjusted_tag);

            if let Some(end) = content[start..].find(&closing_tag) {
                let start_idx = start + tag.len();
                let end_idx = start + end;
                let content_between = &content[start_idx..end_idx];
                return Some(content_between.trim().to_string());
            }
        }

        let meta_pattern = format!("<meta name=\"{}\"", tag);
        if let Some(start) = content.find(&meta_pattern) {
            let after_start = &content[start..];
            if let Some(content_start) = after_start.find('>') {
                let meta_content = &after_start[..=content_start];
                if let Some(value_start) = meta_content.find("content=\"") {
                    let value = &meta_content[value_start + 9..];
                    if let Some(value_end) = value.find('\"') {
                        return Some(value[..value_end].trim().to_string());
                    }
                }
            }
        }

        None
    }
}

pub struct ArxivClient {
    client: reqwest::blocking::Client,
    cache_dir: PathBuf,
}

impl ArxivClient {
    pub fn new() -> Result<Self> {
        let cache_dir = Self::get_cache_dir()?;

        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(ArxivClient { client, cache_dir })
    }

    fn get_cache_dir() -> Result<PathBuf> {
        if let Some(cache_home) = std::env::var_os("XDG_CACHE_HOME") {
            if !cache_home.is_empty() {
                return Ok(PathBuf::from(cache_home).join("ebook-renamer").join("arxiv"));
            }
        }

        if let Some(home) = dirs::cache_dir() {
            return Ok(home.join("ebook-renamer").join("arxiv"));
        }

        Err(anyhow!("Could not determine cache directory"))
    }

    pub fn fetch_metadata(&self, query: &str) -> Result<Option<ExtractedMetadata>> {
        let cache_key = Self::generate_cache_key(query);
        let cache_path = self.cache_dir.join(&cache_key);

        if cache_path.exists() {
            if let Ok(content) = fs::read_to_string(&cache_path) {
                return Self::parse_arxiv_response(&content);
            }
        }

        if let Some(result) = self.fetch_from_api(query)? {
            if let Some(json) = serde_json::to_string(&result).ok() {
                fs::write(&cache_path, &json).ok();
            }
            return Ok(Some(result));
        }

        Ok(None)
    }

    fn generate_cache_key(query: &str) -> String {
        let hash = md5::compute(query);
        format!("{:x}.json", hash)
    }

    fn fetch_from_api(&self, query: &str) -> Result<Option<ExtractedMetadata>> {
        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=all:{}&max_results=1",
            encoded_query
        );

        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let content = response.text()?;
        Self::parse_arxiv_response(&content)
    }

    fn parse_arxiv_response(content: &str) -> Result<Option<ExtractedMetadata>> {
        let entries = content.split("<entry>").collect::<Vec<_>>();

        if entries.len() < 2 {
            return Ok(None);
        }

        let entry = entries[1];

        let mut metadata = ExtractedMetadata::default();

        if let Some(title) = Self::extract_arxiv_field(entry, "<title>") {
            let clean_title = title.lines().map(|l| l.trim()).collect::<Vec<_>>().join(" ");
            metadata.title = Some(clean_title);
        }

        if let Some(author) = Self::extract_arxiv_field(entry, "<author><name>") {
            metadata.author = Some(author.trim().to_string());
        }

        if let Some(published) = Self::extract_arxiv_field(entry, "<published>") {
            if published.len() >= 4 {
                metadata.year = Some(published[..4].to_string());
            }
        }

        if let Some(summary) = Self::extract_arxiv_field(entry, "<summary>") {
            metadata.subject = Some(summary.lines().map(|l| l.trim()).collect::<Vec<_>>().join(" "));
        }

        Ok(Some(metadata))
    }

    fn extract_arxiv_field(entry: &str, tag: &str) -> Option<String> {
        if let Some(start) = entry.find(tag) {
            let adjusted_tag = tag.trim_start_matches('<').trim_end_matches('>');
            let closing_tag = format!("</{}>", adjusted_tag.split_whitespace().next().unwrap_or(adjusted_tag));

            if let Some(end) = entry[start..].find(&closing_tag) {
                let start_idx = start + tag.len();
                let end_idx = start + end;
                return Some(entry[start_idx..end_idx].to_string());
            }
        }
        None
    }

    pub fn detect_arxiv_id(filename: &str) -> Option<String> {
        let patterns = [
            r"arXiv:(\d{4}\.\d{4,5})",
            r"(\d{4}\.\d{4,5})",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(filename) {
                    if let Some(id) = caps.get(1) {
                        return Some(id.as_str().to_string());
                    }
                }
            }
        }

        None
    }
}

impl Default for ArxivClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| ArxivClient {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            cache_dir: PathBuf::from("/tmp/ebook-renamer-arxiv"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_metadata_default() {
        let meta = ExtractedMetadata::default();
        assert!(meta.is_empty());
    }

    #[test]
    fn test_metadata_with_values() {
        let meta = ExtractedMetadata {
            title: Some("Test Book".to_string()),
            author: Some("Test Author".to_string()),
            year: Some("2024".to_string()),
            publisher: None,
            subject: None,
            keywords: None,
        };
        assert!(!meta.is_empty());
    }

    #[test]
    fn test_arxiv_id_detection() {
        let id = ArxivClient::detect_arxiv_id("arXiv:2312.12345v1");
        assert_eq!(id, Some("2312.12345".to_string()));

        let id2 = ArxivClient::detect_arxiv_id("2312.12345.pdf");
        assert_eq!(id2, Some("2312.12345".to_string()));

        let no_id = ArxivClient::detect_arxiv_id("regular_book.pdf");
        assert_eq!(no_id, None);
    }

    #[test]
    fn test_year_extraction() {
        let meta = ExtractedMetadata {
            year: Some("2024".to_string()),
            ..Default::default()
        };
        assert_eq!(meta.year, Some("2024".to_string()));
    }
}
