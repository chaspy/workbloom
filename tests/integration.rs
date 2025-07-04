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
fn test_port_allocation() {
    use workbloom::port::calculate_ports;
    
    let ports1 = calculate_ports("feature/test-1");
    let ports2 = calculate_ports("feature/test-2");
    let ports3 = calculate_ports("feature/test-1");
    
    assert_ne!(ports1.frontend, ports2.frontend);
    assert_ne!(ports1.backend, ports2.backend);
    assert_ne!(ports1.database, ports2.database);
    
    assert_eq!(ports1.frontend, ports3.frontend);
    assert_eq!(ports1.backend, ports3.backend);
    assert_eq!(ports1.database, ports3.database);
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