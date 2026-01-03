use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::time::SystemTime;

use crate::{git::GitRepo, tmux};

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
    crate::outln!(
        "{} Cleaning up worktrees for merged branches...",
        "🧹".yellow()
    );

    let merged_branches = get_filtered_merged_branches(repo, exclude_branch, force)?;

    if merged_branches.is_empty() {
        crate::outln!("{} No merged branches found", "✨".green());
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
    crate::outln!(
        "{} Cleaning up worktrees for merged branches...",
        "🧹".yellow()
    );

    let merged_branches = get_filtered_merged_branches(repo, exclude_branch, false)?;

    if merged_branches.is_empty() {
        crate::outln!("{} No merged branches found", "✨".green());
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
    crate::outln!("{} Getting list of merged branches...", "📋".blue());
    let mut merged_branches = repo.get_merged_branches()?;

    if let Some(exclude) = exclude_branch {
        merged_branches.retain(|branch| branch != exclude);
    }

    // Apply safety filters to prevent deletion of new branches
    merged_branches = apply_safety_filters(repo, merged_branches, force)?;

    Ok(merged_branches)
}

fn display_merged_branches(merged_branches: &[String], exclude_branch: Option<&str>) {
    crate::outln!("Found merged branches:");
    for branch in merged_branches {
        crate::outln!("  - {branch}");
    }
    if let Some(exclude) = exclude_branch {
        crate::outln!("  (excluding: {})", exclude.cyan());
    }
    crate::outln!();
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
        crate::outln!(
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
                    crate::outln!(
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
    crate::outln!(
        "{} Removing worktree for merged branch: {}",
        "🗑️".red(),
        branch
    );
    crate::outln!("    Path: {}", worktree.path.display());

    match repo.remove_worktree(&worktree.path, true) {
        Ok(_) => {
            crate::outln!("    {} Successfully removed", "✅".green());
            stop_tmux_session(&repo.root_dir, &worktree.path);
            WorktreeAction::Removed
        }
        Err(e) => {
            crate::outln!("    {} Failed to remove: {}", "❌".red(), e);
            WorktreeAction::Skipped
        }
    }
}

fn display_cleanup_summary(cleaned_count: usize, skipped_count: usize) {
    crate::outln!();
    crate::outln!("{} Summary:", "📊".blue());
    crate::outln!("  - Cleaned up: {cleaned_count} worktree(s)");
    crate::outln!("  - Skipped: {skipped_count} worktree(s)");

    if cleaned_count == 0 && skipped_count == 0 {
        crate::outln!();
        crate::outln!(
            "{} No merged branch worktrees found to clean up",
            "✨".green()
        );
    } else {
        crate::outln!();
        crate::outln!("{} Cleanup completed!", "✅".green().bold());
    }
}

fn cleanup_merged_only(repo: &GitRepo, force: bool) -> Result<()> {
    cleanup_merged_worktrees_with_force(repo, None, force)
}

fn cleanup_by_pattern(repo: &GitRepo, pattern: &str) -> Result<()> {
    crate::outln!("Removing worktrees matching pattern: {}", pattern.cyan());
    crate::outln!();

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

    crate::outln!(
        "{} Removed {} worktree(s) matching pattern '{}'",
        "✅".green(),
        removed_count,
        pattern
    );
    Ok(())
}

fn interactive_cleanup(repo: &GitRepo) -> Result<()> {
    crate::outln!("Interactive worktree removal");
    crate::outln!();

    let worktrees = repo.list_worktrees()?;

    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            continue;
        }

        if let Some(branch) = &worktree.branch {
            crate::outln!("Worktree: {}", worktree.path.display());
            crate::outln!("Branch: {}", branch.cyan());

            print!("Remove this worktree? (y/n) ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() == "y" {
                remove_worktree_with_branch(repo, &worktree.path, branch)?;
            } else {
                crate::outln!("  Skipped");
            }
            crate::outln!();
        }
    }

    Ok(())
}

fn show_status(repo: &GitRepo) -> Result<()> {
    crate::outln!("Checking merge status of all branches...");
    crate::outln!();

    let worktrees = repo.list_worktrees()?;

    for worktree in &worktrees {
        if worktree.path == repo.root_dir {
            crate::outln!("{} main (current branch)", "📍".blue());
        } else if let Some(branch) = &worktree.branch {
            if repo.is_branch_merged(branch)? {
                crate::outln!("{} {} (merged)", "✅".green(), branch);
            } else {
                crate::outln!("{} {} (not merged)", "❌".red(), branch);
            }
        }
    }

    Ok(())
}

fn remove_worktree_with_branch(repo: &GitRepo, path: &std::path::Path, branch: &str) -> Result<()> {
    crate::outln!("  Removing worktree: {}", path.display());

    if let Err(e) = repo.remove_worktree(path, true) {
        crate::outln!("  {} Failed to remove worktree: {}", "❌".red(), e);
        return Ok(());
    }

    crate::outln!("  {} Worktree removed successfully", "✅".green());
    stop_tmux_session(&repo.root_dir, path);

    if repo.branch_exists(branch)? {
        if let Err(e) = repo.delete_branch(branch) {
            crate::outln!(
                "  {} Could not delete branch '{}': {}",
                "⚠️".yellow(),
                branch,
                e
            );
        } else {
            crate::outln!("  {} Branch '{}' deleted", "✅".green(), branch);
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
                    crate::outln!(
                        "  {} Skipping new branch (same as main): {}",
                        "🔒".yellow(),
                        branch
                    );
                    continue;
                }
            }
            Err(_) => {
                crate::outln!(
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

fn stop_tmux_session(repo_root: &std::path::Path, worktree_path: &std::path::Path) {
    if let Some(dir_name) = worktree_path.file_name().and_then(|n| n.to_str()) {
        let session_name = tmux::session_name(repo_root, dir_name);
        if try_stop_session(&session_name, false) {
            return;
        }

        let legacy_name = tmux::legacy_session_name(repo_root, dir_name);
        if legacy_name != session_name {
            try_stop_session(&legacy_name, true);
        }
    }
}

fn try_stop_session(session_name: &str, legacy: bool) -> bool {
    match tmux::kill_session(session_name) {
        Ok(true) => {
            if legacy {
                crate::outln!(
                    "    {} Closed legacy tmux session: {}",
                    "🌀".blue(),
                    session_name
                );
            } else {
                crate::outln!("    {} Closed tmux session: {}", "🌀".blue(), session_name);
            }
            true
        }
        Ok(false) => {
            let label = if legacy { "legacy tmux session" } else { "tmux session" };
            crate::outln!(
                "    {} No {} to close (session: {})",
                "ℹ️".blue(),
                label,
                session_name
            );
            false
        }
        Err(e) => {
            crate::outln!(
                "    {} Failed to close tmux session '{}': {}",
                "⚠️".yellow(),
                session_name,
                e
            );
            false
        }
    }
}
