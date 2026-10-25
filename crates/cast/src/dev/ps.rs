//! `cast ps`: list running instance containers from their labels.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dev::instance::{LABEL_PREFIX, LABEL_SCHEMA};
use crate::docker::client::DockerClient;

/// Version of the `cast ps` JSON output; bump when fields change meaning.
const OUTPUT_VERSION: u32 = 1;

/// Docker's zero time, reported as StartedAt for never-started containers.
const DOCKER_ZERO_TIME: &str = "0001-01-01T00:00:00Z";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Inspected {
    id: String,
    name: String,
    created: String,
    state: InspectedState,
    config: InspectedConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InspectedState {
    status: String,
    started_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InspectedConfig {
    #[serde(default)]
    labels: Option<HashMap<String, String>>,
}

/// A labelled instance container as reported by `cast ps`.
#[derive(Debug, Serialize)]
pub struct Instance {
    pub id: String,
    pub container: String,
    pub name: Option<String>,
    pub workspace: Option<String>,
    pub workspace_id: Option<String>,
    pub repo: Option<String>,
    pub cast_version: Option<String>,
    /// Docker's `.State.Status`, verbatim.
    pub state: String,
    pub created: i64,
    pub started: Option<i64>,
}

#[derive(Serialize)]
struct Listing {
    version: u32,
    instances: Vec<Instance>,
}

fn to_value(listing: Listing) -> Value {
    serde_json::to_value(listing).expect("listing serializes")
}

/// The `cast ps` JSON document.
fn listing(instances: Vec<Instance>) -> Value {
    to_value(Listing {
        version: OUTPUT_VERSION,
        instances,
    })
}

fn instances_from_inspect(output: &str) -> Result<Vec<Instance>> {
    let inspected: Vec<Inspected> =
        serde_json::from_str(output).context("failed to parse docker inspect output")?;
    let instances = inspected
        .into_iter()
        .map(|c| {
            let mut labels = c.config.labels.unwrap_or_default();
            let mut label = |key: &str| labels.remove(&format!("{LABEL_PREFIX}.{key}"));
            let started = if c.state.started_at == DOCKER_ZERO_TIME {
                None
            } else {
                Some(rfc3339_to_unix(&c.state.started_at)?)
            };
            Ok(Instance {
                container: c.name.trim_start_matches('/').to_string(),
                name: label("name"),
                workspace: label("workspace"),
                workspace_id: label("workspace_id"),
                repo: label("repo"),
                cast_version: label("version"),
                created: rfc3339_to_unix(&c.created)?,
                id: c.id,
                state: c.state.status,
                started,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(instances)
}

/// Parses a UTC RFC 3339 timestamp as Docker prints it
/// (`YYYY-MM-DDTHH:MM:SS[.frac]Z`) into Unix seconds, dropping the fraction.
fn rfc3339_to_unix(s: &str) -> Result<i64> {
    let parse = || -> Option<i64> {
        let b = s.as_bytes();
        if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
            return None;
        }
        if b[13] != b':' || b[16] != b':' || !s.ends_with('Z') {
            return None;
        }
        let frac = &s[19..s.len() - 1];
        if !(frac.is_empty()
            || frac.starts_with('.') && frac[1..].bytes().all(|c| c.is_ascii_digit()))
        {
            return None;
        }
        let num = |r: std::ops::Range<usize>| -> Option<i64> {
            let part = s.get(r)?;
            part.bytes()
                .all(|c| c.is_ascii_digit())
                .then(|| part.parse().ok())?
        };
        let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
        let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
            return None;
        }
        Some(days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60 + ss)
    };
    match parse() {
        Some(t) => Ok(t),
        None => bail!("unsupported timestamp: {s}"),
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Finds labelled instance containers, narrowed by extra `label=` filters
/// (`key=value` under the cast prefix). `all` includes stopped containers.
pub fn find(docker: &DockerClient, all: bool, labels: &[(&str, &str)]) -> Result<Vec<Instance>> {
    let mut args = vec!["ps".to_string(), "-q".to_string(), "--no-trunc".to_string()];
    if all {
        args.push("-a".to_string());
    }
    let filters = std::iter::once(("schema", LABEL_SCHEMA)).chain(labels.iter().copied());
    for (key, value) in filters {
        args.push("--filter".to_string());
        args.push(format!("label={LABEL_PREFIX}.{key}={value}"));
    }
    let ids = docker.query_command(args)?;
    let ids: Vec<String> = ids.split_whitespace().map(str::to_string).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["inspect".to_string()];
    args.extend(ids);
    instances_from_inspect(&docker.query_command(args)?)
}

/// Lists running instance containers as the `cast ps` JSON document.
pub fn list(docker: &DockerClient) -> Result<Value> {
    Ok(listing(find(docker, false, &[])?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    const INSPECT: &str = r#"[
      {
        "Id": "abc123",
        "Name": "/cast-dev-1a2b3c-review",
        "Created": "2026-05-01T10:00:00.123456789Z",
        "State": {"Status": "running", "StartedAt": "2026-05-01T10:00:01.5Z"},
        "Config": {"Labels": {
          "com.palekiwi.cast.schema": "1",
          "com.palekiwi.cast.workspace": "/home/u/repo",
          "com.palekiwi.cast.workspace_id": "1a2b3c",
          "com.palekiwi.cast.repo": "/home/u/repo/.git",
          "com.palekiwi.cast.version": "0.3.0",
          "com.palekiwi.cast.name": "review"
        }}
      },
      {
        "Id": "def456",
        "Name": "/cast-dev-9z",
        "Created": "1970-01-02T00:00:00Z",
        "State": {"Status": "created", "StartedAt": "0001-01-01T00:00:00Z"},
        "Config": {"Labels": {
          "com.palekiwi.cast.schema": "1",
          "com.palekiwi.cast.workspace": "/w",
          "com.palekiwi.cast.workspace_id": "9z",
          "com.palekiwi.cast.repo": "/w/.git",
          "com.palekiwi.cast.version": "0.3.0"
        }}
      }
    ]"#;

    #[test]
    fn converts_inspect_output_to_listing() {
        assert_eq!(
            listing(instances_from_inspect(INSPECT).unwrap()),
            json!({
                "version": 1,
                "instances": [
                    {
                        "id": "abc123",
                        "container": "cast-dev-1a2b3c-review",
                        "name": "review",
                        "workspace": "/home/u/repo",
                        "workspace_id": "1a2b3c",
                        "repo": "/home/u/repo/.git",
                        "cast_version": "0.3.0",
                        "state": "running",
                        "created": 1_777_629_600,
                        "started": 1_777_629_601
                    },
                    {
                        "id": "def456",
                        "container": "cast-dev-9z",
                        "name": null,
                        "workspace": "/w",
                        "workspace_id": "9z",
                        "repo": "/w/.git",
                        "cast_version": "0.3.0",
                        "state": "created",
                        "created": 86_400,
                        "started": null
                    }
                ]
            })
        );
    }

    #[test]
    fn rejects_malformed_timestamp() {
        assert!(rfc3339_to_unix("yesterday").is_err());
    }
}
