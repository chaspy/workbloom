use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::config::Config;
use crate::port::PortAllocation;

pub fn copy_required_files(main_dir: &Path, worktree_dir: &Path, config: &Config) -> Result<()> {
    for file in &config.files_to_copy {
        copy_item(main_dir, worktree_dir, file)?;
    }
    
    for dir in &config.directories_to_copy {
        copy_item(main_dir, worktree_dir, dir)?;
    }
    
    copy_claude_settings(main_dir, worktree_dir, config)?;
    
    Ok(())
}

fn copy_item(main_dir: &Path, worktree_dir: &Path, item: &str) -> Result<()> {
    let source_path = main_dir.join(item);
    let dest_path = worktree_dir.join(item);
    
    if !source_path.exists() {
        println!("{} Warning: {} not found in main directory", "⚠️".yellow(), item);
        return Ok(());
    }
    
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for {}", item))?;
    }
    
    if source_path.is_dir() {
        copy_dir_all(&source_path, &dest_path)?;
        println!("{} Copied directory: {}", "📁".green(), item);
    } else {
        fs::copy(&source_path, &dest_path)
            .with_context(|| format!("Failed to copy {}", item))?;
        println!("{} Copied file: {}", "📄".green(), item);
    }
    
    Ok(())
}

fn copy_claude_settings(main_dir: &Path, worktree_dir: &Path, config: &Config) -> Result<()> {
    let claude_source = main_dir.join(".claude");
    let claude_dest = worktree_dir.join(".claude");
    
    if !claude_source.exists() {
        println!("{} Warning: .claude directory not found in main directory", "⚠️".yellow());
        return Ok(());
    }
    
    fs::create_dir_all(&claude_dest)
        .context("Failed to create .claude directory")?;
    
    for file in &config.claude_files {
        let source_file = claude_source.join(file);
        if source_file.exists() {
            let dest_file = claude_dest.join(file);
            fs::copy(&source_file, &dest_file)
                .with_context(|| format!("Failed to copy .claude/{}", file))?;
            println!("{} Copied file: .claude/{}", "📄".green(), file);
        }
    }
    
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn setup_direnv(worktree_dir: &Path) -> Result<()> {
    let envrc_path = worktree_dir.join(".envrc");
    if !envrc_path.exists() {
        return Ok(());
    }
    
    println!("{} Setting up direnv...", "🔐".blue());
    
    if which::which("direnv").is_ok() {
        Command::new("direnv")
            .arg("allow")
            .current_dir(worktree_dir)
            .status()
            .context("Failed to run direnv allow")?;
        
        println!("{} direnv allowed for worktree", "✅".green());
    } else {
        println!("{} direnv not found. Please run 'direnv allow' manually in the worktree directory", "⚠️".yellow());
    }
    
    Ok(())
}

pub fn update_env_with_ports(worktree_dir: &Path, ports: &PortAllocation) -> Result<()> {
    let env_path = worktree_dir.join(".env");
    
    // Read existing content if file exists
    let existing_content = if env_path.exists() {
        fs::read_to_string(&env_path).context("Failed to read existing .env file")?
    } else {
        String::new()
    };
    
    // Parse existing lines and check for port variables
    let mut lines: Vec<String> = Vec::new();
    let mut has_frontend_port = false;
    let mut has_backend_port = false;
    let mut has_database_port = false;
    
    for line in existing_content.lines() {
        if line.trim_start().starts_with("FRONTEND_PORT=") {
            has_frontend_port = true;
            lines.push(format!("FRONTEND_PORT={}", ports.frontend));
        } else if line.trim_start().starts_with("BACKEND_PORT=") {
            has_backend_port = true;
            lines.push(format!("BACKEND_PORT={}", ports.backend));
        } else if line.trim_start().starts_with("DATABASE_PORT=") {
            has_database_port = true;
            lines.push(format!("DATABASE_PORT={}", ports.database));
        } else {
            lines.push(line.to_string());
        }
    }
    
    // Add missing port variables
    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        lines.push(String::new()); // Add empty line for separation
    }
    
    if !has_frontend_port {
        lines.push(format!("FRONTEND_PORT={}", ports.frontend));
    }
    if !has_backend_port {
        lines.push(format!("BACKEND_PORT={}", ports.backend));
    }
    if !has_database_port {
        lines.push(format!("DATABASE_PORT={}", ports.database));
    }
    
    // Write back to file
    let mut file = fs::File::create(&env_path)
        .context("Failed to create/update .env file")?;
    
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            writeln!(file)?;
        }
        write!(file, "{}", line)?;
    }
    
    // Ensure file ends with newline
    if !lines.is_empty() {
        writeln!(file)?;
    }
    
    println!("{} Updated .env with port allocations", "📝".green());
    
    Ok(())
}