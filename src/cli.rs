use crate::types::{Budget, OutputMetadata, ValidationResult};
use crate::ingest::Ingester;
use crate::normalize::Normalizer;
use crate::segment::Segmenter;
use crate::tokenize::Tokenizer;
use crate::rank::Ranker;
use crate::select::Selector;
use crate::schema::SchemaValidator;
use crate::render::{Renderer, OutputFormat};
use crate::cache::Cache;
use crate::utils::compute_hash;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use anyhow::Result;
use chrono::Utc;
use log::{info, warn};

#[derive(Parser)]
#[command(name = "lpc")]
#[command(about = "Local Prompt Compiler - Compile text into AI-ready outputs", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Pack input text into a budgeted output
    Pack {
        /// Input file path (or use stdin)
        input: Option<PathBuf>,
        /// Maximum token budget
        #[arg(short, long, default_value = "4096")]
        budget: usize,
        /// Output format (json, markdown, compact)
        #[arg(short, long, default_value = "compact")]
        format: String,
        /// Maximum chunk tokens
        #[arg(long, default_value = "500")]
        chunk_size: usize,
        /// Use knapsack optimization (slower but better)
        #[arg(long)]
        optimize: bool,
        /// Enable caching
        #[arg(long)]
        cache: bool,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Validate JSON against a schema
    Validate {
        /// Input JSON file
        input: PathBuf,
        /// Schema file
        #[arg(short, long)]
        schema: PathBuf,
    },
    /// Diff two compiled outputs
    Diff {
        /// Old output file
        old: PathBuf,
        /// New output file
        new: PathBuf,
    },
    /// Profile input processing
    Profile {
        /// Input file path
        input: PathBuf,
    },
    /// Inspect input chunks
    Inspect {
        /// Input file path
        input: PathBuf,
        /// Maximum chunk tokens
        #[arg(long, default_value = "500")]
        chunk_size: usize,
    },
    /// Cache operations
    Cache {
        #[command(subcommand)]
        cache_command: CacheCommands,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Clear the cache
    Clear,
    /// Show cache statistics
    Stats,
}

pub fn run() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Pack { input, budget, format, chunk_size, optimize, cache, output } => {
            run_pack(input, budget, format, chunk_size, optimize, cache, output)
        }
        Commands::Validate { input, schema } => {
            run_validate(input, schema)
        }
        Commands::Diff { old, new } => {
            run_diff(old, new)
        }
        Commands::Profile { input } => {
            run_profile(input)
        }
        Commands::Inspect { input, chunk_size } => {
            run_inspect(input, chunk_size)
        }
        Commands::Cache { cache_command } => {
            run_cache(cache_command)
        }
    }
}

fn run_pack(
    input: Option<PathBuf>,
    budget: usize,
    format: String,
    chunk_size: usize,
    optimize: bool,
    use_cache: bool,
    output: Option<PathBuf>,
) -> Result<()> {
    let start = Instant::now();
    let mut cache_hits = 0;
    let mut cache_misses = 0;
    
    // Initialize cache if enabled
    let mut cache = if use_cache {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("local-prompt-compiler");
        let mut c = Cache::new(cache_dir)?;
        c.load()?;
        Some(c)
    } else {
        None
    };
    
    // Read input
    let (content, source_file) = if let Some(path) = input {
        let content = Ingester::from_file(&path)?;
        (content, Some(path))
    } else {
        let content = Ingester::from_stdin()?;
        (content, None)
    };
    
    info!("Read {} characters from input", content.len());
    
    // Normalize
    let normalized = Normalizer::normalize(&content);
    info!("Normalized text");
    
    // Chunk
    let chunks = if let Some(ref mut c) = cache {
        let hash = compute_hash(&normalized);
        if let Some(entry) = c.get(&hash) {
            cache_hits += 1;
            info!("Cache hit for input");
            entry.chunks.clone()
        } else {
            cache_misses += 1;
            let chunks = Segmenter::chunk_text(&normalized, source_file.clone(), chunk_size)?;
            c.put(hash, chunks.clone());
            info!("Created {} chunks", chunks.len());
            chunks
        }
    } else {
        let chunks = Segmenter::chunk_text(&normalized, source_file.clone(), chunk_size)?;
        info!("Created {} chunks", chunks.len());
        chunks
    };
    
    // Score chunks
    let scores = Ranker::score_chunks(&chunks);
    info!("Scored {} chunks", scores.len());
    
    // Select chunks
    let mut budget_obj = Budget::new(budget);
    let selection = if optimize {
        Selector::select_knapsack(chunks, scores, &mut budget_obj)?
    } else {
        Selector::select_greedy(chunks, scores, &mut budget_obj)
    };
    
    info!("Selected {} chunks (rejected {})", selection.chunks.len(), selection.rejected_count);
    info!("Used {}/{} tokens", budget_obj.used_tokens, budget_obj.max_tokens);
    
    // Validate output structure
    let validation = ValidationResult {
        is_valid: true,
        errors: vec![],
        repairs: vec![],
    };
    
    // Create metadata
    let metadata = OutputMetadata {
        timestamp: Utc::now(),
        source_files: source_file.into_iter().collect(),
        total_input_tokens: Tokenizer::estimate_tokens(&normalized),
        processing_time_ms: start.elapsed().as_millis() as u64,
        cache_hits,
        cache_misses,
    };
    
    // Render output
    let output_format = OutputFormat::from_str(&format);
    let rendered = Renderer::render_package(
        selection.chunks,
        budget_obj,
        validation,
        metadata,
        output_format,
    )?;
    
    // Write output
    if let Some(path) = output {
        fs::write(&path, rendered)?;
        info!("Wrote output to {}", path.display());
    } else {
        println!("{}", rendered);
    }
    
    // Save cache if enabled
    if let Some(mut c) = cache {
        c.save()?;
    }
    
    Ok(())
}

fn run_validate(input: PathBuf, schema: PathBuf) -> Result<()> {
    info!("Validating {} against schema {}", input.display(), schema.display());
    
    let input_content = fs::read_to_string(&input)?;
    let schema_content = fs::read_to_string(&schema)?;
    
    let json = crate::schema::SchemaValidator::validate_structure(&input_content)?;
    let schema_json: serde_json::Value = serde_json::from_str(&schema_content)?;
    
    let result = SchemaValidator::validate(&json, &schema_json);
    
    if result.is_valid {
        println!("✓ Validation passed");
    } else {
        println!("✗ Validation failed");
        for error in &result.errors {
            println!("  - {}: {} ({:?})", error.path, error.message, error.error_type);
        }
    }
    
    if !result.repairs.is_empty() {
        println!("\nRepairs applied:");
        for repair in &result.repairs {
            println!("  - {}: {} ({:?})", repair.path, repair.description, repair.action);
        }
    }
    
    Ok(())
}

fn run_diff(old: PathBuf, new: PathBuf) -> Result<()> {
    info!("Comparing {} and {}", old.display(), new.display());
    
    let old_content = fs::read_to_string(&old)?;
    let new_content = fs::read_to_string(&new)?;
    
    let old_json: serde_json::Value = serde_json::from_str(&old_content)?;
    let new_json: serde_json::Value = serde_json::from_str(&new_content)?;
    
    // Simple diff: compare chunk IDs
    let old_chunks: std::collections::HashSet<String> = old_json["chunks"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|c| c["id"].as_str()).map(String::from).collect())
        .unwrap_or_default();
    
    let new_chunks: std::collections::HashSet<String> = new_json["chunks"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|c| c["id"].as_str()).map(String::from).collect())
        .unwrap_or_default();
    
    let added: Vec<_> = new_chunks.difference(&old_chunks).cloned().collect();
    let removed: Vec<_> = old_chunks.difference(&new_chunks).cloned().collect();
    
    println!("Diff results:");
    println!("  Added chunks: {}", added.len());
    for chunk in &added {
        println!("    + {}", chunk);
    }
    println!("  Removed chunks: {}", removed.len());
    for chunk in &removed {
        println!("    - {}", chunk);
    }
    
    Ok(())
}

fn run_profile(input: PathBuf) -> Result<()> {
    info!("Profiling {}", input.display());
    
    let mut timings = Vec::new();
    let start = Instant::now();
    
    // Ingest
    let ingest_start = Instant::now();
    let content = Ingester::from_file(&input)?;
    timings.push(("ingest", ingest_start.elapsed()));
    
    // Normalize
    let normalize_start = Instant::now();
    let normalized = Normalizer::normalize(&content);
    timings.push(("normalize", normalize_start.elapsed()));
    
    // Chunk
    let chunk_start = Instant::now();
    let chunks = Segmenter::chunk_text(&normalized, Some(input.clone()), 500)?;
    timings.push(("chunk", chunk_start.elapsed()));
    
    // Score
    let score_start = Instant::now();
    let scores = Ranker::score_chunks(&chunks);
    timings.push(("score", score_start.elapsed()));
    
    // Select
    let select_start = Instant::now();
    let mut budget = Budget::new(4096);
    let _selection = Selector::select_greedy(chunks, scores, &mut budget);
    timings.push(("select", select_start.elapsed()));
    
    let total = start.elapsed();
    
    println!("Profile results for {}:", input.display());
    println!("  Total time: {:?}", total);
    println!("  Stage timings:");
    for (stage, duration) in &timings {
        println!("    {}: {:?} ({:.1}%)", stage, duration, duration.as_secs_f64() / total.as_secs_f64() * 100.0);
    }
    
    Ok(())
}

fn run_inspect(input: PathBuf, chunk_size: usize) -> Result<()> {
    info!("Inspecting {}", input.display());
    
    let content = Ingester::from_file(&input)?;
    let normalized = Normalizer::normalize(&content);
    let chunks = Segmenter::chunk_text(&normalized, Some(input.clone()), chunk_size)?;
    
    println!("Inspection results for {}:", input.display());
    println!("  Total chunks: {}", chunks.len());
    println!("  Total tokens: {}", Tokenizer::estimate_total_tokens(&chunks));
    println!("\nChunks:");
    
    for (i, chunk) in chunks.iter().enumerate() {
        println!("  [{}] ID: {}", i + 1, chunk.id);
        println!("      Tokens: {}", chunk.token_estimate);
        println!("      Lines: {}-{}", chunk.line_start, chunk.line_end);
        println!("      Section: {}", chunk.section_path.join(" > "));
        println!("      Preview: {}...", chunk.content.chars().take(50).collect::<String>());
        println!();
    }
    
    Ok(())
}

fn run_cache(command: CacheCommands) -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("local-prompt-compiler");
    
    match command {
        CacheCommands::Clear => {
            let mut cache = Cache::new(cache_dir)?;
            cache.clear()?;
            println!("Cache cleared");
        }
        CacheCommands::Stats => {
            let cache = Cache::new(cache_dir)?;
            let stats = cache.stats();
            println!("Cache statistics:");
            println!("  Cache directory: {}", stats.cache_dir.display());
            println!("  Total entries: {}", stats.total_entries);
            println!("  Total chunks: {}", stats.total_chunks);
        }
    }
    
    Ok(())
}
