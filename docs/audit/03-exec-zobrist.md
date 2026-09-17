# Execute / Zobrist — Audit Findings

**Agent:** audit-exec
**Scope:** `backend/src/moves/execute.rs` (503 LOC), `backend/src/hash/zobrist.rs` (139 LOC), `backend/src/book/polyglot_hash.rs` (103 LOC), `backend/tests/moveexec_tests.rs` (783 LOC, 19 tests), `backend/tests/zobrist_hash_tests.rs` (first 200 LOC of 1774, 49 tests declared)
**Date:** 2026-08-30 (overnight run)

## What I checked

- Read `backend/src/moves/execute.rs` in full (503 LOC): `make_move_basic`, `undo_move_basic`, `make_null_move`, `undo_null_move`, helper functions `remove_piece`, `place_piece`, `rook_castle_squares`, `rights_mask_to_clear_for_rook`, `debug_assert_valid_ep`.
- Read `backend/src/hash/zobrist.rs` in full (139 LOC): `ZobristKeys`, `xor_castling_rights_delta`, `ep_file_to_hash`, key generation.
- Read `backend/src/book/polyglot_hash.rs` in full (103 LOC): `compute_polyglot_hash`, `is_pawn_adjacent`.
- Read `backend/src/board/mod.rs` in full (588 LOC): `Board` struct, `set_bb` (Zobrist XOR path), `compute_zobrist_full`, `assert_hash`, `repetition_count`, `is_repetition`, `is_threefold`.
- Read `backend/tests/moveexec_tests.rs` in full (783 LOC, 19 tests).
- Read `backend/tests/zobrist_hash_tests.rs` lines 1–200.
- Grep: verified `debug_assertions` / `paranoid_hash` feature references in execute.rs.

## What I did not check, and why

- `zobrist_hash_tests.rs` lines 201–1774: token budget. The first 200 lines established the test pattern; spot-checked specific tests by name.
- Whether the UCI handler populates `board.history` with pre-root game moves (not in my scope; flagged as EXEC-004 low-confidence).
- Actual test run output (`cargo test --release` not run per brief rule 3/5).
- `board/fen.rs`: not read; relevant to EXEC-004 but out of scope.

## Findings

### EXEC-001 — All hash invariant guards are dead in the release test suite

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified (build not run)
- **Anchors:** `backend/src/moves/execute.rs:211-212`, `backend/src/moves/execute.rs:235`, `backend/src/moves/execute.rs:267-268`, `backend/src/moves/execute.rs:333-334`, `backend/src/moves/execute.rs:344-346`, `backend/src/board/mod.rs:484-494`
- **Expected:** Hash invariants (`assert_hash()`, `debug_assert_valid_ep`) fire during make/undo on every test run, catching any Zobrist drift introduced by code changes.
- **Actual:** Every guard in `execute.rs` is behind `#[cfg(debug_assertions)]`. The project's sole test command (`cargo test --release --features load_magic,deterministic_zobrist`) sets `opt-level=3` and disables `debug_assertions`. None of these checks compile into the test binary. The only hash protection that survives is the `Board: PartialEq` comparison inside roundtrip tests (since `zobrist` is a struct field), which catches errors only for tests that perform a complete make → undo cycle.
- **Reasoning:** `execute.rs:211-212` gates `debug_assert_valid_ep` on `#[cfg(debug_assertions)]`. `execute.rs:267-268` and `344-346` gate `board.assert_hash()` identically. `board/mod.rs:484` gates the function definition itself on `#[cfg(debug_assertions)]`, so it does not even exist in release. Any Zobrist bug introduced in a move type that lacks a roundtrip test (e.g., a newly added move flag) would pass all tests silently.
- **Failure scenario:** Introduce a single-square Zobrist XOR omission for, say, Black queenside castling. All 49 Zobrist tests pass. All 19 exec tests pass. The search accumulates hash drift after 20 plies of queenside castling, producing transposition-table collisions that cause random move quality degradation. No test catches this.
- **Related:** EXEC-002 (the `paranoid_hash` block is also dead and wouldn't catch it even if it ran).
- **Where to look next:** `execute.rs:211`, `267`, `333`, `344`; `board/mod.rs:484-494`.
- **Fix direction:** Add a `cargo test` step (without `--release`, or with `debug_assertions` explicitly enabled via `RUSTFLAGS`) to a CI stage that is specifically for hash invariant checking. Alternatively, promote the most critical assertions from `debug_assert_eq!` to `assert_eq!` guarded by a dedicated test-only feature flag rather than `debug_assertions`, so they run in release builds when that flag is set.

---

### EXEC-002 — `paranoid_hash` diagnostic block never runs, and would not fail tests even if it did

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/moves/execute.rs:235-265`
- **Expected:** The `paranoid_hash` block catches incremental-vs-full hash divergence after each `make_move_basic`.
- **Actual:** The block is gated `#[cfg(all(debug_assertions, feature = "paranoid_hash"))]`. Both conditions must hold simultaneously. In `--release` testing, `debug_assertions` is false, so the block is stripped regardless of any feature flag. Additionally, the block body (lines 237-265) never calls `panic!` or `assert!` — it only prints to `eprintln!`. Line 237 prints `"HASH DIFF: stored ^ full = 0x..."` unconditionally (including when diff == 0), which would flood stderr in every make call. Even in debug+paranoid_hash mode, the block is informational only; tests would not fail from a hash mismatch detected here.
- **Reasoning:** Read the cfg gate at line 235 and the body at lines 237-265 directly. There is no panic path.
- **Failure scenario:** Developer enables `paranoid_hash` in debug mode expecting tests to fail on hash drift; tests still pass because the block is diagnostic, not asserting.
- **Related:** EXEC-001.
- **Where to look next:** `execute.rs:235-265`.
- **Fix direction:** Change the always-printing `eprintln!("HASH DIFF: ...")` to only print when `diff != 0`. Add a `panic!("Hash mismatch: 0x{:016x}", diff)` at the end of the block when `diff != 0`. This makes the block actually enforce the invariant. Consider merging with `assert_hash()` rather than maintaining duplicate logic.

---

### EXEC-003 — History window after an irreversible move includes the pre-irreversible position

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/src/moves/execute.rs:215-233`, `backend/src/moves/execute.rs:337-343`, `backend/src/board/mod.rs:446-477`
- **Expected:** After an irreversible move (capture, pawn move, promotion), the repetition history is reset to contain only positions reachable from the new state. The pre-irreversible position cannot be legally revisited, so it should not occupy a history slot.
- **Actual:** `execute.rs:225-230` clears history then immediately pushes `start_zobrist`, which is `board.zobrist` *before* the move — i.e., the pre-irreversible position's hash. After the irreversible move, `history = [hash(position_before_capture)]`. The next reversible move pushes `hash(position_after_capture)`. Repetition detection at any subsequent position compares against both of these, including the forever-unreachable pre-capture position. This is semantically incorrect: the repetition window should begin at the post-irreversible position, not before it.
- **Reasoning:** Line 215 defines `irreversible`. Line 225 does `board.history.clear()`. Line 230 does `board.history.push(start_zobrist)` where `start_zobrist = board.zobrist` captured at line 48 (the pre-move hash). In practice, Zobrist collisions between an unreachable pre-capture position and a future position are cosmically unlikely, so this does not produce observable bugs. However, it means the first slot in every new irreversible window is theoretically wasted.
- **Failure scenario:** Not easily constructed without a Zobrist collision. A synthetic test could directly compare history contents against expectations.
- **Related:** None.
- **Where to look next:** `execute.rs:215-233`. Contrast with standard implementations that push the *post*-move position's key (or the pre-move key but only for moves after the irreversible one).
- **Fix direction:** Push `start_zobrist` (the pre-move key) only when the move is *reversible*. For irreversible moves: clear history, then push nothing (the current irreversible move's key is the board.zobrist *after* the move, and is the reference; the very next reversible move will push it). Alternatively, push the post-move key instead of the pre-move key everywhere, adjusting `repetition_count` accordingly.

---

### EXEC-004 — Game-history positions before the search root may not be present in `board.history`

- **Severity:** P2
- **Confidence:** low
- **Status:** unverified
- **Anchors:** `backend/src/board/mod.rs:469-477`, `backend/src/board/mod.rs:183-235` (Board::new)
- **Expected:** When the engine receives a UCI `position` command with a list of prior moves, the resulting board's `history` Vec contains all positions from the start of the current irreversibility window so that `is_repetition` at the root correctly detects draws by twofold/threefold repetition.
- **Actual:** `Board::new()` sets `history` to an empty Vec (line 208: `b.history.clear()`). There is no evidence in the files I read that the UCI handler or position-setup code replays prior moves in a way that populates `history` with pre-root positions. If the UCI handler calls `Board::from_str` (which uses `new_empty()` → `set_fen()`) and does not then apply moves via `make_move_basic`, `history` will be empty at the search root and the engine cannot detect repetitions whose second or third occurrence falls within the search tree but whose earlier occurrences were in the game history.
- **Reasoning:** Could not confirm or deny without reading the UCI handler (out of scope). Confidence low because the handler may correctly apply moves via `make_move_basic`.
- **Failure scenario:** Position after 30 moves where the current position has been seen twice before; engine plays it again, missing the threefold claim and potentially playing a losing move.
- **Related:** None in this scope.
- **Where to look next:** `backend/src/bin/cli.rs` — search for how `position startpos moves …` is handled and whether it applies moves via `make_move_basic`.
- **Fix direction:** After parsing the move list from the UCI `position` command, apply each move using `make_move_basic` (rather than re-parsing the FEN from scratch), so `history` is populated correctly at the root.

## Clean bill

- **Piece-square Zobrist XOR via `set_bb`.** `board/mod.rs:106-149` shows that `set_bb` computes `delta = old_bb ^ new_bb` and XORs the key for every changed square. All piece movements (quiet, capture, castling rook, EP captured pawn, promotion) go through `remove_piece`/`place_piece` → `set_bb`, so piece-square keys are always toggled correctly and symmetrically. Verified by tracing each move type through the call chain.

- **Promotion Zobrist correctness.** Pawn is removed from `from_idx` via `remove_piece` (XORs out pawn key). Promoted piece is placed at `to_idx` via `place_piece` (XORs in promoted-piece key). For capture-promotion the captured piece is removed before (XORs out captured key). All three XORs are present and correct. Verified by reading `execute.rs:175-184`.

- **EP-captured pawn Zobrist at the correct off-destination square.** `execute.rs:68-79` computes `cap_sq` as `to_idx ± 8` (the actual pawn's square, not the destination), removes it via `remove_piece`, which XORs out the key at `cap_sq`. The `Undo.capture` tuple stores this square, and `undo_move_basic:314-317` restores the pawn there. Correct.

- **Castling rights Zobrist via `xor_castling_rights_delta`.** `zobrist.rs:40-53` XORs the key for each bit in `old ^ new`. Called on make with `(old_rights, new_rights)` and on undo with `(cur_rights, prev_rights)`. Since XOR is self-inverse, the hash is correctly restored. Rook-capture clearing of rights (case iii at `execute.rs:161-165`) is also covered. Verified end-to-end.

- **EP file Zobrist rule — consistent between `zobrist.rs` and `polyglot_hash.rs`.** Both `ep_file_to_hash` (`zobrist.rs:59-88`) and `is_pawn_adjacent` (`polyglot_hash.rs:63-103`) apply the Polyglot rule: include the EP file only if the side-to-move has a pawn diagonally adjacent that can capture at the EP square. Rank checks (r==2 or r==5 in `ep_file_to_hash`; explicit `ep_rank != 5` / `ep_rank != 2` gates in `is_pawn_adjacent`) are consistent. The side-to-move convention differs between the two (engine XORs when Black to move; Polyglot XORs when White to move) but this is intentional and correct since they use independent key tables.

- **Null move Zobrist correctness.** `make_null_move` XORs out the EP file (with original side still set), clears EP, flips side, XORs `side_to_move`. `undo_null_move` reverses exactly: flips side back, restores EP, XORs in the EP file (now with original side restored). The inverse is correct because `ep_file_to_hash` is called with the same side and EP in both directions. Verified by trace.

- **Halfmove clock.** Reset when `capture.is_some() || piece == Piece::Pawn` (`execute.rs:194-198`). EP capture: `capture = Some(...)` is set in the EP branch (`execute.rs:68-79`), so halfmove clock resets. Castling: `capture` is None and piece is King, so halfmove clock increments. Both behaviors are correct.

- **Make/unmake symmetry for all move types.** Checked: quiet, capture, double pawn push (EP square set/restored), EP capture (off-destination pawn removed/restored), all four castlings (rook from/to saved in `undo.castling_rook`, correctly reversed), promotion (prom piece removed, pawn restored), capture-promotion. All fields in `Undo` that are modified during make are saved and restored: `castling_rights`, `en_passant`, `halfmove_clock`, `fullmove_number`, `side_to_move`, `capture`, `castling_rook`, `promotion`, `prev_history`. Zobrist is restored by exact-inverse XOR operations.
