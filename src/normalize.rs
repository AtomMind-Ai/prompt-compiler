use unicode_normalization::UnicodeNormalization;
use regex::Regex;

pub struct Normalizer;

impl Normalizer {
    pub fn normalize(text: &str) -> String {
        let text = Self::normalize_unicode(text);
        let text = Self::normalize_line_endings(&text);
        let text = Self::remove_control_chars(&text);
        let text = Self::normalize_whitespace_runs(&text);
        text
    }

    pub fn normalize_unicode(text: &str) -> String {
        text.nfc().collect::<String>()
    }

    pub fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }

    pub fn remove_control_chars(text: &str) -> String {
        text.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect()
    }

    pub fn normalize_whitespace_runs(text: &str) -> String {
        let re = Regex::new(r"[ \t]+").unwrap();
        let text = re.replace_all(text, " ");
        let re = Regex::new(r"\n{3,}").unwrap();
        re.replace_all(&text, "\n\n").to_string()
    }

    pub fn split_sentences(text: &str) -> Vec<String> {
        let re = Regex::new(r"(?<=[.!?])\s+").unwrap();
        re.split(text)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub fn split_sections(text: &str) -> Vec<(String, String)> {
        let mut sections = Vec::new();
        let mut current_section = "root".to_string();
        let mut current_content = String::new();
        
        // Markdown-style headers
        let header_re = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
        
        for line in text.lines() {
            if let Some(caps) = header_re.captures(line) {
                if !current_content.trim().is_empty() {
                    sections.push((current_section.clone(), current_content.clone()));
                }
                current_section = caps[2].trim().to_string();
                current_content.clear();
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
        
        if !current_content.trim().is_empty() {
            sections.push((current_section, current_content));
        }
        
        if sections.is_empty() {
            sections.push(("root".to_string(), text.to_string()));
        }
        
        sections
    }

    pub fn extract_line_offsets(text: &str) -> Vec<(usize, usize)> {
        let mut offsets = Vec::new();
        let mut current_offset = 0;
        
        for line in text.lines() {
            let start = current_offset;
            let end = start + line.len() + 1; // +1 for newline
            offsets.push((start, end));
            current_offset = end;
        }
        
        offsets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_unicode() {
        let input = "cafe\u{0301}"; // cafe with combining acute accent
        let output = Normalizer::normalize_unicode(input);
        assert!(output.chars().count() <= input.chars().count());
    }

    #[test]
    fn test_normalize_line_endings() {
        assert_eq!(Normalizer::normalize_line_endings("a\r\nb\rc"), "a\nb\nc");
    }

    #[test]
    fn test_split_sentences() {
        let sentences = Normalizer::split_sentences("Hello world. How are you? I'm fine.");
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "Hello world.");
    }

    #[test]
    fn test_split_sections() {
        let text = "# Section 1\nContent 1\n\n# Section 2\nContent 2";
        let sections = Normalizer::split_sections(text);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, "Section 1");
    }
}
