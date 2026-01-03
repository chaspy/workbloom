use anyhow::{bail, Context, Result};
use sha1::{Digest, Sha1};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

pub trait TmuxClient: Send + Sync {
    fn is_available(&self) -> bool;
    fn session_exists(&self, session_name: &str) -> Result<bool>;
    fn create_session(&self, session_name: &str, directory: &Path) -> Result<()>;
    fn attach_session(&self, session_name: &str) -> Result<()>;
    fn kill_session(&self, session_name: &str) -> Result<bool>;
}

#[derive(Default, Clone)]
pub struct RealTmuxClient;

impl RealTmuxClient {
    pub fn new() -> Self {
        Self
    }
}

impl TmuxClient for RealTmuxClient {
    fn is_available(&self) -> bool {
        Command::new("tmux")
            .arg("-V")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn session_exists(&self, session_name: &str) -> Result<bool> {
        let status = Command::new("tmux")
            .args(["has-session", "-t", session_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .with_context(|| format!("Failed to check tmux session '{session_name}'"))?;

        match status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => bail!("tmux returned unexpected status while checking session"),
        }
    }

    fn create_session(&self, session_name: &str, directory: &Path) -> Result<()> {
        let status = Command::new("tmux")
            .args(["new-session", "-d", "-s", session_name, "-c"])
            .arg(directory)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .with_context(|| format!("Failed to create tmux session '{session_name}'"))?;

        if status.success() {
            Ok(())
        } else {
            bail!("tmux failed to create session")
        }
    }

    fn attach_session(&self, session_name: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["attach-session", "-t", session_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .with_context(|| format!("Failed to attach to tmux session '{session_name}'"))?;

        if status.success() {
            Ok(())
        } else {
            bail!("tmux failed to attach to session")
        }
    }

    fn kill_session(&self, session_name: &str) -> Result<bool> {
        if !self.is_available() {
            return Ok(false);
        }

        if !self.session_exists(session_name)? {
            return Ok(false);
        }

        let status = Command::new("tmux")
            .args(["kill-session", "-t", session_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .with_context(|| format!("Failed to kill tmux session '{session_name}'"))?;

        if status.success() {
            Ok(true)
        } else {
            bail!(
                "tmux exited with status {:?} while killing session '{}'",
                status.code(),
                session_name
            )
        }
    }
}

static CLIENT: OnceLock<Mutex<Arc<dyn TmuxClient>>> = OnceLock::new();

fn client_store() -> &'static Mutex<Arc<dyn TmuxClient>> {
    CLIENT.get_or_init(|| Mutex::new(Arc::new(RealTmuxClient::new())))
}

pub fn client() -> Arc<dyn TmuxClient> {
    client_store().lock().expect("tmux client poisoned").clone()
}

pub fn set_client(client: Arc<dyn TmuxClient>) {
    *client_store().lock().expect("tmux client poisoned") = client;
}

pub fn reset_client() {
    *client_store().lock().expect("tmux client poisoned") = Arc::new(RealTmuxClient::new());
}

pub fn is_available() -> bool {
    client().is_available()
}

pub fn session_exists(session_name: &str) -> Result<bool> {
    client().session_exists(session_name)
}

pub fn create_session(session_name: &str, directory: &Path) -> Result<()> {
    client().create_session(session_name, directory)
}

pub fn attach_session(session_name: &str) -> Result<()> {
    client().attach_session(session_name)
}

pub fn kill_session(session_name: &str) -> Result<bool> {
    client().kill_session(session_name)
}

#[cfg(test)]
pub(crate) fn test_client_lock() -> &'static Mutex<()> {
    static TMUX_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TMUX_TEST_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn sanitize_session_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }

    let sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        "worktree".to_string()
    } else {
        sanitized
    }
}

pub fn session_name(repo_root: &Path, identifier: &str) -> String {
    session_name_with_hash(repo_root, identifier, &stable_hash(repo_root))
}

pub fn legacy_session_name(repo_root: &Path, identifier: &str) -> String {
    session_name_with_hash(repo_root, identifier, &legacy_hash(repo_root))
}

fn session_name_with_hash(repo_root: &Path, identifier: &str, hash: &str) -> String {
    let repo_segment = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("repo");
    let repo_slug = sanitize_session_name(repo_segment);
    let identifier_slug = sanitize_session_name(identifier);
    sanitize_session_name(&format!("wb-{repo_slug}-{hash}-{identifier_slug}"))
}

fn stable_hash(repo_root: &Path) -> String {
    let mut hasher = Sha1::new();
    hasher.update(repo_root.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let bytes = [digest[0], digest[1], digest[2], digest[3]];
    format!("{:08x}", u32::from_be_bytes(bytes))
}

fn legacy_hash(repo_root: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    repo_root.to_string_lossy().hash(&mut hasher);
    format!("{:08x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::{legacy_session_name, sanitize_session_name, session_name};
    use std::path::PathBuf;

    #[test]
    fn sanitizes_to_tmux_safe_string() {
        let cases = vec![
            ("worktree-feature/foo", "worktree-feature-foo"),
            ("branch:name", "branch-name"),
            ("  spaced  ", "spaced"),
            ("..", ".."),
            ("", "worktree"),
        ];

        for (input, expected) in cases {
            assert_eq!(sanitize_session_name(input), expected);
        }
    }

    #[test]
    fn session_names_include_repo_hash() {
        let repo_a = PathBuf::from("/tmp/repo-a");
        let repo_b = PathBuf::from("/tmp/repo-b");
        let name_a = session_name(&repo_a, "worktree-feature");
        let name_b = session_name(&repo_b, "worktree-feature");
        assert_ne!(name_a, name_b);
        assert!(name_a.starts_with("wb-"));
    }

    #[test]
    fn legacy_and_current_session_names_differ() {
        let repo = PathBuf::from("/tmp/repo-a");
        let current = session_name(&repo, "worktree-feature");
        let legacy = legacy_session_name(&repo, "worktree-feature");
        assert_ne!(current, legacy);
    }
}
