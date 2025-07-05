use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GitRepo {
    pub root_dir: PathBuf,
}

impl GitRepo {
    pub fn new() -> Result<Self> {
        let root_dir = get_main_repo_dir()?;
        Ok(Self { root_dir })
    }

    pub fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch_name}")])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to check if branch exists")?;
        
        Ok(output.status.success())
    }

    pub fn create_branch(&self, branch_name: &str) -> Result<()> {
        Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to create branch")?;
        
        Command::new("git")
            .args(["checkout", "-"])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to switch back to previous branch")?;
        
        Ok(())
    }

    pub fn add_worktree(&self, worktree_path: &Path, branch_name: &str) -> Result<()> {
        Command::new("git")
            .args(["worktree", "add", worktree_path.to_str().unwrap(), branch_name])
            .current_dir(&self.root_dir)
            .status()
            .context("Failed to create worktree")?;
        
        Ok(())
    }

    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to list worktrees")?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        parse_worktree_list(&output_str)
    }

    pub fn get_merged_branches(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["branch", "--merged", "main"])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to get merged branches")?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !line.contains("*"))
            .filter(|line| !line.trim().eq("main") && !line.trim().eq("master"))
            .map(|line| line.trim().trim_start_matches("+ ").to_string())
            .collect())
    }

    pub fn remove_worktree(&self, worktree_path: &Path, force: bool) -> Result<()> {
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(worktree_path.to_str().unwrap());
        
        Command::new("git")
            .args(&args)
            .current_dir(&self.root_dir)
            .status()
            .context("Failed to remove worktree")?;
        
        Ok(())
    }

    pub fn delete_branch(&self, branch_name: &str) -> Result<()> {
        Command::new("git")
            .args(["branch", "-D", branch_name])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to delete branch")?;
        
        Ok(())
    }

    pub fn is_branch_merged(&self, branch_name: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["merge-base", "--is-ancestor", branch_name, "main"])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to check if branch is merged")?;
        
        Ok(output.status.success())
    }
    
    pub fn has_unmerged_commits(&self, branch_name: &str) -> Result<bool> {
        // Check if branch has commits that are not in main
        let output = Command::new("git")
            .args(["rev-list", "--count", &format!("main..{branch_name}")])
            .current_dir(&self.root_dir)
            .output()
            .context("Failed to count unmerged commits")?;
        
        let count_str = String::from_utf8_lossy(&output.stdout);
        let count = count_str.trim().parse::<i32>().unwrap_or(0);
        
        Ok(count > 0)
    }

    pub fn get_current_branch(&self, worktree_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(worktree_path)
            .output()
            .context("Failed to get current branch")?;
        
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_detached: bool,
}

fn get_main_repo_dir() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["worktree", "list"])
        .output()
        .context("Failed to get worktree list")?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(first_line) = output_str.lines().next() {
            if let Some(path) = first_line.split_whitespace().next() {
                return Ok(PathBuf::from(path));
            }
        }
    }
    
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to get git root directory")?;
    
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path))
}

fn parse_worktree_list(output: &str) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut is_detached = false;
    
    for line in output.lines() {
        if line.starts_with("worktree ") {
            if let Some(path) = current_path.take() {
                worktrees.push(WorktreeInfo {
                    path,
                    branch: current_branch.take(),
                    is_detached,
                });
            }
            current_path = Some(PathBuf::from(line.trim_start_matches("worktree ")));
            is_detached = false;
        } else if line.starts_with("branch refs/heads/") {
            current_branch = Some(line.trim_start_matches("branch refs/heads/").to_string());
        } else if line == "detached" {
            is_detached = true;
        }
    }
    
    if let Some(path) = current_path {
        worktrees.push(WorktreeInfo {
            path,
            branch: current_branch,
            is_detached,
        });
    }
    
    Ok(worktrees)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> Result<(TempDir, GitRepo)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path();
        
        // Initialize a git repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()?;
        
        // Set git config to avoid errors
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()?;
        
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()?;
        
        // Create initial commit
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()?;
        
        // Rename to main if needed
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo_path)
            .output()?;
        
        let repo = GitRepo {
            root_dir: repo_path.to_path_buf(),
        };
        
        Ok((temp_dir, repo))
    }

    #[test]
    fn test_has_unmerged_commits_with_new_branch() -> Result<()> {
        let (_temp_dir, repo) = setup_test_repo()?;
        
        // Create a new branch
        repo.create_branch("test-branch")?;
        
        // A new branch without commits should not have unmerged commits
        assert!(!repo.has_unmerged_commits("test-branch")?);
        
        Ok(())
    }

    #[test]
    fn test_branch_exists() -> Result<()> {
        let (_temp_dir, repo) = setup_test_repo()?;
        
        // Main branch should exist
        assert!(repo.branch_exists("main")?);
        
        // Non-existent branch should not exist
        assert!(!repo.branch_exists("non-existent-branch")?);
        
        // Create a branch and check it exists
        repo.create_branch("test-branch")?;
        assert!(repo.branch_exists("test-branch")?);
        
        Ok(())
    }

    #[test]
    fn test_get_merged_branches() -> Result<()> {
        let (_temp_dir, repo) = setup_test_repo()?;
        
        // Create and immediately check merged branches
        repo.create_branch("feature-branch")?;
        
        // Switch back to main
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&repo.root_dir)
            .output()?;
        
        let merged = repo.get_merged_branches()?;
        
        // A branch created from main with no new commits should appear as merged
        assert!(merged.contains(&"feature-branch".to_string()));
        
        Ok(())
    }
}