use crate::types::{
    Budget, Chunk, OutputMetadata, OutputPackage, ValidationResult,
};
use anyhow::Result;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Compact,
    Json,
    Markdown,
}

impl FromStr for OutputFormat {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "compact" | "prompt" => Ok(OutputFormat::Compact),
            _ => Err(()),
        }
    }
}

pub struct Renderer;

impl Renderer {
    pub fn render_package(
        chunks: Vec<Chunk>,
        budget: Budget,
        validation: ValidationResult,
        metadata: OutputMetadata,
        format: OutputFormat,
    ) -> Result<String> {
        let package = OutputPackage {
            chunks,
            budget,
            validation,
            metadata,
        };

        match format {
            OutputFormat::Json => Self::render_json(&package),
            OutputFormat::Markdown => Self::render_markdown(&package),
            OutputFormat::Compact => Self::render_compact(&package),
        }
    }

    fn render_json(package: &OutputPackage) -> Result<String> {
        Ok(serde_json::to_string_pretty(package)?)
    }

    fn render_markdown(package: &OutputPackage) -> Result<String> {
        let mut output = String::new();

        output.push_str("# Compiled Prompt Package\n\n");

        // Metadata
        output.push_str("## Metadata\n\n");
        output.push_str(&format!(
            "- **Timestamp**: {}\n",
            package.metadata.timestamp
        ));
        output.push_str(&format!(
            "- **Source Files**: {}\n",
            package.metadata.source_files.len()
        ));

        for file in &package.metadata.source_files {
            output.push_str(&format!("  - {}\n", file.display()));
        }

        output.push_str(&format!(
            "- **Total Input Tokens**: {}\n",
            package.metadata.total_input_tokens
        ));

        output.push_str(&format!(
            "- **Processing Time**: {}ms\n",
            package.metadata.processing_time_ms
        ));

        output.push_str(&format!(
            "- **Cache Hits**: {}\n",
            package.metadata.cache_hits
        ));

        output.push_str(&format!(
            "- **Cache Misses**: {}\n\n",
            package.metadata.cache_misses
        ));

        // Budget
        output.push_str("## Budget\n\n");

        output.push_str(&format!(
            "- **Max Tokens**: {}\n",
            package.budget.max_tokens
        ));

        output.push_str(&format!(
            "- **Used Tokens**: {}\n",
            package.budget.used_tokens
        ));

        output.push_str(&format!(
            "- **Remaining Tokens**: {}\n\n",
            package.budget.remaining_tokens
        ));

        // Validation
        output.push_str("## Validation\n\n");

        output.push_str(&format!(
            "- **Valid**: {}\n",
            package.validation.is_valid
        ));

        if !package.validation.errors.is_empty() {
            output.push_str("### Errors\n\n");

            for error in &package.validation.errors {
                output.push_str(&format!(
                    "- **{}**: {} ({:?})\n",
                    error.path,
                    error.message,
                    error.error_type
                ));
            }
        }

        if !package.validation.repairs.is_empty() {
            output.push_str("\n### Repairs\n\n");

            for repair in &package.validation.repairs {
                output.push_str(&format!(
                    "- **{}**: {} ({:?})\n",
                    repair.path,
                    repair.description,
                    repair.action
                ));
            }
        }

        output.push('\n');

        // Chunks
        output.push_str("## Selected Chunks\n\n");

        for (i, chunk) in package.chunks.iter().enumerate() {
            output.push_str(&format!(
                "### Chunk {} (ID: {})\n\n",
                i + 1,
                chunk.id
            ));

            output.push_str(&format!(
                "- **Tokens**: {}\n",
                chunk.token_estimate
            ));

            output.push_str(&format!(
                "- **Lines**: {}-{}\n",
                chunk.line_start,
                chunk.line_end
            ));

            output.push_str(&format!(
                "- **Section**: {}\n",
                chunk.section_path.join(" > ")
            ));

            if let Some(file) = &chunk.source.file {
                output.push_str(&format!(
                    "- **Source**: {}\n",
                    file.display()
                ));
            }

            output.push_str(&format!(
                "- **Offsets**: {}-{}\n\n",
                chunk.source.offset_start,
                chunk.source.offset_end
            ));

            output.push_str("```\n");
            output.push_str(&chunk.content);
            output.push_str("\n```\n\n");
        }

        Ok(output)
    }

    fn render_compact(package: &OutputPackage) -> Result<String> {
        let mut output = String::new();

        // Header with metadata
        output.push_str("=== COMPILED PROMPT PACKAGE ===\n");

        output.push_str(&format!(
            "Timestamp: {}\n",
            package.metadata.timestamp
        ));

        output.push_str(&format!(
            "Budget: {}/{} tokens\n",
            package.budget.used_tokens,
            package.budget.max_tokens
        ));

        output.push_str(&format!(
            "Chunks: {}\n",
            package.chunks.len()
        ));

        output.push_str(&format!(
            "Valid: {}\n",
            package.validation.is_valid
        ));

        output.push_str("===\n\n");

        // Chunks
        for (i, chunk) in package.chunks.iter().enumerate() {
            output.push_str(&format!("[{}:{}] ", i + 1, chunk.id));
            output.push_str(chunk.content.trim_end());
            output.push_str("\n\n");
        }

        // Footer with provenance
        output.push_str("=== PROVENANCE ===\n");

        for chunk in &package.chunks {
            if let Some(file) = &chunk.source.file {
                output.push_str(&format!(
                    "{}: {} (lines {}-{}, {} tokens)\n",
                    chunk.id,
                    file.display(),
                    chunk.line_start,
                    chunk.line_end,
                    chunk.token_estimate
                ));
            }
        }

        Ok(output)
    }

    pub fn render_chunks_only(
        chunks: &[Chunk],
        format: OutputFormat,
    ) -> Result<String> {
        match format {
            OutputFormat::Json => {
                Ok(serde_json::to_string_pretty(chunks)?)
            }

            OutputFormat::Markdown => {
                let mut output = String::new();

                for (i, chunk) in chunks.iter().enumerate() {
                    output.push_str(&format!(
                        "## Chunk {}\n\n",
                        i + 1
                    ));

                    output.push_str(&chunk.content);
                    output.push_str("\n\n");
                }

                Ok(output)
            }

            OutputFormat::Compact => {
                Ok(
                    chunks
                        .iter()
                        .map(|c| c.content.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceInfo;

    #[test]
    fn test_render_compact() {
        let chunks = vec![Chunk {
            id: "1".into(),
            content: "Test content".into(),
            source: SourceInfo {
                file: None,
                offset_start: 0,
                offset_end: 12,
            },
            token_estimate: 3,
            line_start: 0,
            line_end: 1,
            section_path: vec!["root".into()],
        }];

        let metadata = OutputMetadata {
            timestamp: chrono::Utc::now(),
            source_files: vec![],
            total_input_tokens: 10,
            processing_time_ms: 100,
            cache_hits: 0,
            cache_misses: 1,
        };

        let result = Renderer::render_package(
            chunks,
            Budget::new(100),
            ValidationResult {
                is_valid: true,
                errors: vec![],
                repairs: vec![],
            },
            metadata,
            OutputFormat::Compact,
        );

        assert!(result.is_ok());

        let output = result.unwrap();

        assert!(output.contains("COMPILED PROMPT PACKAGE"));
        assert!(output.contains("Test content"));
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(
            "json".parse::<OutputFormat>(),
            Ok(OutputFormat::Json)
        );

        assert_eq!(
            "md".parse::<OutputFormat>(),
            Ok(OutputFormat::Markdown)
        );

        assert_eq!(
            "prompt".parse::<OutputFormat>(),
            Ok(OutputFormat::Compact)
        );
    }
}