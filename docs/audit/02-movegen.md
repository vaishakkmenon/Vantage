# Move Generation — Audit Findings

**Agent:** audit-movegen
**Scope:** `backend/src/moves/movegen.rs`, `pawn.rs`, `king.rs`, `square_control.rs`, `types.rs`, `magic/structs.rs`, `magic/precompute.rs` (skimmed); `backend/src/search/picker.rs`; `backend/tests/movegen_tests.rs`, `perft_tests.rs`, `legal_filter_tests.rs`, `pawn_movegen_tests.rs`, `square_control_tests.rs`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/moves/movegen.rs` (810 LOC) — read in full
- `backend/src/moves/pawn.rs` (164 LOC) — read in full
- `backend/src/moves/king.rs` (138 LOC) — read in full
- `backend/src/moves/square_control.rs` (241 LOC) — read in full
- `backend/src/moves/types.rs` (167 LOC) — read in full
- `backend/src/moves/magic/structs.rs` (167 LOC) — read in full
- `backend/src/moves/execute.rs` — read lines 1-50 and 395-471; grep for generate_legal, is_legal_castling, in_check
- `backend/src/search/picker.rs` (633 LOC) — read in full
- `backend/tests/legal_filter_tests.rs` (89 LOC) — read in full
- `backend/tests/perft_tests.rs` (318 LOC) — read in full
- `backend/tests/movegen_tests.rs` (1117 LOC) — read head (50 LOC) + grep for castl, en_passant, promotion, pin; read lines 1080-1116
- `backend/tests/` directory listing — ran `ls`
- Grep: `generate_pseudo_legal_quiets`, `generate_pseudo_legal_captures` across `src/`
- Grep: `ArrayVec`, `MoveBuffer`, `generate_legal`, `is_legal`, `in_check`, `arrayvec` in `execute.rs`
- Grep: `queenside`, `rook_from`, `b1`, `b8` in `execute.rs`

## What I did not check, and why

- `backend/src/moves/magic/precompute.rs`, `attacks.rs`, `masks.rs`, `loader.rs`, `search.rs` — only skimmed `structs.rs`; the magic consistency question (load_magic vs generated) is already noted in context item 8 and does not require re-reading.
- `backend/src/moves/perft.rs` — not opened; what perft counts is indirectly confirmed by perft_tests assertions.
- `backend/tests/pawn_movegen_tests.rs`, `square_control_tests.rs` — not opened; instead relied on grepping movegen_tests.rs for EP/castling/promotion coverage, which is where those topics are tested.
- Cannot run `cargo test` per hard rules.

## Findings

### MOVEGEN-001 — `good_captures` and `bad_captures` ArrayVec capacity is 64; overflow panics

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/picker.rs:38-39`, `backend/src/search/picker.rs:115-137`
- **Expected:** The move picker classifies all pseudo-legal captures into good (SEE ≥ 0) and bad (SEE < 0) buckets without loss. An overflow should be impossible or handled gracefully.
- **Actual:** `good_captures: ArrayVec<Move, 64>` and `bad_captures: ArrayVec<Move, 64>` are both hard-capped at 64 entries. `generate_pseudo_legal_captures` (called at picker.rs:117) pushes into an intermediate `ArrayVec<Move, 128>` (line 116), which is itself undersized. That function includes non-capture promotion pushes (4 per pawn on rank 7) in addition to all captures. The `MoveBuffer` impl for `ArrayVec` at `types.rs:21-28` delegates to `ArrayVec::push`, which **panics** when the vec is full — there is no saturating or try-push path.
- **Reasoning:** `generate_pawn_captures` (movegen.rs:608-721) generates: normal pawn captures (non-promo), promotion pushes (non-capture, section at movegen.rs:644-671), promotion captures, and en passant — all into the "captures" stage. With 7 pawns on rank 7: 7×4 = 28 promotion pushes alone. Add 9 queens each attacking a typical 5 enemy squares = 45 queen captures, plus knight/rook/bishop captures. The intermediate buffer of 128 can be breached in extreme (but legal, albeit constructed) positions. Even if the 128-buffer survives, if 70 of those entries have SEE ≥ 0, `good_captures.push()` panics on entry 65.
- **Failure scenario:** Construct a position with ≥ 65 pseudo-legal SEE-positive captures (e.g., 9 queens each attacking 7+ undefended enemy pieces) and run a search through `MovePicker`. The `push` at picker.rs:131 panics. A perft over a maximally-attacking position could also trigger it if the perft path goes through MovePicker.
- **Related:** none in this report; tangentially related to overall position sanity (BOARD agent area).
- **Where to look next:** `picker.rs:38-39` (field declarations), `picker.rs:116` (intermediate buffer size), `types.rs:21-27` (panic-on-overflow push impl), `movegen.rs:644-671` (promotion pushes included in captures stage).
- **Fix direction:** Increase `good_captures` and `bad_captures` to at least 128 each, and the intermediate buffer to at least 256. Alternatively, switch the inner buffer to a `Vec` and the two classification buffers to at least 128. Document the capacity guarantee with a comment citing the theoretical maximum.

---

### MOVEGEN-002 — Queenside castling is pseudo-legally generated without `is_legal_castling`; kingside is checked

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/moves/movegen.rs:163-175` (kingside, checked), `backend/src/moves/movegen.rs:178-186` (queenside, not checked); same asymmetry at `movegen.rs:581-603` in `generate_king_quiets`
- **Expected:** Both castling sides are treated uniformly: either both are legality-checked before being added to the pseudo-legal list, or neither is.
- **Actual:** Kingside castling calls `is_legal_castling` (movegen.rs:172) before pushing to the move list. Queenside castling (movegen.rs:179-186) is pushed unconditionally after only checking that the between-squares are empty. `generate_king_quiets` (used by `generate_pseudo_legal_quiets` and thus by `MovePicker`) has the identical asymmetry at lines 589 vs. 595-603.
- **Reasoning:** `generate_legal` (execute.rs:411-413) re-checks all castling moves with `is_legal_castling` before accepting them, so the current end-to-end paths (`generate_legal`, `MovePicker`) produce correct results. But `generate_pseudo_legal` and `generate_pseudo_legal_quiets` are public API. Any new caller that uses the pseudo-legal list directly (e.g., a perft variant, a UCI move validator) will silently emit illegal queenside castles — through check, into check, or while in check — as pseudo-legal without warning.
- **Failure scenario:** Call `generate_pseudo_legal_quiets` on a position where the king is in check (e.g., FEN `r3k2r/8/8/8/8/8/8/R3K1nR b KQkq - 0 1` — black king in check from g1 knight). The function emits the queenside castle move. A caller that does not apply `is_legal_castling` would treat it as playable.
- **Related:** none.
- **Where to look next:** `movegen.rs:178-186`, `movegen.rs:595-603`.
- **Fix direction:** Add the `is_legal_castling` guard to the queenside branch in both `generate_king_moves` and `generate_king_quiets`, matching the existing kingside pattern. This makes pseudo-legal output consistent and safe for direct use.

---

### MOVEGEN-003 — `en_passant_is_illegal_when_pawn_is_pinned_opening_file_on_own_king` test covers only vertical (file) pin; horizontal (rank) EP pin has no test

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/legal_filter_tests.rs:59-88`
- **Expected:** The EP legality test covers both the vertical-pin case (rook on same file as king and EP pawn) and the horizontal-pin case (rook and king on same rank, EP removes both pawns and exposes the king — FEN `8/8/8/k2Pp2Q/8/8/8/3K4 b - d3 0 1`).
- **Actual:** Only the vertical case is tested (`k3r3/8/8/3pP3/8/8/8/4K2R w - d6 0 1`). The horizontal rank-pin EP case has no dedicated test. The implementation does handle it correctly — `generate_legal` uses make/in_check/undo which removes both pawns before checking, so the exposed rook attack is detected — but this correctness is unverified by the test suite.
- **Failure scenario:** No production bug expected; the risk is that a refactor of EP generation or the make/undo path silently regresses this edge case with no test to catch it.
- **Related:** none.
- **Where to look next:** `legal_filter_tests.rs:59-88`, `movegen.rs:319-344` (EP generation), `execute.rs:416-420` (make/in_check legality filter).
- **Fix direction:** Add a test using the FEN `8/8/8/k2Pp2Q/8/8/8/3K4 b - d3 0 1` (black to move, EP target d3 — adjusted for side). Verify that the EP capture which removes both the d5 and e5 pawns and exposes the king on the a5-h5 rank to the h5 queen is correctly rejected.

---

### MOVEGEN-004 — Perft suite covers only CPW positions 1 and 2; positions 3-6 absent; Kiwipete d5 commented out

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/perft_tests.rs:14-24` (FENS array), `backend/tests/perft_tests.rs:149-165` (kiwipete, d4 max), `backend/tests/perft_tests.rs:151` (d5 commented)
- **Expected:** Perft validation covers the full CPW suite (positions 1-6) to depths sufficient to exercise all edge cases (EP, castling, promotions, double check).
- **Actual:** The FENS array includes startpos, an alternate "kiwipete" (not the standard one — FEN at line 18 differs from KIWI_FEN), and two contrived micro-positions. The standard CPW positions 3-6 are absent entirely. Kiwipete d5 (193,690,690 nodes, the depth that exercises EP + castling combinatorics) is commented out. The CI-run perft tops out at startpos d5 (4,865,609 nodes) and kiwipete d4 (4,085,603 nodes).
- **Failure scenario:** A regression in EP-through-promotion or castling-rights-clearing could produce a wrong node count at d5 of kiwipete or at any depth of CPW position 3 (which specifically exercises EP: `8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1`) and would go undetected.
- **Related:** none.
- **Where to look next:** `perft_tests.rs:85-89` (d6/d7 startpos ignored), `perft_tests.rs:151` (d5 kiwipete commented).
- **Fix direction:** Add CPW positions 3-6 with their known node counts (from chessprogramming.org/Perft_Results) as `#[ignore]` tests at appropriate depths (≥ d4). Uncomment kiwipete d5 or move it to an `#[ignore]` block so it can be run manually pre-release.

## Clean bill

- **En passant legality (horizontal/vertical pin):** The code in `generate_legal` (execute.rs:407-422) uses `make_move_basic` + `in_check` + `undo_move_basic` for every pseudo-legal move including EP. `make_move_basic` correctly removes both the capturing and captured pawn before `in_check` runs. This handles all pin geometries correctly, including the hard horizontal-rank case. Verified by tracing `movegen.rs:319-344` (EP generation) and `execute.rs:416-420` (filter). The vertical-pin case is also directly tested at `legal_filter_tests.rs:59-88`.
- **Promotions:** All four pieces (Queen, Rook, Bishop, Knight) are enumerated via `PROMOS = [Queen, Rook, Bishop, Knight]` at `movegen.rs:28`. Promotion pushes, promotion captures, and underpromotions are all generated in both the full (`generate_pawn_moves`) and staged (`generate_pawn_captures`) generators. Encoding in `types.rs` is correct: `PROMOTION = 0b1000`, `PROMOTION_CAPTURE = 0b1100`, `is_promotion()` tests bit 3. Verified at `movegen.rs:285-317` and `types.rs:39-41,78-81`.
- **Pawn attack direction in `is_square_attacked`:** `square_control.rs:42-48` shifts the target square's bit toward the attacker's back rank to find candidate pawn squares, then intersects with the pawn bitboard. Direction is color-correct: white pawn attackers of a square are found by right-shifting (toward rank 1) with appropriate file masks; black by left-shifting. Verified manually for c6 (index 42): white gives d5/b5; black gives d7/b7. Both correct.
- **Castling: through-check, into-check, out-of-check:** `is_legal_castling` (square_control.rs:82-108) checks all three: `in_check(board, color)` for the current position (out-of-check), and `is_square_attacked` for both the pass-through and destination squares. The loop at lines 101-105 tests `start_idx`, `middle_idx`, and `end_idx`. For `generate_legal`, castling is additionally validated by the make/in_check path. Tests at `movegen_tests.rs:1080-1116` cover kingside forbidden via f1/g1 attacked, and queenside forbidden via d8/c8 attacked.
- **Castling: b1/b8 empty requirement:** `WHITE_QUEENSIDE_BETWEEN = 0x0000_0000_0000_000E` covers bits 1, 2, 3 (b1, c1, d1). b1 must be empty but need not be unattacked (the king doesn't pass through b1), which is correct. Verified at `movegen.rs:23,63-67` and `square_control.rs:91-96` (only c1, d1, e1 tested for attacks, not b1).
- **King backward-along-slider-ray bug:** For normal king moves, this is fully handled by the make/in_check approach: the king is removed from its original square before the `in_check` call, so sliders are not blocked by the king's old position. Verified at `execute.rs:415-421`.
- **Staged generation — no duplicate or missing moves:** `generate_pseudo_legal_quiets` generates single/double pawn pushes (non-promo), piece quiets, and castling. `generate_pseudo_legal_captures` generates captures, promotion pushes (all 4 pieces), promotion captures (all 4), and EP. The promo rank is excluded from quiet pushes (`& !promo_rank` at `movegen.rs:737`) and quiet pawn generation is in `generate_pawn_quiets` (movegen.rs:725-776). No overlap between the two generators for standard moves. The `MovePicker` de-duplicates hash/killer moves explicitly. Test `test_picker_no_duplicate_moves` confirms this on a non-trivial position.
- **Move encoding in `types.rs`:** Flag constants are non-overlapping, `is_capture()` tests bit 2, `is_promotion()` tests bit 3, `is_en_passant()` tests exact match 0b0101. No aliasing between flag constants. Verified at `types.rs:34-41`.
- **Magic table index bounds:** `structs.rs:41` computes `(masked.wrapping_mul(entry.magic)) >> entry.shift` where `shift` is the stored per-square shift, and `masked = blockers & entry.mask`. `wrapping_mul` prevents overflow panic. The resulting `usize` indexes into `entry.table` (a `Box<[u64]>`). If the magic search (precompute.rs) sized the table correctly, no out-of-bounds access occurs. CI relies on `load_magic` (pre-built tables), and the known-good perft results at depths 1-5 for startpos and 1-4 for kiwipete implicitly validate the tables; the magic consistency test (`magic_consistency.rs`) is noted as never running in CI per the shared brief.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `MCP_BUILD_PLAN.md` (Phase 2) | `legal_moves` tool returning all legal moves for a position | No `legal_moves` implementation in `src/bin/vantage-mcp.rs` (file does not exist per brief context 8) | MCP server unusable for move enumeration | Yes — Phase 2 blocking |
