use anyhow::{bail, Context, Result};
use sha1::{Digest, Sha1};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    Zellij,
    Tmux,
}

impl Backend {
    pub const ALL: [Backend; 2] = [Backend::Zellij, Backend::Tmux];

    pub fn display_name(self) -> &'static str {
        match self {
            Backend::Zellij => "zellij",
            Backend::Tmux => "tmux",
        }
    }

    fn env_var(self) -> &'static str {
        match self {
            Backend::Zellij => "ZELLIJ",
            Backend::Tmux => "TMUX",
        }
    }
}

pub trait MultiplexerClient: Send + Sync {
    fn is_available(&self, backend: Backend) -> bool;
    fn session_exists(&self, backend: Backend, session_name: &str) -> Result<bool>;
    fn create_session(&self, backend: Backend, session_name: &str, directory: &Path) -> Result<()>;
    fn attach_session(&self, backend: Backend, session_name: &str) -> Result<()>;
    fn kill_session(&self, backend: Backend, session_name: &str) -> Result<bool>;
}

#[derive(Default, Clone)]
pub struct RealMultiplexerClient;

impl RealMultiplexerClient {
    pub fn new() -> Self {
        Self
    }
}

impl MultiplexerClient for RealMultiplexerClient {
    fn is_available(&self, backend: Backend) -> bool {
        match backend {
            Backend::Zellij => Command::new("zellij")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false),
            Backend::Tmux => Command::new("tmux")
                .arg("-V")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false),
        }
    }

    fn session_exists(&self, backend: Backend, session_name: &str) -> Result<bool> {
        match backend {
            Backend::Zellij => Ok(zellij_sessions()?.iter().any(|name| name == session_name)),
            Backend::Tmux => {
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
        }
    }

    fn create_session(&self, backend: Backend, session_name: &str, directory: &Path) -> Result<()> {
        match backend {
            Backend::Zellij => {
                Command::new("zellij")
                    .args(["attach", "--create-background", session_name])
                    .current_dir(directory)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .with_context(|| format!("Failed to create zellij session '{session_name}'"))?;

                // Poll until the session appears (up to 3 seconds)
                for _ in 0..15 {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    if zellij_sessions()
                        .unwrap_or_default()
                        .iter()
                        .any(|n| n == session_name)
                    {
                        return Ok(());
                    }
                }
                bail!("zellij session '{session_name}' was not created within timeout")
            }
            Backend::Tmux => {
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
        }
    }

    fn attach_session(&self, backend: Backend, session_name: &str) -> Result<()> {
        match backend {
            Backend::Zellij => {
                let status = Command::new("zellij")
                    .args(["attach", session_name])
                    .status()
                    .with_context(|| {
                        format!("Failed to attach to zellij session '{session_name}'")
                    })?;

                if status.success() {
                    Ok(())
                } else {
                    bail!("zellij failed to attach to session")
                }
            }
            Backend::Tmux => {
                let status = Command::new("tmux")
                    .args(["attach-session", "-t", session_name])
                    .status()
                    .with_context(|| {
                        format!("Failed to attach to tmux session '{session_name}'")
                    })?;

                if status.success() {
                    Ok(())
                } else {
                    bail!("tmux failed to attach to session")
                }
            }
        }
    }

    fn kill_session(&self, backend: Backend, session_name: &str) -> Result<bool> {
        if !self.is_available(backend) {
            return Ok(false);
        }

        if !self.session_exists(backend, session_name)? {
            return Ok(false);
        }

        match backend {
            Backend::Zellij => {
                let status = Command::new("zellij")
                    .args(["delete-session", "--force", session_name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .with_context(|| format!("Failed to delete zellij session '{session_name}'"))?;

                if status.success() {
                    Ok(true)
                } else {
                    bail!("zellij failed to delete session")
                }
            }
            Backend::Tmux => {
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
    }
}

fn zellij_sessions() -> Result<Vec<String>> {
    let output = Command::new("zellij")
        .args(["list-sessions", "--no-formatting", "--short"])
        .output()
        .context("Failed to list zellij sessions")?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout.contains("No active zellij sessions found.")
        || stderr.contains("No active zellij sessions found.")
    {
        return Ok(Vec::new());
    }

    bail!(
        "zellij returned unexpected status while listing sessions: {}",
        output.status
    )
}

static CLIENT: OnceLock<Mutex<Arc<dyn MultiplexerClient>>> = OnceLock::new();

fn client_store() -> &'static Mutex<Arc<dyn MultiplexerClient>> {
    CLIENT.get_or_init(|| Mutex::new(Arc::new(RealMultiplexerClient::new())))
}

pub fn client() -> Arc<dyn MultiplexerClient> {
    client_store()
        .lock()
        .expect("multiplexer client poisoned")
        .clone()
}

pub fn set_client(client: Arc<dyn MultiplexerClient>) {
    *client_store().lock().expect("multiplexer client poisoned") = client;
}

pub fn reset_client() {
    *client_store().lock().expect("multiplexer client poisoned") =
        Arc::new(RealMultiplexerClient::new());
}

pub fn is_available(backend: Backend) -> bool {
    client().is_available(backend)
}

pub fn available_backends() -> Vec<Backend> {
    Backend::ALL
        .iter()
        .copied()
        .filter(|backend| is_available(*backend))
        .collect()
}

pub fn preferred_backend() -> Option<Backend> {
    available_backends().into_iter().next()
}

pub fn current_backend() -> Option<Backend> {
    Backend::ALL.iter().copied().find(|backend| {
        env::var(backend.env_var())
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    })
}

pub fn session_exists(backend: Backend, session_name: &str) -> Result<bool> {
    client().session_exists(backend, session_name)
}

pub fn create_session(backend: Backend, session_name: &str, directory: &Path) -> Result<()> {
    client().create_session(backend, session_name, directory)
}

pub fn attach_session(backend: Backend, session_name: &str) -> Result<()> {
    client().attach_session(backend, session_name)
}

pub fn kill_session(backend: Backend, session_name: &str) -> Result<bool> {
    client().kill_session(backend, session_name)
}

#[cfg(test)]
pub(crate) fn test_client_lock() -> &'static Mutex<()> {
    static MULTIPLEXER_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    MULTIPLEXER_TEST_LOCK.get_or_init(|| Mutex::new(()))
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
    fn sanitizes_to_safe_string() {
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
