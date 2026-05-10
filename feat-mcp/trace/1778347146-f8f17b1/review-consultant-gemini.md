# Code Review: Slices 4.5 & 5 — McpHandler Refactor + Server Infrastructure
**Reviewer:** @consultant-gemini
**Date:** Sat May 10 2026

## Summary
The architecture, Inner Pattern adoption, error handling, and Tokio/Axum integration are all well-written. The two major findings from the previous review have been correctly resolved. One **new major security issue** was identified in the server binding address.

## 🔴 Major

**1. Binding to `0.0.0.0` exposes the server to the entire LAN** (`server.rs:35`)
Binding to all network interfaces allows anyone on the same network to reach the MCP server and potentially execute tools on the host.
- **Recommendation:** Default to `127.0.0.1`. Add an explicit `--host` flag for opt-in to `0.0.0.0`.

**2. No authentication layer**
No credential is required to invoke tools once the host is reached.
- **Recommendation:** Generate a random Bearer token at startup and require it in the `Authorization` header. (Deferred for now).

## 🟡 Minor / Suggestions

**1. Host env capture** (`server.rs:12`)
`std::env::vars()` captures the entire host environment. 
- **Recommendation:** Consider an `env_whitelist` in the future.

**2. `eprintln!` vs `tracing`**
Standard practice for CLI banners; no action needed.

## ✅ Confirmations
- **Factory pattern** (`move || Ok(handler.clone())`): Correct and idiomatic.
- **`validators` / `cached_tools` synchronization**: Solidly implemented in `new()`.
- **`expect()` in `execute_tool`**: Invariant is sound.
- **`allowed_hosts` config**: Provides DNS rebinding protection.

## Verdict
**Approve with changes.** Fix the bind address before this is safe for general use.