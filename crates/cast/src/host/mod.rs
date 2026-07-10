//! Host-identity resolution.
//!
//! Mirrors the imperative-shell pattern of [`crate::user`]: reads a host
//! fact (the kernel hostname) and returns it as data. The policy core
//! [`normalize_hostname`] is pure and fully unit-testable; the syscall
//! wrapper [`get_hostname`] is the thin imperative shell around it.

use anyhow::{Context, Result};

/// Normalize a raw hostname string into a stable grouping key.
///
/// - Empty or whitespace-only input becomes the sentinel `"unknown"`.
/// - Otherwise the trimmed value is returned as-is.
///
/// We do NOT canonicalize short vs FQDN: `gethostname(2)` returns whatever
/// the kernel has set, and grouping only needs stability, not a canonical
/// form. Callers that need a specific form should resolve it downstream.
pub fn normalize_hostname(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Read the host machine's kernel hostname and normalize it.
///
/// Returns `"unknown"` (via [`normalize_hostname`]) when the hostname is
/// empty, and an `Err` only when the underlying syscall fails. The call
/// site in `resolve_run_opts` soft-fails the `Err` case so a metadata
/// lookup never aborts a working sandbox.
pub fn get_hostname() -> Result<String> {
    let raw = gethostname_impl().context("Failed to determine host name")?;
    Ok(normalize_hostname(&raw))
}

/// Thin syscall wrapper around `gethostname(2)`.
///
/// Returns the raw hostname as owned bytes converted to UTF-8. POSIX does
/// not guarantee NUL-termination when the name exactly fills the buffer, so
/// the NUL search is capped at `buf.len()`. The 256-byte buffer is a safe
/// over-allocation (`HOST_NAME_MAX` is 64 on Linux).
fn gethostname_impl() -> Result<String> {
    let mut buf = vec![0u8; 256];
    // SAFETY: `buf` is valid for `buf.len()` bytes. `gethostname` writes at
    // most `len` bytes and NUL-terminates when there is room.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        anyhow::bail!("gethostname() failed: {err}");
    }
    // Truncate at the first NUL. If the name filled the buffer without a
    // NUL, `unwrap_or` falls back to the full length.
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(end);
    String::from_utf8(buf).context("Host name is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── normalize_hostname: pure policy core ─────────────────────────────────

    #[test]
    fn test_normalize_empty_becomes_unknown() {
        assert_eq!(normalize_hostname(""), "unknown");
    }

    #[test]
    fn test_normalize_whitespace_only_becomes_unknown() {
        assert_eq!(normalize_hostname("   \n\t"), "unknown");
    }

    #[test]
    fn test_normalize_short_name_passthrough() {
        assert_eq!(normalize_hostname("myhost"), "myhost");
    }

    #[test]
    fn test_normalize_fqdn_passthrough() {
        // FQDN is accepted as-is; no canonicalization to short name.
        assert_eq!(
            normalize_hostname("myhost.corp.example.com"),
            "myhost.corp.example.com"
        );
    }

    #[test]
    fn test_normalize_trims_surrounding_whitespace() {
        assert_eq!(normalize_hostname("  myhost  \n"), "myhost");
    }

    // ── get_hostname: syscall wrapper (lenient smoke test) ───────────────────
    //
    // Asserts only on the invariant our code guarantees (non-empty after
    // normalization via the sentinel), NEVER on ambient system state like a
    // specific hostname value. This keeps the test reproducible across
    // machines and Nix build sandboxes.

    #[test]
    fn test_get_hostname_returns_non_empty() {
        let h = get_hostname().unwrap();
        assert!(
            !h.is_empty(),
            "get_hostname must be non-empty by construction of the sentinel"
        );
    }
}
