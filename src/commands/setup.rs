use anyhow::{Context, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::process::Command;
use std::time::Duration;

use crate::{config::Config, file_ops, git::GitRepo};

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
    
    let pb = ProgressBar::new(4);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    
    pb.set_message("Checking branch...");
    if !repo.branch_exists(branch_name)? {
        // Check if branch exists on remote
        if repo.remote_branch_exists(branch_name)? {
            println!("{} Branch '{}' exists on remote. Fetching and creating tracking branch...", "🌐".blue(), branch_name);
            repo.fetch_remote_branch(branch_name)?;
            repo.create_tracking_branch(branch_name)?;
        } else {
            println!("{} Branch '{}' does not exist. Creating it...", "📝".yellow(), branch_name);
            repo.create_branch(branch_name)?;
        }
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
    
    pb.set_message("Running setup script...");
    run_setup_script(&worktree_path)?;
    
    pb.set_message("Setting up direnv...");
    file_ops::setup_direnv(&worktree_path)?;
    pb.inc(1);
    
    pb.finish_with_message("Setup completed!");
    
    println!();
    println!("{} Git worktree setup completed!", "✅".green().bold());
    println!("{} Worktree location: {}", "📍".blue(), worktree_path.display());
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

fn run_setup_script(worktree_path: &std::path::Path) -> Result<()> {
    let setup_script_path = worktree_path.join(".workbloom-setup.sh");
    
    if setup_script_path.exists() {
        println!("{} Found .workbloom-setup.sh, executing...", "🚀".cyan());
        
        // Make the script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&setup_script_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&setup_script_path, perms)?;
        }
        
        // Execute the script
        let output = Command::new("bash")
            .arg(&setup_script_path)
            .current_dir(worktree_path)
            .output()
            .context("Failed to execute .workbloom-setup.sh")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("{} Warning: .workbloom-setup.sh failed: {}", "⚠️".yellow(), stderr);
            // Don't fail the entire setup if the script fails
        } else {
            println!("{} Setup script executed successfully", "✨".green());
        }
    }
    
    Ok(())
}

fn run_cleanup_if_exists(repo: &GitRepo, exclude_branch: Option<&str>) -> Result<()> {
    println!("{} Checking for merged branch worktrees to clean up...", "🧹".yellow());
    
    // 常に新しい実装を使用（スクリプトは無視）
    crate::commands::cleanup::cleanup_merged_worktrees_with_exclude(repo, exclude_branch)?;
    
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