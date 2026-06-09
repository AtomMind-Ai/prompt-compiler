use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chunk {
    pub id: String,
    pub content: String,
    pub source: SourceInfo,
    pub token_estimate: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceInfo {
    pub file: Option<PathBuf>,
    pub offset_start: usize,
    pub offset_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkScore {
    pub chunk_id: String,
    pub salience: f64,
    pub novelty: f64,
    pub redundancy: f64,
    pub combined: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_tokens: usize,
    pub used_tokens: usize,
    pub remaining_tokens: usize,
}

impl Budget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            used_tokens: 0,
            remaining_tokens: max_tokens,
        }
    }

    pub fn can_fit(&self, tokens: usize) -> bool {
        self.remaining_tokens >= tokens
    }

    pub fn use_tokens(&mut self, tokens: usize) -> bool {
        if self.can_fit(tokens) {
            self.used_tokens += tokens;
            self.remaining_tokens -= tokens;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionResult {
    pub chunks: Vec<Chunk>,
    pub budget: Budget,
    pub scores: Vec<ChunkScore>,
    pub total_input_chunks: usize,
    pub rejected_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub repairs: Vec<RepairAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
    pub error_type: ErrorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    MissingKey,
    WrongType,
    InvalidFormat,
    TruncatedArray,
    MalformedString,
    SchemaViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairAction {
    pub path: String,
    pub action: RepairType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepairType {
    InsertedDefault,
    ConvertedType,
    TruncatedString,
    FixedArray,
    RemovedInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPackage {
    pub chunks: Vec<Chunk>,
    pub budget: Budget,
    pub validation: ValidationResult,
    pub metadata: OutputMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub timestamp: DateTime<Utc>,
    pub source_files: Vec<PathBuf>,
    pub total_input_tokens: usize,
    pub processing_time_ms: u64,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub content_hash: String,
    pub chunks: Vec<Chunk>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResult {
    pub stage_times: Vec<StageTiming>,
    pub total_time_ms: u64,
    pub memory_peak_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTiming {
    pub stage: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub added_chunks: Vec<Chunk>,
    pub removed_chunks: Vec<Chunk>,
    pub modified_chunks: Vec<(Chunk, Chunk)>,
}
