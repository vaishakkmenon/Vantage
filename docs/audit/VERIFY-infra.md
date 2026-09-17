# Infrastructure & Claims — Verification Report

**Agent:** verify-infra
**Scope:** docs/audit/11-build-ci-deploy.md (BUILD-001..015), 12-test-suite.md (TEST-001..011), 13-doc-claims.md (CLAIM-001..008), 14-repo-hygiene.md (HYGIENE-001..010), 00-baseline.md (BUILD-101..104)
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/Cargo.toml` (49 LOC) — full read; confirmed `[[bin]] name = "vantage-cli"` at line 29, no `[profile.release]`
- `.github/workflows/ci.yml` lines 1-125 — full read; confirmed binary paths at lines 71, 75; release build at line 87
- `backend/src/wasm.rs` lines 1-50 — confirmed bare `include_bytes!("../book.bin")` at line 35
- `backend/src/search/search.rs` lines 420-560 — confirmed PVS implementation
- `VANTAGE_ROADMAP.md` lines 30-155 — confirmed Phase 1.1 PVS description and SPRT testing protocol
- `docs/audit/00-baseline.md` — full read; authoritative test counts
- `docs/audit/11-build-ci-deploy.md`, `12-test-suite.md`, `13-doc-claims.md`, `14-repo-hygiene.md` — full read
- `git status --porcelain` — confirmed analysis/ is untracked (`??`)
- `ls backend/target/release/` — found both `vantage.exe` (stale) and `vantage-cli.exe`
- `git check-ignore -v backend/book.bin` — confirmed gitignored

## Test Count Reconciliation

Authoritative figures from 00-baseline.md verbatim output:

| Configuration | Pass | Ignored | Total |
|---|---|---|---|
| `load_magic,deterministic_zobrist` (CI) | 445 | 4 | 449 |
| `deterministic_magic,deterministic_zobrist` | 457 | 4 | 461 |

The 12-test delta (457 − 445) breaks down as: +6 lib unit tests, +5 magic_tests, +1 magic_consistency — all gated by `not(feature = "load_magic")`.

The 4 ignored tests are perft tests intentionally gated `#[ignore]` for runtime.

The baseline agent's internal inconsistency ("445 tests pass" + "449 observed") is resolved: 445 passing + 4 ignored = 449 total in the load_magic run. Both statements were correct; the summary conflated the two counts.

CLAIM-005 (other agents quoted 445, 449, 457, 461, 468): the 468 figure is a static-analysis overcount (by 7) from a separate agent — likely counting 4 ignored tests twice and miscounting gated helpers as tests. The 445/449/457/461 figures are all directly from baseline output and mutually consistent. CLAUDE.md's "300+ tests" is accurate but understated; the suite now has 449–461 tests depending on feature set.

TEST-001 claim ("permanently dark"): confirmed — magic_consistency shows 0 runs under load_magic, 1 under deterministic_magic.
TEST-002 claim ("11 dark tests in magic_tests"): partially correct — baseline shows 5 additional magic_tests under deterministic_magic (29 vs 24), not 11. The 11 `#[cfg]` gates in the file include helper functions, not all `#[test]` items. Downgrade to: 5 dark magic_tests + 1 dark magic_consistency = 6 dark tests, not 11+1.

## Findings by ID

### BUILD-001 — Binary name mismatch in CI and Dockerfile
- **Original severity:** P1 | **Verdict: CONFIRMED** | **Revised severity: P1**
- `Cargo.toml:29` → `name = "vantage-cli"`. `ci.yml:71,75` → `vantage.exe`/`vantage`. `Dockerfile.engine:40` → `/build/target/release/vantage`.
- On disk: `backend/target/release/` contains BOTH `vantage.exe` (stale pre-rename artifact) and `vantage-cli.exe`. A fresh CI checkout would only produce `vantage-cli.exe`; no `src/main.rs` exists to produce a default `vantage` binary. The stale local artifact does not help CI.
- **The finding is valid.** Every tagged release CI run copies a non-existent path.

### BUILD-003 — Release compiled with `deterministic_zobrist`
- **Original severity:** P2 | **Verdict: CONFIRMED but overstated** | **Revised severity: P3**
- `ci.yml:87` uses `deterministic_zobrist` in the release build. The finding is factually correct. However, fixed Zobrist keys primarily affect reproducibility, not engine strength. For an open-source engine (where keys are readable from source), "hash-collision attacks" provide negligible practical advantage. The main consequence is that all binaries share the same TT hash space, making distributed self-play identical. Downgrade to P3: hygiene, not a meaningful security or correctness risk.

### BUILD-004 — `book.bin` gitignored; releases ship bookless
- **Original severity:** P1 | **Verdict: CONFIRMED** | **Revised severity: P1**
- `git check-ignore -v backend/book.bin` confirms the ignore rule fires. `git status` shows no `book.bin` tracked anywhere. `ci.yml:101` emits a warning and continues without failing. Every tagged release archive lacks the book.

### BUILD-005 — `contents: write` granted to all jobs
- **Original severity:** P3 | **Verdict: CONFIRMED but lower real-world risk** | **Revised severity: P3**
- Factually correct: workflow-level `contents: write` at `ci.yml:26-27`. However, actions used in check/test (`dtolnay/rust-toolchain`, `Swatinem/rust-cache`) are widely trusted. The theoretical risk of supply-chain compromise exists but is common across the industry and not unique to this project. P3 is appropriate; the finding is not overstated for its severity.

### BUILD-010 — No `[profile.release]`; LTO and codegen-units at defaults
- **Original severity:** P2 | **Verdict: CONFIRMED** | **Revised severity: P2**
- `Cargo.toml` has no `[profile.release]` section (confirmed by full read). Performance impact is real but unquantified. The perft agent established an NPS baseline of 3.2–4.9M nodes/sec on this code without LTO; the improvement from `lto = true, codegen-units = 1` is typically 5–15% for search-heavy code. Mark performance impact as UNVERIFIED until a before/after benchmark is run. P2 stands.

### BUILD-011 — `cdylib` in crate-type forces cdylib linkage on native builds
- **Original severity:** P3 | **Verdict: CONFIRMED but negligible impact** | **Revised severity: P3**
- `Cargo.toml:26`: `crate-type = ["rlib", "cdylib"]`. Produces `vantage.dll` on Windows on every native build. Adds link time. However, baseline shows the build completes in under 5s (incremental), confirming the extra cdylib link is fast. The finding is real but the real-world cost is minor. P3 is correct.

### BUILD-014 — `.wasm` not marked binary in `.gitattributes`
- **Original severity:** P3 | **Verdict: PLAUSIBLE, risk overstated** | **Revised severity: P3**
- `.gitattributes` omits `*.wasm binary`. The WASM magic bytes `\0asm` contain a NUL and reliably trigger git's binary heuristic across all modern git versions. The committed `main:frontend/public/wasm/vantage_bg.wasm` (2,495,946 B) has survived 2+ commits without corruption. The residual risk from `git add --renormalize` is real but very low. The more urgent issue with that WASM is staleness (known context 3), not encoding risk. P3, confirmed but not urgent.

### HYGIENE-001 — `run_gauntlet.py` has no SPRT; verdict is `score >= 50%`
- **Original severity:** P1 | **Verdict: CONFIRMED** | **Revised severity: P1**
- `VANTAGE_ROADMAP.md:143`: "SPRT bounds: H0=0, H1=10 for small features". `run_gauntlet.py:153-154`: verdict is `if percentage >= 50: verdict = "PASS"`. Zero instances of `SPRT`, `llr`, `LLR`, `elo0`, `elo1` in the script. Every engine change decision validated by this harness is statistically unsupported.

### HYGIENE-003 — WASM build fails on fresh clone: bare `include_bytes!("../book.bin")`
- **Original severity:** P1 | **Verdict: CONFIRMED** | **Revised severity: P1**
- `backend/src/wasm.rs:35`: `let book_bytes = include_bytes!("../book.bin");` — bare compile-time macro, not inside any `#[cfg]` guard, not behind a feature flag. `wasm.rs` is only compiled on wasm32 target (`lib.rs` gates it with `#[cfg(target_arch = "wasm32")]`), but when WASM IS built, this line always fires. `backend/book.bin` is gitignored and absent from any fresh clone. Any `wasm-pack build` on a fresh clone fails with a file-not-found compile error. This is the precise reason the committed WASM (known context 3) cannot easily be rebuilt. **Most actionable finding in the hygiene file.**

### HYGIENE-005 — `backend/src/analysis/` and `analysis_tests.rs` untracked
- **Original severity:** P1 | **Verdict: CONFIRMED** | **Revised severity: P1**
- `git status --porcelain` shows `?? backend/src/analysis/` and `?? backend/tests/analysis_tests.rs`. The `??` prefix means untracked AND not ignored (an ignored file shows `!!` under `--ignored`). `git clean -fd` would delete both without warning. The entire MCP Phase 0+1.1 implementation exists only in the working tree. Deletions of `src/mcp/mod.rs` and `tests/mcp_tests.rs` are also unstaged (` D`), meaning `git diff HEAD` misrepresents the current state. **Most urgent finding for the human to act on immediately.**

### HYGIENE-006 — No single branch holds current backend + frontend + correct WASM
- **Original severity:** P1 | **Verdict: CONFIRMED** | **Revised severity: P1**
- `git log HEAD ^main`: 13 backend commits only on `feature/backend_agentic_updates`. `main` has the `84517eb` check-highlight frontend commit and the committed WASM blob — neither on this branch. This branch has the wasm apply_move fix (e7cc241) that main lacks. No branch represents the deployable product.

### HYGIENE-009 — `check_engines.py` masks hung engines as "OK (Slow)"
- **Original severity:** P3 | **Verdict: CONFIRMED but low impact** | **Revised severity: P3**
- Real bug at `scripts/check_engines.py:43-45`, but the script is Docker-only (HYGIENE-004), rarely run, and not part of any CI gate. P3 is correct.

### HYGIENE-010 — `git-config-docker.template` tracked without matching ignore
- **Original severity:** P3 | **Verdict: REJECTED** | **Revised severity: N/A**
- Tracking a template while ignoring the instantiated copy (`.gitignore:9`: `git-config-docker`) is a standard and intentional pattern. The template is committed precisely so collaborators can populate their own copy. No inconsistency. Finding should be dropped.

### CLAIM-001 — ROADMAP lists PVS as Phase 1.1 future work; code already implements it
- **Original severity:** P1 | **Verdict: CONFIRMED** | **Revised severity: P1**
- `search.rs:445-544`: move_count==0 gets full-window search `(-beta, -alpha)`; moves 1+ get null-window `(-alpha-1, -alpha)` reduced scout, null-window full-depth re-search on fail-high, then full-window re-search if `score > alpha && score < beta`. This is textbook PVS. `VANTAGE_ROADMAP.md:33-36` describes PVS as Phase 1.1 (unchecked, future). `README.md:66` contradicts the roadmap: "Alpha-Beta Pruning: With Principal Variation Search (PVS) for optimal tree traversal." Roadmap was never updated after the feature shipped.

### BUILD-101..104 (baseline findings)
- **BUILD-101** (deterministic_magic clippy failure): CONFIRMED by baseline verbatim output. P1.
- **BUILD-102** (trailing blank line in lib.rs): CONFIRMED. P3.
- **BUILD-103** (deterministic_magic test suite 2m10s): CONFIRMED. P3, developer-experience.
- **BUILD-104** (dual schemars versions): CONFIRMED by `cargo tree --duplicates` in baseline. P3.

### TEST-001/TEST-002 — magic tests dark in CI
- **TEST-001**: CONFIRMED. magic_consistency: 0 runs under load_magic, 1 under deterministic_magic.
- **TEST-002**: PARTIALLY CONFIRMED. Baseline shows 5 dark magic_tests (not 11 as claimed). The 11-gate count includes helper functions; only 5 are `#[test]` items. Downgrade confidence on the count; the core finding (some magic tests dark) is correct.

## Summary table

| ID | Original | Verdict | Revised | Note |
|---|---|---|---|---|
| BUILD-001 | P1 | CONFIRMED | P1 | vantage.exe on disk is stale; fresh CI has no vantage binary |
| BUILD-003 | P2 | CONFIRMED-OVERSTATED | P3 | Fixed Zobrist is reproducibility-only risk for open-source |
| BUILD-004 | P1 | CONFIRMED | P1 | |
| BUILD-005 | P3 | CONFIRMED | P3 | Not overstated for its severity |
| BUILD-010 | P2 | CONFIRMED | P2 | Impact unquantified; NPS baseline 3.2–4.9M nodes/sec |
| BUILD-011 | P3 | CONFIRMED | P3 | Link overhead negligible in practice |
| BUILD-014 | P3 | PLAUSIBLE | P3 | NUL-byte heuristic reliable; staleness is bigger issue |
| HYGIENE-001 | P1 | CONFIRMED | P1 | No SPRT; 50% threshold |
| HYGIENE-003 | P1 | CONFIRMED | P1 | Bare include_bytes! wasm.rs:35; fresh clone WASM build fails |
| HYGIENE-005 | P1 | CONFIRMED | P1 | ?? untracked; git clean -fd destroys implementation |
| HYGIENE-006 | P1 | CONFIRMED | P1 | |
| HYGIENE-009 | P3 | CONFIRMED | P3 | Docker-only script |
| HYGIENE-010 | P3 | REJECTED | N/A | Intentional template pattern |
| CLAIM-001 | P1 | CONFIRMED | P1 | PVS at search.rs:445-544 vs roadmap Phase 1.1 |
| TEST-001 | P1 | CONFIRMED | P1 | |
| TEST-002 | P1 | PARTIAL | P2 | 5 dark, not 11; downgrade confidence |
| BUILD-101 | P1 | CONFIRMED | P1 | |
| BUILD-102 | P3 | CONFIRMED | P3 | |
| BUILD-103 | P3 | CONFIRMED | P3 | |
| BUILD-104 | P3 | CONFIRMED | P3 | |
