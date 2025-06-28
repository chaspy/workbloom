use anyhow::Result;
use colored::*;
use std::io::{self, Write};

use crate::git::GitRepo;

pub fn execute(mode: CleanupMode) -> Result<()> {
    let repo = GitRepo::new()?;
    
    match mode {
        CleanupMode::Merged => cleanup_merged_only(&repo),
        CleanupMode::Pattern(pattern) => cleanup_by_pattern(&repo, &pattern),
        CleanupMode::Interactive => interactive_cleanup(&repo),
        CleanupMode::Status => show_status(&repo),
    }
}

pub enum CleanupMode {
    Merged,
    Pattern(String),
    Interactive,
    Status,
}

pub fn cleanup_merged_worktrees(repo: &GitRepo) -> Result<()> {
    println!("{} Cleaning up worktrees for merged branches...", "🧹".yellow());
    
    let merged_branches = repo.get_merged_branches()?;
    
    if merged_branches.is_empty() {
        println!("{} No merged branches found", "✨".green());
        return Ok(());
    }
    
    println!("Found merged branches:");
    for branch in &merged_branches {
        println!("  - {branch}");
    }
    println!();
    
    let worktrees = repo.list_worktrees()?;
    let mut cleaned_count = 0;
    let mut skipped_count = 0;
    
    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            continue;
        }
        
        if let Some(branch) = &worktree.branch {
            if worktree.is_detached {
                println!("{} Skipping detached HEAD worktree: {}", "⚠️".yellow(), worktree.path.display());
                skipped_count += 1;
                continue;
            }
            
            if merged_branches.contains(branch) {
                println!("{} Removing worktree for merged branch: {}", "🗑️".red(), branch);
                println!("    Path: {}", worktree.path.display());
                
                match repo.remove_worktree(&worktree.path, true) {
                    Ok(_) => {
                        cleaned_count += 1;
                        println!("    {} Successfully removed", "✅".green());
                    }
                    Err(_) => {
                        println!("    {} Failed to remove (may be in use)", "❌".red());
                    }
                }
                println!();
            }
        }
    }
    
    println!("{} Summary:", "📊".blue());
    println!("  - Cleaned up: {cleaned_count} worktree(s)");
    println!("  - Skipped: {skipped_count} worktree(s)");
    
    if cleaned_count == 0 {
        println!();
        println!("{} No merged branch worktrees found to clean up", "✨".green());
    } else {
        println!();
        println!("{} Cleanup completed!", "✅".green().bold());
    }
    
    Ok(())
}

fn cleanup_merged_only(repo: &GitRepo) -> Result<()> {
    cleanup_merged_worktrees(repo)
}

fn cleanup_by_pattern(repo: &GitRepo, pattern: &str) -> Result<()> {
    println!("Removing worktrees matching pattern: {}", pattern.cyan());
    println!();
    
    let worktrees = repo.list_worktrees()?;
    let mut removed_count = 0;
    
    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            continue;
        }
        
        if worktree.path.to_string_lossy().contains(pattern) {
            if let Some(branch) = &worktree.branch {
                remove_worktree_with_branch(repo, &worktree.path, branch)?;
                removed_count += 1;
            }
        }
    }
    
    println!("{} Removed {} worktree(s) matching pattern '{}'", "✅".green(), removed_count, pattern);
    Ok(())
}

fn interactive_cleanup(repo: &GitRepo) -> Result<()> {
    println!("Interactive worktree removal");
    println!();
    
    let worktrees = repo.list_worktrees()?;
    
    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            continue;
        }
        
        if let Some(branch) = &worktree.branch {
            println!("Worktree: {}", worktree.path.display());
            println!("Branch: {}", branch.cyan());
            
            print!("Remove this worktree? (y/n) ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            if input.trim().to_lowercase() == "y" {
                remove_worktree_with_branch(repo, &worktree.path, branch)?;
            } else {
                println!("  Skipped");
            }
            println!();
        }
    }
    
    Ok(())
}

fn show_status(repo: &GitRepo) -> Result<()> {
    println!("Checking merge status of all branches...");
    println!();
    
    let worktrees = repo.list_worktrees()?;
    
    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            println!("{} main (current branch)", "📍".blue());
        } else if let Some(branch) = &worktree.branch {
            if repo.is_branch_merged(branch)? {
                println!("{} {} (merged)", "✅".green(), branch);
            } else {
                println!("{} {} (not merged)", "❌".red(), branch);
            }
        }
    }
    
    Ok(())
}

fn remove_worktree_with_branch(repo: &GitRepo, path: &std::path::Path, branch: &str) -> Result<()> {
    println!("  Removing worktree: {}", path.display());
    
    if let Err(e) = repo.remove_worktree(path, true) {
        println!("  {} Failed to remove worktree: {}", "❌".red(), e);
        return Ok(());
    }
    
    println!("  {} Worktree removed successfully", "✅".green());
    
    if repo.branch_exists(branch)? {
        if let Err(e) = repo.delete_branch(branch) {
            println!("  {} Could not delete branch '{}': {}", "⚠️".yellow(), branch, e);
        } else {
            println!("  {} Branch '{}' deleted", "✅".green(), branch);
        }
    }
    
    Ok(())
}