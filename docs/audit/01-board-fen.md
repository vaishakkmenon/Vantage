# Board / FEN — Audit Findings

**Agent:** audit-board
**Scope:** `backend/src/board/mod.rs`, `fen.rs`, `fen_tables.rs`, `castle_bits.rs`, `tests.rs`; `src/square.rs`, `src/status.rs`, `src/bitboard.rs`, `src/utils.rs`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/board/mod.rs` (587 LOC) — read in full
- `backend/src/board/fen.rs` (279 LOC) — read in full
- `backend/src/board/fen_tables.rs` (75 LOC) — read in full
- `backend/src/board/castle_bits.rs` (27 LOC) — read in full
- `backend/src/board/tests.rs` (493 LOC, 43 tests) — read in full
- `backend/src/square.rs` (132 LOC) — read in full
- `backend/src/status.rs` (135 LOC) — read in full
- `backend/src/bitboard.rs` (11 LOC) — read in full
- `backend/src/utils.rs` (40 LOC) — read in full
- `backend/src/moves/execute.rs` (first 370 LOC) — read to verify castling-rights-on-capture and history management
- Grepped for `panic!\|\.unwrap()\|\.expect(` in all assigned files
- Grepped for `castling_rights\|CASTLE_` in execute.rs to verify rook-capture logic
- Grepped for `to_digit\|skip\|checked_add` in fen.rs for skip-digit validation

## What I did not check, and why

- `src/hash/zobrist.rs` — out of scope; called by `set_bb` and `compute_zobrist_full` but the invariant I need (Zobrist consistency) is satisfied by the `assert_hash` debug guard already present.
- `src/moves/movegen.rs` and `square_control.rs` — called by `position_status`; confirming they are correct is out of scope for the BOARD agent.
- `execute.rs` lines 370+ (null-move undo, `generate_legal`) — read enough to confirm history management and rook-capture correctness; remainder is MOVEGEN/EXEC scope.
- I did not run any cargo commands; all analysis is static reading.

## Findings

### BOARD-001 — `king_square()` panics on any FEN missing a king — reachable from untrusted UCI/UI input

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified (no build)
- **Anchors:** `backend/src/board/mod.rs:356-366`, `backend/src/board/fen.rs:120-131`
- **Expected:** FEN parsing validates structural legality (exactly one king per side) and returns an `Err` for positions missing kings, so internal callers of `king_square()` are always safe.
- **Actual:** `parse_placement` / `set_fen` accepts any position without king-count validation. `king_square()` panics with `panic!("King missing for {:?}! ...")` when the king bitboard is zero. The panic message calls `self.to_fen()` inside the panic format, which is fine, but the panic itself crashes the process.
- **Reasoning:** `fen.rs:120-131` resets the board and sets pieces from the FEN string. There is no check that `bb(White, King).count_ones() == 1` or `bb(Black, King).count_ones() == 1`. `mod.rs:356-366` directly panics if `king_bb == 0`. Every search path eventually calls `king_square` (at minimum for in-check detection). A UCI `position fen <bad-fen>` command with no king for either side reaches this panic immediately on the first `generate_legal` or `in_check` call.
- **Failure scenario:** `position fen 8/8/8/8/8/8/8/8 w - - 0 1` → `set_fen` succeeds → search calls `king_square(White)` → panic.
- **Related:** None within this scope. EXEC/SEARCH agents: any caller of `king_square` shares exposure.
- **Where to look next:** `mod.rs:356-366` (panic site); `fen.rs:252-267` (`set_fen`, where post-parse validation would go); `status.rs:97` (`position_status` calls `generate_legal` which calls `in_check`).
- **Fix direction:** After `parse_placement` succeeds, check that `bb(White,King).count_ones() == 1` and `bb(Black,King).count_ones() == 1`; return `Err` otherwise. Optionally also check pawn counts ≤ 8, no pawns on ranks 1/8.

---

### BOARD-002 — `position_status` returns draw instead of checkmate when hmc ≥ 100 at mate

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified (no build)
- **Anchors:** `backend/src/status.rs:97-134`
- **Expected:** Checkmate (a forced win) takes priority over any claimable draw condition. Per FIDE Laws of Chess Art. 5.1.1: "The game is won by the player who has checkmated his opponent's king." Claimable draws (fifty-move, threefold) only apply when claimed — they do not supersede a terminal result.
- **Actual:** The priority order in `position_status` checks `DrawThreefold` (line 115) and `DrawFiftyMove` (line 118-119) **before** it generates legal moves and tests for checkmate (lines 123-134). A position that is simultaneously checkmate AND has `halfmove_clock >= 100` returns `DrawFiftyMove`; the function never reaches the mate test.
- **Reasoning:** Reading `status.rs:97-134` in order: automatic draws (fivefold, 75-move, dead position) are checked first — correct because FIDE makes these automatic. Then `DrawThreefold` and `DrawFiftyMove` are checked. Then legal move generation. The claimable draws should only trigger when the game is **not** otherwise decided; a position with 0 legal moves and in-check is always checkmate regardless of the clock.
- **Failure scenario:** Construct a position with `halfmove_clock = 100` where the side to move is in checkmate (e.g. a back-rank mate reached without pawn move or capture in the last 50 moves, which is uncommon but possible in endgame studies). `position_status` returns `DrawFiftyMove`. The engine or UI reports a draw; the winning side is denied the win.
- **Related:** BOARD-001 (both are `status.rs` correctness issues).
- **Where to look next:** `status.rs:113-134`; fix ordering: move `DrawThreefold` and `DrawFiftyMove` checks to AFTER the `legal.is_empty()` block.
- **Fix direction:** Move the `DrawThreefold` and `DrawFiftyMove` checks to after `generate_legal` and the `Checkmate`/`Stalemate` decision. Only then, if the position is `InPlay`, check claimable draws. Automatic draws (fivefold, 75-move) may reasonably remain first.

---

### BOARD-003 — FEN parser silently accepts skip digit '0', admitting malformed positions without error

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified (no build)
- **Anchors:** `backend/src/board/fen.rs:143-148`
- **Expected:** Standard FEN uses digits 1–8 to denote empty squares. '0' is not a legal FEN digit. The parser should reject it.
- **Actual:** `ch.to_digit(10)` returns `Some(0)` for the character '0'. The code adds 0 to `file` (a no-op) and continues. A rank string like `"0P7"` successfully parses as a pawn on a-file with 7 empty squares, indistinguishable from `"P7"`.
- **Reasoning:** `fen.rs:143`: `ch.to_digit(10)` accepts the full decimal range 0–9. There is no lower-bound check on `skip`. Observed: `"0P7"` → file=0, skip=0, file stays 0; next char 'P' sets pawn at a-file, file=1; skip=7, file=8. Loop ends, file==8, success. The malformed FEN is silently normalized.
- **Failure scenario:** Any position FEN with '0' in a rank field (e.g., from a poorly implemented PGN reader or Chess960 export) is accepted and the board state differs from what the sender intended.
- **Related:** None directly.
- **Where to look next:** `fen.rs:143-148`; add `if skip == 0 { return Err }` after extracting the digit.
- **Fix direction:** After `ch.to_digit(10)` returns `Some(skip)`, validate `skip >= 1 && skip <= 8` before adding to `file`.

---

### BOARD-004 — `is_insufficient_material` misclassifies KBB vs K with same-color bishops as sufficient

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified (no build)
- **Anchors:** `backend/src/status.rs:67-86`
- **Expected:** KBB vs K where both bishops share the same color square is a theoretical draw (insufficient material to force checkmate). FIDE Art. 5.2.2 covers dead positions.
- **Actual:** The `total_minors == 2` branch checks `wn==2` (no), `w_minors==1 && b_minors==1` (no, it's 2 vs 0), and falls through to `return false` — "sufficient material." Two same-color bishops cannot force mate against a bare king, so the position should be `DrawDeadPosition`.
- **Reasoning:** `status.rs:68-81`: for KBB vs K, `wb=2, wn=0, bb=0, bn=0`. `total_minors=2`. `wn==2` → false. `w_minors==1 && b_minors==1` → false (2 vs 0). Returns `false`. But two same-color bishops cannot deliver checkmate. Note: two different-color bishops CAN mate, so a blanket KBB→insufficient is wrong; the fix requires a bishop-square-color check.
- **Failure scenario:** Position: `K7/8/8/8/8/8/2b5/1b5k b - - 0 1` (K vs K+same-color-bishops). `position_status` returns `InPlay` instead of `DrawDeadPosition`.
- **Related:** BOARD-002 (both in `status.rs`).
- **Where to look next:** `status.rs:68-81`; also the standard same-color-bishop check uses `(bb_white & 0xAA...AA) == 0 || (bb_white & 0x55...55) == 0` to determine all bishops are on the same color.
- **Fix direction:** In the `total_minors == 2` branch, when `wb == 2 && bn == 0 && wn == 0` (or the mirror), check whether both white bishops are on the same square color (mask with light-squares or dark-squares pattern); if so, return `true` (insufficient). Same for `bb == 2`.

---

### BOARD-005 — `set_fen` calls `compute_zobrist_full()` twice identically

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/board/fen.rs:263-264`
- **Expected:** The Zobrist hash is computed once after all FEN fields are parsed.
- **Actual:** Lines 263 and 264 are identical: `self.zobrist = self.compute_zobrist_full();` — the second call overwrites the first with the same value. `compute_zobrist_full` is a pure read; no side effects, so correctness is unaffected, but it doubles the work.
- **Reasoning:** Looks like a copy/paste artifact from a refactor. The function is pure (no mutation), so the double call is dead.
- **Failure scenario:** No correctness failure. Minor CPU waste on every FEN load.
- **Fix direction:** Delete line 264 (the duplicate).

## Clean bill

- **Castling-rights mask logic** (`castle_bits.rs`): The four bit constants are single-bit, disjoint, and verified by a test. Correct.
- **Castling rights cleared on rook CAPTURE** (`execute.rs:160-165`): The `capture` tuple is checked and `rights_mask_to_clear_for_rook(cap_color, cap_sq.index())` is applied. The classic "rook captured on home square leaves stale castling rights" bug is NOT present here.
- **Castling rights cleared on rook MOVE** (`execute.rs:155-158`): Checked via `rights_mask_to_clear_for_rook(color, mv.from.index())`. Correct.
- **Occupancy/bitboard sync in `set_bb`** (`mod.rs:106-149`): Every call updates `piece_bb`, `occ_white`/`occ_black` via XOR of the delta, then `occ_all = occ_white | occ_black`. `piece_on_sq` is updated per-square in the same loop. The invariant is maintained in a single function; no callers bypass it.
- **`Square::from_str` validation** (`square.rs:46-72`): Validates length==2, file in a–h (with uppercase folding), rank in 1–8. Off-board inputs properly return `Err`.
- **`Square::from_index` reachability of panic**: Called internally with values derived from `trailing_zeros()` on 64-bit integers (produces 0–63 only) or from `parse_en_passant` via `Square::from_str` (validates range). Not reachable from untrusted FEN input with an out-of-range index.
- **`Color::from_u8` / `Piece::from_u8` panics** (`mod.rs:506-537`): Only called from `piece_at` using `piece_on_sq[i] >> 3` (gives 0 or 1) and `piece_on_sq[i] & 0b111` (gives 0–7, but only 0–5 are assigned; 6–7 values never appear because `place_piece_at_sq` only writes `(color<<3)|piece` with `piece` 0–5). Not reachable from external input assuming `set_bb` is the only writer.
- **`repetition_count` / `is_threefold` logic**: History is pushed with pre-move Zobrist in `make_move_basic` (`execute.rs:230`) and cleared on irreversible moves (`execute.rs:225-226`); undo restores the snapshot (`execute.rs:341-343`). `repetition_count` starts count at 1 (current) and scans history, so threefold correctly requires 2 history matches. `is_threefold` checks `>= 3`. This is correct.
- **FEN round-trip for all 16 castling combinations**: `castling_fen` emits exactly the set bits in K/Q/k/q order and `parse_castling_rights` reads them back bijectively. Round-trip is faithful for all 16 values 0–15.
- **`split_fen` defaults for missing fields** (`fen.rs:275-276`): Accepts 4-field FENs (missing halfmove and fullmove) and defaults to "0"/"1" respectively. This is compliant with common practice for abbreviated FEN.
- **`parse_rank` upper-bound overflow** (`fen.rs:136-161`): `checked_add` prevents u8 overflow for large skip digits; the trailing `file != 8` check catches a rank with too-few squares; the early `file >= 8` guard before each character catches too-many squares. Digit 9 (invalid but accepted by `to_digit(10)`) is caught by file==9 ≠ 8 at end. Only gap is digit '0' (BOARD-003 above).

## Test coverage inventory (43 tests)

Tests present: `new_empty_board`, `starting_position_pawns`, `starting_position_white/black_backrank`, `full_starting_position`, `new_board_castling`, `new_board_defaults`, `starting_placement`, `castling_fen`, `en_passant_fen_helper`, `to_fen_starting/empty`, `split_fen_valid/invalid`, `parse_placement_empty/starting`, `parse_active_color`, `parse_castling_rights_none/individual/all/invalid`, `parse_en_passant_none/valid/invalid`, `parse_clocks_valid/invalid_halfmove/invalid_fullmove`, `set_fen_round_trip_starting/custom`, `set_fen_invalid_*` (5 cases), `validate_no_overlap/with_overlap`, `valid_clocks`, `invalid_fullmove_zero`, `parse_fen_from_str/invalid`, `display_fen`.

**Gaps (untested):**
- FEN with 0 kings — should error but currently succeeds (BOARD-001 root cause)
- FEN with 9+ pawns on one side (no count validation)
- FEN with skip digit '0' (BOARD-003)
- Halfmove clock at boundary values 99/100/149/150
- EP square on a semantically wrong rank (e.g., "e4" in active play)
- `position_status` — zero tests for any game outcome
- `is_insufficient_material` — zero tests
- `repetition_count` / `is_threefold` / `is_repetition` — zero tests
- KBB vs K same-color bishop insufficient (BOARD-004)
- `king_square` with a zero king bitboard

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `backend/src/board/fen.rs:252` (set_fen doc) | Implicitly: valid FEN produces a legal position | No post-parse legality check (king count, pawn count, EP rank) | Engine panic from UCI `position fen` with missing king | Yes for BOARD-001 |
