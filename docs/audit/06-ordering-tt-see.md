# Move Ordering, TT, and SEE — Audit Findings

**Agent:** audit-ordering
**Scope:** `backend/src/search/picker.rs` (632 LOC), `backend/src/search/ordering.rs` (99 LOC), `backend/src/search/see.rs` (156 LOC), `backend/src/search/tt.rs` (118 LOC), `backend/tests/ordering_tests.rs` (276 LOC), `backend/tests/see_tests.rs` (120 LOC), `backend/tests/tt_tests.rs` (45 LOC), `backend/tests/tt_move_ordering_tests.rs` (347 LOC), `backend/src/search/search.rs` (780 LOC, read for TT cutoff path), `backend/src/search/context.rs` (55 LOC)
**Date:** 2026-08-30 (overnight run)

## What I checked

- Read all four primary files in full.
- Read all four test files in full.
- Read `src/search/search.rs` in full to trace the TT probe-to-play path.
- Read `src/search/context.rs` in full for killer/history data structures.
- Ran `grep -r new_search backend/` — confirmed `new_search()` is defined only once (`tt.rs:57`) and never called anywhere.
- Verified TT hash move validation path: `probe()` → `hash_move` set in `alpha_beta` → `MovePicker::new(hash_move, ...)` → `picker.rs:246` validates with `is_pseudo_legal` + `is_legal_move`.
- Verified bound-type arithmetic in `alpha_beta` (search.rs:310-315) and `quiescence` (search.rs:163-169).
- Verified SEE x-ray loop (see.rs:100-125) against the standard exchange simulation algorithm.
- Verified killer slot indexing, deduplication, and legality validation.
- Verified history update conditions (search.rs:580-585).
- Checked `order_moves` usage: not imported in search.rs or picker.rs — dead production code.

## What I did not check, and why

- Did not read `VANTAGE_ROADMAP.md` for exact line numbers (not in assigned scope); countermove status inferred from absence in source.
- Did not build or run tests (per brief rules).
- Did not verify the SEE x-ray bug with a programmatic test (no execution allowed); failure scenario is manual trace.

---

## Findings

### ORDER-001 — SEE x-ray refresh triggers on the wrong variable

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/see.rs:110-113`
- **Expected:** After removing a piece at `attacker_sq` from occupancy, the attacker set should be refreshed if that removed piece is a slider (Bishop, Rook, Queen), because sliders can hide other pieces behind them (x-rays). This is the standard SEE x-ray mechanism as described in the Chessprogramming wiki's "Static Exchange Evaluation" page.
- **Actual:** The refresh is gated on `matches!(next_victim, Piece::Bishop | Piece::Rook | Piece::Queen)`, where `next_victim` is the piece type currently occupying the **target square** (the result of the previous capture), not the piece at `attacker_sq` (the piece just removed from occupancy). The condition is checking the wrong piece.
- **Reasoning:** `next_victim` is set before the loop to `piece_type_at(m.from)` (the initial attacker), and updated at the END of each iteration to `piece_at(attacker_sq)` (the current LVA). Inside the loop, at the point of the x-ray check, `next_victim` reflects the piece that made the **previous** capture (now sitting on the target square). The x-ray reveal is caused by removing `attacker_sq` from occupancy, not by the target-square piece. When the initial (or previous) capturer was a non-slider (Pawn, Knight, King) and the current LVA being removed is a slider (Rook, Bishop, Queen), the check evaluates to false and no refresh happens, leaving any piece hidden behind the removed slider undiscovered for at least one more iteration.
- **Failure scenario:** White Pawn b3 captures Black Pawn at c4. Black has a Bishop at d5 (direct diagonal attacker) and a Rook at e6 hidden behind the Bishop on the same diagonal. White then has a second attacker (say, Knight at d2). In the SEE exchange: (1) White Pawn captures (next_victim = Pawn, not slider). (2) Black Bishop captures at c4 — occupancy removes Bishop at d5. Check: `next_victim = Pawn`, not slider → NO refresh. Black Rook at e6 is not revealed. (3) White Knight captures — now `next_victim = Black Bishop` (slider) → refresh fires, Rook finally revealed. But by this point it is White's turn; Rook won't be selected and only appears on Black's next turn. When the exchange has exactly 2 White attackers and Black has [Bishop + hidden Rook], SEE may compute a different gain array than correct play requires. The gain is off whenever the hidden piece would have been the LVA in the missed iteration, or when the exchange terminates before the delayed reveal.
- **Related:** None in this area. The qsearch SEE pruning at `search.rs:217` and the capture classification in `picker.rs:129` both depend on SEE correctness.
- **Where to look next:** `see.rs:107-123` (the loop body); compare `next_victim` vs piece identity of `attacker_sq` piece just before line 110.
- **Fix direction:** Replace the trigger with a check on the type of the piece that was just removed. Fetch the piece type at `attacker_sq` before updating occupancy (line 108), store it in a local variable, and use that local to gate the x-ray refresh instead of `next_victim`. The piece at `attacker_sq` is currently available via `self.piece_at(Square::from_index(attacker_sq))` but is fetched again at line 121 only for `next_victim`; an earlier fetch for the refresh check resolves the issue.

---

### ORDER-002 — `tt.new_search()` is never called; TT generation-based aging is dead code

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/tt.rs:57-59`, `backend/src/search/tt.rs:88`, `backend/src/search/search.rs` (entire file — no call site found)
- **Expected:** The TT replacement policy at `tt.rs:88` includes the condition `entry.generation != self.generation`, intended to preferentially evict entries from previous searches and keep the table fresh. This requires `new_search()` to be called at least once per UCI `go` command (or per iterative-deepening iteration) to advance `self.generation`.
- **Actual:** `grep -r new_search backend/` finds exactly one location: the definition in `tt.rs:57`. There is no call site anywhere in the codebase. `TranspositionTable::generation` is initialized to 0 (line 53) and stays 0 permanently. The condition `entry.generation != self.generation` is always `0 != 0 = false` and never triggers preferential replacement. Replacement falls back to `entry.key == 0 || depth >= entry.depth` — which is purely depth-preferred with empty-slot preference, and stale entries from a previous game are never evicted on their own.
- **Reasoning:** Confirmed by reading every function in search.rs (780 lines) — `tt.new_search()` is not called. The `tt.clear()` method at `tt.rs:61` resets the generation to 0, so even if `clear()` is called between games, generation remains 0. Confirmed by grep returning only the definition.
- **Failure scenario:** After a long game with a 512 MB TT, entries from positions 200 moves ago will remain and be probed (with correct Zobrist key check preventing wrong-position cutoffs, so this is not P0). However, the table never evicts old entries in favor of current-search entries at the same depth. In iterative deepening at depth N, entries from depth N of a prior position can crowd out current-depth-N entries if their Zobrist collides on the same index, making the replacement appear "at least as deep" and preventing the update.
- **Related:** ORDER-004 (dead API parameters on probe/save).
- **Where to look next:** `backend/src/bin/cli.rs` or wherever the UCI `ucinewgame` / `go` command is handled — `tt.new_search()` should be called there before each `search()` call.
- **Fix direction:** Call `tt.new_search()` at the top of the `search()` function in `search.rs` (before the iterative deepening loop), and also when handling the UCI `ucinewgame` command in the CLI. This is a one-line addition.

---

### ORDER-003 — TT cutoff applied at PV nodes, truncating the displayed PV

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs:305-318`, `backend/src/search/context.rs:18-20`
- **Expected:** Standard PVS practice: at PV nodes (nodes where the alpha-beta window is open, i.e., `beta - alpha > 1`), TT cutoffs should be suppressed or at least not used to return early, because doing so truncates the principal variation. The PV table (`ctx.pv_table`) is populated only when moves improve alpha (search.rs:560-566), but a TT cutoff at a PV node short-circuits before any move is searched and leaves `ctx.pv_length[ply]` at 0 (set at line 283).
- **Actual:** The TT cutoff logic at search.rs:309-317 fires whenever `ply > 0` and `tt_depth >= depth`. There is no check for `beta - alpha > 1` (PV node) before applying the cutoff. At PV nodes where an exact TT entry exists at sufficient depth, the function returns immediately with `(tt_score, tt_move)` and the PV table for that ply is not populated.
- **Reasoning:** `ctx.pv_length[ply] = 0` is set at line 283 unconditionally. The PV table is only written at lines 561-565 (when `score > alpha`). A TT exact-bound cutoff at line 311 returns before the move loop, so no PV entries are written. The PV displayed to the GUI is shorter than the actual search depth.
- **Failure scenario:** At depth 8, the root's best continuation passes through a position that has an exact TT hit at depth 6. The PV is truncated at that ply and the GUI receives `pv e2e4 d7d5` instead of the full 8-ply line. This is a display/UX defect, not an engine-strength defect, but misrepresents search depth to the GUI.
- **Related:** None in this area.
- **Where to look next:** `search.rs:305-318` (TT cutoff block). Also `search.rs:309` — adding `&& beta - alpha <= 1` (non-PV node condition) before the match block suppresses the cutoff at PV nodes.
- **Fix direction:** Guard the TT score-based cutoff with a non-PV-node check (`beta - alpha <= 1`). Still extract `hash_move` from the TT entry unconditionally (the hash move for ordering is always valid to use regardless of node type). Only the score return — lines 311, 313, 315 — needs to be gated.

---

### ORDER-004 — `order_moves()` in ordering.rs is dead production code; tests diverge from search

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/ordering.rs:31-99`, `backend/src/search/search.rs` (no import of `order_moves`), `backend/tests/ordering_tests.rs:1-277`
- **Expected:** The ordering logic tested by `ordering_tests.rs` corresponds to the ordering logic used in the actual search.
- **Actual:** `search.rs` imports only `use crate::search::picker::MovePicker` for move selection; it never imports or calls `order_moves`. The `MovePicker` in picker.rs imports only `mvv_lva_score` from ordering.rs (picker.rs:15). The `order_moves` function (ordering.rs:31-99) is exercised only by tests, not by any production code path. The two ordering implementations (flat `order_moves` and staged `MovePicker`) already diverge in how they handle the pawn advancement bonus (different rank thresholds: ordering.rs:91 uses `3..=5`, picker.rs:163-170 uses `3|4` and `5|6`).
- **Reasoning:** Confirmed by reading all imports in search.rs (no `order_moves` reference) and picker.rs (only `mvv_lva_score`).
- **Failure scenario:** A developer modifies `order_moves` to fix a priority ordering bug, all `ordering_tests` pass, and the production search is unaffected because it uses `MovePicker`. Alternatively, a developer adds a new ordering feature to `MovePicker` without adding it to `order_moves`; the tests now test stale logic.
- **Related:** None.
- **Where to look next:** `ordering.rs:31`, `picker.rs:1-17` (imports section).
- **Fix direction:** Either delete `order_moves` entirely (keep only `mvv_lva_score` which IS used) and rewrite `ordering_tests.rs` to test via `MovePicker`, or document clearly that `order_moves` is a reference/legacy function kept only for benchmarking. The pawn-rank-bonus discrepancy between the two should be reconciled regardless.

---

### ORDER-005 — `tt::probe` and `tt::save` expose dead parameters; misleads callers

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/tt.rs:101-117` (`probe`), `backend/src/search/tt.rs:73-98` (`save`)
- **Expected:** All parameters in a public API are either used or clearly marked as intentionally reserved.
- **Actual:** `probe` takes `_depth: u8`, `_alpha: i32`, `_beta: i32`, `_ply: i32` — all prefixed `_` (unused). `save` takes `_ply: i32` — also unused. The internal depth comparison was moved to callers (`search.rs:305`); mate normalization was moved to `score_to_tt`/`score_from_tt` in `search.rs:51-69`. The `tt_tests.rs` comment at line 34 explicitly notes "normalization is now in search," confirming the migration is complete but the dead parameters remain.
- **Failure scenario:** A new caller passes `alpha`/`beta` to `probe` expecting internal cutoff logic and gets none. Or: a future refactor moves normalization back into `tt.rs` and forgets the `_ply` parameter is already there but silently ignored.
- **Related:** ORDER-002 (new_search dead code — similar pattern of TT API with dead contract).
- **Where to look next:** `tt.rs:101-117`, `search.rs:298-300`.
- **Fix direction:** Remove the unused parameters from the function signatures, or add `#[allow(unused_variables)]` doc-comment explaining they are reserved for a planned internal implementation. Removing them requires updating all call sites in search.rs.

---

## Clean bill

- **TT move legality validation (P0 risk checked):** The path from `tt.probe()` to move being played is: `probe()` at search.rs:298 → `hash_move = Some(tm)` at search.rs:302 → passed to `MovePicker::new()` at search.rs:391 → in picker.rs:244-249, the hash move is validated with `is_pseudo_legal(board, hm, tables) && is_legal_move(board, hm, tables)` before being yielded. A Zobrist collision returning a wrong move is caught at the pseudo-legality check. No P0 TT move safety issue.

- **Bound types used correctly:** Exact (0) returns unconditionally; LowerBound (1) returns only when `tt_score >= beta` (correct — lower bound means real score is at least this high); UpperBound (2) returns only when `tt_score <= alpha` (correct — upper bound means real score is no higher). Both `alpha_beta` (search.rs:310-315) and `quiescence` (search.rs:163-169) are correct.

- **Mate score packing:** Score is clamped to `[-32000, 32000]` before i16 cast (tt.rs:83). MATE_SCORE = 31000, adjusted by at most `ply` (up to ~64) → max 31064. Clamp 32000 is within i16 max (32767). No truncation.

- **History updated only on quiet beta cutoffs:** search.rs:580 — `if !mv.is_capture()` guards both killer and history updates. History is not contaminated by captures.

- **History overflow not possible:** `update_history` caps bonus at 400 (`(depth * depth).min(400)`, context.rs:51). The table is divided by 8 each iterative deepening iteration (search.rs:671-675). Values reach equilibrium well within i32 range.

- **Killers indexed by ply, not depth:** `ctx.killer_moves[ply]` (context.rs:12, search.rs:391, 581) — correct.

- **Killer legality validated:** picker.rs:280-287 and 290-308 apply `is_pseudo_legal` + `is_legal_move` before yielding killers. Stale killers from sibling positions are rejected.

- **Captures excluded from killers:** picker.rs:280 `!k1.is_capture()` and picker.rs:294 `!k2.is_capture()` — correct.

- **Killer deduplication:** picker.rs:296-303 explicitly checks that killer 2 is not identical to killer 1 before yielding it. No duplicate killers.

- **Bad captures searched:** `BadCaptures` stage at picker.rs:329-342 is always reached (after Quiets), so SEE-negative captures are searched. The LMP `break` at search.rs:437 exits before bad captures on fail-low all-nodes, which is the intended behavior (bad captures are searched on other node types).

- **No countermove heuristic:** Confirmed absent in picker.rs and ordering.rs. Consistent with VANTAGE_ROADMAP.md describing it as future work.

- **MVV-LVA vs SEE split threshold:** SEE threshold is 0 (picker.rs:129 `board.static_exchange_eval(mv, 0, tables)`). Captures with SEE = 0 (exactly even trades) are classified as good captures and tried before quiets. "Bad" captures (SEE < 0) are deferred to the final stage but are searched. This is correct.

---

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `VANTAGE_ROADMAP.md` (countermove section, exact line not read) | Countermove heuristic as future ordering improvement | Absent from `picker.rs`, `ordering.rs`, `context.rs` | Ordering quality; no crash risk | No |
| `backend/src/search/tt.rs:57-59` (`new_search`) | Generation-based TT aging — entries from old searches evicted preferentially | Method defined, never called anywhere in the codebase | TT replacement quality in long games | No |
