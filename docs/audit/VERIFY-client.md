# Verification Wave — Client Agent (verify-client)

**Agent:** verify-client
**Scope:** 08-wasm-analysis.md (WASM-*, ANALYSIS-*), 09-frontend.md (FE-001..006),
  10-wasm-js-contract.md (CONTRACT-001..008), 18-deep-frontend-state.md (FE-201, root causes A/B/C),
  07-book-uci-cli.md (BOOK-*, UCI-001..005), 00-uci-live.md (UCI-101..111),
  06-ordering-tt-see.md (ORDER-004, ORDER-005), 19-deep-tt.md (ORDER-203, ORDER-204 — refutations)
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/analysis/mod.rs` (206 LOC) — full read
- `backend/src/search/search.rs` — grep for constants (INF, MATE_SCORE), no-legal-moves paths,
  `last_completed_best_score` init and update logic, abort paths (lines 627-790 sed read)
- `backend/src/search/see.rs` (157 LOC) — full read; traced swap loop step-by-step
- `backend/src/search/ordering.rs` — grep for pub fn / order_moves
- `grep -rn order_moves backend/` — all call sites across all files
- `git show main:frontend/public/wasm/vantage.worker.js` — full read
- `git show main:frontend/src/app/chess/hooks/useChessGame.ts` — full read
- `git show main:frontend/src/app/chess/components/ChessGame.tsx` — full read
- `backend/src/wasm.rs` — grep for set_position_fen signature (found: `pub fn set_position_fen(&mut self, fen: &str) -> bool`)
- `backend/src/bin/cli.rs` — grep stop, go, bestmove 0000; sed read around line 315 (no-move path)
- `scripts/run_gauntlet.py` (head 60) — TIME_CONTROL = "20+0.2" (cutechess-cli tc= format)
- `docs/audit/06-ordering-tt-see.md` — full read of ORDER-001, ORDER-004, ORDER-005
- `docs/audit/07-book-uci-cli.md` — full read of BOOK-002, UCI-001..005
- `docs/audit/08-wasm-analysis.md` — full read of ANALYSIS-001..006, WASM-001
- `docs/audit/09-frontend.md` — full read of FE-001..006
- `docs/audit/10-wasm-js-contract.md` — full read of CONTRACT-001..008
- `docs/audit/18-deep-frontend-state.md` — full read of FE-201 and root-cause table
- `docs/audit/19-deep-tt.md` — full read of ORDER-203 and ORDER-204
- `docs/audit/00-uci-live.md` — read UCI-102..111 plus live-test results table

## What I did not check, and why

- `useEngine.ts` exact line numbers — `git show main:frontend/src/app/chess/engine/useEngine.ts`
  returned no output (path might differ); relied on 09-frontend / 10-wasm-js-contract agents'
  excerpts, which cite specific line numbers. Verdict on FE-002/CONTRACT-001 is CONFIRMED via
  ChessGame.tsx and worker.js reads which are the authoritative error-path code.
- Runtime browser behavior for FE-201 — cannot run npm build; routed to UNVERIFIED with repro steps.
- `analysis_tests.rs` — trusted ORDER-203's first-party read rather than re-reading.

---

## Verdicts

### ANALYSIS-001 — `Score::from_raw(i32::MIN)` panics in debug

- **Original severity:** P1 | **Revised severity:** REJECT (no finding)
- **Verdict:** REJECTED
- **Evidence:** `search.rs:20` defines `const INF: i32 = 32000`. `search()` initialises
  `last_completed_best_score = 0` and only updates it after a depth completes (line 735). The
  no-legal-moves checkmate path returns `-MATE_SCORE + ply` where MATE_SCORE = 31000; stalemate
  returns 0. All scores are therefore bounded in [-32000, 32000]. The path from search to
  `Score::from_raw` is `analysis/mod.rs:75` → `Score::from_raw(result.score)`. `i32::MIN` is
  structurally unreachable from this call chain. The original agent correctly flagged medium
  confidence ("I did not read search.rs fully") — reading it settles the question as a non-bug.
  No defensive fix is needed because there is no reachable input.

---

### ORDER-203 — Refutation of ANALYSIS-003 (TT clears are present)

- **Original severity:** P3 (test-gap only) | **Revised severity:** P3
- **Verdict:** CONFIRMED (refutation is sound)
- **Evidence:** Read `analysis/mod.rs` in full. `evaluate`: `tt.clear()` at line 67, before search.
  `analyze_move` legal-move path: `tt.clear()` at line 173 (before first search), `tt.clear()` at
  line 188 (before second search). The illegal-move early return (lines 160-171) exits before
  either clear — but that return path performs no search, so no TT writes occur and there is no
  pollution. Refutation reasoning is correct; ANALYSIS-003's P2 finding is reduced to test-gap
  (P3) as ORDER-203 states.

---

### ORDER-204 — Refutation of ORDER-001 (SEE x-ray one-iteration delay)

- **Original severity:** P2 (ORDER-001) | **Revised severity:** REJECT ORDER-001
- **Verdict:** CONFIRMED (refutation is sound)
- **Evidence:** Traced the `see.rs` swap loop step-by-step (lines 100-125). At iteration d:
  `next_victim` holds the piece type from iteration d-1. The refresh fires at d when
  `next_victim` (= piece removed at d-1) is a slider. Concrete trace: Black Bishop removed at d=1
  (occupancy updated at line 108); at d=2 `next_victim=Bishop` triggers refresh; updated
  occupancy (Bishop and d=2 attacker both removed) reveals the Black Rook. The Rook is selectable
  at d=3, which is Black's turn. If instead we refreshed at d=1 (as ORDER-001 demands), the Rook
  would be in `attackers` from d=1 but White selects at d=2, so Black still takes at d=3. Gain
  array is identical. The delay is exactly one iteration and the revealed piece is always on the
  same side as its blocker — both are selected on that side's turns. No numerical difference
  results. ORDER-001 is REJECTED.

---

### FE-002 / CONTRACT-001 — Same defect, confirmed single fix location

- **Original severity:** P1 | **Revised severity:** P1
- **Verdict:** CONFIRMED; these are the same defect from two sides
- **Evidence:** `vantage.worker.js` onerror sets error state but no `pendingRef` interaction
  (confirmed by reading the worker file). The error UI at `ChessGame.tsx:161` (`if (error) return
  <div className="p-6 text-destructive">Error: {error}</div>`) replaces the board on any
  `onerror` path, so the `isThinking` leak is secondary (UI replaced). The deep-dive's UX claim
  is honest: the user sees an error screen, not a frozen board. Fix location: `useEngine.ts:32-38`
  (the onerror handler). No separate fix needed in ChessGame; the error UI renders correctly.
  `isThinking` stuck true matters only if the component re-mounts without the error state,
  which does not happen in the current implementation.

---

### FE-201 — New Game mid-search race (new P1 finding)

- **Original severity:** P1 | **Revised severity:** PLAUSIBLE — needs runtime confirmation
- **Verdict:** PLAUSIBLE
- **Evidence (code path):** `handleStartGame` at ChessGame.tsx:57 calls `await engine.newGame()`
  which is a Worker postMessage round-trip. The Worker queues `new_game` behind any in-flight
  `search`. If the Worker finishes the search reply before processing `new_game`, the main thread
  receives the search result while still awaiting `engine.newGame()`, meaning `chess.startNewGame()`
  has NOT been called yet and `gameKeyRef.current` still equals `keyAtStart`. The gameKey guard
  passes. `chess.applyLocalMove` advances `chessRef`; then `engine.makeMove` is sent to the
  Worker. The Worker then processes `new_game` (resetting engine state) followed by the stale
  `apply_move` (corrupting the fresh engine). The deep-dive's trace at steps 4-8 is internally
  consistent with the code. No guard is missed — the `gameKeyRef` useEffect update only fires
  after `chess.startNewGame()` dispatches, which happens after the `await`. The race window is
  real. **Runtime confirmation needed.**

- **Manual repro steps (60 seconds in browser):** Open the chess UI. Start a game as White.
  Play 5-6 moves until the engine is at a non-trivial position. At the moment the engine indicator
  shows thinking (eval bar animated), immediately click "New Game" and select Black. Watch the
  first engine move (engine plays White). If the race fires, the engine's first move comes from
  startpos+one_stale_move rather than startpos; the board's first position will show an illegal
  extra move already played. Reproducibility may require a slow search (Expert/5000ms) and fast
  New Game click timing.

---

### CONTRACT-002 — `set_position_fen` bool return dropped

- **Original severity:** P1 | **Revised severity:** P3
- **Verdict:** Downgraded. Finding is real but P1 overstatement.
- **Evidence:** `wasm.rs:54-60` confirmed: `set_position_fen` returns `bool`. Worker line 28 drops
  it. However, the app never constructs FEN strings to send — all FENs originate from
  `engine.get_fen()` which only returns valid FENs, or from startpos (also valid). Grepping
  `set_position` call sites in `ChessGame.tsx` and `useChessGame.ts` confirms there are no
  user-supplied FEN inputs on the main path. An invalid FEN is therefore not reachable in any
  current UI flow. This is defensive hygiene (P3), not a live defect. If an analysis mode is
  added (per `phase-1-frontend.md` intent), this silently-dropped bool becomes P1 again.

---

### UCI-105 — `bestmove 0000` for legal positions at depth ≤ 0

- **Original severity:** P1 | **Revised severity:** P2
- **Verdict:** Downgraded. Defect confirmed; P1 severity overstated.
- **Evidence:** Confirmed in live tests (UCI-109/UCI-123 rows): `go depth 0`, `go depth -1`, and
  `go movetime 0` all return `bestmove 0000`. The gauntlet script (`scripts/run_gauntlet.py`)
  uses `TIME_CONTROL = "20+0.2"` passed to cutechess-cli as `tc=20+0.2` — this sends
  `go wtime/btime/movestogo` commands, never `go depth 0` or `go movetime 0`. Standard GUIs also
  do not send these. The WASM path uses fixed depth (not the CLI). The defect is a UCI violation
  but is not triggered by any of the engine's actual consumers. P2.

---

### UCI-102 — `stop` not implemented; search blocks input loop

- **Original severity:** P1 | **Revised severity:** P1
- **Verdict:** CONFIRMED P1. Severity maintained.
- **Evidence:** `cli.rs:30` single-threaded loop; no `stop` arm (confirmed by grep). Search is
  synchronous. The gauntlet uses cutechess-cli with `tc=20+0.2` (incremental time control).
  cutechess-cli issues `go wtime X btime Y movestogo Z` and will send `stop` when the GUI-side
  clock expires or detects an error. An unresponsive `stop` causes cutechess-cli to declare a
  timeout forfeit. This directly impacts gauntlet reliability and any real tournament play.
  The WASM path is unaffected (no UCI there). P1 stands for the CLI/gauntlet consumer.

---

### BOOK-002 — Book probe non-deterministic (unseeded RNG)

- **Original severity:** P2 | **Revised severity:** P2 (but analysis API is unaffected)
- **Verdict:** CONFIRMED at P2 for UCI play; DOES NOT affect analysis API.
- **Evidence:** `analysis/mod.rs` (full read) shows `evaluate` and `analyze_move` do not accept or
  consult a `PolyglotBook`. The book parameter exists only in `cli.rs/handle_go`. All analysis
  tests create a fresh TT and call `evaluate`/`analyze_move` directly — no book involved. The
  analysis API's determinism is intact. BOOK-002 remains P2 for UCI play (gauntlet games may be
  non-reproducible when opening positions hit the book), but it does NOT threaten the MCP server's
  determinism claims.

---

### ORDER-004 — `order_moves()` is dead production code

- **Original severity:** P3 | **Revised severity:** P3
- **Verdict:** CONFIRMED P3.
- **Evidence:** `grep -rn order_moves backend/` found exactly two files:
  `src/search/ordering.rs:31` (definition) and `tests/ordering_tests.rs` (six call sites in tests
  only). `search.rs` imports only `MovePicker` from `picker.rs`; `picker.rs` imports only
  `mvv_lva_score` from `ordering.rs`. `order_moves` is never called from production code. Tests
  exercise dead logic; the pawn-rank bonus diverges between the two implementations
  (ordering.rs:91 uses `3..=5`, picker.rs uses `3|4` and `5|6`).

---

## Overstatement checks

| ID | Verdict | Revised sev | Note |
|----|---------|-------------|------|
| WASM-001 | CONFIRMED | P2 | `set_position_startpos` returns void; JS cannot detect truncation. Real gap post-e7cc241. |
| CONTRACT-003 | CONFIRMED | P2 | `payload.depth` truthy check: depth=0 silently upgrades to depth=10. Real but no current caller sends depth=0. |
| CONTRACT-004 | CONFIRMED | P2 | Negative movetime wraps via ToUint32. No current UI sends negative values; medium-confidence future risk. |
| CONTRACT-006 | CONFIRMED | P2 | Module Worker unsupported in Firefox <114 and older Safari. No feature-detect guard. |
| CONTRACT-008 | CONFIRMED | P3 | `apply_move` Wasm export unused. Dead binary surface, no runtime impact. |
| FE-006 | CONFIRMED | P3 | COOP/COEP headers only on `/wasm/*`; no SharedArrayBuffer usage so headers serve no function. Misleading config but harmless. |
| UCI-108 | CONFIRMED | P3 | `info string Target time:` on stdout is valid UCI but noisy for a future MCP adapter. |
| UCI-109 | CONFIRMED | P3 | `fen`/`d` commands write raw non-UCI data to stdout. Real protocol violation, P3 since no GUI sends these. |
| UCI-111 | CONFIRMED | P3 | `info depth` lines omit `nps`. Minor GUI-display gap. |
| ANALYSIS-004 | CONFIRMED | P2 | Score serde shape (external tag `{"cp":50}`) diverges from MCP_BUILD_PLAN's planned internal-tag. Locking wrong wire format before Phase 2. |
| ANALYSIS-005 | CONFIRMED | P3 | `Result<_, String>` instead of `AnalysisError` enum. Phase 4 prerequisite gap but not yet blocking. |
| ORDER-005 | CONFIRMED | P3 | Dead `_depth`, `_alpha`, `_beta`, `_ply` params on `tt::probe`/`tt::save`. API misleads callers; no runtime impact. |

---

## Needs browser / runtime confirmation

| ID | What needs testing | Exact repro |
|----|-------------------|-------------|
| FE-201 | New Game race mid-search contaminates fresh engine state | Start game as White. After 5+ moves, when engine-thinking indicator is active, click New Game → choose Black. Observe first engine move: if board shows startpos+extra_move rather than startpos, race fired. Reliable with Expert/5000ms difficulty. |
| FE-001 | Double-move window between executeMove return and isThinking set | On mobile or with precise timing, drag a piece; before spinner appears, immediately drag a second piece. If both moves land, board will show 3-ply state (2 player + 1 engine). |
| CONTRACT-006 | Module Worker failure on Firefox ESR | Open chess.vaishakmenon.com in Firefox 102 ESR. Engine init should fail; error UI should appear (no fallback). |

---

## Clean bill (positively verified)

- **ANALYSIS-001:** Not a bug. `INF=32000`, all search scores bounded ±32000, `i32::MIN` unreachable.
- **ORDER-001 (SEE x-ray):** ORDER-204's refutation is sound. Swap-loop delayed refresh produces identical gain arrays. No wrong SEE values.
- **ANALYSIS-003 (TT pollution):** ORDER-203's refutation is sound. All analysis entry points clear TT before every search.
- **FE-002/CONTRACT-001 error UI:** `if (error) return <div>Error: {error}</div>` at ChessGame.tsx:161 confirms board is replaced (not frozen) on onerror.
- **CONTRACT-002 reachability:** Invalid FEN is unreachable from current UI flows; all FENs are engine-produced.
- **BOOK-002 analysis impact:** Book is not consulted in `evaluate` or `analyze_move`; analysis API determinism is intact.
- **UCI-105 gauntlet impact:** Gauntlet uses `tc=20+0.2` (time-controlled), never `depth 0` or `movetime 0`; defect does not affect gauntlet runs.
