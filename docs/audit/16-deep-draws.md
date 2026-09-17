# Draw Detection — Deep Audit Findings

**Agent:** deep-draws
**Scope:** `backend/src/status.rs`, `backend/src/moves/execute.rs`, `backend/src/board/mod.rs`, `backend/src/search/search.rs`, `backend/src/bin/cli.rs`, `backend/src/wasm.rs`, `frontend/src/app/chess/components/ChessGame.tsx`, `frontend/src/app/chess/hooks/useChessGame.ts`, `frontend/src/app/chess/engine/adapter.ts`, `frontend/src/app/chess/engine/useEngine.ts`, `frontend/src/app/chess/lib/gameStatus.ts`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/status.rs` — 135 LOC. Full read. Priority ordering, all draw conditions, `is_insufficient_material`.
- `backend/src/moves/execute.rs` — ~500 LOC. Grep for history, is_repetition; read lines 47-240 (make_move_basic Zobrist/history block), 320-400 (undo, null move).
- `backend/src/board/mod.rs` — grep + read lines 89-235 (history field, Board::new) and 440-490 (repetition_count, is_repetition, is_threefold).
- `backend/src/search/search.rs` — grep for is_repetition/threefold/draw; read lines 280-310 (repetition check at search root).
- `backend/src/bin/cli.rs` — full read to line 200; focused on handle_position (lines 79-111).
- `backend/src/wasm.rs` — grep for apply_move, get_game_status, status mapping; read lines 60-160, 290-305.
- `frontend/src/app/chess/components/ChessGame.tsx` — `git show main:…`; full read. Status flow traced.
- `frontend/src/app/chess/hooks/useChessGame.ts` — `git show main:…`; full read.
- `frontend/src/app/chess/engine/adapter.ts` and `useEngine.ts` — `git show main:…`; full read.
- `frontend/src/app/chess/lib/gameStatus.ts` — `git show main:…`; full read.
- `frontend/package.json` — `git show main:…`; confirmed `chess.js: ^1.4.0`.
- Existing findings: `docs/audit/01-board-fen.md` (BOARD-002, BOARD-004) and `docs/audit/03-exec-zobrist.md` (EXEC-003, EXEC-004) read in full.

## What I did not check, and why

- chess.js v1.4.0 source for its own insufficient-material logic. Would require fetching an npm package not present; noted where it matters under DRAW-005.
- Actual binary execution against the repetition-sequence hypothesis; the binary is at `backend/target/release/vantage-cli.exe` but the code path from cli.rs is unambiguous without running it.
- `backend/src/search/search.rs` beyond line 310 — threefold/fifty-move handling there is not referenced from status.rs.

## Findings

### DRAW-001 — Fifty-move AND seventy-five-move draw returned instead of checkmate when clocks are high

- **Severity:** P1
- **Confidence:** high
- **Status:** confirmed by code read
- **Anchors:** `backend/src/status.rs:105-119`, `backend/src/status.rs:97-134`
- **Expected:** FIDE Art. 5.1.1: the game is won by checkmate. Claimable draws (fifty-move, Art. 9.3) and automatic draws (seventy-five-move, Art. 9.6) do not override a delivered checkmate. A position that is simultaneously checkmate and beyond any move-count threshold is a win, not a draw.
- **Actual:** `position_status` checks `hmc >= 150` (DrawSeventyFiveMove) at line 105 and `is_threefold()` / `hmc >= 100` (DrawThreefold / DrawFiftyMove) at lines 115-119, all BEFORE `generate_legal` and the checkmate test at lines 123-134. Any position with `halfmove_clock >= 100` AND no legal moves AND in check returns DrawFiftyMove (or DrawSeventyFiveMove at 150) instead of Checkmate.
- **Reasoning:** Read `status.rs` top to bottom. The `DrawFiftyMove` branch at line 118 has no guard against the position being checkmate. The `generate_legal` call that would detect zero moves is at line 125, which is only reached if none of the earlier returns triggered. A mating move that is not a capture and not a pawn advance (e.g., a queen slide that gives back-rank mate) does not reset the halfmove clock, so `hmc >= 100` is reachable at mate in endgame studies or engine vs. engine games.
- **Failure scenario:** FEN `8/8/8/8/8/6k1/6p1/6K1 w - - 100 120` — if White has no legal moves and is in check with hmc=100, `position_status` returns `DrawFiftyMove`. The losing side is credited a draw they did not earn. A concrete reachable endgame: two rooks vs bare king, last 50 moves were non-pawn non-capture (pure rook manoeuvres), then a discovered checkmate is delivered on move 100. The engine or UI reports draw. Constructing an exact FEN requires a tablebase lookup, which is beyond this audit's scope, but the code path is unambiguously reachable.
- **Verdict on BOARD-002:** CONFIRMED (also extends to seventy-five-move).
- **Related:** BOARD-002, DRAW-005 (status.rs is the sole source of truth for the frontend).
- **Where to look next:** `status.rs:113-134`. The fix is to move `DrawThreefold` and `DrawFiftyMove` checks to after the `legal.is_empty()` branch (i.e., only claim a fifty-move draw when the position is not already checkmate or stalemate). The automatic draws (fivefold, seventy-five-move, dead position) may remain first under one interpretation of FIDE, but seventy-five-move should also follow mate detection for consistency.
- **Fix direction:** After calling `generate_legal`, if `legal.is_empty()` is true: return Checkmate or Stalemate as appropriate. Only if `legal` is non-empty, check the claimable draws (threefold, fifty-move). The automatic draws (fivefold, seventy-five-move) are debated in FIDE literature; a conservative fix moves all draw-before-mate checks to after the terminal-result test.

---

### DRAW-002 — Search's `is_repetition` triggers at second occurrence; `status.rs` threefold requires third

- **Severity:** P2
- **Confidence:** high
- **Status:** confirmed by code read
- **Anchors:** `backend/src/search/search.rs:286`, `backend/src/board/mod.rs:469-481`, `backend/src/status.rs:115`
- **Expected:** The search should consistently model draw-by-repetition under the same rule it claims in the final status. Internal asymmetry can cause the engine to evaluate a position as drawn (score 0) that is still "active" from the UI's perspective, or vice versa.
- **Actual:** `search.rs:286`: `if ply > 0 && board.is_repetition()` returns DRAW_SCORE when the current position appears ONCE in history (total occurrences = 2). `is_repetition` at `board/mod.rs:469` returns true on the first history match. By contrast, `status.rs:115` calls `board.is_threefold()` which requires `repetition_count() >= 3` (two history matches plus current = three total). The search engine therefore scores a position 0 after the second occurrence, but `position_status` only reports `DrawThreefold` after the third — meaning the engine avoids positions it calls "drawn" that the GUI wouldn't recognize as drawn yet.
- **Reasoning:** Two code paths, two thresholds. `is_repetition()` and `is_threefold()` are two different functions. `search.rs` calls `is_repetition`; `status.rs` calls `is_threefold`. The gap is intentional in many engines (avoiding 3-fold by treating 2-fold as zero), but the engine may refuse a move that leads to a "claimed draw" position that no human can actually claim.
- **Failure scenario:** Position seen twice in the game history. Side to move is winning. The winning move leads to the position for the third time. The engine sees it as "draw" (is_repetition = true at ply 1+), scores it 0, and chooses an inferior move instead. No draw is ever claimed because the position is only reached twice; the engine threw away a win.
- **Related:** DRAW-001, EXEC-004.
- **Where to look next:** `search.rs:286`, `board/mod.rs:469`.
- **Fix direction:** Either use `repetition_count() >= 2` in search (matching status.rs's claim threshold minus one, so the search reaches but does not play the third repetition) and document the deliberate asymmetry, or switch search to `repetition_count() >= 3` for exact alignment at the cost of the engine sometimes walking into a draw it could have avoided.

---

### DRAW-003 — EXEC-004 REFUTED: game-history positions ARE loaded into `board.history`

- **Severity:** N/A
- **Confidence:** high
- **Status:** refuted
- **Anchors:** `backend/src/bin/cli.rs:99-108`, `backend/src/moves/execute.rs:214-232`
- **Reasoning:** `handle_position` (cli.rs:79-111) starts with `Board::new()` then loops over every move in the `position … moves` list, applying each via `make_move_basic`. `make_move_basic` (execute.rs:214-232) always pushes the pre-move Zobrist to `board.history` (or clears history and pushes it on irreversible). After the full move list, `board.history` contains all positions since the last irreversible move in the game, exactly what repetition detection requires. The WASM path (`set_position_startpos`, wasm.rs:64-75) does the same via `apply_move` → `make_move_basic`. EXEC-004's low-confidence hypothesis that history might be empty at the search root is wrong.
- **Verdict on EXEC-004:** REFUTED.

---

### DRAW-004 — EXEC-003: pre-irreversible hash in history is harmless, not P2

- **Severity:** P3
- **Confidence:** high
- **Status:** confirmed
- **Anchors:** `backend/src/moves/execute.rs:224-232`, `backend/src/board/mod.rs:446-477`
- **Reasoning:** After an irreversible move, `execute.rs:225` clears history, then line 230 pushes `start_zobrist` (the pre-move position). This hash represents a position that can never be legally revisited (a capture removed a piece; a pawn move cannot be undone). `repetition_count()` compares the CURRENT Zobrist against all entries; a 64-bit Zobrist collision between the unreachable pre-capture position and any future position is astronomically unlikely (~1 in 2^64). No false positive repetition is realistically possible. This is a semantic incorrectness (the history window logically begins at the post-irreversible position) but has no observable correctness impact.
- **Verdict on EXEC-003:** Severity is P3 (hygiene), not P2. The finding is correct in its description but overclaims severity.

---

### DRAW-005 — KBB same-color bishops reported as sufficient material; user-visible via the frontend

- **Severity:** P2
- **Confidence:** high
- **Status:** confirmed
- **Anchors:** `backend/src/status.rs:67-86`, `backend/src/wasm.rs:290-302`
- **Expected:** FIDE Art. 5.2.2: the game is drawn when a position is reached from which checkmate cannot occur by any sequence of legal moves (dead position). KBB vs K with both bishops on the same color square is such a position.
- **Actual:** `is_insufficient_material` at `status.rs:68-81`: for KBB vs K (`wb=2, wn=0, bb=0, bn=0`), `total_minors=2`, neither `wn==2` nor `w_minors==1 && b_minors==1` holds, so the function falls through to `return false`. The position is treated as "sufficient material." `position_status` reaches `generate_legal`, finds legal moves, and returns `InPlay`.
- **Frontend impact:** The frontend receives its game status exclusively from `wasm.get_game_status_internal()` → `position_status` → `status.rs`. There is no independent frontend draw check for insufficient material. So the KBB same-color position is displayed as "active" to the user; the game continues beyond what FIDE law allows.
- **Verdict on BOARD-004:** CONFIRMED; also confirmed to be user-visible.
- **Related:** BOARD-004, DRAW-001 (both in status.rs).
- **Where to look next:** `status.rs:68-81`. The fix requires a bishop-color check (AND each bishop's square index with the light-squares mask `0x55AA55AA55AA55AA`; if all are nonzero or all are zero, they share a color).
- **Fix direction:** In the `total_minors == 2` branch, add a case: if `wb == 2 && wn == 0 && b_minors == 0`, check whether both white bishops are on the same square color. If yes, return true. Mirror for `bb == 2`. Two bishops of different colors CAN mate, so the color check is mandatory.

---

## Cross-implementation draw detection table

| Draw condition | `status.rs` | Search internal (`search.rs:286`) | Frontend (status source) | Agree? | User-visible symptom if not |
|---|---|---|---|---|---|
| Threefold repetition | `is_threefold()` → 3rd occurrence | `is_repetition()` → 2nd occurrence → DRAW_SCORE | Rust engine (WASM only) | No — search declares draw 1 occurrence early | Engine avoids third repetition and throws a win |
| Fivefold repetition | `repetition_count() >= 5` → automatic | Same `is_repetition()` at ply>0 | Rust engine | Partial | Fivefold never claimed in search; search already returned draw at 2nd occurrence |
| Fifty-move rule | `hmc >= 100` before checkmate check | Not checked in search at all | Rust engine | No — draw can mask checkmate | Winning player denied checkmate credit (DRAW-001) |
| Seventy-five-move rule | `hmc >= 150` before checkmate check | Not checked in search | Rust engine | No — same masking bug | Same as fifty-move (DRAW-001) |
| Stalemate | Correct (after legal-move gen, no check) | Legal-move-gen returns empty with score 0 | Rust engine | Yes | None |
| Checkmate | Correct — but reachable only if no draw triggers above | Returns mating score | Rust engine | Partial — masked by DRAW-001 | Draw reported instead of checkmate when hmc ≥ 100 |
| Insufficient material: K vs K | `total_minors == 0` → true | Not checked in search | Rust engine | Yes | None |
| Insufficient material: K+N vs K | `total_minors == 1` → true | Not checked in search | Rust engine | Yes | None |
| Insufficient material: K+B vs K | `total_minors == 1` → true | Not checked in search | Rust engine | Yes | None |
| Insufficient material: K+N+N vs K | `wn==2` → true | Not checked | Rust engine | Yes (convention: KNN treated as draw; defensible but not FIDE-required) | None |
| Insufficient material: K+B vs K+B (any color) | `w_minors==1 && b_minors==1` → true | Not checked | Rust engine | Yes — draws all KB-vs-KB regardless of bishop colors (overly conservative but not wrong) | None |
| Insufficient material: K+B+B vs K same-color | Falls through to `false` → NOT detected | Not checked | Rust engine | No | Game continues past a dead position (DRAW-005) |
| Insufficient material: K+B+B vs K different-color | `false` → NOT detected | Not checked | Rust engine | No — but this case CAN mate; false negative is actually a false negative on draw | Game continues correctly; the function is wrong to return false here too (KBB diff-color CAN mate) — CORRECT to return false |
| Dead position (general) | Only via insufficient material check | Not checked | Rust engine | Partial | Missing same-color bishop case |

## Clean bill

- **`position_status` automatic draws (fivefold, 75-move, dead position) are checked before claimable ones.** The comment at `status.rs:88-95` matches the code exactly. The fivefold and 75-move ordering relative to each other is correct per FIDE (automatic before claimable).
- **`handle_position` populates history correctly.** Every move is applied through `make_move_basic`, which maintains the history Vec. EXEC-004 is refuted.
- **`wasm.rs` status-string mapping is complete and correct.** All seven `GameStatus` variants map to the strings the frontend's `GameStatus` type expects (verified against `gameStatus.ts` and the adapter). No missing or misspelled variant.
- **`is_repetition` vs `repetition_count` naming is clear.** Both functions are correctly implemented relative to their own spec. The asymmetry with the search is a design choice (not a bug in either function in isolation).
- **Null-move history push does not cause false repetition.** `make_null_move` pushes the pre-null-move Zobrist (before flipping side). The side-to-move key is XORed in after the push, so the pushed hash differs from the post-null board hash by exactly the side key. A future position can only match it if pieces AND side coincide with the pre-null state — which would require an impossible time reversal.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `status.rs:88-95` (doc comment) | Priority list implies claimable draws are below automatic draws but above checkmate | Code implements draw-before-checkmate for claimable draws | status.rs | No — a correctness fix, not a missing feature |
