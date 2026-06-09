use crate::types::{Chunk, SourceInfo};
use crate::normalize::Normalizer;
use crate::tokenize::Tokenizer;
use crate::utils::{compute_hash, sanitize_section_name};
use std::path::PathBuf;
use anyhow::Result;

pub struct Segmenter;

impl Segmenter {
    pub fn chunk_text(
        text: &str,
        source_file: Option<PathBuf>,
        max_chunk_tokens: usize,
    ) -> Result<Vec<Chunk>> {
        let normalized = Normalizer::normalize(text);
        let sections = Normalizer::split_sections(&normalized);
        let line_offsets = Normalizer::extract_line_offsets(&normalized);
        
        let mut chunks = Vec::new();
        let mut chunk_id_counter = 0;
        
        for (section_name, section_content) in sections {
            let section_chunks = Self::chunk_section(
                &section_content,
                source_file.clone(),
                &section_name,
                max_chunk_tokens,
                &line_offsets,
                &mut chunk_id_counter,
            )?;
            chunks.extend(section_chunks);
        }
        
        Ok(chunks)
    }

    fn chunk_section(
        section_content: &str,
        source_file: Option<PathBuf>,
        section_name: &str,
        max_chunk_tokens: usize,
        line_offsets: &[(usize, usize)],
        chunk_id_counter: &mut usize,
    ) -> Result<Vec<Chunk>> {
        let sentences = Normalizer::split_sentences(section_content);
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_tokens = 0;
        let mut chunk_start_line = 0;
        let mut current_line = 0;
        
        let section_path = vec![sanitize_section_name(section_name)];
        
        for sentence in &sentences {
            let sentence_tokens = Tokenizer::estimate_tokens(sentence);
            
            if current_tokens + sentence_tokens > max_chunk_tokens && !current_chunk.is_empty() {
                // Finalize current chunk
                let chunk = Self::create_chunk(
                    &current_chunk,
                    source_file.clone(),
                    chunk_start_line,
                    current_line,
                    section_path.clone(),
                    line_offsets,
                    *chunk_id_counter,
                )?;
                chunks.push(chunk);
                *chunk_id_counter += 1;
                
                // Start new chunk
                current_chunk = sentence.clone();
                current_tokens = sentence_tokens;
                chunk_start_line = current_line;
            } else {
                if current_chunk.is_empty() {
                    chunk_start_line = current_line;
                }
                current_chunk.push_str(sentence);
                current_chunk.push(' ');
                current_tokens += sentence_tokens;
            }
            
            current_line += sentence.lines().count();
        }
        
        // Add final chunk if not empty
        if !current_chunk.trim().is_empty() {
            let chunk = Self::create_chunk(
                &current_chunk,
                source_file,
                chunk_start_line,
                current_line,
                section_path,
                line_offsets,
                *chunk_id_counter,
            )?;
            chunks.push(chunk);
        }
        
        Ok(chunks)
    }

    fn create_chunk(
        content: &str,
        source_file: Option<PathBuf>,
        line_start: usize,
        line_end: usize,
        section_path: Vec<String>,
        line_offsets: &[(usize, usize)],
        id: usize,
    ) -> Result<Chunk> {
        let content = content.trim().to_string();
        let token_estimate = Tokenizer::estimate_tokens(&content);
        
        let offset_start = line_offsets.get(line_start).map(|&(s, _)| s).unwrap_or(0);
        let offset_end = line_offsets.get(line_end.saturating_sub(1))
            .map(|&(_, e)| e)
            .unwrap_or(offset_start + content.len());
        
        let source = SourceInfo {
            file: source_file,
            offset_start,
            offset_end,
        };
        
        Ok(Chunk {
            id: format!("chunk_{}", id),
            content,
            source,
            token_estimate,
            line_start,
            line_end,
            section_path,
        })
    }

    pub fn merge_small_chunks(chunks: Vec<Chunk>, min_chunk_tokens: usize) -> Vec<Chunk> {
        let mut merged = Vec::new();
        let mut current_merge: Vec<Chunk> = Vec::new();
        let mut current_tokens = 0;
        
        for chunk in chunks {
            if chunk.token_estimate < min_chunk_tokens {
                current_merge.push(chunk);
                current_tokens += chunk.token_estimate;
            } else {
                // Flush current merge if any
                if !current_merge.is_empty() {
                    if current_tokens >= min_chunk_tokens {
                        merged.extend(current_merge);
                    } else {
                        // Merge them together
                        let merged_chunk = Self::do_merge_chunks(current_merge);
                        merged.push(merged_chunk);
                    }
                    current_merge = Vec::new();
                    current_tokens = 0;
                }
                merged.push(chunk);
            }
        }
        
        // Flush remaining
        if !current_merge.is_empty() {
            if current_tokens >= min_chunk_tokens {
                merged.extend(current_merge);
            } else {
                let merged_chunk = Self::do_merge_chunks(current_merge);
                merged.push(merged_chunk);
            }
        }
        
        merged
    }

    fn do_merge_chunks(chunks: Vec<Chunk>) -> Chunk {
        let first = &chunks[0];
        let last = &chunks.last().unwrap();
        
        let content = chunks.iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        
        let token_estimate = Tokenizer::estimate_tokens(&content);
        
        Chunk {
            id: format!("merged_{}", first.id),
            content,
            source: first.source.clone(),
            token_estimate,
            line_start: first.line_start,
            line_end: last.line_end,
            section_path: first.section_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let text = "This is sentence one. This is sentence two. This is sentence three.";
        let chunks = Segmenter::chunk_text(text, None, 20).unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| !c.content.is_empty()));
    }

    #[test]
    fn test_merge_small_chunks() {
        let chunks = vec![
            Chunk {
                id: "1".to_string(),
                content: "small".to_string(),
                source: SourceInfo { file: None, offset_start: 0, offset_end: 5 },
                token_estimate: 1,
                line_start: 0,
                line_end: 1,
                section_path: vec!["root".to_string()],
            },
            Chunk {
                id: "2".to_string(),
                content: "large content here that is definitely longer".to_string(),
                source: SourceInfo { file: None, offset_start: 5, offset_end: 50 },
                token_estimate: 10,
                line_start: 1,
                line_end: 2,
                section_path: vec!["root".to_string()],
            },
        ];
        
        let merged = Segmenter::merge_small_chunks(chunks, 5);
        assert_eq!(merged.len(), 2);
    }
}
