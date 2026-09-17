# UCI Live Behaviour — Audit Findings

**Agent:** baseline-uci
**Scope:** `backend/src/bin/cli.rs`, live binary `backend/target/release/vantage-cli.exe`
**Date:** 2026-08-30 (overnight run)

## What I checked

- Read `backend/src/bin/cli.rs` (full file, ~410 LOC)
- Read `backend/src/search/search.rs` lines 73-110, 630-770 (SearchLimits, main search loop)
- Ran 20+ live binary invocations — full list below
- `grep -n "max_depth|SearchLimits|node_limit" backend/src/search/search.rs`
- `grep -n "stop|Stop|abort" backend/src/search/search.rs`
- `grep -n "Invalid move|eprintln|println|print!|info string" backend/src/bin/cli.rs`
- `grep -n "depth|nodes|infinite|stop|bestmove|movetime|go|parse|node_limit" backend/src/bin/cli.rs`

### Invocations run

| # | Input pipe | Capture | Key observation |
|---|---|---|---|
| 1 | `uci\nisready\nquit` | stdout+stderr separate | Handshake; book message before uci |
| 2 | `uci\nisready\nposition startpos\ngo depth 6\nquit` | stdout | Basic search |
| 3 | `uci\nisready\nposition startpos\ngo depth 8\nquit` | stdout | PV sanity |
| 4 | `quit` (only) | stdout | Confirms pre-uci book message |
| 5 | `uci\nisready\nposition startpos moves e2e4 e7e5 g1f3 b8c6 f1b5\nd\ngo depth 6\nquit` | stdout | Ruy Lopez, `d` command |
| 6 | kiwipete FEN + `go depth 6` | stdout | FEN position |
| 7 | en-passant FEN + `go depth 6` | stdout | EP handling |
| 8 | `go movetime 500` (timed) | stdout | Time control |
| 9 | `go movetime 0` | stdout | Zero movetime |
| 10 | `go wtime 1000 btime 1000 winc 0 binc 0` | stdout | wtime/btime |
| 11 | `go nodes 1000` then `go nodes 100000` | background | Node limit (timed out at 60s) |
| 12 | `go infinite` (timeout 3s wrapper) | stdout | Blocked until killed (exit 124) |
| 13 | `go infinite\nstop\nquit` (timeout 5s) | stdout | stop not acted on |
| 14 | `go depth abc` | background | Falls to depth 64 |
| 15 | `position fen garbage\ngo depth 3` | stdout | Silently ignores bad FEN |
| 16 | `position fen 8/8/8/8/8/8/8/8 w - - 0 1\ngo depth 3` | stdout+stderr | **PANIC** |
| 17 | `position startpos moves e2e5\ngo depth 3` | stderr-captured | Invalid move logged to stderr |
| 18 | `position startpos moves e2e4 e2e4\ngo depth 3` | stderr-captured | Second move invalid |
| 19 | 10000-char junk line (bash repeat) | stdout | Ignored cleanly |
| 20 | EOF without `quit` after depth 3 | stdout | Exits cleanly (code 0) |
| 21 | `ucinewgame` + search + `isready` + search again | stdout | TT cleared, ucinewgame handled |
| 22 | `setoption name Hash value 64` | stdout | Silently accepted |
| 23 | `go depth -1` | stdout | `bestmove 0000` |
| 24 | `go depth 0` | stdout | `bestmove 0000` |
| 25 | `go depth 999` (timeout 3s) | stdout | Runs to depth 11+ then killed |

## What I did not check, and why

- **`go nodes 1000` vs `go nodes 100000` diff**: both timed out at 60s searching to depth 14 with ~33M nodes, confirming they're identical, but I could not capture two separate runs side-by-side before timeout. The source reading at `cli.rs:310` (`node_limit: None`) is definitive.
- **PV legality verification move by move**: depth 8 PV was `d2d4`; I confirmed d2d4 is legal from startpos. Full sequential move-by-move replay via position+FEN was not done (would require scripting 8 position commands). PV appeared plausible at all depths.
- **Benchmark / EPD code paths** (lines 322+): not part of UCI live audit.
- **Mated/stalemate positions**: not tested; `bestmove 0000` for those is correct UCI; the bug is emitting it for positions with legal moves.

---

## Findings

### UCI-101 — `info string` book status printed to stdout before `uci` is received

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:14-26`
- **Expected:** UCI spec requires the engine to be silent until a `uci` command is received. The only permitted pre-handshake output is nothing; all `info`, `id`, `uciok` lines belong in the response to `uci`.
- **Actual:** Lines 22-26 print to stdout unconditionally at process startup, before the main loop begins and before any command is read. Sending `printf 'quit\n'` as the sole input produces `info string No opening book found (book.bin), running engine only` on stdout and exits 0.
- **Reasoning:** `main()` calls `PolyglotBook::load("book.bin").ok()`, inspects the result, and immediately `println!`s the book status (lines 23-26). The UCI read loop (`for line in stdin.lock().lines()`) does not start until line 30. Any data on stdout before that loop is pre-handshake output.
- **Failure scenario:** `printf 'quit\n' | ./vantage-cli.exe` → stdout contains `info string No opening book found (book.bin), running engine only`. A UCI GUI that validates pre-`uciok` silence will flag this as a protocol error or misparse it.
- **Related:** UCI-105 (stdout hygiene), known-context item 1 (stdio MCP server must never write non-protocol data).
- **Where to look next:** `cli.rs:14-29` — startup block before the loop.
- **Fix direction:** Move the book-status print into the `"uci"` arm handler (after `id` lines, before `uciok`), or buffer it to emit as part of the `uci` response.

---

### UCI-102 — `stop` command silently ignored; synchronous search cannot be interrupted

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:43-65` (match arms), `backend/src/bin/cli.rs:55-57` (go arm calls search synchronously)
- **Expected:** UCI spec requires the engine to read `stop` from stdin and return `bestmove` promptly. After `go infinite` (or any long search), `stop` must terminate the search.
- **Actual:** There is no `"stop"` arm in the match statement (`cli.rs:43-65`). The `go` arm calls `handle_go()` → `search()` synchronously on the main thread. While `search()` is running, the main loop is blocked; stdin is not read. Even if `stop` is piped immediately after `go infinite`, the engine never reads it until search finishes. Empirically: `printf 'uci\nisready\nposition startpos\ngo infinite\nstop\nquit\n' | timeout 5 ./vantage-cli.exe` ran to depth 11 (1.7M nodes, 4s elapsed) before the OS killed it at 5s. `bestmove` was never emitted.
- **Reasoning:** Single-threaded synchronous design. `search.rs:642` shows `for depth in 1..=limits.max_depth` running until completion or internal time/node stop. No inter-thread signal path exists for an external `stop`. The match arm is simply absent.
- **Failure scenario:** Arena tournament: GUI sends `go wtime 5000`, engine thinks it has 5s but the GUI decides to abort at move 30 and sends `stop`. Engine continues searching. GUI declares forfeit or plays a stale move.
- **Related:** UCI-103 (`go infinite` blocks), UCI-104 (`go nodes` ignored).
- **Where to look next:** `cli.rs:43-65`, `search.rs:88-130` (SearchContext stop_signal exists but is only set internally).
- **Fix direction:** Move search to a background thread; use an `Arc<AtomicBool>` stop flag passed into `SearchContext::stop_signal`. The main loop reads `stop` and sets the flag. The search polls it (already polls `stop_signal` at `search.rs:107,116,123,194,278,290,380,550,600`). Wire those polling points to the shared flag.

---

### UCI-103 — `go nodes N` silently ignored; node limit always `None`

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:308-312`, `backend/src/search/search.rs:73-75`
- **Expected:** `go nodes 1000` should limit the search to ~1000 nodes and return `bestmove`.
- **Actual:** The `"nodes"` token has no arm in the `while i < parts.len()` parser (`cli.rs:193-245`). `SearchLimits { node_limit: None }` is always constructed (line 310). `go nodes 1000` produces the same 14-depth, 33M-node search as `go nodes 100000`; both timed out at 60s without returning `bestmove`.
- **Reasoning:** `grep "nodes"` in the go parser returns only the `nodes` field in the `info` output. There is no `"nodes" => { … }` match arm.
- **Failure scenario:** `go nodes 1` → engine searches to depth 64+; never returns in reasonable time.
- **Related:** UCI-102 (stop), `search.rs:120` (node_limit check exists but is unreachable since it's always None).
- **Where to look next:** `cli.rs:193-245` (the while-i parser), `search.rs:91` (`node_limit` field in SearchContext).
- **Fix direction:** Add a `"nodes"` arm to the parser that sets `limits.node_limit = Some(n)`.

---

### UCI-104 — `go infinite` is depth-100 hardcode, not truly infinite; also cannot be interrupted

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:237-240`
- **Expected:** `go infinite` means search until `stop` is received. Depth should be unbounded (the loop just keeps deepening). Returning from depth 100 without a `stop` is incorrect.
- **Actual:** `"infinite" => { depth = 100; time_limit = None; }`. If the engine ever reaches depth 100 (extremely unlikely in practice, but not for trivial positions), it will emit `bestmove` without waiting for `stop`.
- **Reasoning:** The literal assignment `depth = 100` caps `for depth in 1..=limits.max_depth` at 100.
- **Failure scenario:** Endgame position where engine solves to depth 100 → emits unsolicited `bestmove` while GUI expects `stop`-driven termination.
- **Related:** UCI-102 (`stop` not wired).
- **Where to look next:** `cli.rs:237-240`.
- **Fix direction:** Set `depth = i32::MAX` (or a constant like `MAX_SEARCH_DEPTH`) and rely solely on the `stop` signal from a background thread to terminate the loop.

---

### UCI-105 — `bestmove 0000` (null move) returned for legal positions when depth ≤ 0

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:181,197,315-318`, `backend/src/search/search.rs:74,642`
- **Expected:** `bestmove 0000` is the UCI convention for checkmate/stalemate (no legal move). It must not be returned when legal moves exist.
- **Actual:** `go depth 0`: `"0".parse::<i32>()` = 0 → `for depth in 1..=0` is empty → `result.best_move = None` → `bestmove 0000`. Same for `go depth -1` (-1 parses successfully as i32 = -1). Same for `go movetime 0` (time limit 0ms → no iteration executes before time expires → None → `bestmove 0000`).
- **Reasoning:** `SearchLimits.max_depth: i32` (`search.rs:74`). Negative values parse successfully. The Rust range `1..=-1` is empty so the depth loop body never runs. `result.best_move` remains None. `cli.rs:318` prints `bestmove 0000`.
- **Failure scenario:** GUI sends `go depth 0` (seen in some GUIs as a perft-0 probe) → engine declares no move → GUI treats as immediate resign or loss.
- **Related:** UCI-103.
- **Where to look next:** `cli.rs:181,197`; `search.rs:642`.
- **Fix direction:** Clamp depth: `depth = depth.max(1)` after parsing, and handle `movetime 0` by returning the best move from a depth-1 search rather than aborting.

---

### UCI-106 — Panic on position with no kings (`king_square()` unreachable → abort)

- **Severity:** P0
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/board/mod.rs:359`, `backend/src/bin/cli.rs:93` (FEN parse → Board)
- **Expected:** Malformed positions (no king) should be rejected gracefully. The engine should emit an error or ignore the position and not crash.
- **Actual:** `position fen 8/8/8/8/8/8/8/8 w - - 0 1` followed by `go depth 3` causes:

  ```
  thread 'main' (87084) panicked at src\board\mod.rs:359:13:
  King missing for White!
  FEN: 8/8/8/8/8/8/8/8 w - - 0 1
  Occ: 0x0
  ```
  
  Process exits with code 101. A UCI GUI running the engine as a subprocess detects unexpected termination.

- **Reasoning:** `Board::from_str` (called at `cli.rs:93`) successfully parses the FEN (the FEN is syntactically valid — just missing kings). The search then calls `king_square(side)` which panics via `panic!("King missing for {side:?}!")` at `mod.rs:359`. No guard exists at the CLI level to reject king-less positions before search.
- **Failure scenario:** Pipe `position fen 8/8/8/8/8/8/8/8 w - - 0 1\ngo depth 1\n` → crash, exit 101, GUI loses connection.
- **Related:** Known-context item 9 (static agent already flagged this).
- **Where to look next:** `backend/src/board/mod.rs:355-365`, `cli.rs:93` (after `Board::from_str`).
- **Fix direction:** After `Board::from_str`, verify both sides have exactly one king bit set before accepting the position. If not, keep the existing board and optionally emit `info string rejected invalid position`.

---

### UCI-107 — `go depth abc` (unparseable depth) falls back to depth 64, not an error

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:197`
- **Expected:** UCI spec says unknown tokens in `go` should be ignored. An unparseable depth could reasonably fall back to a sensible default. However, falling back to depth 64 (effectively unbounded) is dangerous — the engine will run for minutes.
- **Actual:** `"abc".parse::<i32>()` fails → `unwrap_or(64)` → `max_depth = 64`. Live run: searched to depth 14 (33M nodes, 35s) before timed out; `bestmove` never returned.
- **Reasoning:** `cli.rs:197`: `depth = parts[i + 1].parse().unwrap_or(64)`. The fallback of 64 is indistinguishable from a user sending `go depth 64`.
- **Failure scenario:** Protocol fuzzing or a buggy GUI sends malformed `go` → engine hangs for minutes.
- **Related:** UCI-102, UCI-104.
- **Where to look next:** `cli.rs:195-199`.
- **Fix direction:** Use `.ok()` and only update depth if parse succeeds; leave depth at a safe default like 20 or 1. Or emit `info string ignoring unparseable depth token`.

---

### UCI-108 — `info string` leaks internal diagnostic to stdout

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:306`
- **Expected:** Per `CLAUDE.md` "stdio server must never print to stdout except protocol JSON-RPC" (written for the MCP server, but the principle applies). Internal diagnostics should go to stderr. The `info string` channel is valid UCI but should carry user-relevant info, not internal timing details.
- **Actual:** `info string Target time: 42ms` is printed to stdout for every time-managed search. Every invocation with wtime/btime or movetime produces this line.
- **Reasoning:** `cli.rs:306`: `println!("info string Target time: {}ms", limit.as_millis())`. A GUI that counts info strings might display this to the user. Any future MCP adapter that wraps the engine will see this line mixed into the response stream.
- **Failure scenario:** MCP adapter (planned) reads engine stdout expecting `bestmove` lines; sees `info string Target time:` as an unexpected token in a structured parse.
- **Related:** UCI-101 (other pre-handshake stdout).
- **Where to look next:** `cli.rs:305-307`.
- **Fix direction:** Move to `eprintln!` or remove entirely.

---

### UCI-109 — `fen` and `d`/`display` commands write non-UCI data to stdout

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:58-64`
- **Expected:** UCI defines a fixed set of engine-to-GUI messages: `id`, `uciok`, `readyok`, `info`, `bestmove`, `option`, `copyprotection`, `registration`. Other output on stdout is a protocol violation.
- **Actual:** `fen` command: `println!("{}", board.to_fen())` — raw FEN string, no `info string` wrapper. `d`/`display` command: `println!("{}", board)` — raw ASCII board. Both go to stdout.
- **Reasoning:** `cli.rs:58-64`. A GUI that reads only known UCI tokens will see unknown data and potentially hang or misbehave.
- **Failure scenario:** GUI sends `fen` hoping for an error (unknown command) but instead receives a FEN string that it cannot interpret as any UCI message type.
- **Where to look next:** `cli.rs:58-64`.
- **Fix direction:** Wrap in `info string` (valid UCI) or write to stderr (for human debugging only).

---

### UCI-110 — `uci` response advertises no options; `setoption` silently swallowed

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/bin/cli.rs:73-77`
- **Expected:** UCI requires the engine to advertise configurable options (at minimum `Hash` size if it uses a TT) before `uciok`. A GUI uses advertised options to decide what `setoption` to send. `setoption` for an undeclared option should be ignored or logged, but a GUI that assumes `Hash` is supported and sends `setoption name Hash value 256` will be silently ignored.
- **Actual:** `handle_uci()` prints only `id name`, `id author`, `uciok`. No `option name Hash type spin ...` line. `setoption name Hash value 64` is silently accepted (no match arm for `setoption`). The TT is always allocated at 512MB (`cli.rs:19`).
- **Reasoning:** `cli.rs:73-77`: three lines total in `handle_uci()`. `cli.rs:19`: `TranspositionTable::new(512)` hardcoded.
- **Failure scenario:** Arena GUI sets Hash=128 for low-memory mode; engine uses 512MB, causing OOM on a 1GB system.
- **Fix direction:** Add `option name Hash type spin default 512 min 1 max 2048` to `handle_uci()`. Add a `"setoption"` arm to the main match that reads `name` and `value` and re-initialises the TT.

---

### UCI-111 — `info depth` lines omit `nps` field

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs` (info output lines, not yet read)
- **Expected:** UCI strongly recommends `nps` (nodes per second) in `info` lines for GUI display of engine speed.
- **Actual:** Every `info depth` line observed: `info depth N score cp X nodes Y time Z pv <moves>`. No `nps` field. Example: `info depth 8 score cp 7 nodes 114565 time 133 pv d2d4`.
- **Reasoning:** All 25+ info lines captured across runs show the same four fields. NPS can be computed as `nodes/time*1000` but the GUI cannot display it without the field.
- **Where to look next:** Search for `println!("info` in `search.rs`.
- **Fix direction:** Add `nps {}` to the info format: `nodes * 1000 / time_ms.max(1)`.

---

## Confirmation of static CLI agent's five findings

| Static finding | UCI-ID | Verdict | Evidence |
|---|---|---|---|
| `stop` not implemented | UCI-102 | **CONFIRMED** | No "stop" arm in `cli.rs:43-65`; `go infinite\nstop\nquit` still searched to depth 11 in 4s |
| Search runs synchronously, blocks input loop | UCI-102 | **CONFIRMED** | `cli.rs:55-57` calls `handle_go()` inline; stdin not read while searching |
| `go infinite` cannot be interrupted | UCI-102/104 | **CONFIRMED** | 3s timeout killed process at exit 124; no `bestmove` emitted |
| `go nodes` silently ignored, `node_limit` always `None` | UCI-103 | **CONFIRMED** | `cli.rs:310` literal `node_limit: None`; `go nodes 1000` searched 33M nodes in 35s |
| `info string` book messages emitted before `uci` received | UCI-101 | **CONFIRMED** | `printf 'quit\n' \| ./vantage-cli.exe` → `info string No opening book found...` on stdout, exit 0 |

---

## Clean bill

- **Handshake sequence** (`id name`, `id author`, `uciok` order): correct. `readyok` in response to `isready` is correct.
- **Basic search completeness**: `info depth N` emitted for each iteration 1..depth; exactly one `bestmove` emitted; move `e2e4` is legal from startpos. PASS.
- **Move parsing**: `position startpos moves e2e4 e7e5 g1f3 b8c6 f1b5` correctly produces Ruy Lopez FEN `r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3`. PASS.
- **FEN loading**: kiwipete and ep-square FENs parsed and searched without crash. PASS.
- **`go movetime 500`**: returned in 545ms (9% over); depth-9 search used 323ms then stopped. Acceptable.
- **`go wtime 1000`**: allocated 42ms target, returned well inside budget. PASS.
- **`ucinewgame`**: resets board (`Board::new()`) and clears TT (`tt.clear()`) — `cli.rs:46-49`. Post-newgame node counts identical to fresh process, confirming TT is cleared. PASS.
- **Illegal move handling**: `Invalid move: e2e5` goes to **stderr** (`cli.rs:104: eprintln!`), not stdout. Correct. Engine falls back to previous valid position and searches from there.
- **Junk input**: 10000-char line silently ignored; empty line skipped (`cli.rs:36-38`). PASS.
- **EOF without `quit`**: clean exit code 0. PASS.
- **`quit` as first command**: clean exit code 0. PASS.

---

## Scenario table

| ID | Input | Observed (trimmed) | Expected | Verdict | Exit | Time |
|---|---|---|---|---|---|---|
| UCI-101 | `quit` only | `info string No opening book found...` | silence until `uci` | FAIL | 0 | <100ms |
| UCI-102 | `uci\nisready\nquit` | `info string...` then `id name...id author...uciok\nreadyok` | same minus pre-uci info | WARN | 0 | <200ms |
| UCI-103 | `go depth 6` startpos | `info depth 1..6`, `bestmove e2e4` | same | PASS | 0 | 17ms |
| UCI-104 | `go depth 8` startpos | `info depth 1..8`, `bestmove d2d4`, PV plausible | same | PASS | 0 | 133ms |
| UCI-105 | Ruy Lopez position + `d` | correct FEN on stdout (non-UCI) | not a UCI response | WARN | 0 | 22ms |
| UCI-106 | kiwipete FEN + `go depth 6` | `info depth 1..6`, `bestmove d5e6` | same | PASS | 0 | 26ms |
| UCI-107 | ep FEN + `go depth 6` | `bestmove d2d4` (not e5f6, but legal) | legal move | PASS | 0 | 12ms |
| UCI-108 | `go movetime 500` | returns in ~545ms | ~500ms | PASS | 0 | 545ms |
| UCI-109 | `go movetime 0` | `bestmove 0000` | depth-1 bestmove | FAIL | 0 | <200ms |
| UCI-110 | `go wtime 1000 btime 1000` | target 42ms, returned in budget | same | PASS | 0 | 254ms |
| UCI-111 | `go nodes 1000` | searched 33M nodes (never returned) | ~1000 nodes, bestmove | FAIL | timeout | 60s+ |
| UCI-112 | `go infinite` (3s timeout) | depth 10 reached, killed | (stop would terminate) | FAIL | 124 | 3s |
| UCI-113 | `go infinite\nstop\nquit` | depth 11 reached, killed at 5s | bestmove on stop | FAIL | 124 | 5s |
| UCI-114 | `go depth abc` | searched to depth 14 (never returned) | bestmove at reasonable depth | FAIL | timeout | 35s+ |
| UCI-115 | `position fen garbage` | silently ignored, searches startpos | graceful | PASS | 0 | <200ms |
| UCI-116 | `position fen 8/8/.../8 w - - 0 1 + go depth 3` | **PANIC** exit 101 | graceful error | FAIL P0 | 101 | <100ms |
| UCI-117 | `moves e2e5` (illegal) | "Invalid move" on stderr, searches prev pos | same | PASS | 0 | <200ms |
| UCI-118 | `moves e2e4 e2e4` | second move invalid on stderr | same | PASS | 0 | <200ms |
| UCI-119 | 10000-char junk | ignored | ignored | PASS | 0 | <200ms |
| UCI-120 | EOF without quit | clean exit | clean exit | PASS | 0 | <200ms |
| UCI-121 | `ucinewgame` + search x2 | TT cleared, both return same moves/nodes | same | PASS | 0 | ~50ms |
| UCI-122 | `setoption name Hash value 64` | silently accepted | silently accepted | PASS | 0 | <100ms |
| UCI-123 | `go depth -1` | `bestmove 0000` | legal bestmove | FAIL | 0 | <200ms |
| UCI-124 | `go depth 0` | `bestmove 0000` | legal bestmove | FAIL | 0 | <200ms |

---

## Incomplete implementations

| Intent source | What it promises | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `MCP_BUILD_PLAN.md` Phase 2 | `legal_moves` tool in MCP server | `src/bin/vantage-mcp.rs` does not exist; `src/analysis/mod.rs` exists (evaluate + analyze_move only) | MCP server cannot answer legal-move queries | Yes (Phase 2 blocked) |
| `vantage-engine-pass.md:51` | "ucinewgame resets all state" | `ucinewgame` resets board + TT; search context is stack-allocated, reset per call | Fulfilled for the defined state | No |
| `CLAUDE.md` (MCP server section) | "stdio server must never print to stdout except protocol JSON-RPC" | Engine prints `info string Target time:`, book status, and FEN/board to stdout | Future MCP wrapper will mix engine diagnostics with protocol JSON | Yes (when MCP wraps CLI) |
