use anyhow::{Context, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::{config::Config, file_ops, git::GitRepo, tmux};

const PROGRESS_STEPS: u64 = 4;

pub fn execute(
    branch_name: &str,
    start_shell: bool,
    use_tmux: bool,
    print_path: bool,
) -> Result<()> {
    let repo = GitRepo::new()?;
    let config = Config::load_from_file(&repo.root_dir).unwrap_or_else(|_| Config::default());

    let worktree_dir_name = format!("worktree-{}", branch_name.replace('/', "-"));
    let worktree_path = repo.root_dir.join(&worktree_dir_name);
    let display_worktree_path = display_worktree_path(&repo.root_dir, &worktree_dir_name);
    let tmux_session_name = tmux::session_name(&repo.root_dir, &worktree_dir_name);

    crate::outln!("{} Setting up git worktree...", "🌲".green());
    crate::outln!("Branch: {}", branch_name.cyan());
    crate::outln!("Worktree directory: {}", worktree_path.display());
    crate::outln!();

    run_cleanup_if_exists(&repo, Some(branch_name))?;

    let pb = build_progress_bar(print_path);

    pb.set_message("Checking branch...");
    ensure_branch_ready(&repo, branch_name)?;
    pb.inc(1);

    pb.set_message("Creating worktree...");
    crate::outln!("{} Creating git worktree...", "🔧".blue());
    repo.add_worktree(&worktree_path, branch_name)?;
    pb.inc(1);

    pb.set_message("Copying files...");
    crate::outln!("{} Copying required files...", "📦".blue());
    file_ops::copy_required_files(&repo.root_dir, &worktree_path, &config)?;
    pb.inc(1);

    pb.set_message("Running setup script...");
    run_setup_script(&worktree_path)?;

    pb.set_message("Setting up direnv...");
    file_ops::setup_direnv(&worktree_path)?;
    pb.inc(1);

    pb.finish_with_message("Setup completed!");

    crate::outln!();
    crate::outln!("{} Git worktree setup completed!", "✅".green().bold());
    crate::outln!(
        "{} Worktree location: {}",
        "📍".blue(),
        display_worktree_path.display()
    );
    crate::outln!();

    handle_post_setup(
        print_path,
        start_shell,
        use_tmux,
        &display_worktree_path,
        &worktree_path,
        &tmux_session_name,
    )?;

    Ok(())
}

fn run_setup_script(worktree_path: &std::path::Path) -> Result<()> {
    let setup_script_path = worktree_path.join(".workbloom-setup.sh");

    if setup_script_path.exists() {
        crate::outln!("{} Found .workbloom-setup.sh, executing...", "🚀".cyan());

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
            eprintln!(
                "{} Warning: .workbloom-setup.sh failed: {}",
                "⚠️".yellow(),
                stderr
            );
            // Don't fail the entire setup if the script fails
        } else {
            crate::outln!("{} Setup script executed successfully", "✨".green());
        }
    }

    Ok(())
}

fn run_cleanup_if_exists(repo: &GitRepo, exclude_branch: Option<&str>) -> Result<()> {
    crate::outln!(
        "{} Checking for merged branch worktrees to clean up...",
        "🧹".yellow()
    );

    // 常に新しい実装を使用（スクリプトは無視）
    crate::commands::cleanup::cleanup_merged_worktrees_with_exclude(repo, exclude_branch)?;

    crate::outln!();
    Ok(())
}

fn build_progress_bar(print_path: bool) -> ProgressBar {
    if print_path {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(PROGRESS_STEPS);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("##-"),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }
}

fn ensure_branch_ready(repo: &GitRepo, branch_name: &str) -> Result<()> {
    if repo.branch_exists(branch_name)? {
        return Ok(());
    }

    if repo.remote_branch_exists(branch_name)? {
        crate::outln!(
            "{} Branch '{}' exists on remote. Fetching and creating tracking branch...",
            "🌐".blue(),
            branch_name
        );
        repo.fetch_remote_branch(branch_name)?;
        repo.create_tracking_branch(branch_name)?;
    } else {
        crate::outln!(
            "{} Branch '{}' does not exist. Creating it...",
            "📝".yellow(),
            branch_name
        );
        repo.create_branch(branch_name)?;
    }

    Ok(())
}

fn handle_post_setup(
    print_path: bool,
    start_shell: bool,
    use_tmux: bool,
    display_worktree_path: &Path,
    worktree_path: &Path,
    tmux_session_name: &str,
) -> Result<()> {
    if print_path {
        println!("{}", display_worktree_path.display());
        return Ok(());
    }

    if start_shell {
        crate::outln!("{} Starting worktree session...", "📂".blue());
        let inside_tmux = env::var("TMUX").map(|val| !val.is_empty()).unwrap_or(false);
        let started =
            manage_tmux_session(use_tmux, inside_tmux, worktree_path, tmux_session_name)?;

        if !started {
            crate::outln!("{} Launching shell in worktree directory...", "📂".blue());
            start_shell_in_directory(worktree_path)?;
        }

        return Ok(());
    }

    crate::outln!("{} Moving to worktree directory...", "📂".blue());
    crate::outln!("cd {}", display_worktree_path.display());
    crate::outln!();
    crate::outln!(
        "💡 Tip: Default behavior prints the worktree path. Use 'workbloom setup <branch> --shell' to start a shell",
    );
    Ok(())
}

fn manage_tmux_session(
    use_tmux: bool,
    inside_tmux: bool,
    worktree_path: &Path,
    tmux_session_name: &str,
) -> Result<bool> {
    if !use_tmux {
        return Ok(false);
    }

    if inside_tmux {
        crate::outln!(
            "{} Already inside tmux. Launching a regular shell instead...",
            "ℹ️".blue()
        );
        return Ok(false);
    }

    if !tmux::is_available() {
        crate::outln!(
            "{} tmux is not available. Starting a normal shell instead...",
            "⚠️".yellow()
        );
        return Ok(false);
    }

    match start_tmux_session(tmux_session_name, worktree_path) {
        Ok(_) => Ok(true),
        Err(err) => {
            crate::outln!(
                "{} tmux session failed (falling back to shell): {}",
                "⚠️".yellow(),
                err
            );
            Ok(false)
        }
    }
}

fn start_tmux_session(session_name: &str, worktree_path: &std::path::Path) -> Result<()> {
    if tmux::session_exists(session_name)? {
        crate::outln!(
            "{} Re-attaching to existing tmux session: {}",
            "🌀".blue(),
            session_name
        );
        tmux::attach_session(session_name)?;
        return Ok(());
    }

    crate::outln!(
        "{} Creating new tmux session: {} (dir: {})",
        "🌀".blue(),
        session_name,
        worktree_path.display()
    );
    tmux::create_session(session_name, worktree_path)?;
    tmux::attach_session(session_name)
}

fn start_shell_in_directory(worktree_path: &std::path::Path) -> Result<()> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

    Command::new(&shell)
        .current_dir(worktree_path)
        .status()
        .context("Failed to start shell in worktree directory")?;

    Ok(())
}

fn display_worktree_path(repo_root: &Path, worktree_dir_name: &str) -> PathBuf {
    if let Some(pwd_root) = preferred_pwd_root(repo_root) {
        return pwd_root.join(worktree_dir_name);
    }

    display_root_alias(repo_root).join(worktree_dir_name)
}

fn preferred_pwd_root(repo_root: &Path) -> Option<PathBuf> {
    let pwd = env::var("PWD").ok()?;
    let pwd_path = PathBuf::from(&pwd);
    let canonical_pwd = fs::canonicalize(&pwd_path).ok()?;
    if canonical_pwd == repo_root {
        Some(pwd_path)
    } else {
        None
    }
}

fn display_root_alias(repo_root: &Path) -> PathBuf {
    if let Ok(stripped) = repo_root.strip_prefix("/private") {
        let alias_root = Path::new("/").join(stripped);
        if alias_root.exists() {
            return alias_root;
        }
    }
    repo_root.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;
    use crate::tmux::{self, TmuxClient};
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

    static TMUX_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[derive(Clone)]
    struct MockTmuxClient {
        available: bool,
        state: Arc<Mutex<MockTmuxState>>,
    }

    #[derive(Default)]
    struct MockTmuxState {
        sessions: HashSet<String>,
        created: Vec<String>,
        attached: Vec<String>,
    }

    impl MockTmuxClient {
        fn new(available: bool) -> Self {
            Self {
                available,
                state: Arc::new(Mutex::new(MockTmuxState::default())),
            }
        }

        fn with_session(self, name: &str) -> Self {
            {
                let mut state = self.state.lock().unwrap();
                state.sessions.insert(name.to_string());
            }
            self
        }

        fn created_sessions(&self) -> Vec<String> {
            self.state.lock().unwrap().created.clone()
        }

        fn attached_sessions(&self) -> Vec<String> {
            self.state.lock().unwrap().attached.clone()
        }
    }

    impl TmuxClient for MockTmuxClient {
        fn is_available(&self) -> bool {
            self.available
        }

        fn session_exists(&self, session_name: &str) -> Result<bool> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .sessions
                .contains(session_name))
        }

        fn create_session(&self, session_name: &str, _directory: &Path) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            state.sessions.insert(session_name.to_string());
            state.created.push(session_name.to_string());
            Ok(())
        }

        fn attach_session(&self, session_name: &str) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            if !state.sessions.contains(session_name) {
                bail!("session not found");
            }
            state.attached.push(session_name.to_string());
            Ok(())
        }

        fn kill_session(&self, session_name: &str) -> Result<bool> {
            let mut state = self.state.lock().unwrap();
            Ok(state.sessions.remove(session_name))
        }
    }

    fn with_mock_tmux<F: FnOnce()>(mock: Arc<MockTmuxClient>, test: F) {
        struct ResetGuard<'a> {
            _lock: MutexGuard<'a, ()>,
            original: Arc<dyn TmuxClient>,
        }

        impl<'a> Drop for ResetGuard<'a> {
            fn drop(&mut self) {
                tmux::set_client(self.original.clone());
            }
        }

        let lock = TMUX_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let original = tmux::client();
        let guard = ResetGuard {
            _lock: lock,
            original,
        };
        let trait_obj: Arc<dyn TmuxClient> = mock.clone();
        tmux::set_client(trait_obj);

        test();
        drop(guard);
    }

    #[test]
    fn manage_tmux_session_reattaches_existing_session() {
        let mock = Arc::new(MockTmuxClient::new(true).with_session("session-a"));
        with_mock_tmux(mock.clone(), || {
            let started = manage_tmux_session(
                true,
                false,
                Path::new("/tmp/worktree"),
                "session-a",
            )
            .unwrap();
            assert!(started);
        });
        assert_eq!(mock.created_sessions(), Vec::<String>::new());
        assert_eq!(mock.attached_sessions(), vec!["session-a".to_string()]);
    }

    #[test]
    fn manage_tmux_session_creates_new_session_when_missing() {
        let mock = Arc::new(MockTmuxClient::new(true));
        with_mock_tmux(mock.clone(), || {
            let started = manage_tmux_session(
                true,
                false,
                Path::new("/tmp/worktree"),
                "session-b",
            )
            .unwrap();
            assert!(started);
        });
        assert_eq!(mock.created_sessions(), vec!["session-b".to_string()]);
        assert_eq!(mock.attached_sessions(), vec!["session-b".to_string()]);
    }

    #[test]
    fn manage_tmux_session_skips_when_unavailable() {
        let mock = Arc::new(MockTmuxClient::new(false));
        with_mock_tmux(mock.clone(), || {
            let started = manage_tmux_session(
                true,
                false,
                Path::new("/tmp/worktree"),
                "session-c",
            )
            .unwrap();
            assert!(!started);
        });
        assert!(mock.created_sessions().is_empty());
        assert!(mock.attached_sessions().is_empty());
    }
}
