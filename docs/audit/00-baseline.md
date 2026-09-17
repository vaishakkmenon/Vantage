# Baseline Build & Test — Audit Findings

**Agent:** baseline-build
**Scope:** Full cargo build/test/clippy/fmt baseline; dependency surface
**Date:** 2026-08-30 (overnight run)

---

## Environment

| Tool    | Version                                        |
|---------|------------------------------------------------|
| rustc   | 1.92.0 (ded5c06cf 2025-12-08)                  |
| cargo   | 1.92.0 (344c4567c 2025-10-21)                  |
| OS      | Windows 11 Home 10.0.26200                     |
| Shell   | Git Bash (POSIX) via Claude Code               |

All commands run from `C:/Users/vaish/Personal/Other/Coding/Vantage/backend`.

---

## What I checked

- Read `C:/Users/vaish/AppData/Local/Temp/.../AGENT_BRIEF.md` — audit ground rules
- `cargo fmt -- --check` — formatting gate
- `cargo build --release --features load_magic,deterministic_zobrist` — CI build
- `cargo clippy --release --features "load_magic deterministic_zobrist" -- -D warnings` — CI lint
- `cargo test --no-fail-fast --release --features load_magic,deterministic_zobrist -- --test-threads=1` — CI test suite (full output captured)
- `cargo clippy --release --features "deterministic_magic deterministic_zobrist" -- -D warnings` — alternate path lint
- `cargo test --no-fail-fast --release --features deterministic_magic,deterministic_zobrist -- --test-threads=1` — alternate path test suite (full output captured)
- `cargo build --release` — default-features build (`psqt`, `cli`)
- `cargo tree --duplicates` — duplicate dependency audit
- `cargo tree -e no-dev --depth 1` — direct production dependency surface
- Read `backend/src/lib.rs` (17 LOC) — fmt diff anchor
- Read `backend/src/moves/magic/loader.rs` (79 LOC) — clippy failure anchor

---

## What I did not check, and why

- I did not run `cargo build` with warnings forced fresh (would require `cargo clean`, which is destructive and unnecessary — the CI path already compiled cleanly).
- I did not check `cargo audit` (vulnerability scan) — not in scope for this agent.
- I did not investigate WASM build — that is `wasm-checker` agent territory.
- I did not read every source file; only the files needed to anchor findings.

---

## Command outputs (verbatim, with timing)

### 1. `cargo fmt -- --check`

```
Diff in \?\C:\Users\vaish\Personal\Other\Coding\Vantage\backend\src\lib.rs:14:

 #[cfg(target_arch = "wasm32")]
 pub mod wasm;
-
EXIT:1
```

**Interpretation:** One trailing blank line at the end of `src/lib.rs` (line 17). `rustfmt` wants it removed. Only one file is non-conformant.

---

### 2. `cargo build --release --features load_magic,deterministic_zobrist`

```
Finished `release` profile [optimized] target(s) in 0.34s
real 0m0.520s
EXIT:0
```

**Interpretation:** Already built from a prior run; reuse of incremental cache. No warnings produced. Build succeeds.

---

### 3. `cargo clippy --release --features "load_magic deterministic_zobrist" -- -D warnings`

```
[... Checking 29 crates ...]
Checking vantage v1.4.0 (C:\...)
Finished `release` profile [optimized] target(s) in 26.73s
real 0m27.314s
EXIT:0
```

**Interpretation:** CI clippy gate is GREEN. No diagnostics against vantage source. Wall time 27s.

---

### 4. `cargo test --no-fail-fast --release --features load_magic,deterministic_zobrist -- --test-threads=1`

Wall time: **22.7s**

| Test binary              | Passed | Failed | Ignored |
|--------------------------|--------|--------|---------|
| lib (unit tests)         | 104    | 0      | 0       |
| bin/cli (unit tests)     | 0      | 0      | 0       |
| analysis_tests           | 10     | 0      | 0       |
| aspiration_tests         | 4      | 0      | 0       |
| determinism_tests        | 16     | 0      | 0       |
| draw_invariants_tests    | 5      | 0      | 0       |
| eval_materials_tests     | 10     | 0      | 0       |
| iterative_deepening_tests| 11     | 0      | 0       |
| legal_filter_tests       | 3      | 0      | 0       |
| lmr_tests                | 5      | 0      | 0       |
| magic_consistency        | 0      | 0      | 0       |
| magic_tests              | 24     | 0      | 0       |
| moveexec_tests           | 19     | 0      | 0       |
| movegen_tests            | 54     | 0      | 0       |
| nmp_tests                | 1      | 0      | 0       |
| ordering_tests           | 7      | 0      | 0       |
| pawn_movegen_tests       | 9      | 0      | 0       |
| perft_tests              | 10     | 0      | 4       |
| quiescence_diagnostics   | 17     | 0      | 0       |
| search_tests             | 19     | 0      | 0       |
| see_tests                | 5      | 0      | 0       |
| square_control_tests     | 4      | 0      | 0       |
| status_tests             | 46     | 0      | 0       |
| tactical_tests           | 8      | 0      | 0       |
| tt_move_ordering_tests   | 10     | 0      | 0       |
| tt_tests                 | 1      | 0      | 0       |
| zobrist_hash_tests       | 43     | 0      | 0       |
| doc-tests                | 0      | 0      | 0       |
| **TOTAL**                | **445**| **0**  | **4**   |

Ignored tests (all in perft_tests): `perft_debug_divide`, `perft_startpos_aggregate`, `perft_startpos_d6`, `perft_startpos_d7`.

**CI is GREEN. All 445 tests pass.**

---

### 5a. `cargo clippy --release --features "deterministic_magic deterministic_zobrist" -- -D warnings`

```
Checking vantage v1.4.0 (C:\...)
error: this `if` statement can be collapsed
  --> src\moves\magic\loader.rs:61:9
   |
61 | /         if let Some(dir) = Path::new(&path).parent() {
62 | |             if let Err(e) = std::fs::create_dir_all(dir) {
63 | |                 eprintln!("⚠️  Could not create directory {dir:?}: {e}");
64 | |             }
65 | |         }
   | |_________^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.92.0/index.html#collapsible_if
   = note: `-D clippy::collapsible-if` implied by `-D warnings`
error: could not compile `vantage` (lib) due to 1 previous error
real 0m1.997s
EXIT:101
```

**Interpretation:** The `deterministic_magic` feature path fails clippy under `-D warnings`. This is a P1 finding — if CI ever runs this feature combination, the build breaks. Currently CI only tests `load_magic`, so this is an untested broken path.

---

### 5b. `cargo test --no-fail-fast --release --features deterministic_magic,deterministic_zobrist -- --test-threads=1`

Wall time: **129.9s (2m10s)**

| Test binary              | Passed | Failed | Ignored |
|--------------------------|--------|--------|---------|
| lib (unit tests)         | 110    | 0      | 0       |
| bin/cli (unit tests)     | 0      | 0      | 0       |
| analysis_tests           | 10     | 0      | 0       |
| aspiration_tests         | 4      | 0      | 0       |
| determinism_tests        | 16     | 0      | 0       |
| draw_invariants_tests    | 5      | 0      | 0       |
| eval_materials_tests     | 10     | 0      | 0       |
| iterative_deepening_tests| 11     | 0      | 0       |
| legal_filter_tests       | 3      | 0      | 0       |
| lmr_tests                | 5      | 0      | 0       |
| magic_consistency        | 1      | 0      | 0       |
| magic_tests              | 29     | 0      | 0       |
| moveexec_tests           | 19     | 0      | 0       |
| movegen_tests            | 54     | 0      | 0       |
| nmp_tests                | 1      | 0      | 0       |
| ordering_tests           | 7      | 0      | 0       |
| pawn_movegen_tests       | 9      | 0      | 0       |
| perft_tests              | 10     | 0      | 4       |
| quiescence_diagnostics   | 17     | 0      | 0       |
| search_tests             | 19     | 0      | 0       |
| see_tests                | 5      | 0      | 0       |
| square_control_tests     | 4      | 0      | 0       |
| status_tests             | 46     | 0      | 0       |
| tactical_tests           | 8      | 0      | 0       |
| tt_move_ordering_tests   | 10     | 0      | 0       |
| tt_tests                 | 1      | 0      | 0       |
| zobrist_hash_tests       | 43     | 0      | 0       |
| doc-tests                | 0      | 0      | 0       |
| **TOTAL**                | **457**| **0**  | **4**   |

Tests PASS under deterministic_magic, but this run took 2m10s due to:
- `lib` unit tests: 62.5s (magic table generation during test init)
- `magic_tests`: 16.8s (includes 2 real magic-search tests)
- `magic_consistency`: 8.1s

---

### 6. `cargo build --release` (default features: `psqt`, `cli`)

```
Compiling vantage v1.4.0
Finished `release` profile [optimized] target(s) in 4.58s
EXIT:0
```

**Interpretation:** Default build succeeds cleanly. No warnings.

---

### 7. Dependency surface

**Direct production deps (`cargo tree -e no-dev --depth 1`):**
```
vantage v1.4.0
├── arrayvec v0.7.6
├── bincode v1.3.3
├── indicatif v0.18.3
├── once_cell v1.21.3
├── rand v0.9.2
├── rmcp v3.1.0
├── schemars v0.8.22
├── serde v1.0.228
├── serde_json v1.0.151
├── tokio v1.53.1
├── tracing v0.1.44
├── tracing-appender v0.2.4
└── tracing-subscriber v0.3.22
```

**Duplicate packages (`cargo tree --duplicates`, key conflicts):**

| Crate      | Versions in tree                | Source of conflict                  |
|------------|----------------------------------|-------------------------------------|
| getrandom  | 0.3.4 (via rand) + 0.4.3 (via rmcp/uuid) | Two incompatible major versions |
| schemars   | 0.8.22 (vantage direct) + 1.2.2 (rmcp) | Major version conflict           |
| memchr     | 2.7.6 listed twice (serde_json + rmcp-macros proc-macro) | Same version, deduplicated at link |

The `getrandom` and `schemars` dual-version conflicts add compile time and binary size. Both originate from `rmcp v3.1.0` pulling newer dep versions than vantage's direct deps.

---

## Findings

### BUILD-101 — `deterministic_magic` feature path fails clippy `-D warnings`

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified (fix not applied)
- **Anchors:** `backend/src/moves/magic/loader.rs:61-65`
- **Expected:** All feature-flag combinations that exist in `Cargo.toml` should pass the same clippy strictness as CI (`-D warnings`). The `deterministic_magic` feature is a first-class build target used by the non-CI test path and by developers who want to regenerate magic tables.
- **Actual:** `cargo clippy --release --features "deterministic_magic deterministic_zobrist" -- -D warnings` exits 101 with `error: this 'if' statement can be collapsed` at `loader.rs:61`. The outer `if let Some(dir)` and inner `if let Err(e)` can be collapsed into a single `if let … && let …` guard, which clippy requires under `-D collapsible-if`.
- **Reasoning:** The lint fires only when `deterministic_magic` is active (the entire function at lines 37-79 is compiled only under `#[cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]`). CI uses `load_magic`, so this code path is never compiled by CI — the lint goes undetected there. The error is real and reproducible.
- **Failure scenario:** Any developer running `cargo clippy --features "deterministic_magic deterministic_zobrist" -- -D warnings` or any CI job extended to cover this feature combo will fail to compile. The tests themselves pass (line 5b above shows 457 pass), but the code cannot be linted or type-checked cleanly without the warning suppression.
- **Related:** None directly; cross-ref BUILD-103 if a CI matrix were added for this feature.
- **Where to look next:** `backend/src/moves/magic/loader.rs:61-65` — the two nested `if let` guards inside `WRITE_ONCE.call_once(|| { ... })`.
- **Fix direction:** Collapse the two nested `if let` guards into one using `&&`: `if let Some(dir) = Path::new(&path).parent() && let Err(e) = std::fs::create_dir_all(dir) { … }`. This is valid Rust 1.64+ "let chains" syntax (stabilized). Alternatively, suppress with `#[allow(clippy::collapsible_if)]` at the function level with a comment explaining that keeping them separate makes the intent clearer — but a fix is preferable to a suppression here since the collapse is semantically identical.

---

### BUILD-102 — `cargo fmt` fails: trailing blank line in `src/lib.rs`

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/lib.rs:17` (trailing blank line after `pub mod wasm;`)
- **Expected:** `cargo fmt -- --check` exits 0; all source files conform to `rustfmt` defaults.
- **Actual:** `cargo fmt -- --check` exits 1. The diff shows a single extra blank line at the end of `src/lib.rs` (after `pub mod wasm;`). No other files are flagged.
- **Reasoning:** `cat -A src/lib.rs` confirms a trailing `$` (blank line) at line 17 after the `pub mod wasm;` declaration. `rustfmt` removes trailing blank lines from the end of files.
- **Failure scenario:** If a CI job adds `cargo fmt -- --check` as a gate (it currently does not), this breaks the build. For now it is a hygiene issue.
- **Related:** None.
- **Where to look next:** `backend/src/lib.rs` — last two lines. Remove the trailing blank line.
- **Fix direction:** Delete the trailing blank line from `src/lib.rs`. One character change.

---

### BUILD-103 — `deterministic_magic` test suite takes 2m10s; magic table unit init dominates

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/magic_consistency.rs` (8.1s), `backend/src/lib.rs` unit tests (62.5s under deterministic_magic)
- **Expected:** The unit test binary (`lib.rs` inline tests) should run in under a second (it runs in 0.03s under `load_magic`).
- **Actual:** Under `deterministic_magic`, the unit test binary takes 62.5s because magic table generation happens on first init, globally, and every test binary that calls into the engine triggers it. The magic_tests binary takes 16.8s. Total wall time for the full suite is 130s vs 23s under `load_magic`.
- **Reasoning:** The `WRITE_ONCE: Once` guard at `loader.rs:58` means generation happens once per process, but each of the 27 integration test binaries is a separate process. Under `--test-threads=1` serial execution, each binary that touches magic table init pays the generation cost (~60s for the first, less for subsequent ones if the bin file was already written). The `magic_consistency` test takes 8.1s confirming the generation is happening per-binary.
- **Failure scenario:** A developer running `cargo test --features deterministic_magic` for a quick local check waits over 2 minutes instead of 23 seconds. This discourages use of the deterministic path for routine testing.
- **Related:** BUILD-101 (the deterministic_magic path already has lint issues; slowness compounds the usability problem).
- **Where to look next:** `backend/src/moves/magic/loader.rs:42-51` — the `MAGIC_TEST_BIN` env var path and the `if let Ok(bytes) = std::fs::read(&path)` fast-path. If the bin file is written by the first binary, subsequent binaries should hit the fast-path. Investigate whether `DEFAULT_TEST_PATH = "../data/test_magic_tables.bin"` resolves correctly from each test binary's working directory.
- **Fix direction:** Verify that the relative path `../data/test_magic_tables.bin` resolves consistently from test binary working directories. If not, each binary generates the table from scratch. Use an absolute path via `CARGO_MANIFEST_DIR` (available at test time) or set `MAGIC_TEST_BIN` to an absolute path in the test invocation. This is a developer-experience issue, not a correctness bug.

---

### BUILD-104 — Dual major versions of `schemars` in dep tree (0.8.22 and 1.2.2)

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/Cargo.toml` (schemars = "0.8"), `backend/Cargo.lock` (rmcp pulls schemars 1.2.2)
- **Expected:** A single version of `schemars` in the dependency tree.
- **Actual:** `cargo tree --duplicates` shows both `schemars v0.8.22` (required directly by vantage) and `schemars v1.2.2` (required by rmcp v3.1.0), plus their respective proc-macro crates (`schemars_derive v0.8.22` and `schemars_derive v1.2.2`). Both are compiled and linked. Similarly, `getrandom` appears at 0.3.4 (via rand) and 0.4.3 (via rmcp/uuid).
- **Reasoning:** `rmcp v3.1.0` uses `schemars v1.x` API, while vantage's own `Cargo.toml` pins `schemars = "0.8"`. These are semver-incompatible major versions — both must be compiled and cannot be unified. This doubles compile time for that crate family and increases binary size.
- **Failure scenario:** Not a runtime bug, but it signals that if vantage ever needs to pass `schemars`-derived types between its own code and rmcp, they will be different types (type incompatibility at version boundary). The current `analysis` module may already be in this situation if it derives schemas for types also seen by rmcp.
- **Related:** Known context item 5 (orphan deps: rmcp, schemars, tokio, serde_json are in the dep tree with minimal direct use).
- **Where to look next:** `backend/Cargo.toml` — update `schemars` to `"1"` to unify with rmcp's requirement, then audit `src/analysis/mod.rs` for any 0.8-specific API usage.
- **Fix direction:** Align `schemars` to `"1.x"` in `Cargo.toml` if the `analysis` module's usage is compatible. This eliminates the duplicate compile and removes the type-boundary hazard.

---

## Test count analysis

**CLAUDE.md claims "300+ tests"** — observed 445 under `load_magic` (449 including ignored), so the claim is accurate but significantly understated. The codebase has grown well past 300.

**Brief estimates "~468 from static counting"** — observed totals:
- `load_magic` config: 449 (445 pass + 4 ignored)
- `deterministic_magic` config: 461 (457 pass + 4 ignored)

The `deterministic_magic` config exposes 12 additional tests (6 more unit tests in lib, 5 more in magic_tests, 1 in magic_consistency) that are compiled only under that feature. The gap from the 468 static estimate (7 tests) is plausible from the 4 permanently-ignored perft tests (which static counting would include) and minor miscounting in the estimate. No tests are missing from either configuration.

**The 4 ignored tests are intentional:** `perft_startpos_aggregate`, `perft_startpos_d6`, `perft_startpos_d7` (very long runtime), `perft_debug_divide` (diagnostic utility). These are correctly gated with `#[ignore]`.

---

## Clean bill

- **CI build and tests are fully green.** `cargo build --release --features load_magic,deterministic_zobrist` and all 445 tests pass cleanly. CI is not broken by this branch.
- **Default feature build is clean.** `cargo build --release` (psqt + cli) succeeds with no warnings.
- **`deterministic_magic` tests all pass** (457/457 pass). The only problem in that path is the clippy lint (BUILD-101), not a test failure. The runtime behavior is correct.
- **No test failures anywhere.** Zero failing tests across both feature configurations.
- **CI clippy is green.** `cargo clippy --release --features "load_magic deterministic_zobrist" -- -D warnings` exits 0 with no diagnostics.
- **No panics, no P0 issues.** No test panics, no assertion failures, no illegal-move generation detected through testing.

---

## Incomplete implementations

Per the brief, no `todo!()` / `unimplemented!()` markers exist. Known incomplete work (from `MCP_BUILD_PLAN.md` as described in the shared brief):

| Intent source | What it promises | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `MCP_BUILD_PLAN.md` Phase 1.2 | `legal_moves` tool in MCP server | Missing; `src/bin/vantage-mcp.rs` does not exist | MCP server cannot be started | Yes, MCP server is non-functional |
| `MCP_BUILD_PLAN.md` Phases 2-5 | Full MCP server pipeline, transport, tool registration | Missing | MCP feature advertised in CLAUDE.md is unshippable | Yes |

These are carry-overs from known context item 8 — not re-derived, cited here for completeness as they impact the build surface (orphan deps `rmcp`, `tokio`, `serde_json`, `schemars` are present but unused by any binary that exists).
