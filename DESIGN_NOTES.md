# Design Notes: Local Prompt Compiler

## Overview

The Local Prompt Compiler is a deterministic local inference utility designed to transform raw text into compact, budgeted, schema-valid AI-ready outputs. This document explains the architectural decisions, algorithm choices, and implementation strategies.

## Architecture Principles

### 1. Modularity

The codebase is organized into clear, single-responsibility modules:

- **ingest**: Input handling (files, stdin, memory mapping)
- **normalize**: Text preprocessing (unicode, whitespace, structure)
- **tokenize**: Token estimation (character/word-based heuristics)
- **segment**: Chunking with metadata preservation
- **rank**: Deterministic scoring heuristics
- **select**: Budget-aware selection algorithms
- **schema**: JSON schema validation and repair
- **render**: Output formatting (compact, JSON, Markdown)
- **cache**: Hash-based incremental caching
- **utils**: Shared utilities (hashing, path handling)

This modularity enables:
- Easy testing of individual components
- Clear separation of concerns
- Future extensibility
- Parallel development

### 2. Determinism

All processing is deterministic:
- No random scoring
- No learned model dependencies
- Reproducible outputs for identical inputs
- Predictable performance characteristics

This is critical for:
- Debugging and testing
- Reproducible pipelines
- Trust in automated systems
- Caching effectiveness

### 3. Performance

Performance considerations:

- **Memory**: Streaming for large files, memory mapping for efficiency
- **CPU**: Single-threaded processing, no GPU requirements
- **Algorithms**: Greedy selection for speed, knapsack for optimization
- **Data Structures**: Efficient collections (HashMap, HashSet, Vec)

### 4. Type Safety

Strong type definitions for:
- Chunks with metadata
- Budget constraints
- Validation results
- Output packages

This prevents:
- Type errors at runtime
- Invalid state transitions
- Data corruption

## Algorithm Choices

### Token Estimation

**Problem**: Need token counts without external tokenizer libraries.

**Solution**: Character/word-based heuristic:
- Character estimate: `char_count / 4`
- Word estimate: `word_count * 1.3`
- Final: `max(char_estimate, word_estimate)`

**Rationale**:
- No external dependencies
- Reasonable accuracy for English text
- Conservative (overestimates slightly)
- Fast computation

**Trade-offs**:
- Less accurate than proper tokenizers
- Language-dependent (optimized for English)
- May overestimate for code or technical content

### Chunking Strategy

**Problem**: Split text into semantically meaningful chunks.

**Solution**: Hierarchical chunking:
1. Split by sections (headers)
2. Split by sentences within sections
3. Merge sentences up to token limit
4. Preserve metadata (offsets, line numbers, section path)

**Rationale**:
- Preserves document structure
- Maintains semantic coherence
- Enables provenance tracking
- Supports section-aware selection

**Trade-offs**:
- May break at suboptimal boundaries
- Section detection is heuristic-based
- Limited to Markdown-style headers

### Ranking Heuristics

**Problem**: Score chunks for relevance without ML models.

**Solution**: Multi-factor scoring:
- **Salience** (40%): Header emphasis, length preference, keyword density
- **Novelty** (40%): Inverse document frequency of terms
- **Redundancy** (20% penalty): Overlap with other chunks

**Rationale**:
- Deterministic and reproducible
- Captures multiple relevance signals
- Penalizes duplicate content
- No training data required

**Trade-offs**:
- Heuristic-based, not learned
- May not capture domain-specific importance
- Requires tuning for different content types

### Selection Algorithm

**Problem**: Select chunks under budget constraints.

**Solution**: Dual-strategy approach:
- **Greedy** (default): Sort by score, select top-fit
- **Knapsack** (optional): DP optimization for small inputs (< 100 chunks)

**Rationale**:
- Greedy is fast and scalable
- Knapsack is optimal for small inputs
- Automatic strategy selection based on input size
- Respects hard budget constraints

**Trade-offs**:
- Greedy is not globally optimal
- Knapsack has exponential complexity
- No consideration of chunk interactions

### Schema Validation & Repair

**Problem**: Validate JSON against schema and fix common issues.

**Solution**:
- Use `jsonschema` crate for validation
- Attempt repairs for common issues:
  - Insert defaults for missing keys
  - Attempt type conversions
  - Clean malformed strings
  - Remove invalid array entries

**Rationale**:
- Standards-compliant validation
- Automatic repair reduces manual work
- Graceful degradation for irreparable issues
- Clear error reporting

**Trade-offs**:
- Limited to common repair patterns
- May not fix complex issues
- Risk of over-correcting valid data

## Data Structures

### Chunk

```rust
struct Chunk {
    id: String,              // Unique identifier
    content: String,         // Text content
    source: SourceInfo,      // File and offset info
    token_estimate: usize,   // Approximate token count
    line_start: usize,       // Starting line number
    line_end: usize,         // Ending line number
    section_path: Vec<String>, // Document section hierarchy
}
```

**Design choices**:
- String content for simplicity (no Cow)
- Explicit token estimate for budgeting
- Rich metadata for provenance
- Section path for structural awareness

### Budget

```rust
struct Budget {
    max_tokens: usize,       // Hard limit
    used_tokens: usize,      // Current usage
    remaining_tokens: usize, // Available capacity
}
```

**Design choices**:
- Immutable max, mutable usage
- Remaining computed for convenience
- Methods for safe token allocation

### ValidationResult

```rust
struct ValidationResult {
    is_valid: bool,
    errors: Vec<ValidationError>,
    repairs: Vec<RepairAction>,
}
```

**Design choices**:
- Separate errors and repairs
- Detailed error classification
- Action-oriented repair descriptions

## Caching Strategy

### Hash-Based Caching

**Approach**:
- Compute SHA-256 hash of normalized content
- Store chunk metadata keyed by hash
- Cache entries expire after 30 days
- File-based persistence in system cache directory

**Rationale**:
- Content-addressable storage
- Automatic invalidation on content change
- Low overhead (hash computation is fast)
- Cross-session persistence

**Trade-offs**:
- Cache invalidation is all-or-nothing
- No partial caching for large files
- Cache directory may fill over time

## Error Handling

### Strategy

- Use `anyhow` for error propagation
- Use `thiserror` for custom error types (if needed)
- Contextual error messages with `with_context`
- Graceful degradation where possible

### Examples

- File not found: Clear error with path
- Invalid JSON: Parse error with location
- Schema violation: Detailed path and message
- Budget exceeded: Clear indication of overflow

## Testing Strategy

### Unit Tests

- Test each module in isolation
- Cover edge cases and error conditions
- Use `tempfile` for file system tests
- Mock dependencies where appropriate

### Integration Tests

- Test full pipeline end-to-end
- Test CLI commands with `assert_cmd`
- Use example files for realistic testing
- Validate output formats

### Test Coverage Goals

- Core logic: > 80%
- CLI commands: 100%
- Error paths: > 70%

## Performance Considerations

### Memory Management

- Use memory mapping for large files (> 10MB)
- Avoid unnecessary cloning (use references)
- Prefer streaming over loading entire content
- Clear cache entries periodically

### Algorithmic Complexity

- Chunking: O(n) where n is text length
- Ranking: O(n * m) where n is chunks, m is terms
- Greedy selection: O(n log n) for sorting
- Knapsack selection: O(n * budget) - limited to small n

### Scalability Limits

- Maximum file size: Limited by available RAM (with memory mapping)
- Maximum chunks: Practical limit ~10,000 (performance degrades)
- Maximum budget: Limited by usize (effectively unlimited)

## Future Enhancements

### Potential Improvements

1. **Better Token Estimation**: Integrate tiktoken or similar
2. **Advanced Chunking**: Semantic splitting with embeddings
3. **ML-Based Ranking**: Train lightweight models for scoring
4. **Incremental Processing**: Process only changed sections
5. **Parallel Processing**: Multi-threaded chunking and ranking
6. **Plugin System**: Custom ranking and selection strategies

### Extension Points

- Custom scoring functions
- Alternative selection algorithms
- Additional output formats
- Pluggable tokenizers
- Custom cache backends

## Conclusion

The Local Prompt Compiler demonstrates that sophisticated text processing can be achieved with:
- Deterministic algorithms
- Minimal dependencies
- Strong type safety
- Clean architecture
- Practical utility

The design prioritizes reliability, performance, and maintainability over flashy features, making it suitable for production use in AI infrastructure pipelines.
