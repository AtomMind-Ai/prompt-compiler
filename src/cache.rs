use crate::types::{Chunk, CacheEntry};
use crate::utils::compute_hash;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{Utc, Duration};
use anyhow::{Result, Context};

pub struct Cache {
    cache_dir: PathBuf,
    entries: HashMap<String, CacheEntry>,
    max_age_days: i64,
}

impl Cache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;
        
        Ok(Self {
            cache_dir,
            entries: HashMap::new(),
            max_age_days: 30,
        })
    }

    pub fn load(&mut self) -> Result<()> {
        let cache_file = self.cache_file();
        
        if cache_file.exists() {
            let content = fs::read_to_string(&cache_file)
                .with_context(|| format!("Failed to read cache file: {}", cache_file.display()))?;
            
            let loaded: HashMap<String, CacheEntry> = serde_json::from_str(&content)
                .with_context(|| "Failed to parse cache file")?;
            
            // Filter out old entries
            let cutoff = Utc::now() - Duration::days(self.max_age_days);
            self.entries = loaded.into_iter()
                .filter(|(_, entry)| entry.timestamp > cutoff)
                .collect();
        }
        
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let cache_file = self.cache_file();
        let content = serde_json::to_string_pretty(&self.entries)?;
        
        fs::write(&cache_file, content)
            .with_context(|| format!("Failed to write cache file: {}", cache_file.display()))?;
        
        Ok(())
    }

    pub fn get(&self, content_hash: &str) -> Option<&CacheEntry> {
        self.entries.get(content_hash)
    }

    pub fn put(&mut self, content_hash: String, chunks: Vec<Chunk>) {
        let entry = CacheEntry {
            content_hash: content_hash.clone(),
            chunks,
            timestamp: Utc::now(),
        };
        self.entries.insert(content_hash, entry);
    }

    pub fn get_or_compute<F>(&mut self, content: &str, compute_fn: F) -> Result<Vec<Chunk>>
    where
        F: FnOnce() -> Result<Vec<Chunk>>,
    {
        let hash = compute_hash(content);
        
        if let Some(entry) = self.get(&hash) {
            return Ok(entry.chunks.clone());
        }
        
        let chunks = compute_fn()?;
        self.put(hash, chunks.clone());
        Ok(chunks)
    }

    pub fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        let cache_file = self.cache_file();
        if cache_file.exists() {
            fs::remove_file(&cache_file)
                .with_context(|| format!("Failed to remove cache file: {}", cache_file.display()))?;
        }
        Ok(())
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.entries.len(),
            total_chunks: self.entries.values().map(|e| e.chunks.len()).sum(),
            cache_dir: self.cache_dir.clone(),
        }
    }

    pub fn invalidate(&mut self, content_hash: &str) -> bool {
        self.entries.remove(content_hash).is_some()
    }

    pub fn invalidate_by_path(&mut self, path: &Path) -> Result<usize> {
        let path_str = path.to_string_lossy().to_string();
        let mut removed = 0;
        
        self.entries.retain(|_, entry| {
            let should_keep = entry.chunks.iter()
                .all(|c| c.source.file.as_ref().map(|f| f.to_string_lossy() != path_str).unwrap_or(true));
            
            if !should_keep {
                removed += 1;
            }
            should_keep
        });
        
        Ok(removed)
    }

    fn cache_file(&self) -> PathBuf {
        self.cache_dir.join("cache.json")
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_chunks: usize,
    pub cache_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceInfo;
    use tempfile::TempDir;

    #[test]
    fn test_cache_get_put() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = Cache::new(temp_dir.path().to_path_buf()).unwrap();
        
        let chunks = vec![
            Chunk {
                id: "1".to_string(),
                content: "test".to_string(),
                source: SourceInfo { file: None, offset_start: 0, offset_end: 4 },
                token_estimate: 1,
                line_start: 0,
                line_end: 1,
                section_path: vec!["root".to_string()],
            }
        ];
        
        cache.put("hash123".to_string(), chunks.clone());
        let retrieved = cache.get("hash123");
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().chunks.len(), 1);
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = Cache::new(temp_dir.path().to_path_buf()).unwrap();
        
        cache.put("hash123".to_string(), vec![]);
        assert_eq!(cache.stats().total_entries, 1);
        
        cache.clear().unwrap();
        assert_eq!(cache.stats().total_entries, 0);
    }

    #[test]
    fn test_cache_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = Cache::new(temp_dir.path().to_path_buf()).unwrap();
        
        let chunks = vec![
            Chunk {
                id: "1".to_string(),
                content: "test".to_string(),
                source: SourceInfo { file: None, offset_start: 0, offset_end: 4 },
                token_estimate: 1,
                line_start: 0,
                line_end: 1,
                section_path: vec!["root".to_string()],
            }
        ];
        
        cache.put("hash123".to_string(), chunks);
        cache.save().unwrap();
        
        let mut cache2 = Cache::new(temp_dir.path().to_path_buf()).unwrap();
        cache2.load().unwrap();
        
        assert_eq!(cache2.stats().total_entries, 1);
    }
}
