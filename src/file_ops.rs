use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::Config;

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
        crate::outln!("{} Warning: {} not found in main directory", "⚠️".yellow(), item);
        return Ok(());
    }
    
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for {item}"))?;
    }
    
    if source_path.is_dir() {
        copy_dir_all(&source_path, &dest_path)?;
        crate::outln!("{} Copied directory: {}", "📁".green(), item);
    } else {
        fs::copy(&source_path, &dest_path)
            .with_context(|| format!("Failed to copy {item}"))?;
        crate::outln!("{} Copied file: {}", "📄".green(), item);
    }
    
    Ok(())
}

fn copy_claude_settings(main_dir: &Path, worktree_dir: &Path, config: &Config) -> Result<()> {
    let claude_source = main_dir.join(".claude");
    let claude_dest = worktree_dir.join(".claude");
    
    if !claude_source.exists() {
        crate::outln!("{} Warning: .claude directory not found in main directory", "⚠️".yellow());
        return Ok(());
    }
    
    fs::create_dir_all(&claude_dest)
        .context("Failed to create .claude directory")?;
    
    for file in &config.claude_files {
        let source_file = claude_source.join(file);
        if source_file.exists() {
            let dest_file = claude_dest.join(file);
            fs::copy(&source_file, &dest_file)
                .with_context(|| format!("Failed to copy .claude/{file}"))?;
            crate::outln!("{} Copied file: .claude/{}", "📄".green(), file);
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
    
    crate::outln!("{} Setting up direnv...", "🔐".blue());
    
    if which::which("direnv").is_ok() {
        Command::new("direnv")
            .arg("allow")
            .current_dir(worktree_dir)
            .status()
            .context("Failed to run direnv allow")?;
        
        crate::outln!("{} direnv allowed for worktree", "✅".green());
    } else {
        crate::outln!("{} direnv not found. Please run 'direnv allow' manually in the worktree directory", "⚠️".yellow());
    }
    
    Ok(())
}
