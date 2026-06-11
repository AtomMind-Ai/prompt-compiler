use sha2::{Sha256, Digest};
use std::path::Path;

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[allow(dead_code)]
pub fn detect_file_format(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?;
    match extension.to_lowercase().as_str() {
        "txt" => Some("text".to_string()),
        "md" => Some("markdown".to_string()),
        "json" => Some("json".to_string()),
        _ => None,
    }
}

pub fn sanitize_section_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

#[allow(dead_code)]
pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("test");
        let hash2 = compute_hash("test");
        let hash3 = compute_hash("different");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_detect_file_format() {
        assert_eq!(detect_file_format(Path::new("test.txt")), Some("text".to_string()));
        assert_eq!(detect_file_format(Path::new("test.md")), Some("markdown".to_string()));
        assert_eq!(detect_file_format(Path::new("test.json")), Some("json".to_string()));
        assert_eq!(detect_file_format(Path::new("test.unknown")), None);
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("hello  world"), "hello world");
        assert_eq!(normalize_whitespace("hello\n\tworld"), "hello world");
        assert_eq!(normalize_whitespace("  hello  world  "), "hello world");
    }
}
