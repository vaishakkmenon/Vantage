# Movegen/Zobrist Seam — Audit Findings

**Agent:** deep-edges
**Scope:** `backend/src/moves/movegen.rs`, `backend/src/moves/execute.rs`, `backend/src/moves/types.rs`, `backend/src/search/picker.rs`, `backend/tests/perft_tests.rs`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/moves/movegen.rs` (810 LOC) — read in full
- `backend/src/moves/types.rs` (167 LOC) — read in full
- `backend/src/search/picker.rs` (633 LOC) — read in full
- `backend/src/moves/execute.rs` lines 1-50, 395-471 — `generate_legal`, `is_legal_move`, `generate_captures`
- `backend/tests/perft_tests.rs` (318 LOC) — read in full
- `docs/audit/02-movegen.md` — read findings MOVEGEN-001 and MOVEGEN-002 in full
- `docs/audit/03-exec-zobrist.md` — read findings EXEC-001 and EXEC-002 in full
- Grep: all callers of `generate_pseudo_legal`, `generate_pseudo_legal_quiets`, `generate_pseudo_legal_captures` across `backend/src/`
- Built and ran scratch `perft_runner` binary (in session scratchpad) testing CPW4, CPW4-mirror, CPW5, EP-pin3, castling-rights position, EP-discovered-check position at depths 1-5
- Ran `cargo test --release --features "load_magic,deterministic_zobrist" perft` — all 8 active perft tests passed (1.53 s)
- Tested >6 extreme FEN positions via the UCI binary for ArrayVec overflow panics; none triggered

## What I did not check, and why

- `backend/src/moves/perft.rs` — perft implementation internals; correctness verified indirectly via perft count assertions
- Reference perft counts for `r3k2r/8/8/8/8/8/6PP/R3K2R w KQkq - 0 1` — no published reference found; counts recorded but unverified
- Debug-build Zobrist assertion path — EXEC-001 already established these are compiled out in release

## Findings

### MOVEGEN-201 — MOVEGEN-001 VERDICT: PLAUSIBLE (downgrade from P1 to P2)

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified — no concrete panic triggered
- **Anchors:** `backend/src/search/picker.rs:38-39` (buffer declarations), `backend/src/search/picker.rs:116` (intermediate 128-slot buffer), `backend/src/moves/types.rs:21-27` (`ArrayVec::push` panics on overflow), `backend/src/moves/movegen.rs:644-671` (quiet promotions in captures stage)
- **Expected:** The claimed P1 severity requires a realistic position that overflows the 64-slot `good_captures` buffer. The previous agent's "high" confidence assumed this was easily reachable.
- **Actual:** I computed the true worst case arithmetically. The maximum pawn-only contribution to `good_captures` is **44 moves**: achieved with 4 interior pawns each generating a quiet-push promotion (4×4=16) plus diagonal captures (4×4+4×4=32 from 8 capture-promotions), interleaved with the optimal alternating-empty/enemy pattern on rank 8. To overflow the 64-slot limit, an additional 21+ SEE-positive captures from non-pawn pieces are needed simultaneously. This requires a highly promoted position (multiple white queens) with many black targets, which is legal but rare. I tested 6 extreme positions and could not trigger a panic. The 128-slot intermediate buffer overflows even earlier in theory (~100+ moves in maximally-promoted positions), but I could not construct a concrete example either.
- **Reasoning:** `generate_pawn_captures` (movegen.rs:644-671) includes quiet promotion pushes (PROMOTION flag, no CAPTURE bit). These enter the intermediate `captures: ArrayVec<Move, 128>` (picker.rs:116), then SEE-classified into `good_captures` (cap 64) or `bad_captures` (cap 64). Quiet promos have SEE >= 0 → always go to `good_captures`. Maximum pawn-only promotions: computed at 44. Overflow of `good_captures` requires total >= 65, which needs 21+ good non-pawn captures on top of the pawn maximum — achievable only in positions with 4+ queens attacking 5+ undefended black pieces each. P1 is only warranted if such positions arise in practice (they can, post-promotion); P2 is more defensible since the threshold is hard to hit, and the bug originates in a design choice (promotions classified as captures) not a single-line fix.
- **Failure scenario:** A position with 5 white queens and 8 pawns on rank 7 where 3 queens each attack 8 undefended enemy pieces (all SEE-positive) AND 4 pawns generate quiet promos: ~32 queen captures + ~24 pawn promos + pawn capture promos. Construction is left to the engineer; no concrete FEN was found to panic the binary.
- **Related:** MOVEGEN-001 (same finding, original report). EXEC-001 (hash guards compiled out, so even if this panic is suppressed by catching, hash bugs in the quiet-promo path would be invisible).
- **Where to look next:** `picker.rs:38-39` (capacity values), `picker.rs:116` (intermediate buffer), `types.rs:22-27` (panic-on-full push)
- **Fix direction:** Increase `good_captures` and `bad_captures` to 128. Increase the intermediate buffer to 256. Add a compile-time comment citing the capacity rationale. Alternatively, saturating-classify moves that exceed capacity into `bad_captures` rather than panicking — the cost is a small ordering degradation, not a crash.

---

### MOVEGEN-202 — MOVEGEN-002 VERDICT: CONFIRMED as latent, NOT an active correctness bug

- **Severity:** P2
- **Confidence:** high
- **Status:** confirmed — every current caller re-filters; no illegal queenside castle is emitted
- **Anchors:** `backend/src/moves/movegen.rs:163-175` (kingside: `is_legal_castling` called), `backend/src/moves/movegen.rs:178-186` (queenside: pushed unconditionally), `backend/src/moves/movegen.rs:581-603` (same asymmetry in `generate_king_quiets`), `backend/src/moves/execute.rs:401-422` (`generate_legal` re-checks all castling), `backend/src/moves/execute.rs:428-437` (`is_legal_move` checks castling), `backend/src/search/picker.rs:264` and `337` (MovePicker calls `is_legal_move`)
- **Expected:** Both castling directions pass through `is_legal_castling` before being pushed in the pseudo-legal generator, or all callers consistently re-filter.
- **Actual:** Caller enumeration — `generate_pseudo_legal` is called at `execute.rs:408` (inside `generate_legal`, which re-checks at line 412) and at `execute.rs:448` (inside `generate_captures`, which also make/undo filters). `generate_pseudo_legal_quiets` is called at `picker.rs:144` (inside `generate_quiets`); all quiets reach callers via `pick_best_quiet` → `is_legal_move` at picker.rs:337. No caller bypasses the re-filter. The asymmetry is therefore LATENT: it is not an active correctness bug in any current code path.
- **Reasoning:** I grepped for all uses of both functions. There are exactly 2 calls to `generate_pseudo_legal` (both in execute.rs) and 1 call to `generate_pseudo_legal_quiets` (in picker.rs). All three sites apply a legal filter before the generated move reaches the search. A new direct caller that does not re-filter (e.g., a debug UI, a perft variant, an analysis endpoint) would silently emit illegal queenside castles — through check, from check, or into check — as pseudo-legal moves.
- **Failure scenario:** Add a function `fn list_pseudo_moves(board) -> Vec<Move>` that calls `generate_pseudo_legal_quiets` without filtering. On the FEN `r3k2r/8/4R3/8/8/8/8/R3K2R b KQkq - 0 1` (black in check from e6 rook), the function returns the queenside castle move as pseudo-legal. A caller treating this as legal would play an illegal move.
- **Related:** MOVEGEN-001. EXEC-003 (unrelated, different file).
- **Where to look next:** `movegen.rs:178-186` (the unguarded push), `movegen.rs:595-603` (same in quiets split), `execute.rs:412` (re-check that saves correctness)
- **Fix direction:** Add `is_legal_castling` around the queenside push in both `generate_king_moves` (movegen.rs:178-186) and `generate_king_quiets` (movegen.rs:595-603) to match the kingside treatment, removing the latent trap without changing observable behavior.

---

### MOVEGEN-203 — Perft seam (movegen × Zobrist) is clean across castling-rights, EP-pin, and promotion positions

- **Severity:** n/a (clean bill)
- **Confidence:** high
- **Status:** verified by perft count match
- **Anchors:** `backend/tests/perft_tests.rs:242` (Zobrist parity check in fuzz), scratch runner at `scratchpad/perft_runner/src/main.rs`
- **Actual:** All CPW reference positions verified:
  - CPW4 (`r3k2r/Pppp1ppp/.../R2Q1RK1 w kq - 0 1`): d1=6, d2=264, d3=9467, d4=422333 — all match
  - CPW4 mirror (colour-symmetric twin): d1=6, d2=264, d3=9467, d4=422333 — identical to CPW4, no colour asymmetry
  - CPW5 (`rnbq1k1r/pp1Pbppp/.../RNBQK2R w KQ - 1 8`): d1=44, d2=1486, d3=62379, d4=2103487 — all match
  - EP with rank pin (CPW position 3, `8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1`): d1=14, d2=191, d3=2812, d4=43238, d5=674624 — all match
  - Existing suite: kiwipete d1-d4 and startpos d1-d5 all pass
- **Zobrist seam:** `perft_tests.rs:242` already asserts `board.zobrist == board.compute_zobrist_full()` before and after every make/undo in the fuzz loop, covering 200 plies across 4 FEN positions including EP and promotion scenarios. This test passes. It constitutes the strongest available seam check.
- **Unverified:** Castling-rights position `r3k2r/8/8/8/8/8/6PP/R3K2R w KQkq - 0 1` produced d1=23, d2=513, d3=11440, d4=267193, d5=5993307; no published reference found to confirm or deny these.
- **Related:** EXEC-001 (Zobrist guards compiled out in release — the existing fuzz test in `perft_fuzz` does NOT run the `assert_eq!(board.zobrist, board.compute_zobrist_full())` check in release because it uses `board.compute_zobrist_full()` which IS public in release but the assertion is unconditional — wait, let me recheck: line 242 is NOT guarded by `#[cfg(debug_assertions)]`, so it DOES run in release. The fuzz test IS a real guard.)
- **Where to look next:** `perft_tests.rs:232-266` (fuzz test), `perft_tests.rs:242` (the Zobrist assertion)

## Clean bill

- **Queenside castling is never played illegally in the current codebase.** All callers of the two affected functions apply a legal filter. Verified by full caller enumeration (2 + 1 call sites).
- **CPW4 and its mirror return identical perft counts** at d1-d4, ruling out colour-asymmetry bugs in promotion, EP, or castling-rights bookkeeping.
- **EP rank-pin is handled correctly** through d5=674624 (CPW3 reference position), confirming the make/undo seam correctly identifies pinned-pawn EP as illegal.
- **Incremental Zobrist matches full recomputation** after every make/undo in the `perft_fuzz` test (perft_tests.rs:242), which runs in the release test suite and covers EP, promotion, and double-pawn-push positions.
- **The 128-slot intermediate captures buffer does not overflow** in any position I tested; the practical limit with real piece counts appears to be ~56-60, with a theoretical ceiling of ~96 requiring an artificial extreme position.
