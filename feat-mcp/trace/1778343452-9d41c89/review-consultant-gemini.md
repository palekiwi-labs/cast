# Code Review: Slice 4 (MCP Handler)
**Reviewer:** @consultant-gemini
**Date:** Sat May 09 2026

## Summary
The implementation is fundamentally sound and structurally prepared for the final transport integration. No critical security issues were found. The bypass of `#[tool_router]` is correct for dynamic tools.

## Findings

### 🔴 Major
1. **Schema Re-compilation:** `jsonschema::validator_for` is called on every request. This is expensive. **Recommendation:** Pre-compile validators in `McpHandler::new`.
2. **Incorrect Error Code:** "Tool Not Found" uses `MethodNotFound` (-32601). **Recommendation:** Use `InvalidParams` (-32602) as the method exists but the name parameter is invalid.
3. **Missing Integration Tests:** No tests exercise the full pipeline from `McpHandler` down to `exec::run_command`. **Recommendation:** Add a direct `handler.call_tool` test.

### 🟡 Minor / Suggestions
- Use `.as_ref()` for `Cow<str>` instead of `&*`.
- Add `tracing::error!` logging for internal failures.
- Empty schemas should ideally be explicit `{"type": "object", "properties": {}}`.

## Verdict
**Approve with changes.**
