use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::time::{Duration, SystemTime};

use crate::git::GitRepo;

pub fn execute(mode: CleanupMode) -> Result<()> {
    let repo = GitRepo::new()?;

    match mode {
        CleanupMode::Merged { force } => cleanup_merged_only(&repo, force),
        CleanupMode::Pattern(pattern) => cleanup_by_pattern(&repo, &pattern),
        CleanupMode::Interactive => interactive_cleanup(&repo),
        CleanupMode::Status => show_status(&repo),
    }
}

pub enum CleanupMode {
    Merged { force: bool },
    Pattern(String),
    Interactive,
    Status,
}

pub fn cleanup_merged_worktrees(repo: &GitRepo) -> Result<()> {
    cleanup_merged_worktrees_with_exclude(repo, None)
}

pub fn cleanup_merged_worktrees_with_force(
    repo: &GitRepo,
    exclude_branch: Option<&str>,
    force: bool,
) -> Result<()> {
    println!(
        "{} Cleaning up worktrees for merged branches...",
        "🧹".yellow()
    );

    let merged_branches = get_filtered_merged_branches(repo, exclude_branch, force)?;

    if merged_branches.is_empty() {
        println!("{} No merged branches found", "✨".green());
        return Ok(());
    }

    display_merged_branches(&merged_branches, exclude_branch);

    let (cleaned_count, skipped_count) = process_worktrees(repo, &merged_branches)?;

    display_cleanup_summary(cleaned_count, skipped_count);

    Ok(())
}

pub fn cleanup_merged_worktrees_with_exclude(
    repo: &GitRepo,
    exclude_branch: Option<&str>,
) -> Result<()> {
    println!(
        "{} Cleaning up worktrees for merged branches...",
        "🧹".yellow()
    );

    let merged_branches = get_filtered_merged_branches(repo, exclude_branch, false)?;

    if merged_branches.is_empty() {
        println!("{} No merged branches found", "✨".green());
        return Ok(());
    }

    display_merged_branches(&merged_branches, exclude_branch);

    let (cleaned_count, skipped_count) = process_worktrees(repo, &merged_branches)?;

    display_cleanup_summary(cleaned_count, skipped_count);

    Ok(())
}

fn get_filtered_merged_branches(
    repo: &GitRepo,
    exclude_branch: Option<&str>,
    force: bool,
) -> Result<Vec<String>> {
    println!("{} Getting list of merged branches...", "📋".blue());
    let mut merged_branches = repo.get_merged_branches()?;

    if let Some(exclude) = exclude_branch {
        merged_branches.retain(|branch| branch != exclude);
    }

    // Apply safety filters to prevent deletion of new branches
    merged_branches = apply_safety_filters(repo, merged_branches, force)?;

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
        println!(
            "{} Skipping detached HEAD worktree: {}",
            "⚠️".yellow(),
            worktree.path.display()
        );
        return WorktreeAction::Skipped;
    }

    if !merged_branches.contains(&branch.to_string()) {
        return WorktreeAction::Ignored;
    }

    // Additional safety check: if the worktree directory was created recently (within 24 hours),
    // skip it to avoid deleting newly created branches
    if let Ok(metadata) = std::fs::metadata(&worktree.path) {
        if let Ok(created) = metadata.created() {
            let now = SystemTime::now();
            if let Ok(age) = now.duration_since(created) {
                let hours_old = age.as_secs() / 3600;
                if hours_old < 24 {
                    println!(
                        "{} Skipping recently created worktree: {} (created {} hours ago)",
                        "⚠️".yellow(),
                        branch,
                        hours_old
                    );
                    return WorktreeAction::Skipped;
                }
            }
        }
    }

    // At this point, we've already verified this branch was actually merged
    // The 24-hour check above provides additional safety
    remove_worktree_and_report(repo, worktree, branch)
}

fn remove_worktree_and_report(
    repo: &GitRepo,
    worktree: &crate::git::WorktreeInfo,
    branch: &str,
) -> WorktreeAction {
    println!(
        "{} Removing worktree for merged branch: {}",
        "🗑️".red(),
        branch
    );
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
        println!(
            "{} No merged branch worktrees found to clean up",
            "✨".green()
        );
    } else {
        println!();
        println!("{} Cleanup completed!", "✅".green().bold());
    }
}

fn cleanup_merged_only(repo: &GitRepo, force: bool) -> Result<()> {
    cleanup_merged_worktrees_with_force(repo, None, force)
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

    println!(
        "{} Removed {} worktree(s) matching pattern '{}'",
        "✅".green(),
        removed_count,
        pattern
    );
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
    let now = SystemTime::now();
    let stale_threshold = Duration::from_secs(14 * 24 * 60 * 60);
    let mut stale_candidates: Vec<(crate::git::WorktreeInfo, String, Duration)> = Vec::new();

    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            println!("{} main (current branch)", "📍".blue());
            continue;
        }

        if worktree.is_detached {
            println!(
                "{} {} (detached HEAD)",
                "⚠️".yellow(),
                worktree.path.display()
            );
            continue;
        }

        if let Some(branch) = &worktree.branch {
            let merged = repo.is_branch_merged(branch)?;
            let activity = repo
                .get_branch_last_commit_time(branch)?
                .and_then(|ts| now.duration_since(ts).ok());
            let activity_label = activity
                .map(format_duration)
                .unwrap_or_else(|| "unknown".to_string());

            if merged {
                let warn_old = activity
                    .map(|duration| duration >= Duration::from_secs(24 * 60 * 60))
                    .unwrap_or(true);
                if warn_old {
                    println!(
                        "{} {} (merged, last activity {}, ⚠️ >24h)",
                        "✅".green(),
                        branch,
                        activity_label
                    );
                } else {
                    println!(
                        "{} {} (merged, last activity {})",
                        "✅".green(),
                        branch,
                        activity_label
                    );
                }
            } else {
                let is_stale = activity
                    .map(|duration| duration >= stale_threshold)
                    .unwrap_or(false);

                if is_stale {
                    println!(
                        "{} {} (not merged, last activity {} ⚠️ stale)",
                        "❌".red(),
                        branch,
                        activity_label
                    );
                } else {
                    println!(
                        "{} {} (not merged, last activity {})",
                        "❌".red(),
                        branch,
                        activity_label
                    );
                }

                if let Some(duration) = activity {
                    if duration >= stale_threshold {
                        stale_candidates.push((worktree.clone(), branch.clone(), duration));
                    }
                }
            }
        } else {
            println!(
                "{} {} (unknown branch)",
                "⚠️".yellow(),
                worktree.path.display()
            );
        }
    }

    if !stale_candidates.is_empty() {
        println!();
        println!(
            "{} The following worktrees have seen no activity for 14 days or more:",
            "🧭".blue()
        );
        for (_, branch, duration) in &stale_candidates {
            println!(
                "  - {} (last activity {})",
                branch.cyan(),
                format_duration(*duration)
            );
        }
        println!();

        for (worktree, branch, duration) in stale_candidates {
            println!(
                "{} Branch '{}' has been inactive for {}",
                "⏳".yellow(),
                branch,
                format_duration(duration)
            );
            println!("    Worktree path: {}", worktree.path.display());
            print!("    Remove this worktree? (y/N) ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().eq_ignore_ascii_case("y") {
                remove_worktree_with_branch(repo, &worktree.path, &branch)?;
            } else {
                println!("    Skipped");
            }
            println!();
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
            println!(
                "  {} Could not delete branch '{}': {}",
                "⚠️".yellow(),
                branch,
                e
            );
        } else {
            println!("  {} Branch '{}' deleted", "✅".green(), branch);
        }
    }

    Ok(())
}

fn apply_safety_filters(
    repo: &GitRepo,
    branches: Vec<String>,
    _force: bool,
) -> Result<Vec<String>> {
    if branches.is_empty() {
        return Ok(branches);
    }

    // Skip remote existence checks so merged worktrees are cleaned even if the
    // corresponding remote branch has already been deleted.
    filter_identical_commits(repo, branches)
}

fn filter_identical_commits(repo: &GitRepo, branches: Vec<String>) -> Result<Vec<String>> {
    // Get main branch head for comparison
    let main_head = get_branch_head(repo, "main")?;
    let mut safe_branches = Vec::new();

    for branch in branches {
        // Safety check: Don't delete branches that point to the same commit as main
        // This protects newly created branches with no commits
        match get_branch_head(repo, &branch) {
            Ok(branch_head) => {
                if branch_head == main_head {
                    println!(
                        "  {} Skipping new branch (same as main): {}",
                        "🔒".yellow(),
                        branch
                    );
                    continue;
                }
            }
            Err(_) => {
                println!(
                    "  {} Skipping branch (cannot get HEAD): {}",
                    "⚠️".yellow(),
                    branch
                );
                continue;
            }
        }

        safe_branches.push(branch);
    }

    Ok(safe_branches)
}

fn get_branch_head(repo: &GitRepo, branch_name: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", branch_name])
        .current_dir(&repo.root_dir)
        .output()
        .context("Failed to get branch HEAD")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    let days = secs / (24 * 60 * 60);
    if days > 0 {
        return format!("{days}d ago");
    }

    let hours = secs / (60 * 60);
    if hours > 0 {
        return format!("{hours}h ago");
    }

    if secs < 60 {
        "<1m ago".to_string()
    } else {
        let minutes = secs / 60;
        format!("{minutes}m ago")
    }
}
