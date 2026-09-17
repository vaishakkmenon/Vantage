# Vantage Audit — Executive Summary

**Run:** overnight, 2026-08-30. 26 subagents across 5 waves. 27 evidence files, ~4,500 lines.
**Branch audited:** `feature/backend_agentic_updates` for the backend, `main` for the frontend (read via `git show`, no checkout).
**Nothing was edited.** Only `docs/audit/` was created; `backend/target/` was written by builds.

---

## The one-paragraph version

The engine's core is sound. Perft is perfect — 37 node counts across all six standard positions match published values exactly, including the Position-4 colour-mirror pair, so move generation and make/unmake are correct. All 445 tests pass. Zobrist hashing is validated in release by a fuzz test nobody remembered was there. What is broken sits in three places: **one reachable panic**, **the principal variation**, and **everything around the code** — CI release, Docker, the WASM build on a fresh clone, the strength-testing harness, and the git state itself. Six raised findings were disproved on re-attack and are recorded as rejected.

---

## Fix these first

### 1. P0 — the engine panics on a kingless FEN
`backend/src/board/mod.rs:359`. Feed `position fen 8/8/8/8/8/8/8/8 w KQkq - 0 1` and the process dies with `King missing for White!`, exit 101. Both colours. Reachable from any UCI GUI, and from the frontend's FEN path. Verification also found the inverse problem: two-king and 9-pawn FENs are *silently accepted* and searched normally. The FEN parser neither validates nor fails safely.
→ `01-board-fen.md`, `00-uci-live.md`, `VERIFY-engine.md`

### 2. P1 — the PV is truncated to one move, which blocks the MCP server
`search.rs:283` resets `ctx.pv_length[ply] = 0` on every node entry; the TT exact-bound cutoff at 309–317 early-returns *before* any PV write; the parent therefore copies zero items and reports length 1. `last_completed_pv` is assembled from that same array, so it inherits the truncation.

Live proof: warm-TT depth-12 startpos searched **386 nodes** (cold: 4,160,860) and printed `pv e2e4` at every depth.

This means `EvaluateResult.pv` and `analyze_move`'s `better_line` — the entire value proposition of the planned MCP tools — return a single move. There is a **second, independent** defect stacked on top: `search.rs:762` prints `valid_mv.to_uci()` rather than iterating the PV vector, so the UCI `info` line would show one move even after the data bug is fixed. Fix both, and fix the data one first.

This is also exactly the hazard `vantage-engine-pass.md:35` warned about, though via a different mechanism than the one it predicted (TT cutoff at PV nodes, not the PVS scout).
→ `19-deep-tt.md`, `15-deep-pv.md`, `VERIFY-search.md`

### 3. P1 — your uncommitted MCP work is one command from deletion
`backend/src/analysis/` and `backend/tests/analysis_tests.rs` are untracked *and* unignored. The `src/mcp/` deletions are unstaged. A `git clean -fd` erases MCP Phase 0 + 1.1 with no recovery. This has been sitting since roughly Jul 27.
→ `14-repo-hygiene.md`

### 4. P1 — the WASM build cannot work on a fresh clone
`backend/src/wasm.rs:35` is a bare `include_bytes!("../book.bin")` with no `cfg` guard, and `book.bin` is gitignored. Your working copy only builds because the file happens to exist locally. Anyone cloning the repo — including CI, were it ever wired up — gets a hard compile error.
→ `14-repo-hygiene.md`, `VERIFY-infra.md`

### 5. P1 — release, Docker, and the opening book are all broken in CI
- The release job and `Dockerfile.engine` copy `target/release/vantage`. The only binary is `vantage-cli`. Every tagged release fails at the copy step.
- `Dockerfile.web` copies `/build/dist`; Next.js emits `.next/`. The web image serves nothing.
- `book.bin` is gitignored, so a CI checkout never has it. Every release ships bookless, printing a warning and continuing.
- CI triggers are main-push and main-PR only, so **these 13 commits have never been CI-verified at all.**
→ `11-build-ci-deploy.md`

### 6. P1 — every strength measurement you have made is statistically unsupported
`scripts/run_gauntlet.py` contains no SPRT. Its verdict is literally `percentage >= 50`. `VANTAGE_ROADMAP.md:137-148` makes SPRT the acceptance gate for every engine change, and the harness does not implement it. It also runs `20+0.2` where the roadmap specifies `10+0.1`, so past results are not comparable to the stated baseline either.
→ `14-repo-hygiene.md`

### 7. P1 — the frontend has three genuine state races
Six reported symptoms reduce to three root causes: (A) `executeMove` never sets `isThinking`, leaving a window for a second move; (B) `worker.onerror` never rejects pending promises; (C) the new-game flow does not re-establish the engine position, so a stale search result can be applied after the reset. (C) is the one that visibly corrupts a game — pick Black, hit New Game mid-search, and the engine may move from a contaminated position.
→ `18-deep-frontend-state.md`, `09-frontend.md`

### 8. P1 — the only test that checks your magic tables never runs
`tests/magic_consistency.rs` is gated `not(feature = "load_magic")`. CI always passes `load_magic`. It is the sole check that the bincode-loaded tables match the generated ones, and it has never executed in CI. Baseline runs confirm the gap: 445 tests under `load_magic` vs 457 under `deterministic_magic`.
→ `12-test-suite.md`, `00-baseline.md`

---

## What is verifiably healthy

This matters as much as the defect list — it tells you where *not* to spend time.

- **Move generation and make/unmake are correct.** All 37 perft counts match published values: startpos d1–6, Kiwipete d1–5, positions 3–6. The Position-4 mirror pair is identical, ruling out colour asymmetry in movegen.
- **Zobrist hashing is validated in release.** `perft_fuzz` asserts `board.zobrist == compute_zobrist_full()` before and after every make/undo across 5 seeds × 4 positions × 200 plies, unconditionally. An early finding claiming otherwise was corrected.
- **The full test suite passes**: 445 + 4 ignored under the CI configuration, 457 + 4 under `deterministic_magic`. Default build clean.
- **The TT hash move is validated** (`is_pseudo_legal` + `is_legal_move`) before use — no illegal-move-from-collision risk.
- **Castling rights on rook capture** — the classic bug — is handled correctly.
- **Polyglot EP hashing** correctly implements the conditional rule.
- **The analysis API is order-independent**: `evaluate` and `analyze_move` both clear the TT.
- **All 15 eval terms the roadmap claims are present**, and all 6 it defers are genuinely absent.
- **The deployed `.wasm` exports everything the current worker calls** — no live API breach on chess.vaishakmenon.com.
- **The frontend has a single source of truth for game status** (the Rust engine); chess.js is only used for animation.
- **NPS baseline recorded for the first time**: 3.2–4.9M nodes/sec, single-threaded release, Windows 11.

---

## Counts

| Severity | Count | Notes |
|---|---|---|
| P0 | 1 | reproduced with exit code |
| P1 | 17 | 16 CONFIRMED, 1 PLAUSIBLE (needs a browser) |
| P2 | ~45 | |
| P3 | ~40 | |
| **REJECTED** | **6** | disproved on re-attack, documented in `TRIAGE.md` |
| **Downgraded** | **13** | including the initial P0 on ply overflow |

The rejections and downgrades are the point of the verification wave. A P0 that cannot be triggered is a P2; an overstated claim that survives into a report costs you more time than it saves.

---

## Suggested order of work

1. Commit the untracked `analysis/` work (**item 3** — do this before anything else touches the tree).
2. Fix the kingless-FEN panic and add FEN validation (**item 1**).
3. Fix the PV truncation, both defects (**item 2**) — this unblocks the MCP server, which is your active project.
4. Fix `include_bytes!` on `book.bin` (**item 4**) so the repo builds from a clean clone.
5. Fix the CI binary name, `Dockerfile.web`, and the CI trigger so this branch is actually tested (**item 5**).
6. Decide what to do about branch divergence (**`HYGIENE-006`**) — you currently cannot deploy a build containing both halves of the project.
7. Rewrite the gauntlet verdict as a real SPRT before making any further strength claims (**item 6**).
8. The frontend races (**item 7**) whenever you next touch the UI.

Read `README.md` for how the audit was run and what it did not cover, `TRIAGE.md` for the full table, `INCOMPLETE.md` for the unfinished-work register, and `UNVERIFIED.md` for the handful of claims that need a build or a browser to settle.
