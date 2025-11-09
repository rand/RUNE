//! Integration tests for RUNE CLI commands

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;
use std::io::Write;

/// Test the version command
#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rune"));
}

/// Test the help command
#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("High-performance authorization"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("eval"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("benchmark"))
        .stdout(predicate::str::contains("serve"));
}

/// Test eval command with default parameters
#[test]
fn test_eval_basic() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--action").arg("read")
        .arg("--resource").arg("/tmp/file.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Evaluating request"))
        .stdout(predicate::str::contains("Decision"));
}

/// Test eval command with principal
#[test]
fn test_eval_with_principal() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--action").arg("write")
        .arg("--principal").arg("user:alice")
        .arg("--resource").arg("/data/secret.txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Evaluating request"));
}

/// Test eval command with JSON format
#[test]
fn test_eval_json_format() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--action").arg("delete")
        .arg("--resource").arg("/tmp/file.txt")
        .arg("--format").arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("decision"));
}

/// Test eval command with verbose flag
#[test]
fn test_eval_verbose() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("--verbose")
        .arg("eval")
        .arg("--action").arg("read")
        .arg("--resource").arg("/tmp/file.txt")
        .assert()
        .success();
}

/// Test eval command with missing required arguments
#[test]
fn test_eval_missing_action() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--resource").arg("/tmp/file.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

/// Test eval command with missing resource
#[test]
fn test_eval_missing_resource() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--action").arg("read")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

/// Test validate command with valid config
#[test]
fn test_validate_valid_config() {
    // Create a valid config file
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, r#"version = "rune/1.0"

[data]
debug = true

[rules]
user(alice).

[policies]
permit (
    principal == User::"alice",
    action == Action::"read",
    resource
);
"#).unwrap();
    temp_file.flush().unwrap();

    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .arg(temp_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Valid"));
}

/// Test validate command with invalid config
#[test]
fn test_validate_invalid_config() {
    // Create an invalid config file
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "invalid syntax [[[").unwrap();
    temp_file.flush().unwrap();

    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .arg(temp_file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("Invalid").or(predicate::str::contains("Error")));
}

/// Test validate command with missing file
#[test]
fn test_validate_missing_file() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .arg("/nonexistent/file.rune")
        .assert()
        .failure();
}

/// Test validate command without file argument
#[test]
fn test_validate_no_file() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

/// Test benchmark command with default parameters
#[test]
fn test_benchmark_default() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("benchmark")
        .assert()
        .success()
        .stdout(predicate::str::contains("requests"))
        .stdout(predicate::str::contains("threads"));
}

/// Test benchmark command with custom parameters
#[test]
fn test_benchmark_custom() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("benchmark")
        .arg("--requests").arg("100")
        .arg("--threads").arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("100"))
        .stdout(predicate::str::contains("2"));
}

/// Test benchmark command with verbose flag
#[test]
fn test_benchmark_verbose() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("--verbose")
        .arg("benchmark")
        .arg("--requests").arg("10")
        .assert()
        .success();
}

/// Test benchmark command with invalid requests value
#[test]
fn test_benchmark_invalid_requests() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("benchmark")
        .arg("--requests").arg("not_a_number")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

/// Test benchmark command with zero threads
#[test]
fn test_benchmark_zero_threads() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("benchmark")
        .arg("--threads").arg("0")
        .assert()
        .failure();
}

/// Test serve command help
#[test]
fn test_serve_help() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("serve")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Start RUNE server"))
        .stdout(predicate::str::contains("port"));
}

/// Test subcommand help
#[test]
fn test_eval_help() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Evaluate an authorization request"))
        .stdout(predicate::str::contains("action"))
        .stdout(predicate::str::contains("principal"))
        .stdout(predicate::str::contains("resource"));
}

/// Test validate help
#[test]
fn test_validate_help() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Validate a RUNE configuration file"));
}

/// Test benchmark help
#[test]
fn test_benchmark_help() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("benchmark")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Run benchmark tests"))
        .stdout(predicate::str::contains("requests"))
        .stdout(predicate::str::contains("threads"));
}

/// Test unknown command
#[test]
fn test_unknown_command() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("unknown")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized"));
}

/// Test CLI without any arguments
#[test]
fn test_cli_no_args() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

/// Test eval with different action types
#[test]
fn test_eval_different_actions() {
    let actions = vec!["read", "write", "delete", "execute", "list"];

    for action in actions {
        let mut cmd = Command::cargo_bin("rune").unwrap();
        cmd.arg("eval")
            .arg("--action").arg(action)
            .arg("--resource").arg("/test/resource")
            .assert()
            .success()
            .stdout(predicate::str::contains("Evaluating request"));
    }
}

/// Test eval with different resource formats
#[test]
fn test_eval_different_resources() {
    let resources = vec![
        "/file/path.txt",
        "database:users",
        "api://endpoint",
        "s3://bucket/key",
    ];

    for resource in resources {
        let mut cmd = Command::cargo_bin("rune").unwrap();
        cmd.arg("eval")
            .arg("--action").arg("read")
            .arg("--resource").arg(resource)
            .assert()
            .success();
    }
}

/// Test eval with different principal formats
#[test]
fn test_eval_different_principals() {
    let principals = vec![
        "user:alice",
        "agent-1",
        "service:api",
        "admin:root",
    ];

    for principal in principals {
        let mut cmd = Command::cargo_bin("rune").unwrap();
        cmd.arg("eval")
            .arg("--action").arg("read")
            .arg("--principal").arg(principal)
            .arg("--resource").arg("/test")
            .assert()
            .success();
    }
}

/// Test validate with config containing only version
#[test]
fn test_validate_minimal_config() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, r#"version = "rune/1.0""#).unwrap();
    temp_file.flush().unwrap();

    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .arg(temp_file.path())
        .assert()
        .success();
}

/// Test validate with config containing only rules
#[test]
fn test_validate_config_with_rules() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, r#"version = "rune/1.0"

[rules]
user(alice).
admin(alice).
can_access(U) :- user(U), admin(U).
"#).unwrap();
    temp_file.flush().unwrap();

    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .arg(temp_file.path())
        .assert()
        .success();
}

/// Test validate with config containing only policies
#[test]
fn test_validate_config_with_policies() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, r#"version = "rune/1.0"

[policies]
permit (
    principal == User::"alice",
    action == Action::"read",
    resource
);
"#).unwrap();
    temp_file.flush().unwrap();

    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("validate")
        .arg(temp_file.path())
        .assert()
        .success();
}

/// Test benchmark with very small number of requests
#[test]
fn test_benchmark_small() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("benchmark")
        .arg("--requests").arg("1")
        .arg("--threads").arg("1")
        .assert()
        .success();
}

/// Test eval command text format explicitly
#[test]
fn test_eval_text_format() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--action").arg("read")
        .arg("--resource").arg("/tmp/file.txt")
        .arg("--format").arg("text")
        .assert()
        .success()
        .stdout(predicate::str::contains("Decision"));
}

/// Test eval with invalid format
#[test]
fn test_eval_invalid_format() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("eval")
        .arg("--action").arg("read")
        .arg("--resource").arg("/tmp/file.txt")
        .arg("--format").arg("xml")
        .assert()
        .success(); // Still succeeds but might use default format
}

/// Test combinations of global and command flags
#[test]
fn test_global_and_command_flags() {
    let mut cmd = Command::cargo_bin("rune").unwrap();
    cmd.arg("--verbose")
        .arg("eval")
        .arg("--action").arg("read")
        .arg("--principal").arg("user:test")
        .arg("--resource").arg("/file")
        .arg("--format").arg("json")
        .assert()
        .success();
}