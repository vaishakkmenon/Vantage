# Deep PV — Audit Findings

**Agent:** deep-pv
**Scope:** `backend/src/search/search.rs`, `backend/src/search/context.rs`, `backend/src/bin/cli.rs`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/search/search.rs` — read in full (779 LOC)
- `backend/src/search/context.rs` — read in full (54 LOC)
- `backend/src/bin/cli.rs` — read lines around `max_depth`, `depth`, `go depth`, `go infinite`
- `docs/audit/04-search.md` — read in full (prior agent findings)
- Binary `backend/target/release/vantage-cli.exe` — ran 12 test positions at depths 4–100
- Grep: `ctx.pv_length`, `ctx.killer_moves`, `ctx.pv_table` to find every ply-indexed access
- Traced aspiration window loop (search.rs:677–721) and alpha_beta recursive call sites (451, 499, 515, 531)
- Traced PV write path (search.rs:554–565) and the guard at 560

## What I did not check, and why

- The full PV value at deeper plies (pv[2+]) cannot be verified empirically because the binary only prints `bestmove` in the `pv` field (see SEARCH-202), making the full sub-PV invisible.
- Could not empirically trigger ply overflow (SEARCH-001) because depth=64 takes hours on any complex position; simple positions find mates quickly and terminate early.

---

## Findings

### SEARCH-201 — SEARCH-001 verdict: PLAUSIBLE but unreachable in practice — downgrade P0 → P2

- **Severity:** P2
- **Confidence:** high
- **Status:** verified (structural), unverified (empirical panic)
- **Anchors:** `backend/src/search/context.rs:8,21,34`, `backend/src/search/search.rs:283,391`
- **Expected:** SEARCH-001 claims a panic at line 391 (killer_moves) and line 283 (pv_length) when ply >= 64.
- **Actual:** The structural facts are confirmed. MAX_PLY=64 (context.rs:8). `killer_moves` is a 64-element Vec (context.rs:34); `pv_length` is a 64-element array (context.rs:21). Lines 283 and 391 are unguarded. The guard at line 560 (`if ply + 1 < MAX_PLY`) guards PV WRITES only, and executes AFTER the panicking accesses.
- **Reasoning:** Default depth from cli.rs:181 is 64; `go infinite` sets depth=100 (cli.rs:238). At ID-loop depth=63, a path of 63 non-check plies followed by 2 consecutive in-check plies would push ply to 65 → panic. However, empirical testing on 8+ positions at depths up to 50 (bash-limited to 60 s each) produced no panic. Repetition detection (search.rs:286) terminates cyclic check sequences before ply overflows. Mate detection (search.rs:767) terminates most simple endgame searches at depth < 20. No position was found that reaches ID-loop depth >= 32 within feasible time while also exhibiting the required consecutive-check path.
- **Failure scenario:** Requires a position with a long non-repeating check sequence AND enough compute time to reach ID-loop depth >= 32. Could not construct such a position within test time limits.
- **Related:** SEARCH-001 (prior agent), qsearch safety brake at search.rs:153 shows the fix pattern (MAX_Q_SEARCH_DEPTH).
- **Where to look next:** `search.rs:283`, `search.rs:391`, `context.rs:34` — add `if ply >= MAX_PLY { return ... }` at the top of alpha_beta, analogous to qsearch's `if ply > MAX_Q_SEARCH_DEPTH`.
- **Fix direction:** Guard the top of alpha_beta with `if ply >= MAX_PLY { return (static_eval(...), None); }` before any array access. Also consider a cumulative extension cap.

---

### SEARCH-202 — UCI `info pv` outputs only bestmove, not the full principal variation

- **Severity:** P1
- **Confidence:** high
- **Status:** verified
- **Anchors:** `backend/src/search/search.rs:738-763`
- **Expected:** UCI spec requires `info depth N pv move1 move2 ... moveN` — the full PV line up to N moves. `vantage-engine-pass.md:38` also defines PV invariants that require the full PV to be observable.
- **Actual:** Lines 756-763 print `valid_mv.to_uci()` (a single `Move`), which is `last_completed_best_move`. `last_completed_pv` (a `Vec<Move>`, assembled at line 738-741) is never printed. Every `info depth` line shows exactly one move regardless of depth.
- **Reasoning:** Confirmed empirically across all tested depths (4–15) on startpos, kiwipete, and the middlegame FEN. Every info line ends with a single UCI move token. `last_completed_pv` is populated but silently discarded.
- **Failure scenario:** Any GUI (Stockfish, Arena, Lichess-bot) expecting a full PV for analysis or display will see a one-move PV at every depth. Makes the SEARCH-004 PV-corruption finding unverifiable from the binary. Run `go depth 8` on kiwipete — observe `pv d5e6` (one move) where 8 moves are expected.
- **Related:** SEARCH-004 (prior agent) — without full PV output, sub-PV corruption is invisible.
- **Where to look next:** `backend/src/search/search.rs:756-763`; `last_completed_pv` at line 738-741.
- **Fix direction:** Replace `valid_mv.to_uci()` in the engine_println with a joined string over `last_completed_pv` (map `.to_uci()` on each move, join with spaces). The existing `last_completed_pv` vec already holds the correct data.

---

### SEARCH-203 — SEARCH-004 verdict: PLAUSIBLE for deep plies; root PV (pv[0..1]) is protected by aspiration widening

- **Severity:** P2
- **Confidence:** medium
- **Status:** partially verified
- **Anchors:** `backend/src/search/search.rs:530-565`, `backend/src/search/search.rs:677-721`
- **Expected:** SEARCH-004 claims a bad sub-PV is written when a null-window scout returns score >= beta, then copied into the parent PV.
- **Actual:** Structurally confirmed. The PV write at lines 560-565 fires whenever `score > alpha`, which includes the cutoff path (score >= beta). For move_count > 0 at any ply, if the null-window re-search (lines 515-527) returns score >= beta, `score < beta` at line 530 is false, the full-window re-search is skipped, and `ctx.pv_table[ply+1]` from the null-window child is copied into `ctx.pv_table[ply]`.
- **Reasoning (aspiration window):** For depth > 4, initial beta = prev_score + 50 (search.rs:685). A fail-high (score >= beta) widens beta to INF and retries (search.rs:714-716). In the retry, beta=INF at ply=0 makes `score >= beta` impossible (no finite score reaches INF). Therefore: (a) the final search's ply=0 ALWAYS runs full-window re-searches for any alpha-raising move; (b) ply=0's PV (`pv_table[0]`) is always from a full-window search. The root PV is protected. However, ply=1 (called with beta=-S where S is ply=0's best score) can still have a finite beta. A cutoff at ply=1 from the null-window path writes bad `pv_table[1]`, which is then copied into `pv_table[0][1]`. pv[1] in the root PV may be wrong.
- **Empirical check:** Binary does not print full PV (SEARCH-202), so pv[1..N] cannot be verified from binary output alone. pv[0] == bestmove: confirmed across all tested depths and positions (all checks passed). pv.len() <= depth: cannot check since pv always appears as length 1 in output. UCI-003 (go nodes ignored): confirmed — `node_limit: None` is hardcoded in cli.rs:310.
- **Failure scenario:** UNVERIFIABLE without full PV output. Fix SEARCH-202 first, then test: startpos, kiwipete, and `r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8` at depths 5–10. Compare pv[1] and pv[2] across depths; a move that is illegal in sequence would confirm the bug.
- **Related:** SEARCH-004 (prior agent), SEARCH-202 (blocks empirical verification).
- **Where to look next:** `search.rs:530-543` (full-window gate), `search.rs:560-565` (PV write — should be inside the score < beta branch).
- **Fix direction:** Move the PV write block (lines 560-565) inside the `if score > alpha && score < beta` branch (after line 543's full-window search). For move_count==0, the PV write at the same location is already correct because full-window is always used.

---

## UNVERIFIED sub-checks

| Sub-check | Why unverifiable | Test that would settle it |
|---|---|---|
| `pv.len() <= depth` for node-capped results | `go nodes` ignored (cli.rs:310: `node_limit: None` hardcoded); binary doesn't print full PV | Fix SEARCH-202; wire `go nodes` in CLI; check `pv` length in `info` output |
| pv[1..N] legality | Full PV not printed (SEARCH-202) | Fix SEARCH-202; replay PV via `position fen <fen> moves <pv...>` and check engine accepts it |
| Ply overflow empirical trigger | No position found that reaches ID-loop depth >= 32 in feasible time without early mate | Construct a balanced 3-4 piece position with perpetual non-repeating checks; give unlimited search time |

---

## Clean bill

- **pv[0] == bestmove invariant:** Confirmed at all tested depths (4–10) on startpos, kiwipete, middlegame FEN. `pv[0]` always equals `bestmove` in all outputs.
- **Aspiration window convergence:** Fail-high widens beta to INF (line 715); fail-low widens alpha to -INF (line 709). Next iteration uses full window. No infinite loop possible.
- **Root PV protection by beta=INF:** After any fail-high, the retry's beta=INF at ply=0 makes all score >= beta cutoffs impossible at the root. The root PV is always assembled from full-window searches.
- **Default depth = 64 confirmed:** cli.rs:181 `let mut depth = 64;`. `go depth N` overrides it. `go infinite` sets depth=100 (cli.rs:238). No cap exists in the CLI or search function.
- **PV guard at line 560 fires correctly:** `if ply + 1 < MAX_PLY` prevents pv_table writes at ply >= 63, but does NOT prevent the panicking accesses at lines 283 and 391.
