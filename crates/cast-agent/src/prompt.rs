//! Harness-agnostic prompt resolution: `--file` > stdin > positional.
//!
//! All three harnesses read the prompt from stdin, so the orchestrator
//! resolves a single payload string and pipes it to the child uniformly.

use anyhow::{Result, bail};
use std::io::Read;
use std::path::Path;

/// Pure precedence resolver over already-captured inputs.
///
/// Precedence is `--file` > stdin > positional. A `stdin` value that is empty
/// or whitespace-only is treated as absent (nothing was piped in), so a
/// positional prompt still applies.
pub fn choose_prompt(
    file: Option<String>,
    stdin: Option<String>,
    positional: Option<String>,
) -> Result<String> {
    if let Some(f) = file {
        return Ok(f);
    }
    if let Some(s) = stdin
        && !s.trim().is_empty()
    {
        return Ok(s);
    }
    if let Some(p) = positional {
        return Ok(p);
    }
    bail!("no prompt provided: supply --file, pipe via stdin, or a positional argument")
}

/// Resolve the prompt performing the actual IO. Reads `--file` if given;
/// otherwise reads stdin when it is not a terminal; otherwise falls back to
/// the positional argument.
pub fn resolve_prompt(file: Option<&Path>, positional: Option<String>) -> Result<String> {
    let file_content = match file {
        Some(p) => Some(std::fs::read_to_string(p)?),
        None => None,
    };

    let stdin_content = if file_content.is_none() && !stdin_is_terminal() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Some(buf)
    } else {
        None
    };

    choose_prompt(file_content, stdin_content, positional)
}

#[cfg(unix)]
fn stdin_is_terminal() -> bool {
    // SAFETY: isatty on the stdin fd is always safe.
    unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
}

#[cfg(not(unix))]
fn stdin_is_terminal() -> bool {
    false
}
