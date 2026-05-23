use similar::{ChangeTag, TextDiff};

/// Format a unified text diff between two configuration JSON values.
///
/// Returns a plain string with `+`/`-`/` ` line prefixes (no ANSI codes).
/// Callers are responsible for applying colors when printing to a terminal.
pub fn format_config_diff(old: &serde_json::Value, new: &serde_json::Value) -> String {
    let old_str = serde_json::to_string_pretty(old).unwrap_or_default();
    let new_str = serde_json::to_string_pretty(new).unwrap_or_default();

    let diff = TextDiff::from_lines(&old_str, &new_str);
    let mut output = String::new();

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{}\n", hunk.header()));
        for change in hunk.iter_changes() {
            let prefix = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            output.push_str(&format!("{}{}", prefix, change));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_diff_shows_changed_value() {
        let old = json!({ "memory": "1024m", "cpus": 1.0 });
        let new = json!({ "memory": "2048m", "cpus": 1.0 });

        let diff = format_config_diff(&old, &new);

        assert!(diff.contains("-"), "Should have removal lines");
        assert!(diff.contains("+"), "Should have addition lines");
        assert!(diff.contains("1024m"), "Should show old value");
        assert!(diff.contains("2048m"), "Should show new value");
    }

    #[test]
    fn test_diff_identical_values_is_empty() {
        let config = json!({ "memory": "1024m" });
        let diff = format_config_diff(&config, &config);
        assert!(diff.is_empty(), "Diff of identical values should be empty");
    }

    #[test]
    fn test_diff_shows_added_field() {
        let old = json!({ "memory": "1024m" });
        let new = json!({ "memory": "1024m", "cpus": 2.0 });

        let diff = format_config_diff(&old, &new);
        assert!(diff.contains("+"), "Should show added lines");
        assert!(diff.contains("cpus"), "Should mention added field");
    }
}
