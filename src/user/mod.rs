use anyhow::{Context, Result};
use std::process::Command;

/// The resolved container user identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUser {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
}

/// Imperative shell — reads user identity from the host.
/// Call this from the command handler.
pub fn get_user() -> Result<ResolvedUser> {
    let username = run_id("-un").context("Failed to determine username")?;
    let uid = run_id("-u")?
        .parse::<u32>()
        .context("Failed to parse uid")?;
    let gid = run_id("-g")?
        .parse::<u32>()
        .context("Failed to parse gid")?;
    Ok(ResolvedUser { username, uid, gid })
}

fn run_id(flag: &str) -> Result<String> {
    let output = Command::new("id")
        .arg(flag)
        .output()
        .context("Failed to execute `id` command (is it in PATH?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "`id {}` command failed with status: {}\n{}",
            flag,
            output.status,
            stderr.trim()
        );
    }

    let s = std::str::from_utf8(&output.stdout).context("Output of `id` is not valid UTF-8")?;
    Ok(s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_resolves_on_real_system() {
        let user = get_user().unwrap();
        assert!(!user.username.is_empty());
        assert!(user.uid > 0);
        assert!(user.gid > 0);
    }
}
