# Doc Claims — Audit Findings

**Agent:** audit-claims
**Scope:** VANTAGE_ROADMAP.md, vantage-engine-pass.md, MCP_BUILD_PLAN.md, CLAUDE.md, README.md
**Date:** 2026-08-30 (overnight run)

## What I checked

- `VANTAGE_ROADMAP.md` (183 lines) — full read
- `vantage-engine-pass.md` (65 lines) — full read
- `MCP_BUILD_PLAN.md` (222 lines) — full read
- `CLAUDE.md` (57 lines) — full read
- `README.md` (274 lines, read lines 1-80 plus grep for binary/build/run claims)
- `backend/src/search/search.rs` — read lines 1-769 (full file)
- `backend/src/analysis/mod.rs` — full read (206 lines)
- `backend/src/wasm.rs` — read lines 85-135
- `backend/src/bin/cli.rs` — grep for SearchLimits, handle_go, node_limit
- `backend/Cargo.toml` — grep for bin names, features
- `backend/src/search/tt.rs` — grep for TT size
- `scripts/` — directory listing only (gauntlet results not stored in repo)
- Grep: aspiration windows, LMP constants, PVS markers, singular, IIR, bishop_pair, countermove, psqt cfg sites, legal_moves, vantage-mcp, test count

## What I did not check, and why

- `AGENTS.md`, `DOCKER.md`, `phase-1-frontend.md` — time budget exhausted; lower priority per brief
- ELO claims (ROADMAP lines 11, 159-165) — unverifiable without gauntlet results; no results stored in repo
- `scripts/run_gauntlet.py` — exists but no stored output
- `README.md` lines 80-274 — scanned key claims via grep; full read of lower half skipped

## Findings

### CLAIM-001 — ROADMAP lists PVS as Phase 1.1 (future work) but code already implements it

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `VANTAGE_ROADMAP.md:33-36`, `backend/src/search/search.rs:445-544`, `README.md:66`
- **Expected:** ROADMAP Phase 1.1 describes PVS as not yet implemented: "Search first move with full window / Remaining moves: null window search, re-search with full window only on fail-high."
- **Actual:** `search.rs` lines 445-544 implement exactly that pattern — move_count==0 gets full-window search, moves 1+ get null-window reduced search, null-window full-depth re-search on fail-high, then full-window re-search if `score > alpha && score < beta`. README.md line 66 also confirms: "Alpha-Beta Pruning: With Principal Variation Search (PVS) for optimal tree traversal." PVS is live.
- **Reasoning:** I read all 769 lines of search.rs. The ID loop calls `alpha_beta` and inside `alpha_beta` lines 445-544 are unmistakably PVS. The null-window scout + conditional re-search is the textbook PVS pattern. The ROADMAP was never updated after the feature shipped.
- **Failure scenario:** If a developer follows the ROADMAP strictly ("complete phases in order"), they will attempt to implement PVS in Phase 1.1 and overwrite the already-working implementation, introducing subtle bugs in a search component that is currently correct.
- **Related:** CLAIM-002 (PV table bug in PVS context)
- **Where to look next:** `search.rs:445-544` for the PVS logic; `VANTAGE_ROADMAP.md:33-36` for the stale claim.
- **Fix direction:** Mark Phase 1.1 as DONE in the roadmap. Add a note with the commit reference (commits bf5ad05 and aa8d27a mentioned in brief). Move Phase 2.3 (LMR retune) to Phase 1 since PVS is already in.

---

### CLAIM-002 — PV is updated from null-window child data on beta-cutoff, truncating the reported line

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs:499-566`, `vantage-engine-pass.md:35`
- **Expected:** `vantage-engine-pass.md` line 35 explicitly warns: "only update the PV on a full-window result (a true PV node), never on a null-window scout search — otherwise the PV comes out truncated or wrong. This is the classic PV+PVS bug; budget time for it."
- **Actual:** Lines 554-566 update `ctx.pv_table[ply]` using `ctx.pv_length[ply+1]` unconditionally after any score improvement. When a non-first move beats beta via a null-window search (the `score > alpha && r > 0` branch at lines 514-527), the full-window re-search at lines 530-544 is skipped (because `score < beta` is false). The PV update then copies from `ctx.pv_table[ply+1]` which was populated by the null-window child, where `pv_length` was likely set to 0 at start (line 283) and never incremented because the null-window child hit its own beta cutoff immediately. Result: `pv_length[ply] = 0 + 1 = 1` — only the immediate move, no continuation.
- **Reasoning:** In a null-window search, `alpha = beta - 1`. A move that beats alpha in the child immediately returns (beta cutoff in the child). That child sets `pv_length[child_ply] = 0` at entry and may only set it to 1 if it updates a best move there. The parent then sets its own pv_length to `child_pv_length + 1`. When the child's pv_length is 0, the parent PV is 1 move. The spec document anticipated this exact bug.
- **Failure scenario:** Search position `r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8` at depth 8. If the second root-level move causes a fail-high at an interior node via null-window, `SearchResult.pv` may contain only 1-2 moves rather than 8. The `pv.len() <= depth` invariant from vantage-engine-pass.md harness test 3 will pass (trivially satisfied) but `pv.len() == 1` at depth 8 is wrong.
- **Related:** CLAIM-001 (PVS is live), `vantage-engine-pass.md:35,48`
- **Where to look next:** `search.rs:499-566` — the entire PVS branch; specifically the condition at line 530 (`score > alpha && score < beta`) and what happens when score >= beta at lines 560-587.
- **Fix direction:** Guard the PV update at lines 560-566 with an explicit `is_pv_node` flag, or restructure: only copy the child's PV into the parent's PV table when the child was searched with a full window (i.e., the `score > alpha && score < beta` branch at 530-544 was taken). One common approach: move the PV copy into the full-window re-search branch itself rather than after the fact.

---

### CLAIM-003 — CLAUDE.md run command uses wrong binary name and non-portable echo

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `CLAUDE.md:38`, `backend/Cargo.toml:29`
- **Expected:** CLAUDE.md line 38: `echo "uci\nisready\nposition startpos\ngo depth 6\nquit" | ./backend/target/release/vantage`
- **Actual:** `Cargo.toml` line 29 declares `name = "vantage-cli"` for the `[[bin]]` target. The compiled binary is `vantage-cli`, not `vantage`. Additionally, `echo` in PowerShell (the user's primary shell per env) does not interpret `\n` as newline — it emits the literal string `uci\nisready\n...` as a single line. The command fails on two independent grounds.
- **Reasoning:** `Cargo.toml:29` is unambiguous: `name = "vantage-cli"`. The CI workflow (known context #1) also copies `vantage-cli` but the path says `vantage` — a related stale reference. For PowerShell: `echo` = `Write-Output`, which does not process escape sequences. The command would send one long garbage line to the engine's stdin.
- **Failure scenario:** A developer copies this command verbatim into PowerShell. It fails with "The system cannot find the path specified" because `vantage` does not exist. Even if corrected to `vantage-cli`, the `\n` issue means the engine receives `"uci\nisready\nposition startpos\ngo depth 6\nquit"` as a single line and outputs `Unknown command` or hangs.
- **Related:** README.md:54 has same wrong binary name
- **Where to look next:** `backend/Cargo.toml:28-30`; `CLAUDE.md:38`; `README.md:54`
- **Fix direction:** Correct binary name to `vantage-cli`. Replace `echo "...\n..."` with a heredoc or `@'...'@` in PowerShell, or note that this command works only in bash with `echo -e` or `printf`.

---

### CLAIM-004 — CLAUDE.md WASM build command specifies dead feature flag `psqt`

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `CLAUDE.md:36`, `backend/Cargo.toml:38`, grep for `cfg(feature = "psqt")` in `backend/src/`
- **Expected:** CLAUDE.md line 36: `wasm-pack build --target web --no-default-features --features psqt` — implies `psqt` is a meaningful feature that gates PSQT evaluation in WASM builds.
- **Actual:** Grep over all `backend/src/` shows zero `#[cfg(feature = "psqt")]` or `cfg!(feature = "psqt")` sites. The `psqt` feature flag exists in `Cargo.toml:38` and is listed in `default = ["psqt", "cli"]` at line 33, but is never queried in any source file. Passing `--features psqt` to wasm-pack compiles the same code as not passing it. The `--no-default-features` flag does meaningfully suppress the `cli` feature, preventing terminal-only deps from compiling.
- **Reasoning:** Brief known-context item 4 confirms: `psqt` is a dead flag with zero cfg sites. I independently verified via grep returning no output.
- **Failure scenario:** A developer looking at this command assumes `psqt` conditionally enables piece-square table evaluation, and may think that omitting it produces a WASM build without PSQT. This is false — PSQT is always compiled unconditionally via `get_psqt()` in `eval.rs`.
- **Related:** Known context #4 (dead feature flags)
- **Where to look next:** `backend/Cargo.toml:33,38`; `backend/src/search/eval.rs:71` (`get_psqt` unconditionally defined)
- **Fix direction:** Either add a `#[cfg(feature = "psqt")]` gate to the PSQT code (to make the flag meaningful) or remove `psqt` from the feature list and the WASM build command, replacing the command with the accurate `--no-default-features --features load_magic,deterministic_zobrist` for a deterministic WASM build.

---

### CLAIM-005 — CLAUDE.md and README.md claim "300+ tests" but count is 468

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `CLAUDE.md:7`, `README.md:47`
- **Expected:** Both files claim "300+ tests."
- **Actual:** `grep -rn "#\[test\]"` across `backend/src/` and `backend/tests/` returns 468 matches.
- **Reasoning:** The claim "300+" is technically true (468 > 300) but significantly understated. This is a minor hygiene issue, not a correctness problem.
- **Failure scenario:** Not a failure — just stale documentation. A developer checking whether tests were added may undercount expected failures if they believe there are only ~300.
- **Fix direction:** Update both files to "460+ tests" (a round number that stays true after small additions).

---

### CLAIM-006 — CLAUDE.md MCP section describes vantage-mcp in present tense but binary does not exist

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `CLAUDE.md:23-25`, `backend/src/bin/` listing
- **Expected:** CLAUDE.md lines 23-25: "`vantage-mcp` — stdio MCP server exposing `evaluate`, `analyze_move`, `legal_moves` over the `analysis::` module."
- **Actual:** `ls backend/src/bin/` returns only `cli.rs`. No `vantage-mcp.rs` exists. `legal_moves` is not implemented in `analysis/mod.rs` (confirmed by grep returning no output for `legal_moves` in that file). Only `evaluate` and `analyze_move` exist in the module.
- **Reasoning:** Known context #8 confirms `src/bin/vantage-mcp.rs` does not exist. I verified independently. CLAUDE.md uses present tense ("exposing") which implies it is operational.
- **Failure scenario:** A new Claude Code session starting by reading CLAUDE.md will believe the MCP server is registered and the three tools are callable. Any attempt to use the MCP tools fails silently or with a connection error.
- **Fix direction:** Change CLAUDE.md to past/future tense: "vantage-mcp (in progress — Phases 3-5 of MCP_BUILD_PLAN.md not yet started). Currently only `evaluate` and `analyze_move` exist in `analysis/`; `legal_moves` and the server binary are not yet implemented."

---

### CLAIM-007 — vantage-engine-pass.md Task 1 optional `nodes` arm: not implemented in CLI

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `vantage-engine-pass.md:28`, `backend/src/bin/cli.rs` (grep for "nodes")
- **Expected:** vantage-engine-pass.md line 28: "Optional: add a `nodes` arm to the `go` parser in `cli.rs` for manual binary testing."
- **Actual:** Grep for `"nodes"` in `cli.rs` returns no output. The `handle_go` function does not parse a `nodes` subcommand. `SearchLimits` is constructed with `node_limit: None` at cli.rs lines 310, 374.
- **Reasoning:** The spec marks this as "Optional" so this is not a gap — it's expected. Recording it here for completeness because readers of the spec may wonder if it was skipped.
- **Failure scenario:** None — optional feature, CLI testing of node budget requires the lib API directly.
- **Fix direction:** None required. If desired for convenience, add `nodes N` branch to `handle_go` that sets `node_limit: Some(N)`.

---

### CLAIM-008 — ROADMAP evaluation claims "bishop mobility (5cp/sq), rook mobility (3cp/sq)" — these are granular and need verification

- **Severity:** P3
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `VANTAGE_ROADMAP.md:19`
- **Expected:** "bishop mobility (5cp/sq), rook mobility (3cp/sq)"
- **Actual:** Not personally verified in `eval.rs` — out of time budget. Known to exist (the module has mobility evaluation) but exact constants not confirmed.
- **Reasoning:** eval.rs was not read in full. This is a Confidence: medium finding marked for follow-up.
- **Where to look next:** `backend/src/search/eval.rs` — grep for `mobility`, `bishop_mob`, `rook_mob` and confirm the per-square bonuses.
- **Fix direction:** If constants differ, update the roadmap comment.

---

## Clean bill

- **CLAUDE.md "WASM builds use 64MB TT, native uses 512MB":** TRUE. `wasm.rs:32` shows `TranspositionTable::new(64)` and `cli.rs:19` shows `TranspositionTable::new(512)`. `tt.rs:29-32` confirms the argument is in MB.
- **CLAUDE.md "Engine is single-threaded (no Lazy SMP)":** TRUE. No thread spawning, no `Arc<Mutex<...>>`, no `rayon` dep in `Cargo.toml`. `SearchContext` is passed `mut` by value, not shared.
- **VANTAGE_ROADMAP.md aspiration windows "(50cp, depth > 4)":** TRUE. `search.rs:680` `let window = 50;`, `search.rs:682-686` `if depth > 4 { alpha = last_completed_best_score - window; ... }`.
- **VANTAGE_ROADMAP.md LMP "(threshold: 3+6×depth)":** TRUE. `search.rs:41-43` `LMP_BASE_MOVES: 3`, `LMP_MOVE_MULTIPLIER: 6`; `search.rs:432` `let lmp_threshold = LMP_BASE_MOVES + LMP_MOVE_MULTIPLIER * depth`.
- **VANTAGE_ROADMAP.md null move pruning "(R=2-3 dynamic)":** TRUE. `search.rs:360-361` `let r = if depth > 6 { 3 } else { 2 };`.
- **VANTAGE_ROADMAP.md delta pruning "(200cp)":** TRUE. `search.rs:210` `stand_pat + captured_value + 200 < alpha`.
- **SearchLimits and SearchResult field shapes (vantage-engine-pass.md Task 0):** TRUE. `search.rs:73-86` shows exact fields: `SearchLimits { max_depth: i32, node_limit: Option<u64>, time_limit: Option<Duration> }`, `SearchResult { score: i32, best_move: Option<Move>, pv: Vec<Move>, depth: i32, nodes: u64 }`.
- **search() signature (vantage-engine-pass.md Task 0):** TRUE. `search.rs:627-632` `pub fn search(board: &mut Board, tables: &MagicTables, tt: &mut TranspositionTable, limits: SearchLimits) -> SearchResult`.
- **Ripple to cli.rs and wasm.rs (vantage-engine-pass.md Task 0):** TRUE. `cli.rs:11` imports `SearchLimits, search`; `wasm.rs:9` imports `SearchLimits, SearchResult, search`. Both construct `SearchLimits` correctly.
- **Node budget implemented (vantage-engine-pass.md Task 1):** TRUE. `search.rs:120-124` checks `node_limit` in `TimeManager::check_time`; `search.rs:661-668` checks node limit before each ID depth iteration.
- **Triangular PV table implemented (vantage-engine-pass.md Task 2):** TRUE (but see CLAIM-002 for the PVS update bug). `search.rs:283,561-565` shows standard triangular PV table mechanics.
- **Quiescence-PV deferred (vantage-engine-pass.md Task 2, Phase 1 decision):** TRUE. `quiescence()` at `search.rs:141-258` has no PV collection code — `best_move` is tracked locally but never written to `ctx.pv_table`.
- **VANTAGE_ROADMAP.md Phases 1.2, 1.3, 2.1, 2.2 (countermove, bishop pair, rook open file, singular extension, IIR) as future work:** TRUE — grep for these features returns nothing in `backend/src/`.
- **MCP_BUILD_PLAN Phase 0 + 1.1 done:** TRUE. `analysis/mod.rs` exists (renamed from mcp), has `evaluate` and `analyze_move`.
- **MCP_BUILD_PLAN Phase 1.2 (legal_moves) not done:** TRUE — no `legal_moves` function in `analysis/mod.rs`.
- **MCP_BUILD_PLAN Phases 2-5 not started:** TRUE — no `src/bin/vantage-mcp.rs`, no structured response types with `EngineConfig` or `ScoreJson`, no MCP registration.
- **MCP_BUILD_PLAN anti-drift "VANTAGE_ROADMAP.md untouched":** TRUE — the roadmap has not been modified to include engine-strength ideas from the MCP sprint (the PVS staleness predates this sprint).
- **analysis::classify thresholds 1000/300/100 (known context #9):** TRUE. `analysis/mod.rs:137-141` shows exactly `c if c >= 1000 => "blunder"`, `c if c >= 300 => "mistake"`, `c if c >= 100 => "inaccuracy"`.
- **Thresholds NOT echoed in AnalyzeMoveResult:** TRUE. `AnalyzeMoveResult` at `analysis/mod.rs:29-38` has no `thresholds` field — MCP_BUILD_PLAN Phase 2 requirement unmet.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `MCP_BUILD_PLAN.md:47-49` | "`pub fn legal_moves(fen: &str) -> Result<Vec<String>, AnalysisError>`" | No `legal_moves` fn in `analysis/mod.rs` | MCP server phase 4 cannot be built without it | Yes — blocks MCP Phase 4 |
| `MCP_BUILD_PLAN.md:119-122` | "`src/bin/vantage-mcp.rs` skeleton" | File does not exist | No MCP server binary; can't register with Claude Code | Yes — blocks Phases 3-5 |
| `MCP_BUILD_PLAN.md:82-105` | Structured `EngineConfig`, `ScoreJson`, `EvaluateResult` with config fingerprint | `EvaluateResult` lacks `fen` echo, `config` field, `ScoreJson` variant names differ (`Cp`/`Mate` but as plain enum not tagged) | Every response missing reproducibility fingerprint; grounding claim unverifiable | Partial — evaluate/analyze_move work but don't satisfy the Phase 2 contract |
| `MCP_BUILD_PLAN.md:104` | `AnalyzeMoveResult` echoes thresholds: `{"inaccuracy": 50, ...}` | `AnalyzeMoveResult` has no `thresholds` field; thresholds hardcoded in `classify()` at lines 137-141 | Model must invent severity language rather than reading it from the payload | No (deferred to Phase 2) |
| `vantage-engine-pass.md:35` | "only update the PV on a full-window result — otherwise the PV comes out truncated or wrong" | PV is updated on null-window beta-cutoff paths (`search.rs:560-566`) | `SearchResult.pv` may be truncated in interior nodes, causing `better_line` and `refutation` in `AnalyzeMoveResult` to be short | No (engine move is correct; PV quality reduced) |
| `vantage-engine-pass.md:58-59` | "Optional: add `nodes` arm to `go` parser in `cli.rs`" | Not present in `cli.rs` | Manual CLI testing of node budget impossible | No (optional) |
| `MCP_BUILD_PLAN.md:201-205` (DoD) | Four checkbox items: vantage-mcp binary, three tools, demo transcript, README updated | None of the four boxes are checked | Project appears unfinished at sprint boundary | Depends on priority |
