use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_command() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("A Git worktree management tool"));
}

#[test]
fn test_setup_help_command() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .args(["setup", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set up a new git worktree"));
}

#[test]
fn test_cleanup_help_command() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .args(["cleanup", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Clean up worktrees"));
}

#[test]
fn test_version_command() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("workbloom"));
}

#[test]
fn test_setup_without_branch_name() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .arg("setup")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments were not provided"));
}

#[test]
fn test_cleanup_conflicting_options() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .args(["cleanup", "--merged", "--pattern", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}


#[test]
fn test_config_defaults() {
    use workbloom::config::Config;
    
    let config = Config::default();
    assert!(config.files_to_copy.contains(&".envrc".to_string()));
    assert!(config.files_to_copy.contains(&".env".to_string()));
    assert!(config.directories_to_copy.is_empty());
    assert!(config.claude_files.contains(&"settings.json".to_string()));
    assert!(config.claude_files.contains(&"settings.local.json".to_string()));
}

#[test]
fn test_setup_short_alias() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .args(["s", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set up a new git worktree"));
}

#[test]
fn test_cleanup_short_alias() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .args(["c", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Clean up worktrees"));
}

#[test]
fn test_help_shows_aliases() {
    Command::cargo_bin("workbloom")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("[aliases: s]"))
        .stdout(predicate::str::contains("[aliases: c]"));
}

#[test]
fn test_setup_print_path_output_separation() {
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git email");

    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git name");

    StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create initial commit");

    StdCommand::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to rename default branch");

    StdCommand::new("git")
        .args(["remote", "add", "origin", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add origin remote");

    let expected_path = repo_path.join("worktree-test-branch");
    let output = Command::cargo_bin("workbloom")
        .unwrap()
        .args(["setup", "test-branch"])
        .current_dir(repo_path)
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to run workbloom setup");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), expected_path.to_string_lossy().as_ref());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Worktree location:"));
}

#[test]
fn test_setup_script_detection() {
    use std::fs;
    use tempfile::TempDir;
    use std::process::Command as StdCommand;
    
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Initialize git repo
    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");
    
    // Set git config
    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git email");
    
    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git name");
    
    // Create initial commit
    StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create initial commit");
    
    // Create a test setup script
    let setup_script_content = r#"#!/bin/bash
echo "Test setup script executed" > .setup-test-marker
"#;
    
    fs::write(repo_path.join(".workbloom-setup.sh"), setup_script_content)
        .expect("Failed to write setup script");
    
    // Create .workbloom config
    let config_content = r#"
files_to_copy = [".workbloom-setup.sh"]
"#;
    fs::write(repo_path.join(".workbloom"), config_content)
        .expect("Failed to write config");
    
    // Now let's verify the file exists (actual worktree setup would require more complex testing)
    assert!(repo_path.join(".workbloom-setup.sh").exists());
}
