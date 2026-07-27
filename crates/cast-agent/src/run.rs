//! The harness-agnostic orchestrator: persist-before-spawn, supervise the
//! child, extract the final message, and write the `result.json` verdict.
//!
//! `main.rs` is a thin CLI shell over this. Keeping the orchestration in the
//! library lets the end-to-end path be driven by a scripted fake harness in
//! tests without depending on a real harness binary.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::finalize::{Outcome, build_verdict, classify, write_result_json};
use crate::harness::Harness;
use crate::supervisor::{Limits, RunPaths, supervise};

/// The condensed result of an orchestrated run for the CLI layer: what to
/// print on stdout and which exit code to use. `result.json` (the full
/// verdict) has already been written to the run directory.
#[derive(Debug)]
pub struct RunReport {
    pub outcome: Outcome,
    pub final_message: Option<String>,
    pub exit_code: i32,
}

/// Standard artifact paths within a run directory.
pub struct RunLayout {
    pub stream_jsonl: PathBuf,
    pub stderr_log: PathBuf,
    pub result_json: PathBuf,
    pub prompt_txt: PathBuf,
    pub pid_file: PathBuf,
}

impl RunLayout {
    pub fn new(run_dir: &Path) -> Self {
        Self {
            stream_jsonl: run_dir.join("stream.jsonl"),
            stderr_log: run_dir.join("stderr.log"),
            result_json: run_dir.join("result.json"),
            prompt_txt: run_dir.join("prompt.txt"),
            pid_file: run_dir.join("cast-agent.pid"),
        }
    }
}

/// Orchestrate one supervised run end to end.
///
/// `exe`/`args` are the fully-formed command (in production `exe =
/// harness.base_command()` and `args = harness.headless_args()`); tests inject
/// a scripted fake). `harness` is still used for `extract_result` and identity.
#[cfg(unix)]
pub async fn orchestrate(
    harness: &dyn Harness,
    exe: &str,
    args: &[String],
    prompt: &str,
    run_dir: &Path,
    limits: Limits,
) -> Result<RunReport> {
    let layout = RunLayout::new(run_dir);

    // Persist-before-spawn: the prompt and our PID must exist before the child
    // starts so an instant crash/EPIPE cannot lose the recovery artifacts.
    std::fs::write(&layout.prompt_txt, prompt.as_bytes())?;
    std::fs::write(&layout.pid_file, std::process::id().to_string())?;

    let paths = RunPaths {
        stream_jsonl: layout.stream_jsonl.clone(),
        stderr_log: layout.stderr_log.clone(),
    };

    let start = Instant::now();
    let out = supervise(exe, args, prompt.as_bytes(), &paths, limits).await?;
    let duration = start.elapsed();

    let (outcome, _exit) = classify(&out.end);
    // The final message is only meaningful/wanted on the happy path.
    let final_message = if outcome == Outcome::Completed {
        harness.extract_result(&out.events)
    } else {
        None
    };

    let verdict = build_verdict(
        harness.name(),
        &out.end,
        &out.events,
        final_message.clone(),
        &layout.stream_jsonl,
        &layout.prompt_txt,
        duration,
    );
    write_result_json(&layout.result_json, &verdict)?;

    Ok(RunReport {
        outcome,
        final_message,
        exit_code: outcome.exit_code(),
    })
}

/// Convenience: the grace window is fixed at 3s for the MVP.
pub fn limits_from_timeout(timeout: Duration) -> Limits {
    Limits {
        deadline: timeout,
        grace: Duration::from_secs(3),
    }
}
