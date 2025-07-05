use anyhow::{Context, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::process::Command;
use std::time::Duration;

use crate::{config::Config, file_ops, git::GitRepo, port};

pub fn execute(branch_name: &str, start_shell: bool) -> Result<()> {
    let repo = GitRepo::new()?;
    let config = Config::load_from_file(&repo.root_dir)
        .unwrap_or_else(|_| Config::default());
    
    let worktree_dir_name = format!("worktree-{}", branch_name.replace('/', "-"));
    let worktree_path = repo.root_dir.join(&worktree_dir_name);
    
    println!("{} Setting up git worktree...", "🌲".green());
    println!("Branch: {}", branch_name.cyan());
    println!("Worktree directory: {}", worktree_path.display());
    println!();
    
    run_cleanup_if_exists(&repo, Some(branch_name))?;
    
    let pb = ProgressBar::new(6);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    
    pb.set_message("Checking branch...");
    if !repo.branch_exists(branch_name)? {
        println!("{} Branch '{}' does not exist. Creating it...", "📝".yellow(), branch_name);
        repo.create_branch(branch_name)?;
    }
    pb.inc(1);
    
    pb.set_message("Creating worktree...");
    println!("{} Creating git worktree...", "🔧".blue());
    repo.add_worktree(&worktree_path, branch_name)?;
    pb.inc(1);
    
    pb.set_message("Copying files...");
    println!("{} Copying required files...", "📦".blue());
    file_ops::copy_required_files(&repo.root_dir, &worktree_path, &config)?;
    pb.inc(1);
    
    pb.set_message("Setting up direnv...");
    file_ops::setup_direnv(&worktree_path)?;
    pb.inc(1);
    
    pb.set_message("Calculating ports...");
    let ports = port::calculate_ports(branch_name);
    pb.inc(1);
    
    pb.set_message("Updating .env with ports...");
    file_ops::update_env_with_ports(&worktree_path, &ports)?;
    
    pb.finish_with_message("Setup completed!");
    
    println!();
    println!("{} Git worktree setup completed!", "✅".green().bold());
    println!("{} Worktree location: {}", "📍".blue(), worktree_path.display());
    println!();
    
    println!("{} Port allocation for this worktree:", "🌐".blue());
    println!("   Frontend: http://localhost:{}", ports.frontend.to_string().cyan());
    println!("   Backend:  http://localhost:{}", ports.backend.to_string().cyan());
    println!("   Database: localhost:{}", ports.database.to_string().cyan());
    println!();
    
    if start_shell {
        println!("{} Starting new shell in worktree directory...", "📂".blue());
        start_shell_in_directory(&worktree_path)?;
    } else {
        println!("{} Moving to worktree directory...", "📂".blue());
        println!("cd {}", worktree_path.display());
        println!();
        println!("💡 Tip: Default behavior now starts a shell. Use 'workbloom setup {branch_name} --no-shell' to skip");
    }
    
    Ok(())
}

fn run_cleanup_if_exists(repo: &GitRepo, exclude_branch: Option<&str>) -> Result<()> {
    println!("{} Checking for merged branch worktrees to clean up...", "🧹".yellow());
    
    let script_path = repo.root_dir.join("scripts/cleanup-merged-worktrees.sh");
    if script_path.exists() {
        std::process::Command::new("bash")
            .arg(script_path)
            .current_dir(&repo.root_dir)
            .status()
            .context("Failed to run cleanup script")?;
    } else {
        crate::commands::cleanup::cleanup_merged_worktrees_with_exclude(repo, exclude_branch)?;
    }
    
    println!();
    Ok(())
}

fn start_shell_in_directory(worktree_path: &std::path::Path) -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    
    Command::new(&shell)
        .current_dir(worktree_path)
        .status()
        .context("Failed to start shell in worktree directory")?;
    
    Ok(())
}