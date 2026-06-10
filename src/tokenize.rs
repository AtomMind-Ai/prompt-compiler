use crate::types::Chunk;

pub struct Tokenizer;

impl Tokenizer {
    /// Estimate token count using a character-based heuristic
    /// This is a fallback when no tokenizer library is available
    /// Approximation: ~4 characters per token for English text
    pub fn estimate_tokens(text: &str) -> usize {
        let char_count = text.chars().count();
        let word_count = text.split_whitespace().count();
        
        // Use a weighted average of character and word-based estimates
        // Characters / 4 is a rough approximation for English
        // Words * 1.3 accounts for subword tokenization
        let char_estimate = char_count / 4;
        let word_estimate = word_count * 13 / 10;
        
        // Take the maximum to be conservative
        char_estimate.max(word_estimate).max(1)
    }

    /// Estimate tokens for a chunk
    pub fn estimate_chunk_tokens(chunk: &Chunk) -> usize {
        Self::estimate_tokens(&chunk.content)
    }

    /// Estimate tokens for multiple chunks
    pub fn estimate_total_tokens(chunks: &[Chunk]) -> usize {
        chunks.iter().map(Self::estimate_chunk_tokens).sum()
    }

    /// Check if content fits within budget
    pub fn fits_budget(text: &str, budget: usize) -> bool {
        Self::estimate_tokens(text) <= budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let estimate = Tokenizer::estimate_tokens("Hello world, this is a test.");
        assert!(estimate > 0);
        assert!(estimate < 100);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(Tokenizer::estimate_tokens(""), 1);
    }

    #[test]
    fn test_estimate_tokens_long() {
        let text = "word ".repeat(100);
        let estimate = Tokenizer::estimate_tokens(&text);
        assert!(estimate > 50);
    }

    #[test]
    fn test_fits_budget() {
        assert!(Tokenizer::fits_budget("short text", 100));
        assert!(!Tokenizer::fits_budget("word ".repeat(1000).as_str(), 10));
    }
}
