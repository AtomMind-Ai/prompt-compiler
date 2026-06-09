# Local Prompt Compiler

A deterministic local inference utility that compiles raw text into compact, budgeted, schema-valid AI-ready outputs. Built in Rust for performance, low memory usage, and single-binary deployment.

## Features

- **Input Handling**: Read `.txt`, `.md`, `.json` files or stdin
- **Text Normalization**: Unicode cleanup, whitespace normalization, sentence/section splitting
- **Token Budgeting**: Deterministic token estimation with user-specified budgets
- **Intelligent Chunking**: Semantically meaningful chunks with rich metadata
- **Ranking & Selection**: Deterministic heuristics for salience, novelty, and redundancy
- **Schema Validation**: JSON schema validation with automatic repair
- **Multiple Output Formats**: Compact, JSON, and Markdown rendering
- **Incremental Caching**: Hash-based caching for unchanged content
- **Observability**: Detailed logging, timing, and profiling

## Installation

### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/example/local-prompt-compiler.git
cd local-prompt-compiler

# Build the project
cargo build --release

# The binary will be available at target/release/lpc
```

### Install via Cargo (when published)

```bash
cargo install local-prompt-compiler
```

## Usage

### Basic Commands

#### Pack Text into Budgeted Output

```bash
# Pack a text file with default settings
lpc pack input.txt

# Specify token budget
lpc pack input.txt --budget 2048

# Output as JSON
lpc pack input.txt --format json --output output.json

# Output as Markdown
lpc pack input.txt --format markdown --output output.md

# Use knapsack optimization (slower but better selection)
lpc pack input.txt --optimize

# Enable caching
lpc pack input.txt --cache

# Read from stdin
echo "Your text here" | lpc pack
```

#### Validate JSON Against Schema

```bash
lpc validate data.json --schema schema.json
```

#### Diff Two Compiled Outputs

```bash
lpc diff old_output.json new_output.json
```

#### Profile Input Processing

```bash
lpc profile input.txt
```

#### Inspect Input Chunks

```bash
lpc inspect input.txt

# Specify chunk size
lpc inspect input.txt --chunk-size 300
```

#### Cache Management

```bash
# Show cache statistics
lpc cache stats

# Clear cache
lpc cache clear
```

### CLI Options

```
lpc 0.1.0
Local Prompt Compiler - Compile text into AI-ready outputs

USAGE:
    lpc [COMMAND]

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

COMMANDS:
    pack       Pack input text into a budgeted output
    validate   Validate JSON against a schema
    diff       Diff two compiled outputs
    profile    Profile input processing
    inspect    Inspect input chunks
    cache      Cache operations
```

## Architecture

The project follows a modular architecture with clear separation of concerns:

```
src/
├── main.rs          # Entry point
├── cli.rs           # Command-line interface
├── types.rs         # Core data structures
├── ingest.rs        # Input handling (files, stdin)
├── normalize.rs     # Text normalization
├── tokenize.rs      # Token estimation
├── segment.rs       # Chunking logic
├── rank.rs          # Ranking heuristics
├── select.rs        # Selection algorithms
├── schema.rs        # Schema validation & repair
├── render.rs        # Output rendering
├── cache.rs         # Caching system
└── utils.rs         # Utility functions
```

### Pipeline Stages

1. **Ingestion**: Read input from files or stdin with memory mapping for large files
2. **Normalization**: Unicode normalization, line ending cleanup, whitespace normalization
3. **Chunking**: Split text into semantically meaningful chunks with metadata
4. **Ranking**: Score chunks using deterministic heuristics (salience, novelty, redundancy)
5. **Selection**: Select chunks under budget using greedy or knapsack algorithms
6. **Validation**: Validate output structure and attempt repairs
7. **Rendering**: Format output as compact, JSON, or Markdown

## Examples

The `examples/` directory contains sample inputs:

- `sample_text.txt` - Sample text document
- `sample_markdown.md` - Sample Markdown document
- `sample_schema.json` - JSON schema for validation
- `sample_data.json` - Valid JSON data
- `invalid_data.json` - Invalid JSON data for testing repair

### Example Workflow

```bash
# Compile a document with a 2048 token budget
lpc pack examples/sample_text.txt --budget 2048 --format json --output compiled.json

# Validate JSON data
lpc validate examples/sample_data.json --schema examples/sample_schema.json

# Profile the processing
lpc profile examples/sample_markdown.md

# Inspect chunks
lpc inspect examples/sample_text.txt --chunk-size 300
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_pack_basic

# Run integration tests only
cargo test --test integration_tests
```

## Performance Characteristics

- **Memory**: Low memory footprint with streaming for large files
- **CPU**: CPU-only, no GPU requirements
- **Speed**: Greedy selection for speed, optional knapsack for optimization
- **Scalability**: Handles moderately large inputs without quadratic blowups

### Benchmarks

Typical performance on a modern laptop:

- Small text (< 100KB): < 10ms
- Medium text (100KB - 1MB): < 100ms
- Large text (1MB - 10MB): < 1s

## Design Decisions

### Why Rust?

- **Single Binary**: Easy deployment without runtime dependencies
- **Memory Safety**: No memory leaks or buffer overflows
- **Performance**: Zero-cost abstractions and efficient memory usage
- **Strong Typing**: Compile-time guarantees for data structures
- **Systems Credibility**: Demonstrates serious engineering capability

### Deterministic Processing

All processing is deterministic with no random scoring or learned model dependencies. This ensures reproducible outputs and predictable behavior.

### Token Estimation

Uses a character-based heuristic (~4 characters per token) as a fallback when no tokenizer library is available. This provides reasonable estimates without external dependencies.

### Caching Strategy

Hash-based caching stores chunk metadata for unchanged content, enabling fast reprocessing of large documents with minor modifications.

## Limitations

- Token estimation is approximate (character/word-based heuristic)
- Schema repair is limited to common issues (missing keys, type mismatches)
- Knapsack optimization is only used for small inputs (< 100 chunks)
- No support for binary file formats
- No network or external service integration

## Contributing

This is a solo project designed as a technical portfolio piece. Contributions are not currently accepted.

## License

MIT License - see LICENSE file for details

## Author

Built as a demonstration of AI infrastructure and local inference tooling capabilities.
