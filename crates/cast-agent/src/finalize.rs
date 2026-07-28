//! Outcome classification + the structured `result.json` verdict.
//!
//! The supervisor reports a raw `EndReason`; only the supervisor knows *why* a
//! run stopped (the stream cannot say). This module maps that plus the
//! harness-extracted final message into a harness-agnostic `Outcome`, a
//! documented exit code, and the `result.json` "receipt" that references the
//! `stream.jsonl` "evidence" (see `note/recovery-outcome-contract.md`).

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

#[cfg(unix)]
use crate::supervisor::EndReason;

/// Harness-agnostic verdict for a supervised run. `result.json.outcome` is the
/// authoritative classification; the process exit code mirrors it for shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Exit 0 (the final message, if any, is the payload).
    Completed,
    /// The child exited non-zero.
    Failed,
    /// The child died by a signal (segfault / OOM / externally killed).
    Crashed,
    /// The wall-clock deadline fired.
    TimedOut,
    /// cast-agent received SIGINT/SIGTERM and tore the run down.
    Interrupted,
}

impl Outcome {
    /// The `outcome` string written to `result.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::Failed => "failed",
            Outcome::Crashed => "crashed",
            Outcome::TimedOut => "timed_out",
            Outcome::Interrupted => "interrupted",
        }
    }

    /// The documented process exit code for this outcome (see the exit-code
    /// table in the recovery contract). Note: `2` (usage error) is emitted by
    /// the CLI layer before a run starts, so it is not produced here.
    pub fn exit_code(self) -> i32 {
        match self {
            Outcome::Completed => 0,
            Outcome::Failed => 1,
            Outcome::TimedOut => 3,
            Outcome::Interrupted => 4,
            Outcome::Crashed => 5,
        }
    }
}

/// The child's exit disposition: a normal exit `code` XOR a terminating
/// `signal`. Both are `Option` because a given path populates only one.
#[derive(Debug, Serialize)]
pub struct ExitInfo {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

/// The serialized `result.json` document. It REFERENCES the trace via
/// `log_path`; it never embeds the (potentially multi-MB) stream inline.
#[derive(Debug, Serialize)]
pub struct Verdict {
    pub outcome: &'static str,
    pub final_message: Option<String>,
    pub exit: ExitInfo,
    pub harness: String,
    pub log_path: String,
    pub prompt_path: String,
    pub event_count: usize,
    pub duration_ms: u64,
    /// A short human-readable failure reason when one is available (spawn /
    /// supervision failure). Omitted from `result.json` when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,
    /// The signal cast-agent itself received that triggered an interruption
    /// (distinct from `exit.signal`, which is how the child actually died).
    /// Present only for `interrupted` outcomes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt_signal: Option<i32>,
}

/// Derive `ExitInfo` from a reaped child status: a normal `code`, else the
/// terminating `signal`. When the status is unavailable (could not reap), fall
/// back to `fallback_signal` (the signal cast-agent last sent the group).
#[cfg(unix)]
fn exit_info_from_status(
    status: &Option<std::process::ExitStatus>,
    fallback_signal: i32,
) -> ExitInfo {
    use std::os::unix::process::ExitStatusExt;
    match status {
        Some(s) => {
            if let Some(code) = s.code() {
                ExitInfo {
                    code: Some(code),
                    signal: None,
                }
            } else {
                ExitInfo {
                    code: None,
                    signal: s.signal().or(Some(fallback_signal)),
                }
            }
        }
        None => ExitInfo {
            code: None,
            signal: Some(fallback_signal),
        },
    }
}

/// Classify a supervisor `EndReason` into an `Outcome` + child `ExitInfo`.
#[cfg(unix)]
pub fn classify(end: &EndReason) -> (Outcome, ExitInfo) {
    use std::os::unix::process::ExitStatusExt;
    match end {
        EndReason::Exited(status) => {
            if let Some(code) = status.code() {
                let outcome = if code == 0 {
                    Outcome::Completed
                } else {
                    Outcome::Failed
                };
                (
                    outcome,
                    ExitInfo {
                        code: Some(code),
                        signal: None,
                    },
                )
            } else {
                // No exit code => terminated by a signal (WIFSIGNALED).
                (
                    Outcome::Crashed,
                    ExitInfo {
                        code: None,
                        signal: status.signal(),
                    },
                )
            }
        }
        EndReason::TimedOut { child_status } => (
            Outcome::TimedOut,
            exit_info_from_status(child_status, libc::SIGKILL),
        ),
        EndReason::Interrupted { child_status, .. } => (
            Outcome::Interrupted,
            exit_info_from_status(child_status, libc::SIGKILL),
        ),
        EndReason::SpawnFailed(_) | EndReason::SuperviseFailed(_) => (
            Outcome::Crashed,
            ExitInfo {
                code: None,
                signal: None,
            },
        ),
    }
}

/// Assemble the full `result.json` verdict for a finished run.
#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
pub fn build_verdict(
    harness: &str,
    end: &EndReason,
    events: &[Value],
    final_message: Option<String>,
    log_path: &Path,
    prompt_path: &Path,
    duration: Duration,
) -> Verdict {
    let (outcome, exit) = classify(end);
    let error_detail = match end {
        EndReason::SpawnFailed(m) | EndReason::SuperviseFailed(m) => Some(m.clone()),
        _ => None,
    };
    let interrupt_signal = match end {
        EndReason::Interrupted { trigger, .. } => Some(*trigger),
        _ => None,
    };
    Verdict {
        outcome: outcome.as_str(),
        final_message,
        exit,
        harness: harness.to_string(),
        log_path: log_path.to_string_lossy().into_owned(),
        prompt_path: prompt_path.to_string_lossy().into_owned(),
        event_count: events.len(),
        duration_ms: duration.as_millis() as u64,
        error_detail,
        interrupt_signal,
    }
}

/// Write the verdict to `path` ATOMICALLY (write a sibling `.tmp` then
/// rename). A tailing orchestrator must never observe a torn verdict: the
/// "log exists but no result.json => hard-killed" recovery heuristic depends
/// on `result.json` appearing whole or not at all.
pub fn write_result_json(path: &Path, verdict: &Verdict) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(verdict)?;
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    #[test]
    fn zero_exit_is_completed() {
        let end = EndReason::Exited(ExitStatus::from_raw(0));
        let (outcome, exit) = classify(&end);
        assert_eq!(outcome, Outcome::Completed);
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(exit.code, Some(0));
        assert_eq!(exit.signal, None);
    }

    #[test]
    fn nonzero_exit_is_failed() {
        // raw wait status for "exited with code 7" is `7 << 8`.
        let end = EndReason::Exited(ExitStatus::from_raw(7 << 8));
        let (outcome, exit) = classify(&end);
        assert_eq!(outcome, Outcome::Failed);
        assert_eq!(outcome.exit_code(), 1);
        assert_eq!(exit.code, Some(7));
    }

    #[test]
    fn signal_death_is_crashed() {
        // raw wait status for "killed by signal 9" is `9` (WIFSIGNALED).
        let end = EndReason::Exited(ExitStatus::from_raw(libc::SIGKILL));
        let (outcome, exit) = classify(&end);
        assert_eq!(outcome, Outcome::Crashed);
        assert_eq!(outcome.exit_code(), 5);
        assert_eq!(exit.code, None);
        assert_eq!(exit.signal, Some(libc::SIGKILL));
    }

    #[test]
    fn timed_out_reflects_child_death_signal() {
        // The child was SIGKILLed by the timeout path; exit mirrors that.
        let end = EndReason::TimedOut {
            child_status: Some(ExitStatus::from_raw(libc::SIGKILL)),
        };
        let (outcome, exit) = classify(&end);
        assert_eq!(outcome, Outcome::TimedOut);
        assert_eq!(outcome.exit_code(), 3);
        assert_eq!(exit.signal, Some(libc::SIGKILL));
    }

    #[test]
    fn timed_out_without_status_falls_back_to_sigkill() {
        let (outcome, exit) = classify(&EndReason::TimedOut { child_status: None });
        assert_eq!(outcome, Outcome::TimedOut);
        assert_eq!(exit.signal, Some(libc::SIGKILL));
    }

    #[test]
    fn interrupted_exit_reflects_child_disposition_not_trigger() {
        // Trigger was SIGINT, but the child honored the graceful stop and died
        // by SIGTERM. exit.signal must be the child's death (SIGTERM), while
        // interrupt_signal (built in the verdict) carries the trigger.
        let end = EndReason::Interrupted {
            trigger: libc::SIGINT,
            child_status: Some(ExitStatus::from_raw(libc::SIGTERM)),
        };
        let (outcome, exit) = classify(&end);
        assert_eq!(outcome, Outcome::Interrupted);
        assert_eq!(outcome.exit_code(), 4);
        assert_eq!(exit.signal, Some(libc::SIGTERM));

        let verdict = build_verdict(
            "opencode",
            &end,
            &[],
            None,
            Path::new("s"),
            Path::new("p"),
            Duration::from_millis(1),
        );
        assert_eq!(verdict.interrupt_signal, Some(libc::SIGINT));
        assert_eq!(verdict.exit.signal, Some(libc::SIGTERM));
    }

    #[test]
    fn write_result_json_is_atomic_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("result.json");
        let verdict = build_verdict(
            "opencode",
            &EndReason::Exited(ExitStatus::from_raw(0)),
            &[serde_json::json!({"type": "text"})],
            Some("the answer".to_string()),
            &dir.path().join("stream.jsonl"),
            &dir.path().join("prompt.txt"),
            Duration::from_millis(1234),
        );
        write_result_json(&path, &verdict).unwrap();

        // No stray .tmp left behind.
        assert!(!dir.path().join("result.json.tmp").exists());

        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["outcome"], "completed");
        assert_eq!(v["final_message"], "the answer");
        assert_eq!(v["harness"], "opencode");
        assert_eq!(v["event_count"], 1);
        assert_eq!(v["duration_ms"], 1234);
        assert_eq!(v["exit"]["code"], 0);
    }
}
