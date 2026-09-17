# Book & UCI/CLI — Audit Findings

**Agent:** audit-book-cli
**Scope:** `backend/src/book/` (polyglot_book.rs, polyglot_entry.rs, polyglot_hash.rs, mod.rs), `backend/src/bin/cli.rs`, `backend/src/output.rs`, `backend/src/logger.rs`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/book/polyglot_hash.rs` (103 LOC) — full read
- `backend/src/book/polyglot_book.rs` (88 LOC) — full read
- `backend/src/book/polyglot_entry.rs` (138 LOC) — full read
- `backend/src/book/mod.rs` (6 LOC) — full read
- `backend/src/bin/cli.rs` (515 LOC) — full read
- `backend/src/output.rs` (16 LOC) — full read
- `backend/src/logger.rs` (51 LOC) — full read
- `grep -n "unwrap"` across all book files and cli.rs
- `grep -n "println!"` across cli.rs

## What I did not check, and why

- `polyglot_keys.rs` (783 LOC generated constants): spot-checked the index layout against spec in the hash logic only; did not read the raw values against a published reference table — would need a known-position test vector to verify definitively.
- Whether `SearchLimits::node_limit` is read inside `search()` — that is outside my file scope.
- Whether `san_to_uci` handles all SAN disambiguation edge cases correctly — complex coverage analysis needs tests.

## Findings

### UCI-001 — `stop` command not implemented; search blocks the input loop

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:43-69` (match block, no `"stop"` arm), `cli.rs:313` (`search(...)` call is synchronous, on the main thread)
- **Expected:** Per the UCI specification, the engine must be able to receive and honour a `stop` command at any time during a search, immediately returning a `bestmove`.
- **Actual:** There is no `"stop"` arm in the command match. Because `search()` is called synchronously on the main I/O thread (line 313), no further lines are read from stdin until the search finishes. A `stop` sent by a GUI during search is silently dropped into the OS pipe buffer and consumed as the _next_ command after search completes — it does not interrupt the search.
- **Reasoning:** The `for line in stdin.lock().lines()` loop at `cli.rs:30` is single-threaded. `handle_go` (called at line 56) calls `search(...)` at line 313 and does not return until search ends. Any `stop` sent by the GUI during that window arrives in the pipe buffer, is read _after_ search finishes, and hits the `_ => {}` fallback — never interrupting anything.
- **Failure scenario:** GUI sends `go wtime 60000 btime 60000`; engine starts a deep search. GUI detects the user moved and sends `stop`. Engine ignores it, continues for the full allocated time, then emits `bestmove` late. Next command is whatever the GUI sent after `stop` (likely `position`), which the engine now processes in place of the lost `stop`. Causes time violations and protocol desync in any real GUI.
- **Related:** UCI-002 (infinite compound case)
- **Where to look next:** `cli.rs:30-70` (I/O loop), `cli.rs:313` (blocking search call)
- **Fix direction:** Spawn the search on a separate thread; use a shared atomic flag (`AtomicBool`) for stop signalling. The main thread keeps reading stdin and sets the flag when `stop` arrives. The search checks the flag at its normal time-check points.

---

### UCI-002 — `go infinite` cannot be stopped

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:237-241` (`infinite` arm), `cli.rs:313`
- **Expected:** `go infinite` should search until `stop` arrives.
- **Actual:** `infinite` sets `depth = 100` and `time_limit = None`. Because `stop` is unimplemented (UCI-001) and search is synchronous, the engine will block until it reaches depth 100 or the OS kills it. On even moderate hardware, depth 100 is unreachable in finite time for non-trivial positions.
- **Reasoning:** The `infinite` arm does not set a flag that the I/O loop checks; the search just runs to the configured `max_depth` with no time limit. Combined with the inability to receive `stop`, this is a hang.
- **Failure scenario:** `echo "uci\nisready\nposition startpos\ngo infinite" | vantage-cli` — engine will never emit `bestmove` without an external kill signal.
- **Related:** UCI-001
- **Where to look next:** `cli.rs:237-241`, `cli.rs:308-313`
- **Fix direction:** Same threaded fix as UCI-001. `infinite` should set a flag that only `stop` clears, rather than mapping to a large depth constant.

---

### BOOK-001 — Weighted book selection returns None instead of continuing when decode fails

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/book/polyglot_book.rs:68-73`
- **Expected:** When weighted selection picks an entry that cannot be decoded (no piece at from-square), the probe should try the remaining candidates.
- **Actual:** `return entry.decode_move(board)` propagates `None` immediately when the selected entry decodes to `None`, exiting the loop and returning `None` from `probe`. Remaining candidates — which may decode successfully — are never tried.
- **Reasoning:** `decode_move` returns `None` at `polyglot_entry.rs:47` when `board.piece_at(from_sq)` yields `None` (no piece at the from-square recorded in the book entry). For a valid `book.bin` in the correct position this should not occur, but a hash collision, a corrupt book, or a book generated from a different move-encoding scheme could produce this. When it does, `probe` silently returns `None` even though valid alternatives exist in `candidates`.
- **Failure scenario:** Book has two candidates for a hash: entry A (malformed, from_sq empty) with weight 100 and entry B (valid) with weight 50. Pick = 49 → selects entry A → `decode_move` returns None → probe returns None → engine falls through to search. Engine misses both book moves.
- **Related:** BOOK-002
- **Where to look next:** `polyglot_book.rs:68-73`, `polyglot_entry.rs:47`
- **Fix direction:** After the `if pick < w` check, if `decode_move` returns `None`, do not return — `continue` to the next iteration (decrement `pick` by `w` so the weighted accounting is still correct, or just try in sequence after the weight-selected entry fails).

---

### BOOK-002 — Book probe is non-deterministic (unseeded RNG)

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/book/polyglot_book.rs:65-66`
- **Expected:** In analysis and test contexts the engine is expected to be deterministic (the test suite uses `--features deterministic_zobrist`).
- **Actual:** `rand::rng()` at line 65 uses the thread-local entropy-seeded RNG. Each `probe` call in the same position may return a different book move. Two `go` commands on the same position can produce different `bestmove` replies.
- **Reasoning:** `rand::rng()` in the `rand` 0.9 API is equivalent to `SmallRng` seeded from entropy. No seed is set. The analysis module (see context item 9) makes determinism claims; book randomness undermines those for positions covered by the book. Testing regression is also impacted: a CI run that exercises a book position can non-deterministically pass or fail depending on which book move is selected.
- **Failure scenario:** Analysis test: `position fen <start> moves e2e4` → book hit → first run selects `e7e5`, second run selects `c7c5`. If the test asserts a specific best move, it is flaky.
- **Related:** (ANALYSIS prefix tests in context item 9)
- **Where to look next:** `polyglot_book.rs:65-66`
- **Fix direction:** Accept an optional `rand::SeedableRng` parameter or a seeding enum in `probe`, so callers in analysis/test contexts can pass a deterministic seed. UCI play can keep entropy-seeded randomness. Alternatively, expose a `probe_best` that always returns the highest-weight entry and a `probe_random` variant.

---

### UCI-003 — `nodes` time control parameter silently ignored

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:192-246` (go parse loop, no `"nodes"` arm), `cli.rs:308-312` (`SearchLimits { node_limit: None }` always)
- **Expected:** UCI `go nodes N` should limit the search to N nodes.
- **Actual:** There is no `"nodes"` arm in the go-parameter parse loop. `node_limit` in `SearchLimits` is hardcoded to `None` on line 311.
- **Reasoning:** The `SearchLimits` struct has a `node_limit` field (used by tests), but `handle_go` never populates it from the `go nodes` parameter.
- **Failure scenario:** GUI sends `go nodes 100000`; engine ignores the limit and searches to full depth/time. GUI's node-budget accounting breaks.
- **Related:** UCI-001
- **Where to look next:** `cli.rs:192-246`, `cli.rs:311`
- **Fix direction:** Add a `"nodes"` arm that parses into a `u64` and populates `SearchLimits::node_limit`.

---

### UCI-004 — `stop` silently consumed as unknown command post-search

- **Severity:** P3** (symptom documented under UCI-001; this is the hygiene note)
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:68` (`_ => {}`)
- **Expected:** Unknown UCI commands should be silently ignored.
- **Actual:** They are silently ignored — which is correct per spec, but `stop` is not unknown, it is a required command. Its absence compounds UCI-001 rather than being a separate fault. Filed here to track the missing match arm independently.
- **Fix direction:** Add `"stop" => {}` as a named arm once UCI-001's threading fix is in place, so `stop` handling can be wired in without restructuring the match.

---

### UCI-005 — `info string` noise on stdout at startup may confuse GUI parsers

- **Severity:** P3
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:23-26` (book load messages printed before any command)
- **Expected:** Per UCI, the engine should not emit anything before receiving the `uci` command.
- **Actual:** Lines 23-26 emit `info string` messages during `main()` init, before the command loop starts — i.e., before any `uci` command is received. Most GUIs tolerate `info string` at any time, but the spec implies the engine waits for `uci` before speaking.
- **Reasoning:** Minor protocol deviation; most GUIs handle it, but strict parsers may error.
- **Fix direction:** Defer the book-load message to the handler for `uci` or print it as part of `info string` only after `uci` is received.

## Clean bill

- **EP conditional hashing** (`polyglot_hash.rs:47-53`): The code correctly implements the Polyglot conditional rule — it XORs the EP file random only when `is_pawn_adjacent` confirms an enemy pawn can actually capture. This is the right behaviour. The `is_pawn_adjacent` helper checks the correct capture rank (4 for white, 3 for black) and adjacent files.
- **Castling key index order** (`polyglot_hash.rs:33-44`): Indices 768 (WK), 769 (WQ), 770 (BK), 771 (BQ) match the published Polyglot order.
- **Piece kind encoding** (`polyglot_hash.rs:14-27`): `piece_kind = pt_idx * 2 + color_offset` produces 0=black pawn … 11=white king, matching the Polyglot spec's interleaved color-minor ordering.
- **Castling move decode** (`polyglot_entry.rs:53-93`): All four king-takes-rook encodings (e1h1→e1g1, e1a1→e1c1, e8h8→e8g8, e8a8→e8c8) are explicitly mapped with correct KINGSIDE/QUEENSIDE_CASTLE flags.
- **Entry byte parsing** (`polyglot_entry.rs:18-21`): The four `unwrap()` calls on `try_into()` cannot panic. `from_bytes` is only ever called with an exactly 16-byte slice (caller at `polyglot_book.rs:23` uses `&data[start..start + 16]` where `count = data.len() / 16`). The fixed-size coercions are infallible.
- **Truncated book.bin** (`polyglot_book.rs:17-27`): A file whose length is not a multiple of 16 silently discards the trailing partial entry (`count = data.len() / 16` rounds down). No panic; the engine proceeds with fewer entries.
- **Missing book.bin** (`cli.rs:21`): `PolyglotBook::load(...).ok()` converts `io::Error` to `None`. `handle_go` guards with `if let Some(b) = book`. Clean fallback to search.
- **Endianness** (`polyglot_entry.rs:18-21`): All fields parsed `from_be_bytes` — correct, Polyglot is big-endian.
- **Promotion decode** (`polyglot_entry.rs:99-105`): Bits 12-14 → 0=none,1=knight,2=bishop,3=rook,4=queen. Matches Polyglot spec.
- **`parse_uci_move` legality** (`cli.rs:144-161`): Generates legal moves and matches against them, so only legal moves from the move string are accepted.
- **`logger.rs`** (`logger.rs:1-51`): Uses `tracing-appender` writing to a file, not stdout. Will not corrupt a stdio protocol stream.
- **`output.rs` `engine_println!`** (`output.rs:1-16`): The macro routes to `println!` on native and `console::log_1` on WASM. It is not used in `cli.rs` (cli uses raw `println!` correctly for UCI). The macro itself does not corrupt any protocol stream.
- **Malformed FEN in `position fen`** (`cli.rs:93`): `Board::from_str(&fen_string).ok()?` propagates `None`; `handle_position` returns `None`; the board state is unchanged. No panic.
- **`go depth abc`** (`cli.rs:197`): `parse().unwrap_or(64)` falls back to 64. No panic.
- **EPD runner missing file** (`cli.rs:324-335`): Both path attempts wrapped in `Err(_)` arms; prints an error string and returns. No panic.
- **`setoption` absent**: Falls to `_ => {}`. Correct per roadmap (explicitly out of scope).
- **Time management zero-time guard** (`cli.rs:261`): `t.saturating_sub(safety_buffer)` prevents underflow. The final allocation is floored at 10ms when remaining time permits (`cli.rs:297-299`). `movetime 0` would set `Duration::from_millis(0)` — depends on search internals whether that hangs; outside this file scope.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `vantage-engine-pass.md:59` (implied) | `setoption` UCI command | No `setoption` arm; silent `_ => {}` | Cannot configure engine options from GUI | No — roadmap marks as out-of-scope |
| `cli.rs:311` (`node_limit: None`) | `go nodes N` node budget | Field exists in `SearchLimits`; never populated from UCI | GUI node-count control broken | No |
| UCI spec / `cli.rs:68` | `stop` command | No arm in match; search is synchronous | Engine unusable in GUI; UCI-001 | Yes — P1 |
