# WASM & Analysis Module — Audit Findings

**Agent:** audit-wasm-analysis
**Scope:** `backend/src/wasm.rs` (304 LOC), `backend/src/analysis/mod.rs` (205 LOC), `backend/tests/analysis_tests.rs` (182 LOC)
**Date:** 2026-08-30 (overnight run)

## What I checked

- Read `backend/src/wasm.rs` (all 304 lines)
- Read `backend/src/analysis/mod.rs` (all 205 lines)
- Read `backend/tests/analysis_tests.rs` (all 182 lines)
- Read `MCP_BUILD_PLAN.md` (all 210 lines)
- Read `AGENT_BRIEF.md` (shared brief)
- Grepped `web-time`, `std::time`, `Instant`, `Duration` across `wasm.rs`, `Cargo.toml`, `src/search/search.rs`
- Grepped `fn set_fen`, `panic!`, `unwrap()` in `backend/src/board/fen.rs`
- Grepped `fn set_fen` in `backend/src/board/mod.rs`
- Listed `backend/src/board/` directory

## What I did not check, and why

- `backend/src/search/search.rs` full read — did not verify that the search never returns `i32::MIN` as a final score (sentinel leak). The grep confirmed `Instant` is correctly cfg-gated; the full logic was not audited (out of scope for this agent).
- Runtime test execution — brief forbids running cargo.
- `backend/src/board/fen.rs` full read — only grepped for panics; no panics found, `set_fen` signature confirmed returning `Result`. Full parser correctness was not traced.

## Findings

### ANALYSIS-001 — `from_raw(i32::MIN)` overflows in debug mode, produces nonsense Cp in release

- **Severity:** P1
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/src/analysis/mod.rs:42`
- **Expected:** `Score::from_raw` handles every value the search may return, including the `-∞` sentinel (`i32::MIN`) that iterative-deepening search implementations commonly use to represent "no score yet". Even if the engine should never return this, a robust boundary converts it without UB.
- **Actual:** `score.abs()` at line 42 is called on `i32::MIN`. In debug builds this panics (integer overflow). In release builds it wraps to `i32::MIN` (still negative), so the branch takes `Score::Cp(i32::MIN)` — approximately −2.1 billion centipawns. This is an invisible corruption: the serialized JSON appears valid but the value is nonsense.
- **Reasoning:** `i32::MIN = -2147483648`. Rust's `-(-2147483648)` in i32 arithmetic overflows. In debug mode: panic. In release mode (wasm32 and native release): wraps to `i32::MIN`. The condition `score.abs() >= MATE_THRESHOLD` evaluates as `i32::MIN >= 30000` which is false (wrapping makes it negative), so `Score::Cp(i32::MIN)` is returned. I did not read `search.rs` fully, so I cannot confirm the engine leaks this sentinel; confidence is medium.
- **Failure scenario:** Any code path in search that returns `i32::MIN` as a final score — e.g., a position with zero legal moves that the search handles improperly — would produce `Score::Cp(-2147483648)` rather than an error or a mate score.
- **Related:** WASM-003 (any WASM search result goes through from_raw); if BOARD agent found illegal-position handling, overlap there.
- **Where to look next:** `backend/src/search/search.rs` — find the initial alpha/beta bounds and any early-return paths that might propagate a sentinel.
- **Fix direction:** Guard `from_raw` with a saturating-abs or explicit clamp before comparing to `MATE_THRESHOLD`. Alternatively, define a symbolic `NEG_INF` constant in the search module and assert it is never returned in a debug assertion at the `evaluate` call site.

---

### ANALYSIS-002 — `classify` arm ordering causes `Cp → Mate(+)` improvements to silently fall to `_` with classification "best", but `Mate(+) → Cp` with high cp-value is classified as "blunder" — mixed-score arm gap

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/analysis/mod.rs:124-148`, `backend/src/analysis/mod.rs:126`
- **Expected:** When eval_before is `Mate(n > 0)` (we have a forced mate) and eval_after is `Cp(900)` (still heavily winning but mate missed), the move dropped a forced win. The plan (`MCP_BUILD_PLAN.md:104`) describes thresholds for cp_loss; a missed forced mate should be at minimum a mistake. The first arm currently marks it "blunder" (correct), but with `cp_loss = None`, which means the `cp_loss` field in `AnalyzeMoveResult` is `None` even though the position is Cp-comparable.
- **Actual:** Line 126: `(Score::Mate(n), _) if *n > 0 && !still_winning_mate(eval_after) => ("blunder".to_string(), None)`. When eval_after is `Cp(900)`, `still_winning_mate(Cp(900))` returns false, so the arm fires → "blunder" with `cp_loss = None`. This is correct for the *classification* but the missing `cp_loss` means the JSON payload carries `"cp_loss": null` even when a numeric loss could be estimated (e.g., `MATE_SCORE - Cp_value`). More critically: the fallthrough `_` arm (line 146) returns `("best", None)` for any mixed `(Cp, Mate(+))` case — i.e., if the move leads from a centipawn position into a winning forced mate, that is correctly "best" but carries `cp_loss = None`. MCP_BUILD_PLAN:104 says thresholds should be echoed in the payload; `None` is opaque.
- **Reasoning:** Traced each arm in order against all `(Score, Score)` combinations. The `_` arm at line 146 handles `(Cp(_), Mate(m > 0))` — a position improvement — and returns `("best", None)`. This is semantically correct but the `None` cp_loss is unverifiable by a caller. The brief's concern about Cp-Mate subtraction (e.g., `31000 - 50`) is NOT present in the code: those mixed cases never compute a numeric difference, they fall to named arms. So the original concern is not a bug, but the missing cp_loss on Mate-boundary blunders is a real gap.
- **Failure scenario:** MCP client calls `analyze_move` for a position where White had mate in 3 and played a suboptimal move. Response: `{"classification":"blunder","cp_loss":null}`. The LLM has no numeric grounding for "how bad", contrary to MCP_BUILD_PLAN:104's intent.
- **Related:** ANALYSIS-005 (error type), ANALYSIS-006 (legal_moves gap).
- **Where to look next:** `backend/src/analysis/mod.rs:126-128`, `MCP_BUILD_PLAN.md:104`.
- **Fix direction:** For the `Mate(+) → non-winning` blunder arm, compute an approximate cp_loss using `MATE_SCORE` as a ceiling; or document explicitly that `cp_loss = None` means "mate was lost" and ensure the MCP tool description states this so callers don't interpret `None` as "no loss". Do not subtract raw mate scores from Cp scores.

---

### ANALYSIS-003 — Tests never verify order-independence across calls sharing a single `TranspositionTable`

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/analysis_tests.rs:16-28, 61-76`, `backend/src/analysis/mod.rs:67, 173`
- **Expected:** `vantage-engine-pass.md` (cited in brief) names order-independence as an explicit acceptance gate. Calling `evaluate(A)` then `evaluate(B)` on the same `&mut TranspositionTable` must give the same result for B as calling `evaluate(B)` alone.
- **Actual:** Both `evaluate` (line 67) and `analyze_move` (lines 173, 188) call `tt.clear()` on entry, which makes them order-independent in the code. However, every determinism test in `analysis_tests.rs` creates a **fresh** `TranspositionTable` for each call (lines 19-20, 63-65, 82-84 etc.), so the tests never exercise the reuse path. If `tt.clear()` were accidentally removed, the tests would not catch the regression.
- **Reasoning:** Checked all 10 tests: each calls `TranspositionTable::new(64)` before every `evaluate`/`analyze_move` call. No test passes the same `tt` across two calls to different positions.
- **Failure scenario:** Remove `tt.clear()` at `mod.rs:67`. All existing tests still pass. But a caller that reuses the same TT object (the expected MCP server usage) now gets different results for the second query.
- **Related:** None.
- **Where to look next:** `backend/tests/analysis_tests.rs` — add one test that evaluates STARTPOS, then KIWIPETE, then STARTPOS again on the same `&mut tt`, and asserts the third call matches the first.
- **Fix direction:** Add an order-independence regression test. The fix is in tests, not in the production code (which is already correct).

---

### ANALYSIS-004 — `Score` serde shape diverges from MCP_BUILD_PLAN Phase 2 specification

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/analysis/mod.rs:12-17`, `MCP_BUILD_PLAN.md:87-89`
- **Expected:** `MCP_BUILD_PLAN.md:87` specifies `#[serde(tag = "type", content = "value", rename_all = "snake_case")]`, producing `{"type":"cp","value":50}` — an internally-tagged shape that is self-describing and matches the planned `JsonSchema` derivation.
- **Actual:** `#[serde(rename_all = "lowercase")]` with no tag/content attributes uses serde's default *external* tagging: `Score::Cp(50)` → `{"cp":50}`, `Score::Mate(3)` → `{"mate":3}`. The variant name becomes the key. This shape is not self-describing in the same way and will not match the tool schema the MCP client expects once Phase 2 is implemented. Any code or test that snapshots the JSON wire format today locks in the wrong shape.
- **Reasoning:** Default serde enum serialization is external tagging. `rename_all = "lowercase"` only lowercases the variant name. No `tag` or `content` attributes are present.
- **Failure scenario:** Phase 2 snapshot tests (MCP_BUILD_PLAN:109) are written against `{"cp":50}`. Phase 2 then adds `#[serde(tag="type",content="value")]` to match the plan, producing `{"type":"cp","value":50}`. Snapshot tests break and the MCP client tool schema changes at Phase 2 boundary.
- **Related:** ANALYSIS-005.
- **Where to look next:** `backend/src/analysis/mod.rs:12-17`.
- **Fix direction:** Decide the wire format now, before Phase 2 adds snapshot tests. The plan's tagged format is more explicit and safer for schema generation. Align the current enum attributes with Phase 2's intent.

---

### WASM-001 — `set_position_startpos` returns `()` on illegal move — JS caller cannot detect truncation

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/wasm.rs:66-75`
- **Expected:** Commit e7cc241 ("stop applying moves after first illegal move in wasm apply_move loop") fixes the infinite-loop / undefined-state problem. A complete fix should also let the caller know where the sequence was truncated, so the UI can surface an error rather than silently showing a wrong position.
- **Actual:** `set_position_startpos` returns `()` (void in JS). When an illegal move causes `apply_move` to return false (line 70-72), the loop breaks, but the return value to JS is still `undefined`. The JS side receives the engine in a partially-applied state with no indication of which move failed or how many moves were processed.
- **Reasoning:** Inspected lines 66-75 directly. The `break` was the e7cc241 fix. No return value, no error flag, no count of applied moves is communicated.
- **Failure scenario:** JS calls `engine.set_position_startpos("e2e4 e7e5 z9z9 g1f3")`. Engine applies e2e4, e7e5, stops at z9z9, ignores g1f3. Engine is now at the position after 1. e4 e5 but JS believes it applied all four moves. Any UI display or further move validation operates on the wrong state.
- **Related:** Context #3 (stale wasm on live site).
- **Where to look next:** `backend/src/wasm.rs:66-75`; the `apply_move` caller in the frontend JS.
- **Fix direction:** Change the return type of `set_position_startpos` to return the number of moves successfully applied (a `u32`), or return a `bool` that is false if any move was skipped. This is a wasm-bindgen ABI change and requires updating the JS caller.

---

### WASM-002 — `go_movetime` passes `std::time::Duration` to search that times itself with `web_time::Instant` — Duration arithmetic is compatible but the time *source* in wasm is `Performance.now()`, not `Date.now()`, via web-time

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/wasm.rs:13, 95`, `backend/src/search/search.rs:15-18`
- **Expected:** All time arithmetic in the wasm32 build uses a consistent time source.
- **Actual:** `wasm.rs` imports `std::time::Duration` (line 13) and constructs a `Duration::from_millis` value (line 95) that is passed to `SearchLimits::time_limit`. The search checks elapsed time using `web_time::Instant::now()` (guarded by `#[cfg(target_arch = "wasm32")]` at `search.rs:18`). The `web_time::Instant::elapsed()` method returns a `std::time::Duration`, so the comparison is homogeneous. This is CLEAN. Noted here to confirm the brief concern is resolved.
- **Failure scenario:** N/A — no bug.
- **Fix direction:** None required.

---

### ANALYSIS-005 — `Result<_, String>` error type in public API, no dedicated `AnalysisError`

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/analysis/mod.rs:59-81, 150-205`, `MCP_BUILD_PLAN.md:47`
- **Expected:** `MCP_BUILD_PLAN.md:47` states "one error type for the whole module."
- **Actual:** Both `evaluate` and `analyze_move` return `Result<_, String>`. No `AnalysisError` type exists.
- **Reasoning:** Direct read. No `enum AnalysisError` in the file.
- **Failure scenario:** MCP error handler cannot pattern-match on structured error kinds ("invalid FEN" vs "search timeout"); all failures serialize to opaque strings.
- **Related:** ANALYSIS-004.
- **Where to look next:** `backend/src/analysis/mod.rs` top-level.
- **Fix direction:** Define `pub enum AnalysisError { InvalidFen(String), InvalidMove(String) }` and update the `?` propagation. This is prerequisite to Phase 4 (MCP_BUILD_PLAN:174).

---

### ANALYSIS-006 — `evaluate` has no test for malformed FEN input

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/analysis_tests.rs:133-141`, `backend/src/analysis/mod.rs:65-66`
- **Expected:** Both `evaluate` and `analyze_move` share the same FEN-parse path. The test at line 133 covers `analyze_move` with a malformed FEN implicitly (the move string is malformed, but the FEN itself is valid). No test calls `evaluate` with a malformed FEN.
- **Actual:** `fen.rs:253` confirms `set_fen` returns `Result<(), String>` with no panics; the `?` at `mod.rs:66` propagates correctly. This is clean by inspection. However, the absence of a test means a future refactor of `set_fen` to panic instead of return Err would be undetected.
- **Reasoning:** Scanned all 10 tests in `analysis_tests.rs`. Test 8 (line 133) tests malformed move on valid FEN for `analyze_move`. No test calls `evaluate(BAD_FEN, ...)`.
- **Failure scenario:** Not currently broken. Risk is undetected regression.
- **Related:** None.
- **Where to look next:** `backend/tests/analysis_tests.rs`.
- **Fix direction:** Add a test: `evaluate("not a fen", NODES, &tables, &mut tt)` should return `Err(_)`.

---

## Clean bill

- **`console_error_panic_hook::set_once()`** is called at `wasm.rs:29` in the `#[wasm_bindgen(constructor)]` entry point. This is the correct and only entry point for wasm initialisation. Any panic after construction will produce a useful browser console error.
- **TT 64MB** confirmed at `wasm.rs:32`: `TranspositionTable::new(64)`. Matches `CLAUDE.md` spec.
- **`eval_after` negation** in `analyze_move` is present and correct at `analysis/mod.rs:190`: `Score::from_raw(-result_after.score)`. The sign flip converts from opponent's perspective back to the original side's perspective before `classify` is called. Test 10 (`engines_own_best_move_is_not_penalized`) would catch a missing negation.
- **`still_winning_mate` logic** is correct for both colours: `matches!(score, Score::Mate(m) if *m > 0)` returns true only for a positive Mate (delivering mate), false for Cp scores and negative Mate (being mated).
- **`find_legal_move` on bad FEN**: `analyze_move` calls `board.set_fen(fen)?` at line 158 before `find_legal_move`, so a bad FEN returns `Err` before the move search begins. `set_fen` at `fen.rs:253` confirmed to return `Result<(), String>` with no panics in the parser.
- **e7cc241 loop-break fix** is present at `wasm.rs:70-73`. The `break` correctly stops the loop on illegal move. (Incompleteness of error reporting is tracked as WASM-001, not a revert of the fix.)
- **`parse_uci_move` promotion handling**: if the 5th character is unrecognised, `promo_piece = None` and the code looks for a non-promotion move. On a pawn-to-8th-rank square all legal moves have a promotion piece, so `None` won't match any and the function returns `None`. Clean failure.
- **`classify` Cp-Mate subtraction**: the brief flagged "subtracting a mate score from a centipawn score gives nonsense". Traced every arm: mixed Cp/Mate combinations never reach the `(Cp(a), Cp(b))` arm and never compute a numeric difference. The `_` fallthrough returns `("best", None)`. The concern is not present as a code bug.
- **Order independence** in production code: `evaluate` clears TT at line 67; `analyze_move` clears at lines 173 and 188. Calls sharing a TT are order-independent. (Test coverage gap tracked as ANALYSIS-003.)
- **FEN error propagation**: `evaluate` at line 66 and `analyze_move` at line 158 both use `?` to propagate `set_fen` errors. Combined with `fen.rs` having no panics, bad FEN returns `Err(String)` cleanly.
- **`web_time`/`std::time` consistency**: `wasm.rs` uses `std::time::Duration` (correct — Duration is not platform-specific), search uses `#[cfg(wasm32)] web_time::Instant` (correct). Time arithmetic is homogeneous.

---

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `MCP_BUILD_PLAN.md:43` | `pub fn legal_moves(fen: &str) -> Result<Vec<String>, AnalysisError>` | `find_legal_move` (single-move lookup) exists; no `legal_moves` enumeration function | Blocks Phase 4 (`analyze_move` and `legal_moves` wiring) and the MCP server demo | Yes — Phase 4 cannot proceed |
| `MCP_BUILD_PLAN.md:47` | "one error type for the whole module" / `AnalysisError` | `Result<_, String>` used throughout | MCP error handling in Phase 4 will need unstructured string matching | Partial block |
| `MCP_BUILD_PLAN.md:87-89` | `#[serde(tag="type",content="value")]` tagged Score enum | External-tagged `{"cp":50}` shape | Phase 2 snapshot tests will lock the wrong wire format | Partial block — caught before Phase 2 |
| `MCP_BUILD_PLAN.md:118-154` | `src/bin/vantage-mcp.rs` stdio server | Binary does not exist | Entire MCP server is absent | Yes |
| `MCP_BUILD_PLAN.md:198` | Three tools callable in inspector, definitions-of-done checklist | None implemented (Phases 2-5 unstarted) | Demo transcript, README section, `MCP_STATUS.md` | Yes |
