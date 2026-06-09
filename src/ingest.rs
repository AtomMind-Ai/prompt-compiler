use crate::utils::detect_file_format;
use anyhow::{Result, Context};
use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::{Path, PathBuf};
use memmap2::Mmap;

pub struct Ingester;

impl Ingester {
    pub fn from_file(path: &Path) -> Result<String> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;
        
        // Use memory mapping for large files
        if file.metadata()?.len() > 10 * 1024 * 1024 { // 10MB threshold
            let mmap = unsafe { Mmap::map(&file)? };
            Ok(String::from_utf8_lossy(&mmap).to_string())
        } else {
            let mut buf_reader = BufReader::new(file);
            let mut content = String::new();
            buf_reader.read_to_string(&mut content)?;
            Ok(content)
        }
    }

    pub fn from_stdin() -> Result<String> {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        Ok(content)
    }

    pub fn detect_format(path: &Path) -> Option<String> {
        detect_file_format(path)
    }

    pub fn read_multiple(paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>> {
        let mut results = Vec::new();
        for path in paths {
            let content = Self::from_file(path)?;
            results.push((path.clone(), content));
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "test content").unwrap();
        
        let content = Ingester::from_file(temp_file.path()).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_detect_format() {
        assert_eq!(Ingester::detect_format(Path::new("test.txt")), Some("text".to_string()));
        assert_eq!(Ingester::detect_format(Path::new("test.md")), Some("markdown".to_string()));
        assert_eq!(Ingester::detect_format(Path::new("test.json")), Some("json".to_string()));
    }
}
