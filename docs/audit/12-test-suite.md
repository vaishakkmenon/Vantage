# Test Suite — Audit Findings

**Agent:** audit-tests
**Scope:** `backend/tests/` (25 files, 9,324 LOC, 358 `#[test]`) + 14 inline `#[cfg(test)]` modules in `backend/src/`
**Date:** 2026-08-30 (overnight run)

## What I checked

- **Read in full:** `determinism_tests.rs` (264 LOC/16 tests), `perft_tests.rs` (319 LOC/15 tests), `tactical_tests.rs` (245 LOC/8 tests), `tt_tests.rs` (45 LOC/1 test), `nmp_tests.rs` (43 LOC/1 test), `aspiration_tests.rs` (105 LOC/4 tests), `lmr_tests.rs` (149 LOC/5 tests), `draw_invariants_tests.rs` (188 LOC/5 tests), `search_tests.rs` (207 LOC/19 tests), `analysis_tests.rs` (182 LOC/10 tests), `magic_consistency.rs` (83 LOC/1 test)
- **Read vantage-engine-pass.md** lines 44-53 (6-test gate definition)
- **Grep:** `cfg/load_magic` gates in `magic_tests.rs`; `#[test]` count; `once_cell/static/Mutex` in `src/`; `test-threads` in `.github/workflows/ci.yml`; inline `#[cfg(test)]` modules across all `src/` files; `has_major_pieces` definition in `src/board/mod.rs`
- **Checked:** all 46 `src/` files listed by `find` for inline test presence

## What I did not check, and why

- Did not read in full: `zobrist_hash_tests.rs` (1,774 LOC), `status_tests.rs` (1,346 LOC), `movegen_tests.rs` (1,117 LOC) — skimmed structure via grep; full read would exceed priority budget
- Did not run `cargo test` — baseline agent handles execution
- Did not verify runtime behavior of `has_major_pieces` return values beyond reading the source definition

## Findings

### TEST-001 — `magic_consistency.rs` permanently dark in CI

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified (needs build confirmation)
- **Anchors:** `backend/tests/magic_consistency.rs:1`, `.github/workflows/ci.yml:60`
- **Expected:** The test that verifies loaded magic tables are byte-identical to a fresh deterministic generation should run in CI.
- **Actual:** `magic_consistency.rs:1` gates the entire file with `#![cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]`. CI always passes `--features "load_magic deterministic_zobrist"` (ci.yml:60). The `load_magic` feature negates the gate — this test file never compiles or runs in CI.
- **Reasoning:** The `#![cfg(…)]` at the top of the file controls whether the file is compiled at all. `not(feature = "load_magic")` is false in CI. The sole test `test_magic_table_consistency` therefore never runs under any CI build. If someone corrupts or misaligns the embedded magic binary, CI will not detect it.
- **Failure scenario:** Embed stale magic tables (e.g., re-run build script with a different seed, without updating the embedded bytes). CI stays green. Moves that rely on correct attack tables become subtly wrong — perft diverges at depth 4+, but no CI test catches the table mismatch.
- **Related:** TEST-002 (same feature gate pattern in magic_tests.rs); known context item #3 (stale WASM on live site — also undetected by CI).
- **Where to look next:** `backend/tests/magic_consistency.rs:1-16`; `backend/src/moves/magic/loader.rs:25-36`.
- **Fix direction:** Add a CI step that runs the magic consistency test under `--features "deterministic_zobrist deterministic_magic"` (without `load_magic`). This is a separate `cargo test` invocation targeting only that file. Alternatively, invert the feature-gate contract so the verification works against the embedded tables under `load_magic`.

---

### TEST-002 — 11 of 29 tests in `magic_tests.rs` dark in CI

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/magic_tests.rs:1,15,18,25,29,32,343,361,379,399,419`; `ci.yml:60`
- **Expected:** All magic table tests run in CI.
- **Actual:** grep shows 11 `#[cfg(not(feature = "load_magic"))]` gates in `magic_tests.rs`. The file has 29 `#[test]` functions. In CI (`load_magic` enabled), all 11 gated tests are excluded. Exactly which functional areas they cover is visible at those line numbers.
- **Reasoning:** The count of `not(feature = "load_magic")` gates (11) vs total tests (29) means ~38% of magic tests never run in CI. These are likely the generation-path tests (masks, search-space verification) — the parts most likely to differ between loaded and generated tables.
- **Failure scenario:** Same as TEST-001: a magic table corruption goes undetected because the tests that would catch it are silenced in CI.
- **Related:** TEST-001.
- **Where to look next:** `backend/tests/magic_tests.rs:15-32` (gated helper fns), lines 343-419 (gated tests).
- **Fix direction:** Same as TEST-001 — a second CI step with `not(load_magic)` features, or restructure tests so they are feature-gate-agnostic where possible.

---

### TEST-003 — Three tests that can never fail

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/tactical_tests.rs:131-153`, `backend/tests/tactical_tests.rs:102-128`, `backend/tests/lmr_tests.rs:8-35`
- **Expected:** Every `#[test]` function asserts a correctness property.
- **Actual:**
  1. `test_fork_opportunity` (tactical_tests.rs:131-153): The only statement after the search call is `println!("Fork position score: {}", score)`. No `assert_*`. File comment (line 152) admits: "This test mainly ensures no crash." Cannot fail.
  2. `test_avoid_hanging_piece` (tactical_tests.rs:102-128): Extracts `move_uci`, prints it (line 127), but asserts nothing about the move's quality. Cannot fail.
  3. `test_scholar_mate_position_analysis` (lmr_tests.rs:8-35): Loops depths 1-6, calls search, and only runs `println!`. No `assert_*` at any depth. Cannot fail.
- **Reasoning:** Grep for `assert` in these functions returns zero hits (excluding the outer `expect("valid FEN")`). A test that merely calls the function and doesn't check the result provides no signal — a regression in move selection (e.g., engine returns None or picks a losing move) would go undetected.
- **Failure scenario:** Engine's fork detection regresses. `test_fork_opportunity` stays green. `test_avoid_hanging_piece` stays green even if the engine drops a knight every time.
- **Related:** TEST-004 (wall-clock-only assertions are similarly weak).
- **Where to look next:** `backend/tests/tactical_tests.rs:102-153`; `backend/tests/lmr_tests.rs:8-35`.
- **Fix direction:** For `test_fork_opportunity`: verify the engine picks a move whose score exceeds a material-win threshold. For `test_avoid_hanging_piece`: assert the chosen move's UCI string is NOT the knight's source square (i.e., the engine moves the knight away, or assert `score >= -50`). For `test_scholar_mate_position_analysis`: assert depth-4+ score is within a reasonable range, or assert `best_move.is_some()` as a minimum.

---

### TEST-004 — Wall-clock time as the sole correctness assertion

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/aspiration_tests.rs:104`, `backend/tests/lmr_tests.rs:117-121`, `backend/tests/search_tests.rs:178`
- **Expected:** Tests assert search correctness, not runtime performance.
- **Actual:**
  - `test_aspiration_performance` (aspiration_tests.rs:82-105): sole functional assertion is `duration.as_secs() < 30`. On a fast CI machine this always passes; on a slow machine it flakes. No score or move quality check.
  - `test_lmr_performance_improvement` (lmr_tests.rs:95-122): sole assertion is `duration.as_secs() < 30`. Name implies it verifies LMR reduces node count but makes no such comparison.
  - `test_quiescence_depth_limit` (search_tests.rs:171-179): sole assertion is `elapsed.as_secs() < 180` (3 minutes!). Score (`_score`) is discarded. A qsearch that returns a wildly wrong score passes this test.
- **Reasoning:** `assert!(duration.as_secs() < 30)` only fails on hardware slower than the threshold by an order of magnitude. It says nothing about correctness of the search result.
- **Failure scenario:** LMR is accidentally disabled (someone removes the depth-reduction branch). The test named "lmr_performance_improvement" stays green because the search completes in under 30 seconds even without LMR.
- **Related:** TEST-003.
- **Where to look next:** `backend/tests/aspiration_tests.rs:82-105`; `backend/tests/lmr_tests.rs:95-122`; `backend/tests/search_tests.rs:171-179`.
- **Fix direction:** Replace or supplement the time assertions with node-count comparisons: run the same position with and without the optimization flag (or at a known depth), assert node count reduced by a meaningful fraction. For `test_quiescence_depth_limit`, assert the score is within a sane range (e.g., `score.abs() < 500`).

---

### TEST-005 — Determinism harness uses 8× larger TT than the spec

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/determinism_tests.rs:15`, `vantage-engine-pass.md:44`
- **Expected:** `vantage-engine-pass.md:44` — "small TT (64MB)" for the determinism harness.
- **Actual:** `determinism_tests.rs:15`: `TranspositionTable::new(512)`. Every `run()` call allocates a 512 MB TT. This is 8× the specified size.
- **Reasoning:** A larger TT changes search behavior (fewer hash collisions, different cutoff patterns, potentially different node counts). If a bug only manifests with a small TT, the harness won't catch it. The spec explicitly chose 64 MB to stress the TT under realistic conditions.
- **Failure scenario:** A TT collision that corrupts a mate score is masked by the 512 MB table avoiding the collision entirely. The MCP server, which the spec says will use a small TT, exposes the bug.
- **Related:** none directly.
- **Where to look next:** `backend/tests/determinism_tests.rs:12-26` (the `run()` helper); `backend/tests/analysis_tests.rs:19` (uses 64MB — correct).
- **Fix direction:** Change `TranspositionTable::new(512)` in the `run()` helper to `TranspositionTable::new(64)` to match the spec.

---

### TEST-006 — Book module (1,118 LOC across 4 files) has zero test coverage

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/book/polyglot_book.rs`, `backend/src/book/polyglot_entry.rs`, `backend/src/book/polyglot_hash.rs`, `backend/src/book/polyglot_keys.rs`
- **Expected:** Core infrastructure has test coverage.
- **Actual:** No integration test file references `book`, `polyglot`, or any book-module type. No inline `#[cfg(test)]` modules exist in any of the four book files. The CLI uses the book (`cli.rs` — `handle_go` consults the book); any bug in hash computation or entry parsing is invisible to the test suite.
- **Reasoning:** Grep across `backend/tests/` for "book" and "polyglot" returns no results. The `find` output shows no inline test modules in book files.
- **Failure scenario:** `polyglot_hash.rs` computes the wrong Zobrist key for en-passant positions. The book returns the wrong move (or no move) for any position with ep rights. No test catches it.
- **Related:** context item #7 (book agent area — known context about book module).
- **Where to look next:** `backend/src/book/polyglot_book.rs` (lookup logic); `backend/src/book/polyglot_hash.rs` (hash computation vs. standard Polyglot spec).
- **Fix direction:** Add at minimum: (1) a test that verifies `polyglot_hash` matches the known Polyglot spec hash for startpos; (2) a test that parses the bundled `data/book.bin` and verifies at least one known response (e.g., startpos → e2e4 or d2d4).

---

### TEST-007 — `src/bin/cli.rs` (515 LOC of UCI parsing) has zero test coverage

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs`
- **Expected:** UCI parsing logic has some integration test coverage.
- **Actual:** No test file targets `cli.rs`. The binary is never invoked from any test. The `handle_go`, `handle_position`, `parse_moves`, and option-parsing paths are entirely untested.
- **Reasoning:** `src/bin/cli.rs` is `cfg(feature = "cli")` and not part of the library crate, so integration tests do not pull it in. There is no test that pipes UCI commands into a child process or calls the parse functions directly.
- **Failure scenario:** `handle_position` fails to parse a FEN with unusual whitespace. The engine silently searches from startpos instead. Undetected until a user reports it.
- **Related:** context item #1 (CI copies wrong binary name — related CLI gap).
- **Where to look next:** `backend/src/bin/cli.rs` (entire file).
- **Fix direction:** Expose `parse_position`, `parse_go`, etc. as `pub(crate)` or library-accessible and add unit tests. Alternatively, add a CLI integration test that spawns `vantage-cli`, pipes `"uci\nisready\nposition startpos\ngo depth 3\nquit\n"`, and asserts the stdout contains `bestmove`.

---

### TEST-008 — `--test-threads=1` required: shared global singleton in Zobrist

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/src/hash/zobrist.rs:5,93`; `.github/workflows/ci.yml:60`
- **Expected:** Tests can run in parallel without interference. If they cannot, the shared resource is documented.
- **Actual:** `zobrist.rs:93` declares `static KEYS: OnceCell<ZobristKeys>`. This is process-global. With `--features deterministic_zobrist`, the keys are seeded from a constant (`ZOBRIST_SEED = 0x9E37_79B9_AAAC_5C87` at line 12), so parallel initialization is deterministic. `once_cell::OnceCell` is thread-safe (it blocks until initialized). CI forces `--test-threads=1`, suggesting defensive serialization.
- **Reasoning:** The `OnceCell` itself is safe, but the CI flag prevents CPU contention from making time-limited searches flaky (tactical_tests.rs uses `time_limit: Some(Duration::from_secs(5))` etc.). If tests run in parallel and CPU is saturated, those limits could fire early, causing `best_move` to return `None` or a shallow result. The flag is defensive — there is no hard data-race, but time-sensitive searches are correct only when they have exclusive CPU access.
- **Failure scenario:** Remove `--test-threads=1`. On a loaded CI machine, `test_back_rank_mate` (tactical_tests.rs:39-62, 3-second limit) or `test_aspiration_performance` (30-second limit) completes in wrong depth. `best_move` is `None` or wrong. This is a flaky failure, not deterministic.
- **Related:** TEST-004 (wall-clock assertions are the proximate cause of flakiness risk).
- **Where to look next:** `backend/src/hash/zobrist.rs:93`; `backend/tests/tactical_tests.rs` time_limit usages.
- **Fix direction:** Document why `--test-threads=1` is required in a comment in `ci.yml`. Remove `time_limit` from tests that are correctness checks (not performance checks); use `node_limit` instead, which is deterministic and independent of CPU speed.

---

### TEST-009 — `has_major_pieces` is a misnomer: includes minor pieces

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/board/mod.rs:343-351`; `backend/tests/nmp_tests.rs:29-35`
- **Expected:** A function named `has_major_pieces` tests for rooks and queens only (standard chess nomenclature: major = R/Q; minor = N/B).
- **Actual:** `mod.rs:343` doc comment says "any non-pawn material (N, B, R, Q)". The implementation returns `true` for knights and bishops. `nmp_tests.rs:29-35` comment says "Knight is a major piece" — this is wrong per standard terminology. The test is correct for the actual implementation, but the name and comment propagate the misnomer.
- **Reasoning:** This is a naming/conceptual mismatch. The NMP guard is semantically correct (use any non-pawn material to avoid zugzwang), but a future author reading the function name might assume it excludes knights/bishops and introduce a bug.
- **Failure scenario:** No runtime failure. A future developer adds a true `has_major_pieces` check (R/Q only) somewhere, or renames this function and passes knights through NMP in K+N endgames.
- **Related:** none.
- **Where to look next:** `backend/src/board/mod.rs:343-351`; `backend/tests/nmp_tests.rs:29`.
- **Fix direction:** Rename to `has_non_pawn_material` and update the test comment. No behavior change.

---

### TEST-010 — `analysis_tests.rs` cannot catch intra-call TT pollution

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/tests/analysis_tests.rs:16-28`; `backend/src/analysis/mod.rs`
- **Expected:** Tests verify that `analyze_move` returns results uncontaminated by the first search (before-move evaluation) tainting the second search (after-move evaluation).
- **Actual:** Every call to `evaluate` or `analyze_move` in the test creates a fresh `TranspositionTable`. The tests compare two independent calls and assert they are equal. They do NOT test whether the first search inside a single `analyze_move` call (eval_before) taints the TT used by the second search (eval_after) within the same call. If `analyze_move` uses one shared TT for both sub-searches, the second result is polluted by the first — and the determinism tests cannot reveal this because they always compare two fresh-TT runs.
- **Reasoning:** The test pair `(a, b)` compares `a.eval_before` to `b.eval_before` and `a.eval_after` to `b.eval_after`. Both `a` and `b` start from empty TTs. If the internal implementation uses one TT for both sub-searches, the pollution is self-consistent (same in both calls), so the equality assertion passes regardless. The gap is that there's no test that searches the pre-move position alone and compares its score to `analyze_move`'s `eval_before`.
- **Failure scenario:** `analyze_move("r3k2r/…", "e1g1")` evaluates the kiwipete position before castling (eval_before), storing entries. Then it searches after castling (eval_after) with a polluted TT. The `eval_before` score is correct; the `eval_after` score is wrong (TT hits from the different-color side corrupt it). The determinism tests pass because the bug is self-consistent.
- **Related:** context item #8 (`src/analysis/mod.rs` is identical to deleted `src/mcp/mod.rs`).
- **Where to look next:** `backend/src/analysis/mod.rs` (how TT is threaded through the two sub-searches); `backend/tests/analysis_tests.rs:61-76`.
- **Fix direction:** Add a test that calls `evaluate(KIWIPETE, NODES, …)` with a fresh TT, then calls `analyze_move(KIWIPETE, "e1g1", NODES, …)` with a fresh TT, and asserts `analyze_move.eval_before == evaluate.score` (they should be the same search on the same position). If they differ, TT pollution is confirmed.

---

### TEST-011 — `assert!(cnt_reset >= 1)` in `draw_invariants_tests.rs` always passes

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/draw_invariants_tests.rs:66-68`
- **Expected:** The assertion verifies a meaningful post-condition.
- **Actual:** `assert!(cnt_reset >= 1)` at line 67 is called on `b.repetition_count()` after undoing all moves back to startpos. `repetition_count()` counts how many times the current position appears in the history (including the current position itself), so the minimum possible return value is 1. The assertion can never be false.
- **Reasoning:** A correct implementation returns >= 1. A completely broken implementation that always returns 100 also satisfies `>= 1`. The assertion provides no discriminating power.
- **Failure scenario:** No observable failure — this is a false-confidence issue. If `repetition_count` was rewritten to return 0 for the initial position (a bug), this test would finally catch it — but every other test using it implicitly catches that already.
- **Related:** none.
- **Where to look next:** `backend/tests/draw_invariants_tests.rs:66-68`.
- **Fix direction:** Change to `assert_eq!(cnt_reset, 1)` — startpos should appear exactly once after fully undoing the knight shuffle, since only the initial position entry remains.

---

## Coverage map (per-file, all 46 `src/` files)

| File | LOC (approx) | Test coverage |
|---|---|---|
| src/lib.rs | ~30 | n/a (re-exports only) |
| src/bitboard.rs | ~80 | None — no inline tests, used implicitly throughout |
| src/square.rs | ~120 | Inline `#[cfg(test)]` module present |
| src/utils.rs | ~50 | **None** |
| src/logger.rs | ~60 | **None** |
| src/output.rs | ~80 | **None** |
| src/wasm.rs | ~200 | **None** (WASM build path untested on CI) |
| src/status.rs | ~300 | `status_tests.rs` (integration, 1,346 LOC/46 tests) |
| src/board/mod.rs | ~400 | Inline + `board/tests.rs` + many integration tests |
| src/board/fen.rs | ~200 | No inline tests; FEN parsing exercised indirectly via `Board::from_str` in all integration tests |
| src/board/fen_tables.rs | ~100 | Inline `#[cfg(test)]` present |
| src/board/castle_bits.rs | ~60 | Inline `#[cfg(test)]` present |
| src/board/tests.rs | ~200 | IS an inline test module |
| src/hash/mod.rs | ~10 | n/a (re-exports) |
| src/hash/zobrist.rs | ~100 | `zobrist_hash_tests.rs` (integration, 1,774 LOC/49 tests) |
| src/book/mod.rs | ~20 | **None** |
| src/book/polyglot_book.rs | ~400 | **None** |
| src/book/polyglot_entry.rs | ~100 | **None** |
| src/book/polyglot_hash.rs | ~400 | **None** |
| src/book/polyglot_keys.rs | ~200 | **None** |
| src/moves/mod.rs | ~10 | n/a |
| src/moves/types.rs | ~150 | No inline tests; types used in all integration tests |
| src/moves/execute.rs | ~400 | `moveexec_tests.rs`, `draw_invariants_tests.rs`, many others |
| src/moves/movegen.rs | ~200 | `movegen_tests.rs` (1,117 LOC/54 tests) |
| src/moves/pawn.rs | ~300 | Inline + `pawn_movegen_tests.rs` |
| src/moves/king.rs | ~150 | Inline `#[cfg(test)]` present |
| src/moves/knight.rs | ~100 | Inline `#[cfg(test)]` present |
| src/moves/perft.rs | ~200 | `perft_tests.rs` |
| src/moves/square_control.rs | ~200 | Inline + `square_control_tests.rs` |
| src/moves/magic/mod.rs | ~20 | n/a |
| src/moves/magic/loader.rs | ~100 | `magic_tests.rs` (partially — see TEST-002) |
| src/moves/magic/attacks.rs | ~100 | Inline `#[cfg(test)]` present |
| src/moves/magic/masks.rs | ~150 | Inline `#[cfg(test)]` present |
| src/moves/magic/precompute.rs | ~300 | Inline `#[cfg(test)]` present |
| src/moves/magic/search.rs | ~200 | `magic_tests.rs` (partially) |
| src/moves/magic/structs.rs | ~100 | Inline `#[cfg(test)]` present |
| src/search/mod.rs | ~10 | n/a |
| src/search/search.rs | ~400 | `determinism_tests.rs`, `search_tests.rs`, many others |
| src/search/eval.rs | ~300 | Inline + `eval_materials_tests.rs` |
| src/search/ordering.rs | ~200 | `ordering_tests.rs`, `tt_move_ordering_tests.rs` |
| src/search/pesto.rs | ~300 | No tests — PSQT tables exercised implicitly by eval tests |
| src/search/picker.rs | ~150 | Inline `#[cfg(test)]` present |
| src/search/context.rs | ~100 | `search_tests.rs` (used via `SearchContext::new()`) |
| src/search/tt.rs | ~300 | `tt_tests.rs` (1 test) + `tt_move_ordering_tests.rs` |
| src/search/see.rs | ~200 | `see_tests.rs` |
| src/analysis/mod.rs | ~200 | `analysis_tests.rs` |
| src/bin/cli.rs | ~515 | **None** |

**Files with zero test coverage: `src/bin/cli.rs`, `src/book/` (4 files), `src/logger.rs`, `src/output.rs`, `src/utils.rs`, `src/wasm.rs`, `src/bitboard.rs`**

---

## Acceptance gate audit (vantage-engine-pass.md lines 44-53)

All 6 required tests are present in `determinism_tests.rs`. Required positions (startpos, kiwipete, `r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8`) are all used.

| Gate test | Status | Test function(s) | Notes |
|---|---|---|---|
| 1. Reproducibility (score/nodes/best_move) | PRESENT | `determinism_startpos/kiwipete/middlegame` | ✓ all 3 positions |
| 2. PV reproducibility | PRESENT | Same 3 tests include `assert_eq!(a.pv, b.pv)` | ✓ |
| 3. PV consistency (pv[0]==best_move, replay legal, len<=depth) | PRESENT | `startpos_test/kiwipete_test/middlegame_test` | ✓ |
| 4. Node-cap reproducibility | PRESENT | `node_limit_deterministic_*` (3 tests) | ✓ |
| 5. Node-cap semantics (equals last completed depth) | PRESENT | `node_limit_returns_last_completed_depth_for_*` (3 tests) | ✓ — cap formula is sound |
| 6. Purity/order-independence | PRESENT | `tt_clear_makes_queries_independent` | ✓ |

One gap: `determinism_tests.rs:15` uses `TranspositionTable::new(512)` (512 MB), not the 64 MB the spec requires. See TEST-005.

---

## Perft depths

Active (not `#[ignore]`):
- Startpos: d1(20), d2(400), d3(8,902), d4(197,281), d5(4,865,609) — all exact ✓
- Kiwipete: d1(48), d2(2,039), d3(97,862), d4(4,085,603) — all exact ✓
- `divide_startpos_d2_matches_total`: node sum vs. 400 ✓

Ignored: startpos d6, d7; `perft_startpos_aggregate`; `perft_debug_divide`.

Depth-5 startpos and kiwipete-d4 are deep enough to catch EP/castling/promotion bugs. The kiwipete d2 breakdown test (`kiwipete_d2_tally`) verifies ep=1 and castles=91 explicitly.

---

## Tactical tests — assertion quality

| Test | Asserts specific move | Move asserted | Notes |
|---|---|---|---|
| `test_mate_in_1_scholars_mate` | No — score only | — | score < -500 |
| `test_back_rank_mate` | Yes | `d8d1` exactly | ✓ |
| `test_capture_hanging_queen` | Partial | `.ends_with("e5")` | Accepts any from-square capturing e5 |
| `test_avoid_hanging_piece` | **No** | — | Cannot fail; see TEST-003 |
| `test_fork_opportunity` | **No** | — | Cannot fail; see TEST-003 |
| `test_starting_position_sanity` | No — score only | — | `score.abs() < 150` |
| `test_piece_up_advantage` | No — score only | — | `score > 700` |
| `test_piece_down_disadvantage` | No — score only | — | `score < -800` |

---

## Clean bill

- **Perft expected values:** All active perft assertions use independently verifiable node counts against the Chessprogramming wiki standard values. ✓
- **PV legality check:** `determinism_tests.rs:35-43` replays every PV move against `generate_legal` — correctly catches illegal PV entries. ✓
- **Draw-by-fifty test:** `fifty_move_rule_becomes_claimable_at_100_halfmoves` directly sets `b.halfmove_clock = 99`, makes one quiet move, asserts `is_draw_by_fifty_move`. Semantically correct. ✓
- **TT mate score test:** `tt_tests.rs` stores a mate score and probes at two different plies; the comment correctly notes normalization happens in `search.rs`. The assertion that the raw stored value is returned unchanged is a valid design verification. ✓
- **`analysis_tests.rs` illegal-move test:** Correctly asserts that `analyze_move` for `"e2e5"` (illegal) returns `Ok` with `legal=false` and all optional fields `None`. ✓
- **Node-cap edge case:** `node_limit_depth_one_always_completes_*` tests (cap=1) verify the degenerate case. ✓
- **Purity test structure:** `tt_clear_makes_queries_independent` correctly calls `tt.clear()` between searches and uses independent `run()` baseline for comparison. ✓

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `vantage-engine-pass.md:44` | "small TT (64MB)" | `determinism_tests.rs:15` uses 512 MB | Determinism harness may miss TT-collision bugs only visible at small TT sizes | No — tests pass, but gate is weakened |
