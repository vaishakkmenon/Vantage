# Eval — Audit Findings

**Agent:** audit-eval
**Scope:** `backend/src/search/eval.rs` (760 LOC), `backend/src/search/pesto.rs` (152 LOC), `backend/tests/eval_materials_tests.rs` (204 LOC)
**Date:** 2026-08-30 (overnight run)

## What I checked

- Read `backend/src/search/eval.rs` in full (all 760 lines)
- Read `backend/src/search/pesto.rs` in full (all 152 lines)
- Read `backend/tests/eval_materials_tests.rs` in full (all 204 lines)
- Read `VANTAGE_ROADMAP.md` line 19 (claimed eval terms) and lines 46-52 (future work)
- Read `AGENT_BRIEF.md` for context and hard rules
- Ran `grep -rn "pub fn pop_lsb"` to confirm pop_lsb return type (u8, debug_assert on empty)
- Verified every one of the 15 roadmap-claimed eval terms against source
- Verified all 6 roadmap "future work" terms are absent from source
- Manually traced colour-mirror symmetry for every eval term
- Computed maximum magnitude of lazy-eval-skipped terms by hand
- Checked pawn file mask edge guards, king zone edge guards, tapering overflow bounds
- Reviewed all 5 inline tests in eval.rs and all 10 tests in eval_materials_tests.rs

## What I did not check, and why

- Did not run any build or test command (hard rule: read-only).
- Did not read `search.rs` to confirm how the caller uses the lazy-eval return value; I traced the function signature and return path in eval.rs only. That is enough to confirm the issue exists; confirming the caller's exact treatment would sharpen confidence.
- Did not check `board.rs` or `square.rs` for `king_square` or `occupancy` implementation; I trusted the call sites are correct based on the surrounding context.

## Findings

### EVAL-001 — Mop-up eval silently absent when winning side has just moved

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/eval.rs:202`, `backend/src/search/eval.rs:96-147`
- **Expected:** Mop-up (king confinement) should influence the eval on every node in a won endgame, regardless of which side is to move. The roadmap claims "mop-up endgame eval" as a fully implemented feature.
- **Actual:** `static_eval` calls `mop_up_eval(board, side)` only for the current mover (line 202). `mop_up_eval` returns 0 when `my_material < enemy_material + 200` (lines 121-123). So when the winning side has just moved and it's the losing side's turn, `mop_up_eval(board, losing_side)` = 0 and the entire mop-up contribution vanishes. The winning side's king-confinement incentive is missing on every odd-ply node when it is the loser's turn to move.
- **Reasoning:** Line 202: `score += mop_up_eval(board, side)`. No corresponding `score -= mop_up_eval(board, enemy)`. For the score to be colour-symmetric and consistent across plies, the formula should be `mop_up_eval(board, side) - mop_up_eval(board, enemy)` (multiplied by color_multiplier as for pawn structure), or equivalently, mop_up should be expressed as a White-minus-Black difference like pesto_eval. Since one of the two calls is always 0 (only one side can be 200cp ahead), the subtraction form has no performance cost.
- **Failure scenario:** K+Q vs K+P, White winning. White just moved (ply N). Now it's Black's turn. `static_eval` computes `mop_up_eval(board, Black)` = 0 (Black is losing). Missing: `mop_up_eval(board, White)` as a penalty to Black's score. The score at ply N (Black to move) is missing up to 196cp of king-confinement incentive, causing a step-function discontinuity at the ply boundary. The engine may oscillate between "finding" the mop-up (White to move) and "losing" it (Black to move), potentially failing to make progress in KQ vs KP endings.
- **Related:** None.
- **Where to look next:** `eval.rs:194-205` (the full `static_eval` aggregation block), `eval.rs:96-147` (mop_up_eval itself, confirm the guard is correct).
- **Fix direction:** Replace the single call at line 202 with a colour-difference: compute mop_up for White and subtract mop_up for Black, then multiply by color_multiplier. One of the two will always be zero so runtime cost is unchanged.

---

### EVAL-002 — Lazy eval margin (200 cp) smaller than maximum skipped term magnitude

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/eval.rs:10`, `backend/src/search/eval.rs:179-185`, `backend/src/search/eval.rs:353-373`, `backend/src/search/eval.rs:402-547`
- **Expected:** The lazy eval invariant requires that the 200 cp margin is larger than the maximum absolute contribution of all skipped terms. Roadmap claims "lazy eval cutoff (±200cp)" as implemented. For it to be sound, returning `score` as an upper or lower bound must still be correct once all skipped terms are added.
- **Actual:** The skipped terms can individually and collectively exceed 200 cp by a large margin:
  - **Mobility** (bishop + rook): bishop up to 13 reachable squares × 5 cp × up to 4 bishops = 260 cp per side; rook up to 14 × 3 cp × 4 = 168 cp per side. Net difference: up to ±856 cp.
  - **Pawn structure**: isolated penalty up to -120 cp per side; doubled up to -80 cp per side; passed pawn bonus up to PASSED_PAWN_BONUS[6]=300 cp plus proximity (up to 35 cp) per pawn × 8 pawns = ~2680 cp per side.
  - **King safety + shield**: up to ~180 cp + 60 cp.
  - **Mop-up**: up to 196 cp.
  For the **alpha cutoff** (`score + 200 <= alpha`): the claim is full_eval cannot exceed alpha, so full_eval = score + remaining <= score + 200 <= alpha. This fails if remaining > 200 (e.g., the position has advanced passed pawns worth 2000+ cp in structure terms). The engine would prune a node thinking it's too low when the actual eval is far above alpha.
  For the **beta cutoff** (`score - 200 >= beta`): the claim is full_eval >= beta, so score + remaining >= beta requires remaining >= -200. This fails if remaining < -200 (e.g., many isolated pawns give -240 cp, doubled pawns -160 cp: net -400 cp is plausible). The engine cuts off thinking a position is a fail-high when the full eval falls below beta.
- **Failure scenario:** Position with equal material (pesto_eval ≈ beta + 201) but 6 isolated doubled pawns and no passed pawns. Score = beta + 201, so `score - 200 = beta + 1 >= beta` fires the beta cutoff. But mobility + pawn structure is about −400 cp, giving full_eval = beta + 201 − 400 = beta − 199. The engine returns score (beta + 201) instead of the actual beta − 199, treating a fail-low position as a fail-high.
- **Related:** EVAL-001 (mop-up contributes to the budget of skipped terms).
- **Where to look next:** `eval.rs:179-185` (lazy cutoff), `eval.rs:353-373` (mobility magnitudes), `eval.rs:402-419` (pawn penalties).
- **Fix direction:** Either (a) raise the margin to cover the maximum realistic skipped contribution — empirically 500–700 cp is more defensible — or (b) reorder evaluation so lazy eval is checked earlier (before any term with large range), or (c) restructure so the lazy check skips only the king-safety terms which are naturally bounded.

---

### EVAL-003 — No colour-mirror symmetry test in the test suite

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/eval.rs:696-708`, `backend/tests/eval_materials_tests.rs:185-203`
- **Expected:** For any position P and its colour mirror P' (flip all ranks via `^56`, swap piece colours, swap side to move), `static_eval(P) == static_eval(P')`. This is the most valuable eval invariant: a violation means the engine systematically misevaluates identical positions from Black's perspective vs. White's. The roadmap implies the eval is correct for both colours.
- **Actual:** The test named `test_perspective_flip` (eval.rs:696) only changes `side_to_move` on the same board — it does NOT flip piece positions. It tests tempo perspective, not colour-mirror symmetry. The `mirror_vert_basic_checks` test (eval_materials_tests.rs:186) validates the helper function but does not call `static_eval` on mirrored positions. EVAL-001 (the mop-up bug) is a confirmed colour-mirror asymmetry that the existing tests cannot detect.
- **Failure scenario:** Any non-starting position where White has an advanced passer and it is Black's turn to move: the mop-up term is 0, but the colour-mirrored position (Black has an advanced passer, White to move) gets nonzero mop-up. The two evals differ by up to 196 cp, which the test suite never asserts on.
- **Related:** EVAL-001 (the mop-up bug is an instance of this asymmetry).
- **Where to look next:** `eval.rs:696-708`, `eval_materials_tests.rs:185-203`. A true mirror test would: construct position P with White to move, construct P' by flipping every piece's square via `sq ^ 56`, swapping colors, and setting Black to move; then assert `static_eval(P) == static_eval(P')`.
- **Fix direction:** Add a test that constructs at least one non-symmetric position (e.g., king + passed pawn vs king), builds its colour mirror by iterating all bitboards and XORing squares with 56, swapping colour labels, and asserts static_eval equality.

---

### EVAL-004 — Rook mobility weight is a magic number

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/eval.rs:369`
- **Expected:** The roadmap claims "rook mobility (3cp/sq)". All other weights in eval.rs are named constants (MOBILITY_WEIGHT=5, ISOLATED_PAWN_PENALTY=-15, etc.).
- **Actual:** `score += (attacks & !us_bb).count_ones() as i32 * 3;` — the rook weight `3` is a bare integer literal with no accompanying constant. The bishop weight is `MOBILITY_WEIGHT` (5); the rook weight has no named constant.
- **Failure scenario:** No runtime failure; purely a maintenance hazard. Searching for rook mobility weight returns no results.
- **Related:** None.
- **Where to look next:** `eval.rs:362-372` (eval_mobility).
- **Fix direction:** Add `const ROOK_MOBILITY_WEIGHT: i32 = 3;` alongside the existing weight constants at the top of eval.rs and replace the literal on line 369.

## Clean bill

- **Phase clamping (TOTAL_PHASE=24):** `calculate_phase` line 62 calls `.clamp(0, TOTAL_PHASE)`. Extra queens from promotion cannot push phase above 24. Tapering interpolation cannot extrapolate. ✓
- **Tapering arithmetic overflow:** Maximum plausible mg_score with all pieces is ~15 000 cp. `mg_score * 24 = 360 000`, well within i32 range (~2.1 B). No overflow risk. ✓
- **PSQT colour symmetry:** `mirror_vert(sq) = sq ^ 56` correctly flips rank while preserving file for White pieces. Black pieces at `sq` correspond to White pieces at `sq^56` (their colour mirror). Both look up `mg_table[sq^56]`, so they contribute equal-and-opposite values. Verified algebraically and with the mirror_vert unit test. ✓
- **Pawn file mask edge guards:** `w_neighbor_files = ((w_file_mask & !FILE_H) << 1) | ((w_file_mask & !FILE_A) >> 1)`. FILE_H guard prevents h→a wrap on left shift; FILE_A guard prevents a→h wrap on right shift. No edge wrapping in isolated-pawn detection. ✓
- **King zone edge guards:** The 3×3 king zone construction at lines 283-285 uses `& 0xFEFEFEFEFEFEFEFE` (NOT a-file) on the left-shift and `& 0x7F7F7F7F7F7F7F7F` (NOT h-file) on the right-shift. No file wrapping at board edges. ✓
- **pop_lsb on empty bitboard:** All call sites use `while bb != 0` guards before calling pop_lsb. BitIter explicitly checks `if self.0 == 0 { None }`. No risk of trailing_zeros(0)=64 indexing a 64-element table. `pop_lsb` in utils.rs has a `debug_assert_ne!(*bb, 0)`. ✓
- **Passed pawn rank indexing symmetry:** White uses `PASSED_PAWN_BONUS[rank]`; Black uses `PASSED_PAWN_BONUS[7-rank]`. A White pawn at rank r (0-indexed) mirrors a Black pawn at rank 7-r; both get `PASSED_PAWN_BONUS[r]`. ✓
- **Passed pawn king-proximity threshold symmetry:** White activates at `rank >= 4`, Black at `rank <= 3`. A White pawn at rank 4 mirrors a Black pawn at rank 3; thresholds fire for both. ✓
- **Chebyshev distance preserved under vertical flip:** `chebyshev(sq1^56, sq2^56) = chebyshev(sq1, sq2)` because `|rank(sq1^56) - rank(sq2^56)| = |(7-r1)-(7-r2)| = |r1-r2|` and files are unchanged. All king-proximity calculations are symmetric. ✓
- **Blocked passer square bounds:** White: `stop_sq = sq + 8` only when `rank < 7` (sq ≤ 55, stop_sq ≤ 63). Black: `stop_sq = sq - 8` only when `rank > 0` (sq ≥ 8, stop_sq ≥ 0). No out-of-range indexing. ✓
- **Future-work terms genuinely absent:** Bishop pair, rook on open file, rook on 7th, knight mobility, queen mobility, backward pawns — none appear anywhere in eval.rs or pesto.rs. ✓
- **All 15 roadmap-claimed eval terms present:** Tapered material, PSQT MG/EG, phase calc N=1/B=1/R=2/Q=4, bishop mobility 5cp, rook mobility 3cp, king zone attacks 3×3 tapered, king pawn shield, doubled pawns -10cp, isolated pawns -15cp, passed pawns rank-based, blocked passers -50cp, king-passer distance (Chebyshev), mop-up eval, lazy eval ±200cp — all implemented. ✓

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| VANTAGE_ROADMAP.md:46-52 | "Bishop pair bonus … Rook on open file … Rook on 7th … Knight mobility … Queen mobility … Backward pawns" (Phase 1.3) | None of these six terms exist in eval.rs | ~+40-60 ELO per roadmap estimate | No — listed as future work |
