use anyhow::{Context, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::{config::Config, file_ops, git::GitRepo, multiplexer};

const PROGRESS_STEPS: u64 = 4;

pub fn execute(
    branch_name: &str,
    start_shell: bool,
    use_multiplexer: bool,
    print_path: bool,
) -> Result<()> {
    let repo = GitRepo::new()?;
    let config = Config::load_from_file(&repo.root_dir).unwrap_or_else(|_| Config::default());

    let worktree_dir_name = format!("worktree-{}", branch_name.replace('/', "-"));
    let worktree_path = repo.root_dir.join(&worktree_dir_name);
    let display_worktree_path = display_worktree_path(&repo.root_dir, &worktree_dir_name);
    let session_name = multiplexer::session_name(&repo.root_dir, &worktree_dir_name);

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
        use_multiplexer,
        &display_worktree_path,
        &worktree_path,
        &session_name,
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
    use_multiplexer: bool,
    display_worktree_path: &Path,
    worktree_path: &Path,
    session_name: &str,
) -> Result<()> {
    if print_path {
        println!("{}", display_worktree_path.display());
        return Ok(());
    }

    if start_shell {
        crate::outln!("{} Starting worktree session...", "📂".blue());
        let inside_backend = multiplexer::current_backend();
        let started = manage_multiplexer_session(
            use_multiplexer,
            inside_backend,
            worktree_path,
            session_name,
        )?;

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

fn manage_multiplexer_session(
    use_multiplexer: bool,
    inside_backend: Option<multiplexer::Backend>,
    worktree_path: &Path,
    session_name: &str,
) -> Result<bool> {
    if !use_multiplexer {
        return Ok(false);
    }

    if let Some(backend) = inside_backend {
        crate::outln!(
            "{} Already inside {}. Launching a regular shell instead...",
            "ℹ️".blue(),
            backend.display_name()
        );
        return Ok(false);
    }

    let Some(backend) = multiplexer::preferred_backend() else {
        crate::outln!(
            "{} No supported multiplexer is available. Starting a normal shell instead...",
            "⚠️".yellow()
        );
        return Ok(false);
    };

    match start_multiplexer_session(backend, session_name, worktree_path) {
        Ok(_) => Ok(true),
        Err(err) => {
            crate::outln!(
                "{} {} session failed (falling back to shell): {}",
                "⚠️".yellow(),
                backend.display_name(),
                err
            );
            Ok(false)
        }
    }
}

fn start_multiplexer_session(
    backend: multiplexer::Backend,
    session_name: &str,
    worktree_path: &std::path::Path,
) -> Result<()> {
    if multiplexer::session_exists(backend, session_name)? {
        crate::outln!(
            "{} Re-attaching to existing {} session: {}",
            "🌀".blue(),
            backend.display_name(),
            session_name
        );
        multiplexer::attach_session(backend, session_name)?;
        return Ok(());
    }

    crate::outln!(
        "{} Creating new {} session: {} (dir: {})",
        "🌀".blue(),
        backend.display_name(),
        session_name,
        worktree_path.display()
    );
    multiplexer::create_session(backend, session_name, worktree_path)?;
    multiplexer::attach_session(backend, session_name)
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
    use crate::multiplexer::{self, Backend, MultiplexerClient};
    use anyhow::bail;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex, MutexGuard};

    #[derive(Clone)]
    struct MockMultiplexerClient {
        available: HashSet<Backend>,
        state: Arc<Mutex<MockMultiplexerState>>,
    }

    #[derive(Default)]
    struct MockMultiplexerState {
        sessions: HashMap<Backend, HashSet<String>>,
        created: Vec<(Backend, String)>,
        attached: Vec<(Backend, String)>,
    }

    impl MockMultiplexerClient {
        fn new(available: &[Backend]) -> Self {
            Self {
                available: available.iter().copied().collect(),
                state: Arc::new(Mutex::new(MockMultiplexerState::default())),
            }
        }

        fn with_session(self, backend: Backend, name: &str) -> Self {
            {
                let mut state = self.state.lock().unwrap();
                state
                    .sessions
                    .entry(backend)
                    .or_default()
                    .insert(name.to_string());
            }
            self
        }

        fn created_sessions(&self) -> Vec<(Backend, String)> {
            self.state.lock().unwrap().created.clone()
        }

        fn attached_sessions(&self) -> Vec<(Backend, String)> {
            self.state.lock().unwrap().attached.clone()
        }
    }

    impl MultiplexerClient for MockMultiplexerClient {
        fn is_available(&self, backend: Backend) -> bool {
            self.available.contains(&backend)
        }

        fn session_exists(&self, backend: Backend, session_name: &str) -> Result<bool> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .sessions
                .get(&backend)
                .map(|sessions| sessions.contains(session_name))
                .unwrap_or(false))
        }

        fn create_session(
            &self,
            backend: Backend,
            session_name: &str,
            _directory: &Path,
        ) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            state
                .sessions
                .entry(backend)
                .or_default()
                .insert(session_name.to_string());
            state.created.push((backend, session_name.to_string()));
            Ok(())
        }

        fn attach_session(&self, backend: Backend, session_name: &str) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            let session_exists = state
                .sessions
                .get(&backend)
                .map(|sessions| sessions.contains(session_name))
                .unwrap_or(false);
            if !session_exists {
                bail!("session not found");
            }
            state.attached.push((backend, session_name.to_string()));
            Ok(())
        }

        fn kill_session(&self, backend: Backend, session_name: &str) -> Result<bool> {
            let mut state = self.state.lock().unwrap();
            Ok(state
                .sessions
                .get_mut(&backend)
                .map(|sessions| sessions.remove(session_name))
                .unwrap_or(false))
        }
    }

    fn with_mock_multiplexer<F: FnOnce()>(mock: Arc<MockMultiplexerClient>, test: F) {
        struct ResetGuard<'a> {
            _lock: MutexGuard<'a, ()>,
            original: Arc<dyn MultiplexerClient>,
        }

        impl<'a> Drop for ResetGuard<'a> {
            fn drop(&mut self) {
                multiplexer::set_client(self.original.clone());
            }
        }

        let lock = multiplexer::test_client_lock().lock().unwrap();
        let original = multiplexer::client();
        let guard = ResetGuard {
            _lock: lock,
            original,
        };
        let trait_obj: Arc<dyn MultiplexerClient> = mock.clone();
        multiplexer::set_client(trait_obj);

        test();
        drop(guard);
    }

    #[test]
    fn manage_multiplexer_session_reattaches_existing_zellij_session() {
        let mock = Arc::new(
            MockMultiplexerClient::new(&[Backend::Zellij])
                .with_session(Backend::Zellij, "session-a"),
        );
        with_mock_multiplexer(mock.clone(), || {
            let started =
                manage_multiplexer_session(true, None, Path::new("/tmp/worktree"), "session-a")
                    .unwrap();
            assert!(started);
        });
        assert_eq!(mock.created_sessions(), Vec::<(Backend, String)>::new());
        assert_eq!(
            mock.attached_sessions(),
            vec![(Backend::Zellij, "session-a".to_string())]
        );
    }

    #[test]
    fn manage_multiplexer_session_falls_back_to_tmux_when_zellij_is_unavailable() {
        let mock = Arc::new(MockMultiplexerClient::new(&[Backend::Tmux]));
        with_mock_multiplexer(mock.clone(), || {
            let started =
                manage_multiplexer_session(true, None, Path::new("/tmp/worktree"), "session-b")
                    .unwrap();
            assert!(started);
        });
        assert_eq!(
            mock.created_sessions(),
            vec![(Backend::Tmux, "session-b".to_string())]
        );
        assert_eq!(
            mock.attached_sessions(),
            vec![(Backend::Tmux, "session-b".to_string())]
        );
    }

    #[test]
    fn manage_multiplexer_session_skips_when_unavailable() {
        let mock = Arc::new(MockMultiplexerClient::new(&[]));
        with_mock_multiplexer(mock.clone(), || {
            let started =
                manage_multiplexer_session(true, None, Path::new("/tmp/worktree"), "session-c")
                    .unwrap();
            assert!(!started);
        });
        assert!(mock.created_sessions().is_empty());
        assert!(mock.attached_sessions().is_empty());
    }

    #[test]
    fn manage_multiplexer_session_skips_when_already_inside_multiplexer() {
        let mock = Arc::new(MockMultiplexerClient::new(&[
            Backend::Zellij,
            Backend::Tmux,
        ]));
        with_mock_multiplexer(mock.clone(), || {
            let started = manage_multiplexer_session(
                true,
                Some(Backend::Zellij),
                Path::new("/tmp/worktree"),
                "session-d",
            )
            .unwrap();
            assert!(!started);
        });
        assert!(mock.created_sessions().is_empty());
        assert!(mock.attached_sessions().is_empty());
    }
}
