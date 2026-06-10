use assert_cmd::Command;
use predicates::str::contains;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_pack_basic() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "This is test content. It has multiple sentences.").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["pack", &input_file.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("COMPILED PROMPT PACKAGE"));
}

#[test]
fn test_pack_with_budget() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "Test content for budget test.").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["pack", &input_file.to_string_lossy(), "--budget", "100"])
        .assert()
        .success()
        .stdout(contains("Budget"));
}

#[test]
fn test_pack_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "Test content for JSON format.").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["pack", &input_file.to_string_lossy(), "--format", "json"])
        .assert()
        .success()
        .stdout(contains("\"chunks\""));
}

#[test]
fn test_pack_markdown_format() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "Test content for markdown format.").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["pack", &input_file.to_string_lossy(), "--format", "markdown"])
        .assert()
        .success()
        .stdout(contains("# Compiled Prompt Package"));
}

#[test]
fn test_pack_with_output_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let output_file = temp_dir.path().join("output.json");
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "Test content for output file.").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args([
            "pack", 
            &input_file.to_string_lossy(), 
            "--format", "json",
            "--output", &output_file.to_string_lossy()
        ])
        .assert()
        .success();
    
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"chunks\""));
}

#[test]
fn test_validate_valid_json() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.json");
    let schema_file = temp_dir.path().join("schema.json");
    
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "{{\"name\": \"test\"}}").unwrap();
    
    let mut schema = File::create(&schema_file).unwrap();
    writeln!(schema, "{{\"type\": \"object\", \"properties\": {{\"name\": {{\"type\": \"string\"}}}}, \"required\": [\"name\"]}}").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["validate", &input_file.to_string_lossy(), "--schema", &schema_file.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("Validation passed"));
}

#[test]
fn test_validate_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.json");
    let schema_file = temp_dir.path().join("schema.json");
    
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "{{\"other\": \"test\"}}").unwrap();
    
    let mut schema = File::create(&schema_file).unwrap();
    writeln!(schema, "{{\"type\": \"object\", \"properties\": {{\"name\": {{\"type\": \"string\"}}}}, \"required\": [\"name\"]}}").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["validate", &input_file.to_string_lossy(), "--schema", &schema_file.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("Validation failed"));
}

#[test]
fn test_diff() {
    let temp_dir = TempDir::new().unwrap();
    let old_file = temp_dir.path().join("old.json");
    let new_file = temp_dir.path().join("new.json");
    
    let mut old = File::create(&old_file).unwrap();
    writeln!(old, "{{\"chunks\": [{{\"id\": \"1\", \"content\": \"old\"}}]}}").unwrap();
    
    let mut new = File::create(&new_file).unwrap();
    writeln!(new, "{{\"chunks\": [{{\"id\": \"2\", \"content\": \"new\"}}]}}").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["diff", &old_file.to_string_lossy(), &new_file.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("Diff results"));
}

#[test]
fn test_profile() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "Test content for profiling.").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["profile", &input_file.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("Profile results"))
        .stdout(contains("Total time"));
}

#[test]
fn test_inspect() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "Test content for inspection.").unwrap();
    
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["inspect", &input_file.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("Inspection results"))
        .stdout(contains("Total chunks"));
}

#[test]
fn test_cache_stats() {
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["cache", "stats"])
        .assert()
        .success()
        .stdout(contains("Cache statistics"));
}

#[test]
fn test_cache_clear() {
    Command::cargo_bin("lpc")
        .unwrap()
        .args(["cache", "clear"])
        .assert()
        .success()
        .stdout(contains("Cache cleared"));
}
