use crate::types::{Chunk, ChunkScore, Budget, SelectionResult};
use anyhow::Result;

pub struct Selector;

impl Selector {
    /// Select chunks under budget using greedy algorithm
    pub fn select_greedy(
        chunks: Vec<Chunk>,
        scores: Vec<ChunkScore>,
        budget: &mut Budget,
    ) -> SelectionResult {
        let mut selected = Vec::new();
        let mut selected_scores = Vec::new();
        let mut rejected_count = 0;
        
        // Create a map of chunk_id to score
        let score_map: std::collections::HashMap<String, ChunkScore> = scores
            .into_iter()
            .map(|s| (s.chunk_id.clone(), s))
            .collect();
        
        // Sort chunks by score
        let mut sorted_chunks: Vec<_> = chunks.into_iter()
            .map(|c| (c.clone(), score_map.get(&c.id).cloned().unwrap_or_else(|| {
                ChunkScore {
                    chunk_id: c.id.clone(),
                    salience: 0.0,
                    novelty: 0.0,
                    redundancy: 0.0,
                    combined: 0.0,
                }
            })))
            .collect();
        
        sorted_chunks.sort_by(|a, b| {
            b.1.combined.partial_cmp(&a.1.combined).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Greedy selection
        let total_input_chunks = sorted_chunks.len();
        for (chunk, score) in sorted_chunks {
            if budget.can_fit(chunk.token_estimate) {
                budget.use_tokens(chunk.token_estimate);
                selected.push(chunk);
                selected_scores.push(score);
            } else {
                rejected_count += 1;
            }
        }
        
        SelectionResult {
            chunks: selected,
            budget: budget.clone(),
            scores: selected_scores,
            total_input_chunks,
            rejected_count,
        }
    }

    /// Select chunks using knapsack-style optimization for smaller inputs
    /// This is more expensive but can find better selections
    pub fn select_knapsack(
        chunks: Vec<Chunk>,
        scores: Vec<ChunkScore>,
        budget: &mut Budget,
    ) -> Result<SelectionResult> {
        // Only use knapsack for smaller inputs to avoid exponential blowup
        if chunks.len() > 100 {
            return Ok(Self::select_greedy(chunks, scores, budget));
        }
        
        let max_budget = budget.max_tokens;
        let n = chunks.len();
        
        // Create score map
        let score_map: std::collections::HashMap<String, ChunkScore> = scores
            .into_iter()
            .map(|s| (s.chunk_id.clone(), s))
            .collect();
        
        // DP table: dp[i][w] = max score using first i items with budget w
        let mut dp = vec![vec![0.0; max_budget + 1]; n + 1];
        
        for i in 1..=n {
            let chunk = &chunks[i - 1];
            let score = score_map.get(&chunk.id)
                .map(|s| s.combined)
                .unwrap_or(0.0);
            let tokens = chunk.token_estimate;
            
            for w in 0..=max_budget {
                if tokens <= w {
                    dp[i][w] = dp[i - 1][w].max(dp[i - 1][w - tokens] + score);
                } else {
                    dp[i][w] = dp[i - 1][w];
                }
            }
        }
        
        // Backtrack to find selected items
        let mut selected = Vec::new();
        let mut selected_scores = Vec::new();
        let mut w = max_budget;
        
        for i in (1..=n).rev() {
            let chunk = &chunks[i - 1];
            let score = score_map.get(&chunk.id)
                .map(|s| s.combined)
                .unwrap_or(0.0);
            let tokens = chunk.token_estimate;
            
            if dp[i][w] != dp[i - 1][w] && tokens <= w {
                selected.push(chunk.clone());
                selected_scores.push(score_map.get(&chunk.id).cloned().unwrap());
                w -= tokens;
            }
        }
        
        selected.reverse();
        selected_scores.reverse();
        
        let used_tokens: usize = selected.iter().map(|c| c.token_estimate).sum();
        budget.used_tokens = used_tokens;
        budget.remaining_tokens = budget.max_tokens - used_tokens;
        
        Ok(SelectionResult {
            chunks: selected,
            budget: budget.clone(),
            scores: selected_scores,
            total_input_chunks: n,
            rejected_count: n - selected.len(),
        })
    }

    /// Remove near-duplicate chunks from selection
    pub fn deduplicate(chunks: Vec<Chunk>, _similarity_threshold: f64) -> Vec<Chunk> {
        let mut deduped = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();
        
        for chunk in chunks {
            let content_hash = crate::utils::compute_hash(&chunk.content);
            
            if !seen_hashes.contains(&content_hash) {
                seen_hashes.insert(content_hash);
                deduped.push(chunk);
            }
        }
        
        deduped
    }

    /// Ensure diverse section coverage
    pub fn ensure_diversity(chunks: Vec<Chunk>, min_sections: usize) -> Vec<Chunk> {
        if chunks.len() <= min_sections {
            return chunks;
        }
        
        // Count sections
        let mut section_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for chunk in &chunks {
            let section = chunk.section_path.get(0).map(|s| s.as_str()).unwrap_or("root");
            *section_counts.entry(section.to_string()).or_insert(0) += 1;
        }
        
        // If we have enough sections, return as-is
        if section_counts.len() >= min_sections {
            return chunks;
        }
        
        // Otherwise, try to include at least one chunk from each section
        let mut seen_sections = std::collections::HashSet::new();
        let mut diverse = Vec::new();
        let mut remaining = Vec::new();
        
        for chunk in chunks {
            let section = chunk.section_path.get(0).map(|s| s.as_str()).unwrap_or("root");
            if seen_sections.contains(section) {
                remaining.push(chunk);
            } else {
                seen_sections.insert(section.to_string());
                diverse.push(chunk);
            }
        }
        
        // Add remaining chunks up to budget
        diverse.extend(remaining);
        
        diverse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceInfo;

    fn create_test_chunk(id: &str, content: &str, tokens: usize) -> Chunk {
        Chunk {
            id: id.to_string(),
            content: content.to_string(),
            source: SourceInfo { file: None, offset_start: 0, offset_end: content.len() },
            token_estimate: tokens,
            line_start: 0,
            line_end: 1,
            section_path: vec!["root".to_string()],
        }
    }

    #[test]
    fn test_select_greedy() {
        let chunks = vec![
            create_test_chunk("1", "high value", 10),
            create_test_chunk("2", "medium value", 20),
            create_test_chunk("3", "low value", 30),
        ];
        
        let scores = vec![
            ChunkScore { chunk_id: "1".to_string(), salience: 0.9, novelty: 0.8, redundancy: 0.1, combined: 1.0 },
            ChunkScore { chunk_id: "2".to_string(), salience: 0.6, novelty: 0.5, redundancy: 0.2, combined: 0.6 },
            ChunkScore { chunk_id: "3".to_string(), salience: 0.3, novelty: 0.3, redundancy: 0.3, combined: 0.3 },
        ];
        
        let mut budget = Budget::new(25);
        let result = Selector::select_greedy(chunks, scores, &mut budget);
        
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].id, "1");
    }

    #[test]
    fn test_deduplicate() {
        let chunks = vec![
            create_test_chunk("1", "same content", 10),
            create_test_chunk("2", "same content", 10),
            create_test_chunk("3", "different content", 10),
        ];
        
        let deduped = Selector::deduplicate(chunks, 0.9);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_ensure_diversity() {
        let chunks = vec![
            Chunk {
                id: "1".to_string(),
                content: "content".to_string(),
                source: SourceInfo { file: None, offset_start: 0, offset_end: 7 },
                token_estimate: 5,
                line_start: 0,
                line_end: 1,
                section_path: vec!["section1".to_string()],
            },
            Chunk {
                id: "2".to_string(),
                content: "content".to_string(),
                source: SourceInfo { file: None, offset_start: 7, offset_end: 14 },
                token_estimate: 5,
                line_start: 1,
                line_end: 2,
                section_path: vec!["section1".to_string()],
            },
            Chunk {
                id: "3".to_string(),
                content: "content".to_string(),
                source: SourceInfo { file: None, offset_start: 14, offset_end: 21 },
                token_estimate: 5,
                line_start: 2,
                line_end: 3,
                section_path: vec!["section2".to_string()],
            },
        ];
        
        let diverse = Selector::ensure_diversity(chunks, 2);
        assert!(diverse.len() >= 2);
    }
}
