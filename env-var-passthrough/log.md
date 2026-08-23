# Project Log

## [4f8ccff] Redesigned env passthrough as approval-gated name allowlist

Resolved the security question raised against the original CAST_ENV__ design before any implementation. Rewrote the task card Design and Acceptance Criteria, and wrote plan/index.md plus an executive plan.

- **Found:** Env::prefixed("CAST_") is merged into Config (config/loader.rs:42) and the approval hash covers serde_json(Config) (config/approval.rs:22-34), so env vars ALREADY participate in approval today. The original design's Config exemption was a hole, not a safe default.
- **Found:** ApprovalStore persists approved_config as cleartext JSON at ~/.local/share/cast/approved_configs.json (approval.rs:94,200), so any hashed value is written to disk and printed by `cast config diff`. Folding secret VALUES into Config is therefore unacceptable.
- **Found:** figment merge (Order::Merge) replaces arrays wholesale and only recurses into dicts (coalesce.rs:30-32), so a Vec<String> config key does not merge across global and project files. Existing nix_extra_substituters and forbidden_paths already behave this way.
- **Found:** DockerClient spawns Command::new("docker") with no env customisation (client.rs:135,160,205), so the child inherits cast's env and the valueless `-e NAME` form needs no plumbing at the spawn site.
- **Decided:** Replace the CAST_ENV__ prefix with env_passthrough: BTreeMap<String, bool> on Config. Names are approval-gated config; values are read from the host env at run time and never hashed, stored, or logged.
- **Decided:** Use a map rather than Vec<String> so global and project allowlists merge (matching extra_data_volumes precedent) and a project can opt out of an inherited name with false.
- **Decided:** Emit the valueless `-e NAME` form so secrets never appear in cast's argv and are not visible via ps to other host users.
- **Decided:** Rejected switching loader.rs to admerge: it would change merge semantics for every existing Vec field and make global forbidden_paths entries unremovable.
- **Decided:** Accepted residual risk: approval gates which names cross the boundary, not the values the host shell put in them. To be stated plainly in env-overrides.md rather than papered over.
- **Open:** Whether load_config_from can be driven with a fake global config path for the merge test, or whether the merge must be asserted at the figment level instead.

## [ab13c21] Phase 1 complete: env_passthrough config schema and merge

Committed ab13c21. Added env_passthrough: BTreeMap<String, bool> to Config with 5 new tests covering global/project merge, project-level opt-out via false, approval sensitivity, value exclusion from serialization, and the CAST_ENV_PASSTHROUGH__ ad-hoc form. Full suite green (245 + 18 + 9), clippy clean.

- **Found:** figment supports driving the hardcoded global config path in tests: figment::Jail (behind the `test` feature) relocates $HOME into a temp dir and restores env and cwd on drop, serialised by a global mutex. So AC 7 is testable through load_config_from with no signature change, and the AGENTS.md no-$HOME constraint is satisfied. The proposed optional-global-path parameter is unnecessary.
- **Found:** figment's Env provider lowercases keys by default, so CAST_ENV_PASSTHROUGH__GH_TOKEN yields the map key `gh_token`, not `GH_TOKEN`. Since env var names are conventionally uppercase, the phase 2 helper must normalise allowlist keys to uppercase before looking them up in the host environment. Pinned by test_env_passthrough_settable_via_cast_env_var.
- **Found:** Jail::create_file does not create intermediate directories; create_dir must be called first.
- **Found:** `cargo fmt --check` reports pre-existing import-ordering diffs in untouched files (commands/cli.rs, dev/build.rs, dev/image.rs), suggesting a rustfmt style/edition mismatch with the current toolchain. Not caused by this work; left alone.
- **Decided:** Kept load_config_from's signature unchanged, using figment::Jail rather than injecting a global config path, since the test-only need is fully served by the library.
- **Decided:** Added figment with the `test` feature as a dev-dependency.
- **Decided:** Suppressed clippy::result_large_err locally on the three Jail-based tests; the large Err variant is figment::Error, not ours.
- **Open:** Phase 2: decide whether uppercase normalisation should also accept a mixed-case cast.json key, and whether a name that normalises to a collision (GH_TOKEN and gh_token both present) should warn or silently dedupe.

## [4f8ccff-dirty] Rejected figment::Jail as unsound for parallel tests; dropped env nesting route

Reversal of two decisions from the previous entry, before any commit. Phase 1 code is complete and green on branch feat/env-passthrough (246 lib + 18 + 9 tests, clippy clean), uncommitted pending review.

- **Found:** figment::Jail contains no `unsafe` keyword only because figment compiles under a pre-2024 edition where std::env::set_var is still safe. It does mutate process-global state (jail.rs:322,353,361-362 set_var/remove_var; jail.rs:141 set_current_dir), and its LOCK (jail.rs:132) serialises only jail-against-jail. Other tests in the same binary run concurrently on other threads and are unprotected — a real getenv/setenv data race, which is precisely why set_var became unsafe in edition 2024. Our binary has ~245 parallel tests, several of which read $HOME.
- **Found:** figment's value parser handles compound literals (value/parse.rs:169-183): `[1,2,3]` parses as an array and `{a=b}` as a map. So a single env var can populate a whole collection field, e.g. CAST_ENV_PASSTHROUGH={GH_TOKEN=true}. Case is preserved because only the key path derived from the variable NAME is lowercased, not a parsed value.
- **Found:** The `gh_token` lowercasing problem was entirely self-inflicted by advertising the CAST_ENV_PASSTHROUGH__NAME nesting route. Dropping that route removes the need for any uppercase normalisation in the helper.
- **Decided:** Reverted to injecting the global config path: added load_config_with_global(base_dir, Option<&Path>), with load_config_from delegating to it. Removed the figment test-feature dev-dependency. Cargo.toml and Cargo.lock are untouched again.
- **Decided:** Dropped support for and documentation of the CAST_ENV_PASSTHROUGH__NAME form; the single-variable literal form is the env route if one is ever needed.
- **Decided:** Added two pinning tests: a Vec field (forbidden_paths) is replaced rather than merged across global/project, and env_passthrough defaults to empty.
- **Open:** Whether to keep BTreeMap<String, bool> or switch to Vec<String>. The map buys global/project merge and per-project opt-out; a Vec gives simpler config syntax but is replaced wholesale by figment's merge. Awaiting user decision.
- **Open:** Process: work must start on a feature branch. Phase 1 was initially committed to master (undone by the user) and is now on feat/env-passthrough.

## [4f8ccff-dirty] Allowlist shape settled: Vec&lt;String&gt; of names

User chose a list over the map. Phase 1 reworked and green (246 lib + 18 + 9, clippy clean) on feat/env-passthrough, still uncommitted.

- **Found:** Replacement precedence is fail-closed and therefore the better security posture, which inverts the argument I had made for the map. With a list, the worst case is an expected name being absent, which surfaces immediately inside the container. With a merging map, the effective allowlist is the union of global and project files, so a forgotten global name silently applies to every project and cannot be audited from one file.
- **Decided:** env_passthrough is Vec<String>, a plain list of names. A map of name to bool invites the misreading 'set GH_TOKEN to true' — unacceptable ambiguity for a security-relevant key.
- **Decided:** Accepted that a project cast.json replaces rather than extends the global allowlist, and pinned that behaviour with a test plus a schema doc comment explaining it is intended rather than incidental.
- **Decided:** Phase 1 tests now: project-replaces-global, global-applies-when-project-silent, defaults-empty, accepts-multiple-names, approval-status-changes-on-addition, name-serialized-but-not-value.
- **Open:** Phase 2: duplicate handling — a name listed twice should emit a single -e pair.
- **Open:** Phase 1 is complete and green but not committed; awaiting go-ahead.

## [31a8e71] Phase 1 committed on feat/env-passthrough

Commit 31a8e71 on branch feat/env-passthrough (base 4f8ccff). Phase 1 of task env-var-passthrough: env_passthrough: Vec&lt;String&gt; on Config, plus load_config_with_global for sound testing of the global config path. 4 files, 162 insertions. Tests 246 lib + 18 + 9 green, clippy clean. Acceptance criteria 6-9 have Evidence filled on the task card. Phase 2 (the pure helper in dev/env_file.rs) handed off to the builder sub-agent.

- **Found:** Only one caller outside the config module used load_config_from (crates/cast/tests/config_test.rs:31), so adding load_config_with_global alongside it required no call-site churn.
- **Decided:** Committed Phase 1 as a single atomic commit covering schema, precedence tests, approval tests, and the loader signature addition; all four files are one logical unit and the tree is green at that point.
- **Decided:** Recorded the figment::Jail rejection in the commit body as well as the plan, so the reasoning survives for anyone reading git history without the cue corpus.
- **Open:** Phase 2 duplicate handling: a name listed twice should emit a single -e pair (chosen over erroring, as dedupe is fail-safe and a duplicate is harmless user error).
- **Open:** Pre-existing cargo fmt --check diffs in commands/cli.rs, dev/build.rs, dev/image.rs (import ordering) suggest a rustfmt style mismatch with the current toolchain. Untouched by this work; may warrant a separate formatting commit.

## [915d69e] Phase 2 committed: build_env_passthrough_args helper

Commit 915d69e on feat/env-passthrough. Pure helper in dev/env_file.rs turning the allowlist plus an injected host env map into valueless `-e NAME` args, with 8 new tests. Suite 254 lib + 18 + 9 green, clippy clean. Phase 2 checkboxes ticked in the executive plan.</body>
<parameter name="found">["Sorting is by byte, so '_' (0x5F) orders after uppercase letters: an allowlist of _PRIVATE and VAR2 emits VAR2 first. My initial test asserted the intuitive alphabetic order and failed; the byte order is now pinned with an explaining comment rather than special-cased in the code.","There is a pre-existing, unrelated test named env_passthrough_from_launched_agent_only in dev/universal/volumes.rs. Filtering tests by 'env_passthrough' picks it up; not a conflict, but worth knowing when reading test output."]

- **Decided:** Deduped with a BTreeSet, which yields dedupe and sort in one step, over erroring on a duplicate name: a repeated name is harmless user error and silently collapsing it is fail-safe.
- **Decided:** Validated names with a hand-written is_valid_env_name rather than adding a regex dependency; the [A-Za-z_][A-Za-z0-9_]* rule is four lines of char checks.
- **Decided:** The debug! call logs the names slice only and is skipped entirely when empty.
- **Open:** Phase 3: whether the new RunOpts host env field should be the full std::env::vars() map or pre-filtered to the allowlist at the call site. Full map keeps build_docker_run_flags the single place that knows the policy; pre-filtering narrows what is carried around.

## [dfc7a38] Phase 3 wired: env passthrough in docker run flags, no RunOpts field

Commits be7585a (helper narrowed to names) and dfc7a38 (wiring) on feat/env-passthrough. Deviated from the plan's RunOpts field after consulting flash, glm and opus; the plan step is marked superseded with the reasoning inline. Suite 256 lib + 18 + 9 green, clippy clean.</body>
<parameter name="found">["cast ALREADY ships this exact mechanism at opencode/env.rs:47: PASSTHROUGH_VARS.iter().filter(|v| env.contains_key(v)).flat_map(|v| [\"-e\", v]) for 30+ provider API key names. Our helper is a config-driven generalization of it with sorting, dedupe and name validation added. That existing code passes the env map as a by-reference parameter and never stores it in RunOpts, so a RunOpts field would have made the new code the sole outlier.","Docker's CLI reads all --env-file entries into the env list first, then appends all --env entries, last occurrence winning. So every -e beats every --env-file REGARDLESS of argv position. Half of the plan's ordering rationale ('place after --env-file so it overrides cast.env') was therefore cosmetic; only placing before cast's own -e USER= does real work, because that precedence is positional.","RunOpts is the only struct in dev/ without #[derive(Debug)] (cf. SessionFlags, RunMode, TtyMode at run.rs:25/36/52), with no comment saying why. A silent load-bearing omission of exactly the kind a contributor 'fixes'.","Rust panic backtraces contain symbol names and addresses only, never argument values. My concern about a full env map leaking via a panic backtrace was unfounded and dropped from the reasoning.","Signature churn favours the parameter: build_docker_run_flags has 1 production and 14 test call sites, all in one file. A RunOpts field would have needed a new field in 10 full struct literals across 7 files.","contains_key is true for a set-but-empty var, and since -e beats --env-file, an empty GH_TOKEN= on the host would silently shadow a real value from cast.env. Fixed by filtering empty values when building the name set.","Cross-layer duplicate emission is possible and benign: allowlisting a name already in PASSTHROUGH_VARS emits -e NAME twice, and docker last-wins with the same value. Not worth cross-layer dedupe."]

- **Decided:** No RunOpts field. build_docker_run_flags takes host_env_names: &BTreeSet<String> as a parameter, reducing the data's reach from ~20 functions to 1.
- **Decided:** Narrowed build_env_passthrough_args from &BTreeMap<String,String> to &BTreeSet<String> in a separate refactor commit. The parameter-vs-field axis does not buy the security property; the TYPE does. Dropping values makes a leak unrepresentable rather than unlikely.
- **Decided:** Removed test_env_passthrough_never_emits_the_value: vacuous once no value is in scope. Runtime assertion traded for a compile-time guarantee, replaced by positional tests at the build_docker_run_flags level.
- **Decided:** Rejected converting HashMap to BTreeMap at the call site: it would clone every host secret into a second allocation purely to satisfy a type parameter that is never read.
- **Decided:** Rejected a generic membership trait bound as over-engineering for one operation.
- **Decided:** Documented the child-inherits-env invariant on both docker spawn sites rather than trying to enforce it in code; it guards two independent callers and predates this work.
- **Open:** ApprovedConfig is a newtype with Deref<Target=Config> (approval.rs:48-55), so build_docker_run_flags(&Config, ...) accepts an UNAPPROVED Config. The approval gate is enforced at run_in_container's signature, not structurally here. Fine with one production caller, but taking &ApprovedConfig would make the gate structural for env_passthrough.
- **Open:** dispatch_run appends extra_args AFTER docker_flags (run.rs:98-99), so agent passthrough -e args land after cast's own infra vars. The new config layer enforces cast authority positionally; the agent layer cannot. No conflict today only because PASSTHROUGH_VARS is disjoint from cast's names.
- **Open:** std::env::vars() at run.rs:151 panics on non-UTF-8 env vars; vars_os would not. Pre-existing, but that line is now more load-bearing.
- **Open:** Phase 4 remains: docs in crates/cast/docs/config/env-overrides.md, plus the trust-boundary note.

## [ae565f8] Phase 4 complete: docs written, task closed

Commit ae565f8 on feat/env-passthrough. Documented env_passthrough in crates/cast/docs/config/env-overrides.md with a cross-reference from config/reference.md. Suite 256 lib + 18 + 9 green, clippy clean. All 12 acceptance criteria have Evidence; task and both plans set to complete. Task env-var-passthrough is done, four commits on the branch: 31a8e71, 915d69e, be7585a, dfc7a38, ae565f8.

- **Found:** docs/README.md links config/overview.md as the section root and does not enumerate individual config pages, so no new entry was needed there. config/reference.md does enumerate fields, so env_passthrough was added next to forbidden_paths.
- **Found:** env-overrides.md was previously only about configuring cast itself via CAST_ prefixed vars. env_passthrough is the opposite direction (host into container), so the new section opens by naming that distinction rather than assuming the reader will infer it.
- **Found:** Two Design claims on the task card had gone stale against the implementation: 'value supplied to the docker child via Command::env' (superseded in phase 3 by relying on inheritance) and 'args emitted after --env-file so they override cast.env' (the override is real but non-positional). Both corrected in place.
- **Decided:** Documented the docker precedence rule as a behavioural guarantee ('passthrough beats cast.env') without leaning on argv position, so a future reordering of build_docker_run_flags does not silently falsify the docs.
- **Decided:** Documented the empty-value filter as user-visible behaviour ('unset or empty is skipped') rather than as an implementation note; it is the difference between a real cast.env value being used and being shadowed.
- **Decided:** AC 10 (values not logged) marked satisfied by the &BTreeSet<String> signature rather than requesting user attestation. The criterion asked for an attestation because at the time it was a runtime property; phase 3 made it a compile-time one, which is stronger evidence than a human confirming a log inspection.
- **Open:** The branch is unmerged and unpushed; master still lacks env_passthrough.
- **Open:** Carried forward from phase 3, unresolved and out of scope: build_docker_run_flags takes &Config rather than &ApprovedConfig, so the approval gate is enforced at run_in_container's signature rather than structurally; dispatch_run appends agent extra_args after docker_flags, so agent-layer passthrough cannot be overridden by cast's own vars; std::env::vars() at run.rs:151 panics on non-UTF-8 env vars.
- **Open:** Pre-existing cargo fmt --check import-ordering diffs in commands/cli.rs, dev/build.rs, dev/image.rs remain untouched and may warrant a separate formatting commit.

## [ae565f8] Branch diff reviewed by gemini-flash and opus

- **Found:** Both reviewers confirm core security design is airtight: names-only boundary holds; no path found where secret values reach argv, logs, cast.json, approval store, or config diff
- **Found:** Docker flag ordering reasoning verified correct by both (readKVStrings collects --env-file first, then --env; last-occurrence-wins at daemon)
- **Found:** opus MAJOR: loader.rs:146-150 doc comment is stale from an earlier design (claims field is a map; claims global allowlist must survive project config) and contradicts both the Vec<String> schema and the replace-semantics tests
- **Found:** opus MAJOR: 'auditable by reading a single file' claim in env-overrides.md and schema.rs is false: CAST_ENV_PASSTHROUGH env var outranks both cast.json files (opus verified empirically; injected name still trips re-approval via merged-config hash, so design is safe but docs are wrong)
- **Found:** opus MAJOR + gemini minor: PATH/HOME/NIX_REMOTE passthrough footgun: image-level ENV (Dockerfile.dev) is the docker defaults list, so an allowlisted PATH or HOME clobbers container infrastructure; docs currently claim this cannot happen; opus proposes RESERVED_NAMES filter with warn!
- **Found:** opus MINOR: approval.rs test_env_passthrough_serializes_name_but_never_value is tautological ('super-secret-value' never introduced); suggested shape-based assertion instead
- **Found:** opus MINOR: invalid/unset names dropped silently, no diagnostic; suggest warn!/debug! per dropped name
- **Found:** opus MINOR: ordering test uses non-colliding GH_TOKEN; suggested a real collision test with USER and CAST_MCP_URL; also missing --env-file ordering, headless, and end-to-end empty-var tests
- **Found:** opus MINOR: formatting churn from wrong rustfmt style edition (import reorders in loader.rs, run.rs, client.rs; repo rustfmt reverses it)
- **Found:** Shared nits: std::env::vars() panics on non-UTF-8 host env (pre-existing, now load-bearing); agent passthrough path forwards set-but-empty vars while config path excludes them; empty-value filter in run_in_container lacks isolated unit test
- **Decided:** Verdicts: gemini-flash approve-with-comments; opus request-changes (narrowly scoped, all cheap fixes)
- **Decided:** No code changes made yet; awaiting user decision on which findings to address

## [604d47b] Commit 1/4: strip redundant rationale comments

- **Decided:** Kept contract facts (skip/dedup/sort, docker ordering, client env-inheritance invariants); dropped design-history essays
- **Decided:** Also restored repo rustfmt import order in loader.rs, run.rs, client.rs, fixing opus finding 7 (branch-introduced churn only; pre-existing violations in cli.rs/build.rs/image.rs left alone)
- **Decided:** approval.rs tautological test comment left for the next commit where the test itself gets fixed

## [fdfa068] Commit 2/4: fix tautological approval test

- **Decided:** Replaced vacuous 'super-secret-value not contained' assert with shape-based assert: env_passthrough must serialize as array of plain strings (no value channel exists to leak into)

## [6ce16c1] Commit 3/4: reserved-names filter

- **Decided:** Reserved set minimal by design: PATH, HOME, NIX_REMOTE only (forwarding cannot work at all), not SHELL/LD_PRELOAD/LD_LIBRARY_PATH (dangerous-but-realisable; approval gates the channel per trust-boundary doc)
- **Decided:** Hard drop + warn! inside build_env_passthrough_args, next to existing name filters — enforcement at the moment it matters, not at config load
- **Decided:** Public RESERVED_ENV_NAMES const so the list is auditable/greppable

## [977ea38] Commit 4/4: docs precedence corrections

- **Decided:** Corrected half-true infra claim: passthrough beats cast.env AND image ENV, loses only to vars cast emits itself; documented reserved-name drop
- **Decided:** Replaced false 'single file' audit claim: CAST_ENV_PASSTHROUGH outranks both cast.json files; audits go through 'cast config diff'; env-injected names still trip re-approval
- **Decided:** All four commits follow review-fix scope agreed with user: comment strip, tautological test, reserved filter, doc claims

## [977ea38] Final diff review by opus: approve-with-comments, all 4 fix commits confirmed

Final verification review of feat/env-passthrough (4f8ccff..977ea38) by diff-reviewer-opus. All four review-fix commits confirmed landed correctly with file:line evidence (604d47b comment strip + rustfmt scoping exact; fdfa068 shape-based assertion has real failure modes; 6ce16c1 reserved filter minimal and value-channel-free; 977ea38 precedence claims verified against run.rs:409-455 and loader merge order). No regressions. Core security property CONFIRMED structurally: values unrepresentable end-to-end (Vec<String> schema -> approval hash over names -> run_in_container projects to names -> BTreeSet<String> signatures -> valueless -e argv -> env inheritance only). Reviewer independently ran cargo test --workspace, clippy, fmt: green. New findings: N1-N3 MINOR (all docs/observability, none security), N4-N7 NITs. Awaiting user decision on which to address before merge.

- **Found:** All 4 fix commits CONFIRMED: comment strip exact-scoped (fmt diffs remain only in 3 pre-existing + 6 untouched mcp-client files), tautological test now fails on map shape, list-of-records, extra or non-string elements, reserved filter drops only (cannot add/reorder/introduce value channel), docs precedence verified arg-by-arg against run.rs:409-455
- **Found:** N1 MINOR: env-overrides.md:64-66 prescribes 'cast config diff' for auditing the effective allowlist, but config diff prints a diff against the approved snapshot and prints nothing once approved; 'cast config show' is the command that prints the merged result (manual-qa.md already uses show)
- **Found:** N2 MINOR: CAST_ENV_PASSTHROUGH is newly advertised with no syntax example; the naive unbracketed CAST_ENV_PASSTHROUGH=GH_TOKEN hard-fails EVERY cast subcommand including config show (figment v.parse() yields Value::String for unbracketed input, ConfiguredValueDe forwards seq to deserialize_any -> invalid type error). Correct form is the bracketed literal '[GH_TOKEN, NPM_TOKEN]'
- **Found:** N3 MINOR: the reserved-name warn! at env_file.rs:55 goes to a file-only tracing subscriber (logging.rs:27-37, no stderr layer anywhere), so the user sees zero terminal feedback when PATH/HOME/NIX_REMOTE are silently dropped; also const doc at env_file.rs:32-35 says 'dropped rather than warned about' contradicting the warn! at :55
- **Found:** N4 NIT: reserved filter warns before host-presence filter and dedup, so unset reserved names and duplicates still warn; N5 NIT: doc example lists ANTHROPIC_API_KEY which every agent already forwards unconditionally (dev/opencode/env.rs:8 etc.), making the second entry a no-op that only adds duplicate argv; N6 NIT: reserved list duplicated in docs prose with no drift guard; N7 NIT: adding the field changes every config's serialized hash, so all existing approvals trip Changed on upgrade (inherent to design, undocumented)
- **Found:** Reviewer verified docker child has no env_clear() anywhere on the docker path (only mcp/exec.rs:80, an unrelated MCP subprocess) and both INVARIANT comments survive in condensed form
- **Decided:** No code changes made from this review; per prior round convention, findings await user triage before any fix commits
- **Open:** User to decide which of N1-N7 to address; N1 and N2 are inaccuracy introduced by the docs-fix commit itself and are the strongest candidates (both are small doc edits)
- **Open:** N3 needs a product decision: eprintln alongside warn!, or surfacing reserved-name feedback via config show/allow, or accepting file-only logging
- **Open:** Manual QA sign-off (AC 13, todo/1786901418-ae565f8/manual-qa.md) still outstanding; branch unpushed/unmerged

## [6a7c88e] Commit 6a7c88e: N1+N2 doc fixes with pinned syntax test; N3 pattern research

Commit 6a7c88e on feat/env-passthrough: env-overrides.md audit command corrected to `cast config show` with the diff-silence caveat (N1), bracketed CAST_ENV_PASSTHROUGH literal documented with the every-invocation-errors failure mode (N2), pinned by a new figment Value-level test in loader.rs. Suite 259 lib + 18 + 9 green, clippy clean. N3 research delivered separately: the codebase has an explicitly documented dual-channel convention for exactly this problem.

- **Found:** N3 answer: strong established eprintln! convention. Two sites carry the exact documented pattern — nix_daemon/daemon.rs:28-30 and dev/image.rs:36-39: '// info! writes to the file log; eprintln! writes to the console. // Status messages go to stderr so stdout stays clean for pipelines.' — the dual-channel info!+eprintln! pair is the canonical idiom, with rationale (stdout must stay clean for `cast run --headless --format json`)
- **Found:** dev/run.rs itself already uses the pair at lines 251-253 (loading global devshell), 283-287 and 309-313 (scaffold notices): info!/warn! to file log, eprintln! for user-visible console messages
- **Found:** cast-mcp-client has a Warning-prefixed variant: eprintln!("Warning: server '{}' is unreachable: {}", ...) at list.rs:65, generate.rs:82, config/loader.rs:6,72 — user-facing warnings on stderr with a 'Warning:' prefix
- **Found:** main.rs:9 uses eprintln!("Error: {:#}", e) as the top-level error channel
- **Found:** figment 0.10.19 Env provider reads process env only (no from_iter injection API), but `impl FromStr for Value` is public, infallible, and is the exact code the env route parses through (parse.rs:101-107); bracketed -> Value::Array, unbracketed -> Value::String, and elements keep case (only key paths are lowercased)
- **Found:** The stray working-tree import flip existed before this session and matches what pinned rustfmt 1.8.0 wants; the branch's committed Figment-first order (604d47b, matching master 4f8ccff) is flagged by the CURRENT pinned toolchain — the known fmt-mismatch open item now demonstrably affects loader.rs too, strengthening the case for the deferred repo-wide formatting commit
- **Decided:** N1 fix: `cast config show` (prints merged config) replaces `cast config diff` (prints only changes vs approved snapshot, silent once approved); parenthetical added so readers understand why diff is wrong for auditing
- **Decided:** N2 fix: documented `export CAST_ENV_PASSTHROUGH='[GH_TOKEN, NPM_TOKEN]'` plus the failure mode; single quotes because the bracket literal is shell glob syntax
- **Decided:** Pinned the doc claim with test_cast_env_passthrough_requires_bracketed_list_literal at the figment Value level (FromStr for Value is public and infallible); rejected setting a real env var in the test — it would race parallel load_config_from tests, the same Jail soundness reason; figment 0.10.19 has no Env::from_iter so the provider itself is not hermetically testable
- **Decided:** Restored loader.rs imports to HEAD's Figment-first order after finding a stray uncommitted providers-first flip in the working tree at session start (source unknown, possibly a cargo fmt run between sessions). Reasoning: pinned fenix-stable rustfmt 1.8.0 wants providers-first, but reversing reviewed commit 604d47b for one file while cli.rs/build.rs/image.rs stay old-style would fragment style ahead of the deferred dedicated formatting commit
- **Open:** N3 implementation decision awaits user: adding eprintln! alongside the existing warn! in build_env_passthrough_args would follow the documented dual-channel convention verbatim (info!/warn! file log + eprintln! console), with the 'Warning:' prefix per cast-mcp-client precedent; note the helper is otherwise pure with injected test seams, so the console line would also fire in unit tests unless the eprintln lives at the run_in_container call boundary instead
- **Open:** env_file.rs:32-35 const doc still says names are 'dropped rather than warned about' while the code warn!s at line 55 (contradiction flagged by opus as part of N3) — worth a one-line doc fix whenever N3 is addressed
- **Open:** N4-N7 nits and manual QA sign-off remain open as before

## [d86c37f] Commits 7903e5d+d86c37f: N3 console warning; active file writer countered

Commits 7903e5d + d86c37f on feat/env-passthrough implement review finding N3 per user decisions: helper stays pure (no eprintln inside build_env_passthrough_args, so its unit tests print nothing), console warning lives at the run_in_container boundary, and the RESERVED_ENV_NAMES doc contradiction is fixed. Suite 261 lib + 18 + 9 green, clippy clean.

- **Found:** CONCURRENT WRITER: run.rs was being mutated in the working tree during the turn — 604d47b's reviewed import order and assert wrapping flipped back to pre-604d47b forms, and my eprintln block was stripped mid-turn (grep found it present, then absent). The reverts were surgical: only 604d47b's hunks, preserving newer edits. No rust-analyzer/cargo fmt/watchexec/bacon process was running (ps checked); mtime tracked the rewrites. Same signature as the stray loader.rs flip found at session start. Cause unknown from inside the session — user should check for a stale editor buffer (pre-604d47b session) with autosave, or any tooling that rewrites files
- **Found:** Current rustfmt (1.8.0, style edition 2024 via edition=2024) actually AGREES with master/604d47b forms: the fmt --check +lines wanted {ResolvedWorkspace, get_workspace}, BuildOptions before docker::args, wrapped asserts. My earlier conclusion that the pinned toolchain 'wants providers-first' in loader.rs was drawn from a phantom-polluted diff; the true remaining fmt offenders (cli.rs, build.rs, image.rs, possibly loader.rs imports) still merit the deferred repo-wide formatting pass
- **Found:** Committed 7903e5d with the eprintln block DOUBLED: my construction script ran its perl substitution twice (once on run.rs.head, then cp to .target and again). Rust shadows the repeated let so it compiled and all 261 tests passed; symptom would have been the warning printing twice. Caught by post-commit occurrence-count verification (git show grep), fixed in d86c37f (exactly 16 deletions), reconstructed from 6a7c88e with single application and verified by count + diff before staging
- **Found:** Technique for committing under an active file writer: build exact desired content in /tmp from a git show extract, verify by diff/occurrence-count against the parent commit, then cp + git add in one chain (the index captures content the writer cannot touch), test, commit, and verify committed content via git show — never trust working-tree greps
- **Decided:** New pure helper reserved_names_in(allowlist) -> BTreeSet<String> in env_file.rs, TDD (2 tests: dedupe+sort+ignore-others, empty-when-none), gives the boundary the drop set without values or side effects
- **Decided:** eprintln at run_in_container right after host_env_names construction, following the documented dual-channel convention (warn! file log + eprintln console, cf. daemon.rs:28, image.rs:36) with the cast-mcp-client 'Warning:' prefix; message names the entries dynamically (avoids N6 prose drift) and tells the user to remove them
- **Decided:** build_env_passthrough_args warn! now fires once per unique reserved name (pre-computed BTreeSet) instead of per allowlist occurrence, aligning file-log and console cardinality and fixing the duplicate half of nit N4
- **Decided:** Const doc rewritten: names pass config approval like any other entry and are dropped at arg-build time with a warning — replacing the false 'dropped rather than warned about at load time' claim
- **Decided:** env-overrides.md reserved bullet now says 'always dropped, with a warning on stderr'
- **Open:** User should identify the concurrent writer (stale editor buffer with autosave is the leading hypothesis); until then, verify run.rs/loader.rs working-tree state against git before blaming code
- **Open:** Working tree was clean at end of turn (writer dormant or satisfied); branch now at d86c37f with 7 commits since 4f8ccff
- **Open:** Remaining review nits N4 (unset reserved names still warn — arguably correct), N5 (doc example ANTHROPIC_API_KEY no-op), N6 (docs prose list drift), N7 (approval invalidation on upgrade) untouched by agreement
- **Open:** Manual QA sign-off (AC 13) still outstanding; branch unpushed/unmerged

## [d86c37f] Pivot settled: base + extra_env_passthrough, zero-trace clean break

User confirmed the base + extra redesign direction and decided every open point. The pivot now has a complete design; implementation awaits task card and plan updates, then appended commits on feat/env-passthrough.

- **Found:** The user already reopened the task card to status in-progress for this pivot (was complete at ae565f8).
- **Found:** Exactly two task cards carry tag: 0.2.0 (env-var-passthrough, design-repo-defined-shells); the tagging key is 'tag' in frontmatter.
- **Found:** crates/cast Cargo.toml is already version 0.2.0 (bumped at the universal-container merge per task tag-0-1-0-release); the only git tag is v0.1.0; cast-mcp-client remains 0.1.0.
- **Found:** Carried-open review items that now dissolve rather than get fixed once the agent layer is deleted: agent path forwards set-but-empty vars while config path filters them; allowlisting a hardcoded name emits -e twice; agent extra_args land after cast's own vars so cast cannot override the agent layer.
- **Decided:** Two keys: env_passthrough (base, intended for global cast.json) + extra_env_passthrough (project-level additions). Effective set = base ++ extra, then the existing single pass (dedupe/sort, name validation, reserved-name drop, unset/empty filter) over the concat.
- **Decided:** Per-key figment replacement is intended for BOTH keys: project config loading is scoped to that workspace, so replacement is precisely per-project override power; a project can never affect another project. The additive reading of 'extra' would recreate the unauditable global-union we rejected for the map design.
- **Decided:** Zero-trace clean break: the three hardcoded PASSTHROUGH_VARS lists (dev/opencode/env.rs, dev/claudecode/env.rs, dev/pi/env.rs) and their build_passthrough_env_args wiring are deleted entirely. The migration-catalog warning proposal is dead; migration prose lives only in the 0.2.0 CHANGELOG. Default forwards nothing user-configurable.
- **Decided:** Naming: extra_env_passthrough (nix CLI extra-substituters style).
- **Decided:** No special audit surface: both keys are plain figment fields shown by cast config show. No effective-set command or section; the concat stays internal to run-time arg construction. CAST_EXTRA_ENV_PASSTHROUGH works by construction (bracketed literal); one docs line.
- **Decided:** History: append the pivot as new commits on feat/env-passthrough; the existing 12 commits are not reworked.
- **Open:** Task card Design section and plan still describe the single-key design; both need the pivot appended before implementation.
- **Open:** Removal touches dispatch wiring that assembles per-agent extra_args; exact call sites to be mapped at implementation time.

## [d86c37f] Handover complete: card carries pivot design, original plans closed

Handover package complete for the next agent, who drafts a new executive plan for the pivot and implements it as appended commits on feat/env-passthrough. No code was changed this turn; only cue artifacts. The task card now carries the authoritative two-key design plus a Status-and-handover section (branch state, commit shape, QA state, concurrent-writer warning, nit dispositions); the original master plan, executive plan, and manual-QA todo are closed as superseded with banners stating what history they preserve.

- **Found:** Manual QA audit of the ticked boxes: sections 1, 3, 4, 5, 6 executed on a real host (valueless -e inheritance proven end to end, no leaks, empty-as-unset, passthrough beats cast.env, cast authoritative for USER); sections 2, 7, 8, 9 and sign-off never ran. Section 2 is the end-to-end approval-gate check — the security property itself is still unproven on a real host and MUST be in the fresh QA checklist.
- **Found:** N5 (ANTHROPIC_API_KEY doc example was a no-op duplicate of hardcoded forwarding) inverts under the pivot: allowlisting provider keys becomes the required way to give agents credentials, so the doc example becomes the canonical usage pattern.
- **Decided:** Closed plan/index.md (was complete) as superseded: its constraints, trust-boundary analysis, and rejected alternatives remain valid for the pivot and are retained in place; the new plan inherits them by reference.
- **Decided:** Closed the executive plan 1785678626-4f8ccff (was in-progress, blocked on manual QA): all steps except the QA block were implemented, and the QA block is moot against the pivot.
- **Decided:** Closed the manual-QA todo as superseded rather than leaving it open: its checklist targets the single-key design, and section 7 tests replace semantics the pivot redefines. A fresh checklist must be derived in the new plan.
- **Decided:** Task card is the single handover document; the next agent drafts the new executive plan from it (no new plan was created this turn by design).
- **Decided:** Release-0.2.0 story tracker updated: design-recorded checkbox ticked; a new unchecked step added for drafting the pivot plan.
- **Open:** Next agent: draft new executive plan for the pivot from the task card; rough commit shape recorded there (schema+concat+tests, delete agent forwarding, docs+CHANGELOG).
- **Open:** Fresh manual QA checklist for the two-key design, including the never-run approval-gate check.
- **Open:** Pivot commits must include the CHANGELOG Unreleased entry for the breaking removal (fold-into-0.2.0 mechanics owned by release-0.2.0).
- **Open:** Concurrent-writer cause still unidentified; verify working tree against git before trusting run.rs/loader.rs local state.

## [d86c37f] Pivot executive plan drafted from task card

Drafted the pivot executive plan at .cue/env-var-passthrough/plan/1787026202-d86c37f/env-passthrough-pivot.md (parent: closed master plan, inherited by reference; status in-progress). Four phases matching the task card's rough commit shape: (1) extra_env_passthrough schema key + effective-set concat + tests, (2) zero-trace deletion of agent forwarding, (3) docs + CHANGELOG Unreleased, (4) fresh manual-QA todo + close-out. Pre-plan survey verified every anchor against the tree at d86c37f (clean). Release-0.2.0 story tracker: 'New executive plan drafted' step ticked.

- **Found:** docs/agents.md:34 names env_passthrough_args — a fourth docs touchpoint for the deletion that the task card's rough shape did not list
- **Found:** In build_universal_run_args (volumes.rs:64) the launched_agent (:66) and env (:69) parameters exist ONLY for layer 3 (env passthrough at :84) — both params and doc bullet 3 die with it, rippling to 6 test call sites and to build_session_run_args (run.rs:126) whose agent+env params also become dead (production caller :189, test :857)
- **Found:** Deletion removes exactly 7 existing tests: 2 opencode env, 2 claudecode env, 1 pi env, 1 claudecode mod (test_env_passthrough_args_includes_anthropic_key), 1 volumes (env_passthrough_from_launched_agent_only)
- **Found:** run_in_container's env snapshot (run.rs:151) feeds ONLY host_env_names after the deletion — the set-but-empty filter and its comment are load-bearing and must survive
- **Found:** mcp/exec.rs host_env (:70,:175) is a separate channel (env_clear + resolve_env for MCP tool subprocesses); explicitly out of the deletion's scope
- **Found:** CHANGELOG.md already has both an [Unreleased] section and a pre-written dated [0.2.0] section (universal-container merge, never tagged); the release-0.2.0 gate already owns folding them
- **Found:** Release-0.2.0 story tracker found at .cue/release-0.2.0/plan/index.md (linked from the task card, kind coord)
- **Decided:** Concat lives in one private helper in dev/run.rs, consumed by both the reserved-name warning and build_docker_run_flags — not a Config method; schema.rs stays pure data and loader.rs does not combine nix_extra_substituters either (consumption-site combining is the precedent)
- **Decided:** agents.md:34 one-line fix rides in the phase 2 deletion commit so docs never reference a deleted API even between commits
- **Decided:** CHANGELOG entry goes to [Unreleased]; folding into the pre-written [0.2.0] section stays owned by release-0.2.0 per the handover
- **Decided:** Phase order kept as card's rough shape: schema key lands before the removal, so a user can migrate config before the forwarding disappears
- **Open:** Implementation awaits go-ahead: phase 1 (schema key + concat) is the next executable step
- **Open:** run_in_container's own agent param may have other uses beyond :189 — check before touching during phase 2
- **Open:** Concurrent-writer hazard still applies to phases touching run.rs

## [1313986] Phase 1 committed: extra_env_passthrough schema key and concat helper

Commit 1313986 on feat/env-passthrough: Phase 1 of the pivot executive plan. Added extra_env_passthrough: Vec<String> to Config with doc comment stating its per-project role, per-key replacement semantics, and the no-values guarantee. New private helper effective_env_passthrough(&Config) in dev/run.rs concatenates base ++ extra and is the single concat site, consumed by both the reserved-name warning (run_in_container) and build_docker_run_flags. 11 new tests: 4 loader (defaults, project-replaces-global, global-when-silent, keys-replace-independently), 2 approval (hash+Changed, bare-name-list shape), 5 run.rs (concat order, empty, reserved-from-extra, base+extra both emit valueless, cross-key dedupe single pair). Suite 272 lib + 18 + 9 green, clippy zero warnings.

- **Found:** CONCURRENT WRITER ACTIVE AGAIN, more aggressive than last session: loader.rs and run.rs import orders and two pre-existing assert wrappings flipped to the stale style WITHIN this session (tree was verified clean at session start), and my four targeted reverts were re-flipped within seconds. No rust-analyzer/bacon/watchexec/fmt process was running (ps checked). Pinned rustfmt 1.8.0 AGREES with HEAD on all flipped spots, so the flips are against both the repo and its own toolchain
- **Found:** The /tmp build technique worked flawlessly under active attack: python3 substitutions with exactly-once count assertions (not perl, which double-applied last session); python3 is not on PATH in the devshell, only via nix-shell -p python3
- **Found:** fmt --check now flags only the three known pre-existing offenders (cli.rs, build.rs, image.rs) once the foreign flips are excluded; my new code has zero fmt complaints
- **Found:** Deref coercion ApprovedConfig -> Config works for the helper's &Config parameter (clippy had flagged && ApprovedConfig needless-borrow); the same one-line text appears in production (:165, reference) and test (:731, value) contexts, so scoped edits are required there
- **Decided:** effective_env_passthrough is private in dev/run.rs, not a Config method: schema.rs stays pure data and consumption-site combining is the existing precedent (loader.rs does not combine nix_extra_substituters either)
- **Decided:** Foreign working-tree hunks (import flips in run.rs and loader.rs, two assert rewraps) were excluded from the commit by building all four files from HEAD extracts in /tmp with exactly-once substitution assertions, verifying imports byte-identical to HEAD, then cp+git add in one chain; committed content verified byte-for-byte via git show | cmp
- **Open:** Concurrent writer still unidentified and now reverts edits within seconds; every future commit on this branch must use the /tmp build + exactly-once substitution + index staging technique
- **Open:** Phase 2 (zero-trace deletion of agent forwarding) is next: 3 env.rs modules, Agent::env_passthrough_args trait method, build_universal_run_args shrink, ~7 tests removed
- **Open:** Working tree was clean immediately post-commit (writer dormant at that moment); re-verify before Phase 2

## [85e030a] [85e030a] Phase 2 complete: zero-trace deletion of hardcoded agent env passthrough

Commit 85e030a on feat/env-passthrough: Phase 2 of the pivot executive plan. Zero-trace deletion of hardcoded agent env passthrough across OpenCode, ClaudeCode, and Pi harnesses. Deleted dev/opencode/env.rs, dev/claudecode/env.rs, dev/pi/env.rs; removed Agent::env_passthrough_args trait method; shrunk build_universal_run_args to (included_agents, config, opts) and build_session_run_args to (config, run_opts); dropped dead agent parameter from run_in_container; updated docs/agents.md and doc comments in run.rs; removed the 7 obsolete tests. Suite 265 lib + 18 + 9 green, clippy clean. Zero references to PASSTHROUGH_VARS or Agent::env_passthrough_args remain in the workspace.

- **Found:** Test suite lib count dropped from 272 to 265 (exactly 7 tests removed: 2 opencode env, 2 claudecode env, 1 pi env, 1 claudecode mod, 1 volumes)
- **Found:** Zero remaining references to PASSTHROUGH_VARS, build_passthrough_env_args, or Agent::env_passthrough_args across crates/
- **Decided:** Config-driven allowlists (env_passthrough and extra_env_passthrough) are now the sole env passthrough channel; all host env forwarding into containers is gated by config approval
- **Decided:** Dropped unused agent parameter from run_in_container to keep container execution signature clean
- **Open:** Phase 3 (docs + CHANGELOG Unreleased breaking change entry) is next
- **Open:** Phase 4 (fresh manual QA checklist and validation) follows Phase 3

## [8602518] [8602518] Phase 3 complete: docs and CHANGELOG updated for two-key pivot

Commit 8602518 on feat/env-passthrough: Phase 3 of the pivot executive plan.
Documented the two-key base (env_passthrough) + extra (extra_env_passthrough) design, per-key replacement semantics, provider API key global examples, reserved names filter, and updated trust boundary in crates/cast/docs/config/env-overrides.md.
Added extra_env_passthrough to crates/cast/docs/config/reference.md.
Documented breaking deletion of hardcoded agent env forwarding with global config migration path under [Unreleased] in CHANGELOG.md, along with config approval hash invalidation on upgrade.
Suite 265 lib + 18 + 9 green, clippy clean.

- **Found:** All tests (265 lib + 18 + 9) and clippy pass cleanly with documentation changes.
- **Decided:** Documented both env_passthrough and extra_env_passthrough with canonical provider API key examples.
- **Decided:** Documented breaking changes and upgrade migration in CHANGELOG.md under [Unreleased].
- **Open:** Phase 4: Create fresh manual QA checklist todo artifact and perform validation.

## [8602518] Diff reviews by Flash and Opus verified by Opus consultant

Saved the branch diff to .cue/env-var-passthrough/tmp/1787479972-8602518/branch.diff and ran independent diff reviews with Gemini Flash and Opus subagents. Combined both outputs and verified every finding with Opus against the codebase, test suite, and toolchain.

- Gemini Flash verdict: Approve (minor finding on warning message specificity).
- Opus Diff Reviewer verdict: Approve with comments (High: allowlist entry containing '=' persists secrets to disk and is silently swallowed at runtime; Medium: orphaned doc comments, unlogged invalid name drops, missing NIX_* reserved names, formatting import churn, figment-only test for CLI contract, untested empty host var filter, docs gaps around breaking change; Low: JSON comments in markdown, duplicate filter logic, public re-export).
- Opus Consultant Verification: Confirmed zero-leak security core holds. Opus #1 verified via working PoC as high priority / medium security impact (recommend validating names at config load boundary and redacting in error messages). Opus #8a verified as medium priority (getting-started.md needs credentials step). Opus #5 verified as formatting regression on touched files against 2024 edition imports. Identified new finding N1 (trust boundary doc clarification regarding cast.env) and N2 (GC_NPROCS image-owned env). Recommended clear phased actions.

- **Found:** Allowlist entry containing '=' (e.g. GH_TOKEN=secret) passes cast config allow and persists secret to disk in approved_configs.json before being silently swallowed at runtime by is_valid_env_name. Validating at config load boundary with redaction fixes this.
- **Found:** Zero-leak security core is verified: valueless -e NAME keeps secrets out of argv/ps/logs, and runtime ordering protects cast internal variables.
- **Found:** Non-empty host env filtering is load-bearing but was inline in run_in_container; extracting to pure helper non_empty_host_env_names allows unit testing.
- **Found:** CAST_ENV_PASSTHROUGH bracketed/unbracketed behavior verified end-to-end against real binary.
- **Decided:** Prioritize Opus finding #1 (validate allowlist entries at config load boundary with error redaction) as the primary merge blocker.
- **Decided:** Fix warning message specificity (Opus #9 / Flash #1) to reference both env_passthrough and extra_env_passthrough.
- **Decided:** Address getting-started documentation gap (Opus #8a) and clean up doc comment / import formatting.
- **Open:** User decision on whether to proceed with recommended fixes before final merge.

## [02f5c96] Applied approved review findings on feat/env-passthrough

Committed 02f5c96 on feat/env-passthrough applying the user-approved review fixes from Flash and Opus:
- Console warning at run_in_container now names both env_passthrough and extra_env_passthrough.
- Disentangled doc comments for effective_env_passthrough and build_docker_run_flags.
- Deduplicated reserved names filtering in build_env_passthrough_args by reusing reserved_names_in.
- Removed unused load_config_with_global re-export from config/mod.rs.
- Removed // comments from JSON code blocks and clarified the trust boundary section in env-overrides.md.
- Formatted touched files according to style edition 2024.
Full test suite (265 lib + 18 + 9 + 18 + 26) green, clippy clean.

- **Found:** All 265 unit tests, 18 CLI tests, 9 config integration tests, and clippy pass cleanly after applying review fixes.
- **Decided:** Applied all user-approved review fixes: warning message specificity, doc comment separation, filter deduplication, public export cleanup, doc corrections, and formatting.
- **Decided:** Dismissed findings on '=' entry validation, getting-started guide additions, empty host var helper extraction, and reserved nix/gc vars per user instructions.
- **Open:** Manual QA validation / sign-off before merge.

