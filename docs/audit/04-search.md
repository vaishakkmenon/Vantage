# Search — Audit Findings

**Agent:** audit-search
**Scope:** `backend/src/search/search.rs` (779 LOC), `backend/src/search/context.rs` (54 LOC)
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/search/search.rs` — read in full twice
- `backend/src/search/context.rs` — read in full
- `C:/Users/vaish/Personal/Other/Coding/Vantage/vantage-engine-pass.md` — read in full (65 lines)
- `C:/Users/vaish/Personal/Other/Coding/Vantage/VANTAGE_ROADMAP.md` — lines 1–80 read
- `backend/tests/determinism_tests.rs` — read in full (264 lines, 16 tests)
- `backend/tests/nmp_tests.rs` — read (first 60 lines, intent check)
- `backend/tests/lmr_tests.rs` — read (first 30 lines, intent check)
- Grep: `unwrap\(\)|expect\(|panic!` across `backend/src/search/` — all hits read
- Glob: all `*.rs` under `backend/tests/` — enumerated, intent checked from names and partial reads
- Traced the full PVS/PV-table write path: lines 445–565 read and annotated
- Traced node-cap / aspiration-window / ID loop: lines 627–779 read and annotated

## What I did not check, and why

- `backend/tests/iterative_deepening_tests.rs`, `search_tests.rs`, `aspiration_tests.rs`, `tactical_tests.rs`, `quiescence_diagnostics_tests.rs` — only their names were enumerated; full reads would duplicate effort after reading the determinism tests, which cover all six harness goals. Intent discerned from test names.
- `backend/src/search/picker.rs`, `eval.rs`, `tt.rs`, `see.rs` — read only the lines surrounding `unwrap`/`expect` hits; full reads are out of scope.
- Could not run `cargo test` or `cargo build` — read-only audit.

---

## Findings

### SEARCH-001 — Ply overflow: unbounded check extension causes out-of-bounds panic

- **Severity:** P0
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs:323-325`, `backend/src/search/search.rs:391`, `backend/src/search/search.rs:283`, `backend/src/search/context.rs:8`, `backend/src/search/context.rs:34`
- **Expected:** `ply` is bounded to at most `MAX_PLY-1` (63) at all times, since `ctx.killer_moves`, `ctx.pv_length`, and `ctx.pv_table` are all 64-entry arrays. The engine should degrade gracefully under deep check sequences rather than panic.
- **Actual:** Check extension (`extension = 1 if in_check_now`) is applied at `search.rs:325` and added to the recursive call's depth: `depth - 1 + extension` at lines 451 and 536. There is no cumulative extension cap and no ply-cap check. `ctx.killer_moves` is a `Vec` of 64 elements (context.rs:34); accessing it at `search.rs:391` with `ply >= 64` panics. `ctx.pv_length[ply]` (64-element array) at line 283 and `ctx.pv_table[ply]` (64-row array) at line 561 also panic if `ply >= 64`.
- **Reasoning:** `alpha_beta` is called with `ply + 1` at each recursive level (lines 451, 509, 521, 537, 374). With `limits.max_depth` up to 64 and one check extension per in-check position, the recursive depth can reach 64+ plies before hitting depth <= 0. The quiescence brake at `search.rs:153` applies only to `quiescence()`; `alpha_beta` has no analogous ply cap. The condition `if depth <= 0 && !in_check_now` (line 327) defers to qsearch, but when in check, `alpha_beta` recurses at depth=1 with extension=1 → next call is again depth=1 if in check again. In a long check sequence (e.g., perpetual check with 64+ half-moves before repetition detected), `ply` grows past 63.
- **Failure scenario:** FEN `r2q1rk1/ppp2ppp/2n5/3p4/1b1P4/2NB4/PPP2PPP/R1BQR1K1 w - - 0 1` searched at `max_depth=20` in a position where the engine explores a 65-ply forced-check sequence. `ctx.killer_moves[64]` is accessed → Rust panic (index out of bounds). Same panic at `ctx.pv_length[64]` or `ctx.pv_table[64]`. Any position with a long tactical check sequence at high depth triggers this.
- **Related:** No other finding; but the qsearch safety brake at `search.rs:153` (MAX_Q_SEARCH_DEPTH=100) shows awareness of unbounded recursion — the same fix was not applied to `alpha_beta`.
- **Where to look next:** `backend/src/search/search.rs:391` (killer_moves index), `search.rs:283` (pv_length index), `search.rs:560-563` (pv_table index), `context.rs:8` (MAX_PLY constant).
- **Fix direction:** Guard every `ctx.*[ply]` access with `ply < MAX_PLY`. Add a ply-cap branch at the top of `alpha_beta` (analogous to `MAX_Q_SEARCH_DEPTH` in qsearch): if `ply >= MAX_PLY`, return `(static_eval(...), None)`. Also consider tracking cumulative extensions and capping them (e.g., no more than `depth / 2` total extensions), which is standard practice.

---

### SEARCH-002 — History heuristic double-squares depth: `bonus = depth²` passed to a function that squares its argument again

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs:582-583`, `backend/src/search/context.rs:50-52`
- **Expected:** History bonus for a beta-cutoff quiet move at depth `d` should scale as `d²` (or some monotone function of `d`), giving deeper cutoffs higher weight. `update_history` was written to accept a raw `depth` and compute the bonus internally.
- **Actual:** `search.rs:582` computes `let bonus = depth * depth;` and then calls `ctx.update_history(mv, bonus)`. Inside `context.rs:51`, `update_history` treats its parameter as `depth` and computes `let bonus = (depth * depth).min(400)`. The actual increment added to `history` is therefore `((d²)²).min(400) = d⁴.min(400)`. For `d >= 5`, this is always 400; for `d=3`, it is 81; for `d=2`, 16; for `d=1`, 1. The difference between a depth-5 and depth-10 cutoff (both capped at 400) is erased.
- **Reasoning:** The parameter name `depth` in `update_history`'s signature makes it clear the function expects the raw depth. The caller pre-computes `depth * depth` and passes it as if it were a bonus value, but the function squares it again. One of the two squarings is wrong. The history table saturates at 400 for all depths >= 5, losing ordering signal that would distinguish "this move cut off a shallow search" from "this move cut off a deep search."
- **Failure scenario:** Positions where depth-10 cutoffs should heavily outweigh depth-2 cutoffs in move ordering. Both get bonus 400. Late-move reductions may not reduce the right moves because history scores are flat for high depths. No panic; just weaker move ordering than intended. Effect is visible in any position requiring accurate history-based ordering at depths > 5.
- **Related:** None.
- **Where to look next:** `backend/src/search/search.rs:582-583`, `backend/src/search/context.rs:50-53`.
- **Fix direction:** Either (a) change the call site to `ctx.update_history(mv, depth)` and let the function compute the square internally, or (b) change `update_history` to just add its argument directly (`self.history[...] += depth`) and keep the call site computing the bonus. Pick one convention and apply it consistently.

---

### SEARCH-003 — Null-move pruning runs at PV nodes; zugzwang risk and missed mates

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs:352-388`
- **Expected:** NMP is guarded so it does not run at PV nodes (`beta - alpha > 1`). Standard practice (Stockfish, Crafty) skips NMP at PV nodes to prevent false beta cutoffs from masking forced mates. The roadmap (VANTAGE_ROADMAP.md:15) lists NMP as implemented but does not call out this deviation.
- **Actual:** `search.rs:354` has the comment `// REMOVED: && (beta - alpha == 1) <--- Unlocks NMP for PV nodes`. NMP runs at any node including PV nodes (open window) as long as `depth >= 4`, not in check, `has_major_pieces`, and `static_eval >= beta`. When the static eval happens to exceed `beta` at a PV node (which is the expected alpha-beta value, not a speculative one), NMP returns early with `beta`, potentially cutting off a variation that actually contains a forced mate.
- **Reasoning:** At a PV node the window is open. `static_eval_val >= beta` can be satisfied if the eval function overestimates a dynamic position. NMP then returns `(beta, None)` at line 385, discarding all remaining moves. If the actual best move was a quiet forcing sequence, it is never searched. NMP mates (`score >= MATE_THRESHOLD`) are correctly skipped (line 382), but non-mate forced wins can be missed.
- **Failure scenario:** Position: `8/8/8/3k4/8/8/3K4/3Q4 w - - 0 1` (white queen vs. black king, white to move, forced mate in a few). Static eval is extremely high (> beta at a PV node). NMP fires, returns `(beta, None)`. The engine fails to find the shorter mate and may report a longer one or an incorrect PV from a prior depth.
- **Related:** None.
- **Where to look next:** `backend/src/search/search.rs:352-388`.
- **Fix direction:** Restore the PV-node guard: add `&& (beta - alpha == 1)` to the NMP condition. NMP still runs at cut nodes (null-window) where it is most beneficial. The comment claims removing it is "a massive speedup" — measure with SPRT before deciding, as correctness matters more.

---

### SEARCH-004 — PVS null-window scout writes sub-PV before beta cutoff at non-root nodes (spec violation)

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs:499-543`, `backend/src/search/search.rs:554-565`
- **Expected:** `vantage-engine-pass.md:35` states: "only update the PV on a full-window result (a true PV node), never on a null-window scout search — otherwise the PV comes out truncated or wrong." The PV-table write at lines 561-565 must be guarded so it executes only after a full-window search completes.
- **Actual:** Lines 554-565 write `ctx.pv_table[ply]` and set `ctx.pv_length[ply]` whenever `score > best_score && score > alpha`. For `move_count > 0`, the sequence is: (1) null-window LMR scout (lines 499-510); (2) if `score > alpha && r > 0`, a null-window re-search at full depth (lines 515-527); (3) only if `score > alpha && score < beta`, a full-window re-search (lines 531-543). The PV write at 561-565 fires after step (3) if full-window ran, but also after step (1) or (2) if `score >= beta` (which skips step 3). In the `score >= beta` case, `ctx.pv_table[ply+1]` contains the null-window child's partial PV line, which is then copied into `ctx.pv_table[ply]`.
- **Reasoning:** At ply=0 with beta=INF, `score >= beta` is impossible (scores are bounded by MATE_SCORE=31000 < INF=32000). So the root PV (`pv_table[0]`) is protected. However, at intermediate plies (1, 2, …), a null-window scout can return `score >= beta` (the child's beta). In that case, `pv_table[ply][1..]` is overwritten with the null-window child's continuation — which is truncated (LMR depth) or wrong (null-window doesn't fully explore the line). The parent then propagates this contaminated sub-PV upward if it also raises alpha. The chain: if ply=1's null-window causes a cutoff, ply=0 gets a cutoff too and doesn't propagate. But at PV nodes with open windows, intermediate plies can corrupt the PV continuation that ultimately reaches ply=0.
- **Failure scenario:** `r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8` at depth=7. At some intermediate PV node (ply=2), move_count=1's LMR scout (depth=4, null-window) raises alpha, score < beta → full-window runs → PV correct. But if the LMR re-search at lines 515-527 returns score >= parent_beta, the PV at ply=2 comes from the LMR re-search (null-window, full-depth), which may point to a different best continuation than the full-window would have. Reported PV from ply=2 onward is wrong; `pv[0] == best_move` invariant holds, but `pv[2..4]` may be illegal or belong to a different variation.
- **Related:** SEARCH-001 (both relate to missing guards in the PV assembly path).
- **Where to look next:** `backend/src/search/search.rs:530-543` (the full-window condition), `backend/src/search/search.rs:554-565` (PV write).
- **Fix direction:** Move the PV-table write (lines 561-565) inside the `if score > alpha && score < beta` branch (after the full-window re-search), not in the outer `if score > best_score` block. For the `move_count == 0` (full-window) path, the write correctly belongs inside `if score > alpha`. Only the `move_count > 0` path needs the guard `score < beta`.

---

### SEARCH-005 — VANTAGE_ROADMAP.md claims PVS is future Phase 1.1 work; code already has PVS

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `VANTAGE_ROADMAP.md:32-36`, `backend/src/search/search.rs:459`, `backend/src/search/search.rs:499-543`
- **Expected:** `VANTAGE_ROADMAP.md:32-36` lists "Principal Variation Search (PVS)" under Phase 1 as work to be done, implying it is not yet implemented.
- **Actual:** The code at `search.rs:459` searches the first move with a full window, and subsequent moves (lines 499-543) use null-window scouts with full-window re-searches on fail-high. This is the PVS algorithm. It was implemented (commits `bf5ad05`/`aa8d27a` per the brief) but the roadmap was not updated.
- **Reasoning:** The roadmap entry for Phase 1.1 is stale documentation. No code change needed; update the roadmap.
- **Failure scenario:** A developer reads the roadmap, believes PVS is not implemented, and re-implements it, potentially doubling the null-window path or introducing a regression.
- **Related:** Known-context item 8 (analysis module docs are stale); this is a pattern of documentation falling behind code.
- **Where to look next:** `VANTAGE_ROADMAP.md:32-36`.
- **Fix direction:** Mark Phase 1.1 as implemented in the roadmap. Add a note that the PVS-at-PV-nodes NMP guard was intentionally removed (per SEARCH-003).

---

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `vantage-engine-pass.md:38` | "pv.len() <= depth" invariant tested | `determinism_tests.rs:33,50,69` asserts `result.pv.len() <= 5` (depth=5) but no test checks `pv.len() <= depth` for node-capped results where `depth` < 5 | Node-capped PV could return `pv.len() > result.depth` if SEARCH-004 strikes during cap | No |
| `vantage-engine-pass.md:25` | "Define the degenerate case (cap below depth-1 completion)" | Depth-1 runs with `node_limit: None` (search.rs:661); degenerate case returns depth-0 (`score=0, best_move=None, pv=[]`) if time runs out during depth-1 (possible for time-limited paths); no test covers time limit expiry during depth-1 | Caller receives `best_move: None`; UCI `go` handler must handle this | No |

---

## Clean bill

- **Node-cap semantics (spec lines 25-28):** `search.rs:661` correctly disables node limit at depth=1 (depth-1 always completes). `search.rs:663-668` checks the accumulated count before starting deeper iterations. `search.rs:729-731` discards a partial depth's result on stop-signal. `last_completed_pv` is only updated after the stop-signal check (line 738). This matches the spec exactly.
- **TT mate score adjustment:** `score_to_tt` (line 51) adds `ply` for mate scores; `score_from_tt` (line 61) subtracts `ply`. Both directions are symmetric and correct — mates are stored as distance-from-root and retrieved as distance-from-current-node.
- **Repetition detection:** `search.rs:286` correctly gates repetition on `ply > 0`. Root is never scored as a draw via this path.
- **Aspiration window termination:** Fail-low sets `alpha = -INF`; fail-high sets `beta = INF`. On the next iteration the window is full and cannot fail low/high again (all scores fall within [-INF, INF]). No infinite loop.
- **`pv[0] == best_move` invariant:** Guaranteed at `search.rs:561` (`pv_table[ply][0] = Some(mv)`) before copying child PV. The first entry is always the current best move.
- **`see.rs:85,122` unwraps:** Both are guarded: line 85 is reached only after a legal move from the current position (piece must exist at `m.from`); line 122 is reached only after `get_lva_square` returns a square != 64, meaning a piece exists there. Reachability of `None` would require corrupted board state.
- **Qsearch stack overflow guard:** `search.rs:153` caps qsearch at `MAX_Q_SEARCH_DEPTH=100` plies, returning static eval. This prevents unbounded recursion inside qsearch.
- **LMP correctly disabled at PV nodes:** `search.rs:430` guards LMP with `alpha == original_alpha`. Once any move raises alpha (PV node), LMP is disabled for that node. Correct.
- **Futility pruning near-mate safety:** The condition `static_eval_val + margin <= alpha` is safe near mate scores: if `alpha = -28000` and margin = 700, the threshold is -27300, which is > alpha, so the condition is false and no move is pruned. Futility correctly does not prune in lost positions where alpha is very negative.
- **Delta pruning exemptions:** `search.rs:210,217` correctly skip delta pruning for promotions (`is_prom`) and en passant (`is_ep`). Only standard captures are pruned.
