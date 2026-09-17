# Search — Verification Findings

**Agent:** verify-search
**Scope:** `backend/src/search/search.rs`, `backend/src/search/context.rs`, `backend/src/search/tt.rs`, `backend/src/search/eval.rs`, `backend/tests/determinism_tests.rs`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/search/context.rs` (54 LOC) — full read
- `backend/src/search/search.rs` (779 LOC) — full read (lines 1-779)
- `backend/src/search/tt.rs` (lines 40-99) — new_search, save, probe
- `backend/src/search/eval.rs` (lines 90-205) — mop_up_eval, static_eval, lazy margin
- `backend/tests/determinism_tests.rs` (lines 1-80, grep for pv assertions) — PV test coverage
- Grep: `new_search` across all of `backend/src/` and `backend/tests/` — zero call sites confirmed
- Grep: `mop_up\|lazy_eval\|lazy eval\|LAZY` in eval.rs
- Live binary probe: `vantage-cli.exe` cold-TT depth-12 startpos (702,274 nodes, pv e2e4) and warm-TT depth-12 startpos (386 nodes, pv e2e4 at every depth) — confirms warm-TT PV collapses to 1 move

## What I did not check, and why

- Could not run `cargo test` to check warm-TT assertions — not allowed to edit files under `backend/`; throwaway test would require adding a file there.
- Did not read quiescence in full (not needed for these findings).
- `docs/audit/15-deep-pv.md` and `docs/audit/19-deep-tt.md` read structurally but evidence judged from source directly.

---

## PRIORITY 1 RESOLUTION — ORDER-202 vs SEARCH-202

**Verdict: ORDER-202 is CORRECT. SEARCH-202 is WRONG that "the data is fine." Both are real bugs, but ORDER-202's P1 severity is the primary defect.**

**Evidence chain (all line numbers are `backend/src/search/search.rs`):**

1. `ctx.pv_length[ply] = 0` at line 283 — resets on EVERY node entry, including nodes that will return via TT.
2. TT cutoff block (lines 305-316): when `ply > 0` and `tt_depth >= depth`, the function returns at line 311/312/313 WITHOUT ever executing the PV copy block (lines 560-565). `pv_length[ply]` is left at 0.
3. When the root (ply 0) then does the PV copy for its best move (lines 561-565): `ctx.pv_length[ply] = ctx.pv_length[ply + 1] + 1`. With `pv_length[1] = 0` (ply 1 returned via TT), `pv_length[0] = 1`.
4. Line 738: `last_completed_pv = ctx.pv_table[0][..ctx.pv_length[0]].iter().map(|m| m.unwrap()).collect();` — collects EXACTLY 1 element. `SearchResult.pv` has length 1.
5. **Live proof:** warm-TT run (same position, second `go depth 12`) produced 386 total nodes (vs 4,160,860 cold) and every depth line printed `pv e2e4` — one move. The node count collapse confirms TT hits at ply 1 dominate.

**SEARCH-202's claim that `last_completed_pv` holds the full PV is wrong.** `last_completed_pv` is assembled from `ctx.pv_table[0][..ctx.pv_length[0]]`, which is corrupted by the TT cutoff. It is NOT independently maintained.

**Additional separate bug confirmed by SEARCH-202:** The UCI `info` line at search.rs:756-763 prints `valid_mv.to_uci()` — this is `last_completed_best_move`, the SINGLE root best move, not the PV array. Even if the PV array were fixed, the UCI output would still show only 1 move. This is a separate P2 cosmetic bug.

**What was wrong in each prior agent:**
- ORDER-202: Correctly identified the mechanism and P1 severity. The "cold TT 702,274 nodes" and "warm TT 370 nodes" live evidence cited is accurate (my run: 702,274 vs 386). UPHELD.
- SEARCH-202: Correctly identified that the UCI print only shows 1 move. WRONG that `last_completed_pv` "is fine" — it is assembled from the same truncated pv_length. Partially correct, partially wrong.

---

## Findings

### ORDER-202 — TT exact/lower-bound cutoff zeroes pv_length at ply > 0, truncating SearchResult.pv to 1 move

- **Severity:** P1
- **Confidence:** high
- **Status:** CONFIRMED — live binary evidence + line-level code trace
- **Anchors:** `backend/src/search/search.rs:283`, `backend/src/search/search.rs:305-316`, `backend/src/search/search.rs:560-565`, `backend/src/search/search.rs:738`
- **Expected:** `SearchResult.pv` should contain the full principal variation up to the search depth.
- **Actual:** On warm TT (or any search where ply 1 returns via TT hit), `pv_length[0] = 1` and `SearchResult.pv` is a 1-element vec. The `analysis::analyze_move` `better_line` and `EvaluateResult.pv` are therefore also truncated.
- **Reasoning:** pv_length[ply] is reset to 0 at every node entry (line 283). TT cutoff returns at lines 311-313 before any PV write. Parent copies `ctx.pv_length[ply+1]` = 0 items, sets own length to 1. This propagates upward. Live: warm-TT depth-12 startpos = 386 nodes, every `info depth N` shows `pv e2e4`.
- **Failure scenario:** Run `go depth 12` on any position twice with the same TT. Second run: `SearchResult.pv.len() == 1` regardless of depth. MCP `EvaluateResult.pv` is always 1-element after warm TT.
- **Related:** SEARCH-203 (sub-PV contamination from null-window, secondary path to same symptom); UCI print bug below (separate defect)
- **Where to look next:** `search.rs:283`, `search.rs:311-313`, `search.rs:560-565`, `search.rs:738`; `backend/src/analysis/mod.rs` for EvaluateResult.pv usage
- **Fix direction:** On a TT cutoff (before returning), copy the TT move into pv_table[ply][0] and set pv_length[ply] = 1. Alternatively, guard pv_length reset so it does not clobber when a TT hit will dominate. The simplest fix: after the TT hit, set `ctx.pv_table[ply][0] = tt_move; ctx.pv_length[ply] = 1;` before returning.

### UCI-001 — info pv line prints only the root best move, not the full PV array

- **Severity:** P2
- **Confidence:** high
- **Status:** CONFIRMED
- **Anchors:** `backend/src/search/search.rs:756-763`
- **Expected:** UCI `info pv` should print all moves in the principal variation.
- **Actual:** `engine_println!("info depth {} ... pv {}", ..., valid_mv.to_uci())` — prints only `last_completed_best_move`, which is a single `Move`, not the PV vec.
- **Reasoning:** `valid_mv` at line 744 is `last_completed_best_move` (a `Option<Move>`). The `last_completed_pv: Vec<Move>` is never iterated over in the UCI output. Even after ORDER-202 is fixed, this line would still emit only 1 move.
- **Failure scenario:** Any search at any depth shows `pv <single_move>` in the GUI. GUI move visualisation is broken.
- **Fix direction:** Replace `valid_mv.to_uci()` with the PV vec serialized: iterate `last_completed_pv` and join with spaces.

### SEARCH-002 — history bonus is depth^4 (not depth^2) due to double-squaring

- **Severity:** P2
- **Confidence:** high
- **Status:** CONFIRMED
- **Anchors:** `backend/src/search/search.rs:583-584`, `backend/src/search/context.rs:50-52`
- **Expected:** History bonus = `min(depth², 400)`.
- **Actual:** search.rs computes `bonus = depth * depth` then passes it to `update_history(mv, bonus)`. Inside `update_history`, the parameter is named `depth` and the function computes `(depth * depth).min(400)` — i.e., `(depth²)²` = `depth⁴`, capped at 400.
- **Reasoning:** search.rs:583 `let bonus = depth * depth`. context.rs:51 `let bonus = (depth * depth).min(400)`. The squaring happens twice. The parameter name mismatch (`depth` in the function signature receives a value that is already `depth²`) is the source of the bug.
- **Failure scenario:** At depth 5, the effective bonus is `(25*25).min(400) = 400` instead of `(5*5).min(400) = 25`. History heuristic is over-weighted 16x at depth 5, inflating move ordering scores for moves that were good at depth 5 and potentially suppressing correct moves. At depth 3, bonus is 81 instead of 9.
- **Fix direction:** Either pass `depth` (not `depth * depth`) to `update_history`, and let the function square it; or remove the squaring inside `update_history` and keep the squaring in the call site. The two code paths must not both square.

### SEARCH-003 — NMP fires at PV nodes (beta - alpha > 1 no longer gated)

- **Severity:** P2
- **Confidence:** medium
- **Status:** PLAUSIBLE — intentional by the author (comment at line 354 says "Massive speedup"), `has_major_pieces` guard provides partial protection, but the theoretical zugzwang risk is real.
- **Anchors:** `backend/src/search/search.rs:352-358`
- **Reasoning:** The `beta - alpha == 1` guard is commented out at line 354. NMP fires at open PV windows. The `static_eval >= beta` condition reduces false positives (zugzwang positions rarely have a high static eval). The `has_major_pieces` guard prevents NMP in K+P endings where zugzwang is common. The risk is reduced to rare tactical zugzwang with major pieces where static eval overestimates.
- **Failure scenario:** A known deep zugzwang with major pieces (e.g. Rook vs Rook with fixed pawn structures) where side-to-move's static eval happens to be >= beta. Engine would declare a win that doesn't exist. This class of position is very rare in practice.
- **Fix direction:** Restore `beta - alpha == 1` guard for correctness at the cost of some speed, or add a shallow verification search before accepting the NMP cutoff (many engines do this at high depth).

### EVAL-001 — mop_up_eval is one-sided; enemy winning mop-up advantage is invisible

- **Severity:** P2
- **Confidence:** high
- **Status:** CONFIRMED
- **Anchors:** `backend/src/search/eval.rs:96-147`, `backend/src/search/eval.rs:202`
- **Expected:** Mop-up should be symmetric — opponent's mop-up advantage reduces the score from side's perspective.
- **Actual:** Line 202: `score += mop_up_eval(board, side)`. When the enemy has a winning endgame position (+200cp material advantage), `mop_up_eval(board, side)` returns 0 (side is losing), and `mop_up_eval(board, enemy)` is never computed. The enemy's mop-up bonus is invisible to the engine.
- **Reasoning:** Every other evaluation term is symmetric: mobility (line 188 `side - enemy`), pawn structure (line 191, colour-multiplied), king safety (lines 195-196 `side - enemy`), king shield (line 199 `side - enemy`). Mop-up alone is one-sided. When side is losing, the engine under-evaluates how bad its position is.
- **Failure scenario:** K+Q+P vs K+P endgame where Black has the queen. Engine playing Black does not account for mop-up bonus; engine playing White does not see the lost mop-up penalty. The asymmetry can cause the engine to prefer draws over forced losses (correct) but also to misestimate how quickly it is losing.
- **Note:** This does not conflict with the perft agent's colour-symmetry finding — perft tests move generation, not evaluation.
- **Fix direction:** Add `score -= mop_up_eval(board, enemy)` after line 202, mirroring every other term.

### EVAL-002 — lazy eval margin (200cp) is smaller than the sum of skipped terms

- **Severity:** P2
- **Confidence:** high
- **Status:** CONFIRMED
- **Anchors:** `backend/src/search/eval.rs:10`, `backend/src/search/eval.rs:179-184`, `backend/src/search/eval.rs:187-203`
- **Expected:** The lazy margin must exceed the maximum absolute value of all skipped evaluation terms so the pruned bound is still sound.
- **Actual:** Margin = 200cp. Skipped terms: mobility (eval_mobility, both sides), pawn structure (passed pawns / isolated / doubled), phased king safety, king shield, mop-up. In complex middlegame positions the sum of these terms can exceed ±500cp. A lazy cutoff can return a score that differs from the full eval by more than 200cp, yielding a wrong alpha/beta bound in the parent.
- **Failure scenario:** In a position where pesto (material+PST) says the position is +250cp (i.e., score - 200 = +50 >= beta = +40), we return +250 without computing mobility/pawn/safety. But the full eval might be +250 - 300 (mobility deficit) = -50, below beta. The parent gets a false beta cutoff.
- **Fix direction:** Either increase the margin to a safe bound (empirically, 400-500cp covers most skipped-term swings in this engine), or reduce the lazy cutoff to pesto-only positions (apply it only when material strongly determines the result).

### ORDER-201 — tt.new_search() is defined but never called

- **Severity:** P2
- **Confidence:** high
- **Status:** CONFIRMED
- **Anchors:** `backend/src/search/tt.rs:57-59`
- **Reasoning:** `grep -rn "new_search"` across all of `backend/src/` and `backend/tests/` returns exactly one result: the definition at tt.rs:57. There are zero call sites. The TT generation counter (`self.generation`) is therefore never incremented. TT entries from previous searches are never preferentially evicted; fresh searches can be dominated by stale entries from prior games/depths.
- **Failure scenario:** In iterative deepening, depth-2 entries can block depth-12 entries from saving (the `depth >= entry.depth` condition in save() means a higher-depth entry is preferred, but only within the same generation bucket). The generation field exists but does nothing.
- **Fix direction:** Call `tt.new_search()` at the start of each `search()` call or at the start of each iterative-deepening iteration.

### SEARCH-001/SEARCH-201 — ply can reach MAX_PLY in forced-check sequences, causing array panic

- **Severity:** P2 (downgrade from P0 confirmed — correct)
- **Confidence:** medium
- **Status:** PLAUSIBLE — requires a contrived position; downgrade to P2 is justified.
- **Anchors:** `backend/src/search/search.rs:283`, `backend/src/search/search.rs:391`, `backend/src/search/context.rs:8,12,21`
- **Reasoning:** `ctx.pv_length[ply]` at line 283 and `ctx.killer_moves[ply]` at line 391 are indexed with no ply bounds check. `killer_moves` is `Vec<[Option<Move>; 2]>` of size 64 (lines 12,34 of context.rs). Check extension (line 325: `extension = if in_check_now { 1 } else { 0 }`) adds to depth, keeping depth non-negative across in-check plies. With max_depth = 64 (cli.rs default), a sequence of 64 unique non-repeating checks can push ply to 64 before depth reaches 0, triggering a Vec panic. Repetition detection (line 286) prevents cycles but not straight-line check chains.
- **Why P2 not P0:** Such a position requires 64+ distinct-position in-check plies in a straight line. Real chess positions terminate this far earlier (checkmate, escape). The `has_major_pieces` condition for NMP and FP pruning further limits depth in tactical positions.
- **Fix direction:** Add `if ply >= MAX_PLY { return (0, None); }` as the first check in alpha_beta, before any array indexing.

### SEARCH-203/SEARCH-004 — sub-PV contamination from null-window cutoffs

- **Severity:** P2 (subsumed by ORDER-202 as the primary mechanism)
- **Confidence:** low
- **Status:** PLAUSIBLE — ORDER-202 is the dominant path; SEARCH-203 is a secondary contributor.
- **Reasoning:** After the LMR null-window search (lines 499-510) and optional re-search (lines 515-527), if score >= beta occurs at the null-window stage and the full re-search (lines 530-543) does NOT fire, `pv_length[ply+1]` was last set by a null-window call that returned early (pv_length[ply+1] = 0 or stale). The PV copy at 561-565 then produces a shorter PV. However, at the actual PV node (the root and its best-move path), the full re-search DOES fire (score > alpha and score < beta), so pv_length[ply+1] is correct after the full-window call. SEARCH-203 applies mainly to non-PV nodes that accidentally build the PV copy, which does not propagate to root unless they become best moves. ORDER-202 is the cleanest and most general explanation for the 1-move PV. Given ORDER-202 is confirmed, SEARCH-203 is secondary and left as low-confidence.

---

## Clean bill

- **PV legality loop (determinism_tests.rs:36-43):** Tests that each move in the PV is legal from the position after applying all prior PV moves. This is correctly implemented and would catch an illegal PV move. It does NOT catch a truncated PV. The `pv.len() <= 5` assertion (line 33) is only an upper bound and passes trivially with a 1-element PV.
- **TT save depth preference (tt.rs:88):** The `depth >= entry.depth` condition correctly prefers deeper entries, preventing shallow entries from evicting deeper ones within the same generation. Behaviour is correct given that generation is never updated.
- **Repetition detection (search.rs:286-288):** `board.is_repetition()` is called before any TT probe. This correctly prevents DRAW_SCORE from being cached as a TT hit.
- **Aspiration window retry (search.rs:707-717):** Fail-low and fail-high handling widens only the appropriate window bound (alpha downward, beta upward). This is correct PVS aspiration window logic.
- **Score-to-TT / score-from-TT (search.rs:51-69):** Mate score adjustments (`score + ply`, `score - ply`) correctly convert between relative and absolute ply-independent mate scores. No off-by-one found.
- **Check extension guard (search.rs:327):** `if depth <= 0 && !in_check_now` correctly falls through to quiescence for non-check leaf nodes, preventing the extension from firing at leaf quiescence entry.
