use crate::types::{Chunk, ChunkScore};
use std::collections::{HashMap, HashSet};

pub struct Ranker;

impl Ranker {
    /// Score chunks using deterministic heuristics
    pub fn score_chunks(chunks: &[Chunk]) -> Vec<ChunkScore> {
        let mut scores = Vec::new();
        
        // Precompute term frequencies for novelty scoring
        let term_freq = Self::compute_term_frequencies(chunks);
        
        for chunk in chunks {
            let salience = Self::compute_salience(chunk);
            let novelty = Self::compute_novelty(chunk, &term_freq);
            let redundancy = Self::compute_redundancy(chunk, chunks);
            let combined = Self::combine_scores(salience, novelty, redundancy);
            
            scores.push(ChunkScore {
                chunk_id: chunk.id.clone(),
                salience,
                novelty,
                redundancy,
                combined,
            });
        }
        
        scores
    }

    fn compute_salience(chunk: &Chunk) -> f64 {
        let mut score = 0.0;
        
        // Header emphasis: chunks in named sections get higher scores
        if chunk.section_path.len() > 1 || chunk.section_path.first().map(|s| s != "root") == Some(true) {
            score += 0.3;
        }
        
        // Length preference: moderate length chunks are preferred
        let length_score = Self::length_score(chunk.content.len());
        score += length_score * 0.2;
        
        // Keyword density: presence of important terms
        let keyword_score = Self::keyword_density(chunk);
        score += keyword_score * 0.3;
        
        // Structural position: earlier chunks in sections get slight preference
        let position_score = 1.0 / (chunk.line_start as f64 + 1.0).sqrt();
        score += position_score * 0.2;
        
        score.min(1.0)
    }

    fn length_score(length: usize) -> f64 {
        // Prefer chunks between 100 and 1000 characters
        if length < 50 {
            length as f64 / 50.0
        } else if length > 1000 {
            1000.0 / length as f64
        } else {
            1.0
        }
    }

    fn keyword_density(chunk: &Chunk) -> f64 {
        let important_terms = [
            "important", "key", "critical", "essential", "main", "primary",
            "significant", "notable", "crucial", "fundamental", "core",
            "definition", "example", "note", "warning", "error", "success",
        ];
        
        let content_lower = chunk.content.to_lowercase();
        let mut count = 0;
        
        for term in &important_terms {
            if content_lower.contains(term) {
                count += 1;
            }
        }
        
        (count as f64 / important_terms.len() as f64).min(1.0)
    }

    fn compute_term_frequencies(chunks: &[Chunk]) -> HashMap<String, usize> {
        let mut freq = HashMap::new();
        
        for chunk in chunks {
            let words: Vec<String> = chunk.content.split_whitespace()
                .map(|w| w.to_lowercase())
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| w.len() > 3)
                .collect();
            
            for word in words {
                *freq.entry(word.to_string()).or_insert(0) += 1;
            }
        }
        
        freq
    }

    fn compute_novelty(chunk: &Chunk, term_freq: &HashMap<String, usize>) -> f64 {
        let words: Vec<String> = chunk.content.split_whitespace()
            .map(|w| w.to_lowercase())
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| w.len() > 3)
            .collect();
        
        if words.is_empty() {
            return 0.0;
        }
        
        let total_chunks = term_freq.values().sum::<usize>() as f64;
        let mut novelty_sum = 0.0;
        
        for word in &words {
            if let Some(&count) = term_freq.get(&word.to_string()) {
                // Inverse document frequency style scoring
                let idf = (total_chunks / count as f64).ln();
                novelty_sum += idf;
            }
        }
        
        (novelty_sum / words.len() as f64).min(1.0)
    }

    fn compute_redundancy(chunk: &Chunk, all_chunks: &[Chunk]) -> f64 {
        let chunk_words: HashSet<String> = chunk.content.split_whitespace()
            .map(|w| w.to_lowercase())
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| w.len() > 3)
            .collect();
        
        if chunk_words.is_empty() {
            return 0.0;
        }
        
        let mut max_overlap: f64 = 0.0;
        
        for other in all_chunks {
            if other.id == chunk.id {
                continue;
            }
            
            let other_words: HashSet<String> = other.content.split_whitespace()
                .map(|w| w.to_lowercase())
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| w.len() > 3)
                .collect();
            
            if other_words.is_empty() {
                continue;
            }
            
            let intersection = chunk_words.intersection(&other_words).count();
            let union = chunk_words.union(&other_words).count();
            
            if union > 0 {
                let overlap = intersection as f64 / union as f64;
                max_overlap = max_overlap.max(overlap);
            }
        }
        
        // Return redundancy penalty (lower is better)
        max_overlap
    }

    fn combine_scores(salience: f64, novelty: f64, redundancy: f64) -> f64 {
        // Weighted combination
        let salience_weight = 0.4;
        let novelty_weight = 0.4;
        let redundancy_penalty = 0.2;
        
        salience * salience_weight + novelty * novelty_weight - redundancy * redundancy_penalty
    }

    /// Sort chunks by score in descending order
    pub fn sort_by_score(chunks: &[Chunk], scores: &[ChunkScore]) -> Vec<(Chunk, ChunkScore)> {
        let mut paired: Vec<_> = chunks.iter()
            .zip(scores.iter())
            .collect();
        
        paired.sort_by(|a, b| {
            b.1.combined.partial_cmp(&a.1.combined).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        paired.into_iter()
            .map(|(c, s)| (c.clone(), s.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceInfo;

    #[test]
    fn test_score_chunks() {
        let chunks = vec![
            Chunk {
                id: "1".to_string(),
                content: "This is important content with key terms.".to_string(),
                source: SourceInfo { file: None, offset_start: 0, offset_end: 50 },
                token_estimate: 10,
                line_start: 0,
                line_end: 1,
                section_path: vec!["important".to_string()],
            },
            Chunk {
                id: "2".to_string(),
                content: "This is important content with key terms.".to_string(),
                source: SourceInfo { file: None, offset_start: 50, offset_end: 100 },
                token_estimate: 10,
                line_start: 1,
                line_end: 2,
                section_path: vec!["root".to_string()],
            },
        ];
        
        let scores = Ranker::score_chunks(&chunks);
        assert_eq!(scores.len(), 2);
        assert!(scores.iter().all(|s| s.combined >= 0.0));
    }

    #[test]
    fn test_sort_by_score() {
        let chunks = vec![
            Chunk {
                id: "1".to_string(),
                content: "low".to_string(),
                source: SourceInfo { file: None, offset_start: 0, offset_end: 3 },
                token_estimate: 1,
                line_start: 0,
                line_end: 1,
                section_path: vec!["root".to_string()],
            },
            Chunk {
                id: "2".to_string(),
                content: "high".to_string(),
                source: SourceInfo { file: None, offset_start: 3, offset_end: 7 },
                token_estimate: 1,
                line_start: 1,
                line_end: 2,
                section_path: vec!["important".to_string()],
            },
        ];
        
        let scores = Ranker::score_chunks(&chunks);
        let sorted = Ranker::sort_by_score(&chunks, &scores);
        
        assert_eq!(sorted.len(), 2);
        assert!(sorted[0].1.combined >= sorted[1].1.combined);
    }
}
