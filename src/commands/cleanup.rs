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
    cleanup_merged_worktrees_with_exclude(repo, None)
}

pub fn cleanup_merged_worktrees_with_exclude(repo: &GitRepo, exclude_branch: Option<&str>) -> Result<()> {
    println!("{} Cleaning up worktrees for merged branches...", "🧹".yellow());
    
    let merged_branches = get_filtered_merged_branches(repo, exclude_branch)?;
    
    if merged_branches.is_empty() {
        println!("{} No merged branches found", "✨".green());
        return Ok(());
    }
    
    display_merged_branches(&merged_branches, exclude_branch);
    
    let (cleaned_count, skipped_count) = process_worktrees(repo, &merged_branches)?;
    
    display_cleanup_summary(cleaned_count, skipped_count);
    
    Ok(())
}

fn get_filtered_merged_branches(repo: &GitRepo, exclude_branch: Option<&str>) -> Result<Vec<String>> {
    let mut merged_branches = repo.get_merged_branches()?;
    
    if let Some(exclude) = exclude_branch {
        merged_branches.retain(|branch| branch != exclude);
    }
    
    // Filter out branches that have no unique commits (newly created branches)
    // These appear as "merged" but are actually just new branches without any work
    merged_branches.retain(|branch| {
        repo.has_unmerged_commits(branch).unwrap_or(true)
    });
    
    Ok(merged_branches)
}

fn display_merged_branches(merged_branches: &[String], exclude_branch: Option<&str>) {
    println!("Found merged branches:");
    for branch in merged_branches {
        println!("  - {branch}");
    }
    if let Some(exclude) = exclude_branch {
        println!("  (excluding: {})", exclude.cyan());
    }
    println!();
}

fn process_worktrees(repo: &GitRepo, merged_branches: &[String]) -> Result<(usize, usize)> {
    let worktrees = repo.list_worktrees()?;
    let mut cleaned_count = 0;
    let mut skipped_count = 0;
    
    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            continue;
        }
        
        if let Some(branch) = &worktree.branch {
            match process_single_worktree(repo, worktree, branch, merged_branches) {
                WorktreeAction::Removed => cleaned_count += 1,
                WorktreeAction::Skipped => skipped_count += 1,
                WorktreeAction::Ignored => {}
            }
        }
    }
    
    Ok((cleaned_count, skipped_count))
}

enum WorktreeAction {
    Removed,
    Skipped,
    Ignored,
}

fn process_single_worktree(
    repo: &GitRepo,
    worktree: &crate::git::WorktreeInfo,
    branch: &str,
    merged_branches: &[String],
) -> WorktreeAction {
    if worktree.is_detached {
        println!("{} Skipping detached HEAD worktree: {}", "⚠️".yellow(), worktree.path.display());
        return WorktreeAction::Skipped;
    }
    
    if !merged_branches.contains(&branch.to_string()) {
        return WorktreeAction::Ignored;
    }
    
    // Check if branch has unmerged commits
    match repo.has_unmerged_commits(branch) {
        Ok(true) => {
            println!("{} Skipping branch with unmerged commits: {}", "⚠️".yellow(), branch);
            WorktreeAction::Skipped
        }
        Err(e) => {
            println!("{} Could not check unmerged commits for {}: {}", "⚠️".yellow(), branch, e);
            WorktreeAction::Skipped
        }
        Ok(false) => {
            remove_worktree_and_report(repo, worktree, branch)
        }
    }
}

fn remove_worktree_and_report(
    repo: &GitRepo,
    worktree: &crate::git::WorktreeInfo,
    branch: &str,
) -> WorktreeAction {
    println!("{} Removing worktree for merged branch: {}", "🗑️".red(), branch);
    println!("    Path: {}", worktree.path.display());
    
    match repo.remove_worktree(&worktree.path, true) {
        Ok(_) => {
            println!("    {} Successfully removed", "✅".green());
            WorktreeAction::Removed
        }
        Err(e) => {
            println!("    {} Failed to remove: {}", "❌".red(), e);
            WorktreeAction::Skipped
        }
    }
}

fn display_cleanup_summary(cleaned_count: usize, skipped_count: usize) {
    println!();
    println!("{} Summary:", "📊".blue());
    println!("  - Cleaned up: {cleaned_count} worktree(s)");
    println!("  - Skipped: {skipped_count} worktree(s)");
    
    if cleaned_count == 0 && skipped_count == 0 {
        println!();
        println!("{} No merged branch worktrees found to clean up", "✨".green());
    } else {
        println!();
        println!("{} Cleanup completed!", "✅".green().bold());
    }
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