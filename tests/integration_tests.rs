use local_prompt_compiler::types::{Chunk, SourceInfo, Budget};
use local_prompt_compiler::ingest::Ingester;
use local_prompt_compiler::normalize::Normalizer;
use local_prompt_compiler::segment::Segmenter;
use local_prompt_compiler::rank::Ranker;
use local_prompt_compiler::select::Selector;
use local_prompt_compiler::schema::SchemaValidator;
use local_prompt_compiler::render::Renderer;
use local_prompt_compiler::cache::Cache;
use tempfile::TempDir;

#[test]
fn test_full_pipeline() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    std::fs::write(&input_file, "# Important Section\nThis is important content.\n\n# Other Section\nThis is other content.").unwrap();
    
    // Ingest
    let content = Ingester::from_file(&input_file).unwrap();
    assert!(!content.is_empty());
    
    // Normalize
    let normalized = Normalizer::normalize(&content);
    assert!(!normalized.is_empty());
    
    // Chunk
    let chunks = Segmenter::chunk_text(&normalized, Some(input_file.clone()), 500).unwrap();
    assert!(!chunks.is_empty());
    
    // Score
    let scores = Ranker::score_chunks(&chunks);
    assert_eq!(scores.len(), chunks.len());
    
    // Select
    let mut budget = Budget::new(100);
    let selection = Selector::select_greedy(chunks, scores, &mut budget);
    assert!(!selection.chunks.is_empty());
    assert!(selection.budget.used_tokens <= selection.budget.max_tokens);
}

#[test]
fn test_pipeline_with_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let input_file = temp_dir.path().join("input.txt");
    std::fs::write(&input_file, "Test content for cache.").unwrap();
    
    let mut cache = Cache::new(cache_dir).unwrap();
    
    let content = Ingester::from_file(&input_file).unwrap();
    let normalized = Normalizer::normalize(&content);
    
    // First call - cache miss
    let chunks1 = cache.get_or_compute(&normalized, || {
        Segmenter::chunk_text(&normalized, Some(input_file.clone()), 500)
    }).unwrap();
    
    // Second call - cache hit
    let chunks2 = cache.get_or_compute(&normalized, || {
        Segmenter::chunk_text(&normalized, Some(input_file.clone()), 500)
    }).unwrap();
    
    assert_eq!(chunks1.len(), chunks2.len());
}

#[test]
fn test_schema_validation_pipeline() {
    let json_str = r#"{"name": "test", "value": 42}"#;
    let schema_str = r#"{"type": "object", "properties": {"name": {"type": "string"}, "value": {"type": "number"}}, "required": ["name"]}"#;
    
    let json = serde_json::from_str(json_str).unwrap();
    let schema = serde_json::from_str(schema_str).unwrap();
    
    let result = SchemaValidator::validate(&json, &schema);
    assert!(result.is_valid);
}

#[test]
fn test_rendering_pipeline() {
    let chunks = vec![
        Chunk {
            id: "1".to_string(),
            content: "Test content".to_string(),
            source: SourceInfo { file: None, offset_start: 0, offset_end: 12 },
            token_estimate: 3,
            line_start: 0,
            line_end: 1,
            section_path: vec!["root".to_string()],
        }
    ];
    
    let budget = Budget::new(100);
    let validation = local_prompt_compiler::types::ValidationResult {
        is_valid: true,
        errors: vec![],
        repairs: vec![],
    };
    
    let metadata = local_prompt_compiler::types::OutputMetadata {
        timestamp: chrono::Utc::now(),
        source_files: vec![],
        total_input_tokens: 10,
        processing_time_ms: 100,
        cache_hits: 0,
        cache_misses: 1,
    };
    
    let json_output = Renderer::render_package(
        chunks.clone(),
        budget.clone(),
        validation.clone(),
        metadata.clone(),
        local_prompt_compiler::render::OutputFormat::Json,
    ).unwrap();
    
    assert!(json_output.contains("\"chunks\""));
    
    let md_output = Renderer::render_package(
        chunks.clone(),
        budget.clone(),
        validation.clone(),
        metadata.clone(),
        local_prompt_compiler::render::OutputFormat::Markdown,
    ).unwrap();
    
    assert!(md_output.contains("# Compiled Prompt Package"));
    
    let compact_output = Renderer::render_package(
        chunks,
        budget,
        validation,
        metadata,
        local_prompt_compiler::render::OutputFormat::Compact,
    ).unwrap();
    
    assert!(compact_output.contains("COMPILED PROMPT PACKAGE"));
}
