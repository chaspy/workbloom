use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn is_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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

pub fn session_exists(session_name: &str) -> Result<bool> {
    let status = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .status()
        .with_context(|| format!("Failed to check tmux session '{}'", session_name))?;

    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => bail!("tmux returned unexpected status while checking session"),
    }
}

pub fn create_session(session_name: &str, directory: &Path) -> Result<()> {
    let status = Command::new("tmux")
        .args(["new-session", "-d", "-s", session_name, "-c"])
        .arg(directory)
        .status()
        .with_context(|| format!("Failed to create tmux session '{}'", session_name))?;

    if status.success() {
        Ok(())
    } else {
        bail!("tmux failed to create session")
    }
}

pub fn attach_session(session_name: &str) -> Result<()> {
    let status = Command::new("tmux")
        .args(["attach-session", "-t", session_name])
        .status()
        .with_context(|| format!("Failed to attach to tmux session '{}'", session_name))?;

    if status.success() {
        Ok(())
    } else {
        bail!("tmux failed to attach to session")
    }
}

pub fn kill_session(session_name: &str) -> Result<bool> {
    if !is_available() {
        return Ok(false);
    }

    if !session_exists(session_name)? {
        return Ok(false);
    }

    let status = Command::new("tmux")
        .args(["kill-session", "-t", session_name])
        .status()
        .with_context(|| format!("Failed to kill tmux session '{}'", session_name))?;

    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::sanitize_session_name;

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
}
