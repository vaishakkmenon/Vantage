# TT / SEE / Analysis — Audit Findings

**Agent:** deep-tt
**Scope:** `backend/src/search/tt.rs`, `backend/src/search/search.rs`, `backend/src/search/see.rs`, `backend/src/analysis/mod.rs`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/search/tt.rs` (119 LOC) — full read
- `backend/src/search/search.rs` (780 LOC) — full read
- `backend/src/search/see.rs` (157 LOC) — full read
- `backend/src/analysis/mod.rs` (206 LOC) — full read
- `backend/src/moves/types.rs` (first 60 LOC) — Move struct layout
- `grep -rn "new_search" backend/src/` — output: only definition at tt.rs:57, zero call sites
- `grep -rn "new_search" backend/tests/` — no output (zero call sites)
- `grep -rn "new_search" backend/src/bin/` — no output (zero call sites)
- Live run: startpos depth 10 (cold TT): 702,274 nodes, bestmove e2e4
- Live run: startpos depth 10 after B/C/D positions (warm TT): 370 nodes, bestmove e2e4
- Live run: startpos depth 12 (cold TT): 4,160,860 nodes, bestmove e2e4; depth-8 pv=d2d4, depth-10 pv=e2e4, depth-12 pv=e2e4 (all single-move info lines)

## What I did not check, and why

- Context struct PV array dimensions (`backend/src/search/context.rs`) — not needed once the truncation mechanism was confirmed in search.rs
- Bench harness / bench files — out of scope
- `src/bin/vantage-mcp.rs` — does not exist yet (per shared brief item 8)

---

## Findings

### ORDER-201 — TT generation counter permanently zero; depth-only replacement locks old entries

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/tt.rs:57-59` (definition), `backend/src/search/tt.rs:88` (replacement condition), zero call sites confirmed by grep across all of `backend/`
- **Expected:** `tt.new_search()` is called once per `go` command so that old entries from prior positions can be evicted by newer shallow entries, implementing a standard age-preferred replacement policy. Standard engines call this at the start of each iterative-deepening root.
- **Actual:** `new_search()` is defined but never called. `self.generation` is initialized to 0 (tt.rs:53) and stays 0 for the process lifetime. All saved entries also get `generation = 0` (tt.rs:97). The replacement condition at tt.rs:88 is `entry.key == 0 || depth >= entry.depth || entry.generation != self.generation`. Since both sides are always 0, the third clause never fires. The effective policy is: **replace only if slot is empty or new depth >= stored depth.**
- **Reasoning:** `grep -rn "new_search" backend/src/` returns exactly one line: the definition at tt.rs:57. `grep -rn "new_search" backend/tests/` returns nothing. `grep -rn "new_search" backend/src/bin/` returns nothing. There is no call site anywhere. The native TT is 512 MB. Move struct is ~8 bytes; TTEntry ≈ 24 bytes; capacity ≈ 22 M entries (rounded to power of 2 at tt.rs:36-38). At ~700 K nodes/search, entries fill. Without aging, slots occupied by high-depth entries from old positions block new low-depth entries. New positions with colliding slots are only stored if they reach equal or greater depth (defeating iterative-deepening's early saves). Over a long analysis session the table degrades silently rather than panicking: probe returns None for non-matching keys so correctness is not violated, but TT effectiveness falls.
- **Failure scenario:** Multi-position analysis session (e.g., MCP server receiving a sequence of `evaluate` calls without process restart). After ~100–200 deep positions the 22 M entry table is saturated; new positions at early iterative-deepening depths (1–5) cannot store entries because the slot already holds a depth-8+ entry from a prior position. Move ordering at shallow depths degrades; search speed increases; analysis quality drops without any error signal. The `clear()` in `evaluate()` resets all entries (analysis/mod.rs:67), so the analysis module itself is immune — but the **native UCI binary** (`vantage-cli`) never clears or calls `new_search()`, so it is the affected consumer.
- **Related:** ANALYSIS-203 (shows `evaluate` calls `tt.clear()` — this is what saves the analysis API from ORDER-201, but the UCI path has no such protection).
- **Where to look next:** `backend/src/search/tt.rs:57-59` for the method, `backend/src/search/search.rs:627-779` (the `search()` function) to confirm there is no call between the iterative-deepening loop start and the alpha_beta call.
- **Fix direction:** Call `tt.new_search()` at the top of the `search()` function, before the iterative-deepening loop. Separately, verify the replacement condition at tt.rs:88 reads as intended (age OR depth), not age AND depth.

---

### ORDER-202 — TT exact-bound cutoff truncates PV at all non-root PV nodes (confirms ORDER-003)

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/search/search.rs:283` (pv_length reset), `backend/src/search/search.rs:309-316` (TT cutoff guard), `backend/src/search/search.rs:756-764` (info line prints only 1 move)
- **Expected:** TT cutoffs should not fire at PV nodes (open-window nodes where `alpha < beta - 1`). Standard practice is to guard TT early return with `is_pv = (beta - alpha > 1)`, allowing only move-hint retrieval at PV nodes. The PV should be as long as the search depth.
- **Actual:** The cutoff guard at search.rs:309 is `if ply > 0` — it fires at every non-root ply regardless of node type (PV or not). When it fires, `ctx.pv_length[ply]` was just set to 0 at line 283, the function returns immediately without updating `ctx.pv_table[ply]`, and the parent sees `pv_length[ply] = 0`. The parent's PV length becomes `pv_length[ply-1] = 0 + 1 = 1` (just the move that led here). At the root, `pv_length[0]` ends up 1 on any depth once even one PV node at ply=1 hits the TT. The full PV array `SearchResult.pv` is therefore 1 entry even at depth 12.
- **Reasoning:** Live test — cold TT startpos depth 12: `pv e2e4` (1 move reported). This is consistent with PV being truncated at ply=1 the moment the TT has any entry for the best child. The UCI info line at search.rs:763 prints `valid_mv.to_uci()` (a single move), not the slice from `last_completed_pv`, so PV truncation is invisible in logs but propagates to `SearchResult.pv` and hence to `analysis::evaluate`'s returned `pv` field (analysis/mod.rs:77) and `better_line` in `analyze_move` (analysis/mod.rs:184). Warm-TT test (same position after three other positions): 370 nodes vs 702,274 cold — confirms TT cutoffs at ply=1 short-circuit nearly the entire search. PV is 1 move in both cases.
- **Failure scenario:** `analysis::evaluate("startpos_fen", 1_000_000, tables, tt)` returns `pv: ["e2e4"]` regardless of depth reached, because ORDER-003 fires at every depth once any TT entry exists for ply=1. Any caller relying on `EvaluateResult.pv` or `AnalyzeMoveResult.better_line` for a continuation line gets only the root move.
- **Related:** ORDER-003 (the sibling agent's finding from the search side), SEARCH-004 (null-window scout; separate mechanism but same symptom).
- **Where to look next:** `backend/src/search/search.rs:309` for the guard, `backend/src/search/search.rs:560-565` for the PV propagation loop that depends on `pv_length[ply+1]`.
- **Fix direction:** Add an `is_pv` boolean (or compute `beta - alpha > 1`) and skip the early-return block for PV nodes. TT hash move can still be extracted for ordering. Separately, fix the info line at search.rs:763 to print the full PV slice, not just `valid_mv`.

---

### ORDER-203 — ANALYSIS-003: order-independence holds due to explicit clear(); within-call second search also clean

- **Severity:** P3 (test coverage gap only)
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/analysis/mod.rs:67` (`evaluate` clears TT), `backend/src/analysis/mod.rs:173` (`analyze_move` clears before first search), `backend/src/analysis/mod.rs:188` (`analyze_move` clears before second search)
- **Expected:** Per `vantage-engine-pass.md:51`, "search A then B gives same B result as B alone" — requires each entry point to clear the TT.
- **Actual:** `evaluate()` calls `tt.clear()` at line 67 before every search. `analyze_move()` calls `tt.clear()` at line 173 before the first search and at line 188 before the second search (after-move position). No cross-call pollution and no within-call pollution. **ANALYSIS-003 is REFUTED as a correctness bug.** However, no test exercises two sequential `evaluate()` calls on different FENs and compares results to solo calls. The acceptance gate in `vantage-engine-pass.md:51` is untested.
- **Failure scenario:** None currently — the clears are in place. Risk is a future refactor removes one of the clears.
- **Related:** ORDER-201 (native UCI path lacks clear/new_search; analysis API is immune precisely because of these clears).
- **Where to look next:** `backend/src/analysis/mod.rs:67,173,188`.
- **Fix direction:** Add an integration test: call `analyze_move(fen_A, mv, ...)`, then call `analyze_move(fen_B, mv, ...)` twice — once sequential, once standalone — and assert identical results. This would have caught any missing clear.

---

### ORDER-204 — SEE x-ray refresh: claimed wrong-square bug is REFUTED

- **Severity:** N/A
- **Confidence:** high
- **Status:** REFUTED
- **Anchors:** `backend/src/search/see.rs:100-125`
- **Reasoning:** ORDER-001 claims the refresh triggers on `next_victim` (wrong piece). Close reading shows `next_victim` at the refresh check site (line 110) is the piece from the PREVIOUS iteration's `attacker_sq`, set at line 120-122 of that iteration. This is semantically "was the piece that just left the board a slider?" The occupancy is updated at line 108 before the check, so when the refresh fires it uses the occupancy with that slider removed and correctly reveals attackers behind it. The check is one-iteration-delayed relative to the removal, but that is correct: removal happens at d=n, refresh for pieces revealed by that removal fires at d=n+1 when `next_victim` (= piece removed at d=n) is a slider. Concrete trace: White Knight captures pawn; Black Rook responds; Rook is removed at d=1; at d=2, `next_victim=Rook` triggers refresh; updated occupancy (Rook removed) reveals Black Queen behind the Rook; Black Queen is available at d=3. This is the correct behavior. No wrong-SEE value is produced by this mechanism.

---

## Clean bill

- **ANALYSIS-003 (TT pollution across analysis calls):** `evaluate()` and `analyze_move()` each call `tt.clear()` explicitly before every search. The two-search sequence in `analyze_move()` has a clear between searches. Order-independence holds for the current API.
- **SEE x-ray refresh (ORDER-001):** The `next_victim` check correctly tracks the previously-removed piece and fires the attacker-set refresh one iteration after the removal, using the already-updated occupancy. Both the initial x-ray (handled by pre-loop occupancy removal at see.rs:94) and subsequent x-rays (handled by the loop's delayed refresh) are correct.
- **`analyze_move` board leak:** The `_undo` at analysis/mod.rs:186 is intentional — the board is local to the function (fresh `Board::default()` at line 158) and is not restored, but the function returns before the board goes out of scope. No resource leak.

---

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `vantage-engine-pass.md:51` | "search A then B gives the same B result as searching B alone; validates per-query tt.clear()" | No test exercises sequential `evaluate`/`analyze_move` calls; `tt.clear()` is in place but uncovered | Analysis API correctness unverified in CI | No (manual verification passed) |
| `backend/src/search/search.rs:763` | UCI `info … pv` field should report the full principal variation | Only `valid_mv.to_uci()` is printed (1 move); `last_completed_pv` slice is never sent | GUI/analysis tools relying on UCI pv line get no continuation | Yes (protocol violation) |
