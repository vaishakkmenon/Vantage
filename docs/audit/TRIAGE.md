# Vantage Audit — Triage Table

Overnight audit, 2026-08-30. 26 agents, 5 waves, 27 evidence files.
Every finding below survived an adversarial verification pass unless marked REJECTED.

**Status key:** `CONFIRMED` = reproduced or proven from source · `PLAUSIBLE` = reasoning holds, execution blocked (see `UNVERIFIED.md`) · `REJECTED` = disproved during verification.

**Severity key:** **P0** breaks correctness · **P1** wrong results or user-visible breakage · **P2** latent bug or real risk · **P3** hygiene.

---

## P0 — fix first

| ID | Area | Title | Status | Anchor | Detail |
|---|---|---|---|---|---|
| BOARD-001 / UCI-106 | board | Kingless FEN panics the engine, exit 101 | CONFIRMED | `backend/src/board/mod.rs:359` | `01-board-fen.md`, `00-uci-live.md`, `VERIFY-engine.md` |

Reproduced: `position fen 8/8/8/8/8/8/8/8 w KQkq - 0 1` produces panic `King missing for White! FEN: ...`, exit code 101. Both colours. Reachable from any UCI GUI and from the frontend's FEN path.

---

## P1 — wrong results or broken delivery

| ID | Area | Title | Status | Anchor | Detail |
|---|---|---|---|---|---|
| ORDER-202 | search/TT | `SearchResult.pv` truncated to 1 move by TT cutoff at PV nodes | CONFIRMED | `search.rs:283,309-317` | `19-deep-tt.md`, `VERIFY-search.md` |
| HYGIENE-005 | git | `backend/src/analysis/` untracked; `git clean -fd` destroys MCP Phase 0+1 | CONFIRMED | `git status --porcelain` | `14-repo-hygiene.md` |
| HYGIENE-003 | build | WASM build fails on fresh clone — `include_bytes!("../book.bin")`, file gitignored | CONFIRMED | `backend/src/wasm.rs:35` | `14-repo-hygiene.md`, `VERIFY-infra.md` |
| HYGIENE-006 | git | No branch holds current backend + frontend + WASM together | CONFIRMED | `main` vs `HEAD` | `14-repo-hygiene.md` |
| HYGIENE-001 | tooling | `run_gauntlet.py` has no SPRT; verdict is `percentage >= 50` | CONFIRMED | `scripts/run_gauntlet.py` | `14-repo-hygiene.md` |
| BUILD-001 | CI | Release job + `Dockerfile.engine` copy `vantage`; only binary is `vantage-cli` | CONFIRMED | `ci.yml:71,75` | `11-build-ci-deploy.md` |
| BUILD-002 | docker | `Dockerfile.web` copies `/build/dist`; Next.js emits `.next/` | CONFIRMED | `docker/Dockerfile.web` | `11-build-ci-deploy.md` |
| BUILD-004 | CI | `book.bin` gitignored, so every release ships bookless; warning only | CONFIRMED | `ci.yml:96-102` | `11-build-ci-deploy.md` |
| TEST-001 | tests | `magic_consistency.rs` never runs in CI (gated `not(load_magic)`) | CONFIRMED | `tests/magic_consistency.rs:1` | `12-test-suite.md`, `VERIFY-infra.md` |
| CLAIM-001 | docs | Roadmap lists PVS as future Phase 1.1; code already implements it | CONFIRMED | `VANTAGE_ROADMAP.md:32-36` | `13-doc-claims.md` |
| CLAIM-003 | docs | `CLAUDE.md` run command: wrong binary name and non-portable `echo` | CONFIRMED | `CLAUDE.md` Common Tasks | `13-doc-claims.md` |
| UCI-102 | UCI | `stop` unimplemented; search blocks input loop; cutechess forfeits | CONFIRMED | `cli.rs` command loop | `00-uci-live.md`, `VERIFY-client.md` |
| UCI-101 | UCI | `info string` book status printed before `uci` is received | CONFIRMED | startup path | `00-uci-live.md` |
| UCI-103 | UCI | `go nodes N` silently ignored; `node_limit` hardcoded `None` | CONFIRMED | `cli.rs:310` | `00-uci-live.md` |
| FE-001 | frontend | Double-move race: board interactive before `isThinking` is set | CONFIRMED | `useChessGame.ts` `executeMove` | `18-deep-frontend-state.md` |
| FE-002 / CONTRACT-001 | frontend | `worker.onerror` never rejects pending promises | CONFIRMED | `useEngine.ts:32-38` | `18-deep-frontend-state.md` |
| FE-201 | frontend | New Game mid-search applies a stale engine move | PLAUSIBLE | `useChessGame.ts` `gameKeyRef` | `18-deep-frontend-state.md` |

---

## P2 — latent bugs and real risks

| ID | Area | Title | Status |
|---|---|---|---|
| SEARCH-001 / 201 | search | Ply >= MAX_PLY(64) unguarded index; empirically unreachable today | CONFIRMED (downgraded from P0) |
| SEARCH-202 | search | UCI `info pv` prints only 1 move regardless of stored PV — separate defect from ORDER-202 | CONFIRMED |
| SEARCH-002 | search | History bonus double-squared to `depth^4`, capped at 400 for depth >= 5 | CONFIRMED |
| SEARCH-003 | search | NMP fires at PV nodes (guard deliberately removed) | PLAUSIBLE |
| SEARCH-203 / 004 | search | Sub-PV contamination from null-window cutoffs at ply >= 1 | PLAUSIBLE (subsumed by ORDER-202) |
| ORDER-201 | TT | `tt.new_search()` never called; generation permanently 0 | CONFIRMED |
| EVAL-001 | eval | Mop-up eval one-sided (`+side`, never `-enemy`) — colour asymmetry | CONFIRMED (downgraded from P1) |
| EVAL-002 | eval | Lazy-eval margin 200cp is smaller than the combined swing of skipped terms | CONFIRMED (downgraded from P1) |
| EVAL-003 | eval | No colour-mirror symmetry test; suite cannot catch EVAL-001 | CONFIRMED |
| BOARD-002 / DRAW-001 | status | Fifty-move and seventy-five-move return before the checkmate test | CONFIRMED (downgraded from P1) |
| BOARD-001b | board | Two-king and 9-pawn FENs silently accepted and searched | CONFIRMED (new, from verification) |
| BOARD-003 | FEN | Parser accepts skip digit `0` via `to_digit(10)` | CONFIRMED |
| MOVEGEN-001 / 201 | movegen | `good_captures` ArrayVec(64) overflow panics; not constructively triggered | PLAUSIBLE (downgraded from P1) |
| MOVEGEN-101 | tests | FEN labelled "Kiwipete" in `perft_tests.rs:19` is not Kiwipete | CONFIRMED |
| TEST-002 | tests | 5 magic tests dark in CI (original claim of 11 overstated) | CONFIRMED (count corrected) |
| TEST-003 | tests | Three tests with no assertions at all | CONFIRMED |
| TEST-004 | tests | Wall-clock time as sole assertion in three tests | CONFIRMED |
| TEST-005 | tests | Determinism harness uses 512MB TT; spec says 64MB | CONFIRMED |
| TEST-006 | tests | Book module (~1,118 LOC) has zero coverage | CONFIRMED |
| TEST-007 | tests | `cli.rs` (515 LOC of UCI parsing) has zero coverage | CONFIRMED |
| BUILD-006 | CI | Only one feature combination is ever built or tested | CONFIRMED |
| BUILD-007 | CI | This 13-commit branch has never been CI-verified | CONFIRMED |
| BUILD-008 | docker | `nginx.conf` lacks COOP/COEP; `next.config.ts` headers do not apply | CONFIRMED |
| BUILD-009 | docker | `.dockerignore` excludes `Cargo.lock`; engine image non-reproducible | CONFIRMED |
| BUILD-010 | build | No `[profile.release]` — no LTO, `codegen-units=16` | CONFIRMED (impact unquantified) |
| BUILD-101 | CI | `clippy -D warnings` fails on the `deterministic_magic` path | CONFIRMED |
| CONTRACT-003 | wasm/js | `payload.depth` truthy check silently upgrades depth 0 to 10 | CONFIRMED |
| CONTRACT-004 | wasm/js | Negative JS movetime wraps via ToUint32 to ~4 billion ms | CONFIRMED |
| CONTRACT-005 | process | No `pkg/` to `public/wasm/` copy script; manual step is the staleness root cause | CONFIRMED |
| CONTRACT-006 | compat | `Worker { type: 'module' }` unsupported in Firefox < 114, no fallback | CONFIRMED |
| FE-003 | frontend | Eval bar renders `M-3` instead of `-M3` for opponent mates | CONFIRMED |
| FE-004 | frontend | Promotion hardcoded to queen; underpromotion impossible | CONFIRMED |
| FE-005 | frontend | No timeout on WASM init; LoadingScreen can persist forever | CONFIRMED |
| UCI-104 | UCI | `go infinite` is a depth-100 hardcode | CONFIRMED |
| UCI-105 | UCI | `bestmove 0000` for legal positions at depth <= 0 or movetime 0 | CONFIRMED (downgraded from P1) |
| UCI-107 | UCI | `go depth abc` falls back to `unwrap_or(64)` | CONFIRMED |
| UCI-110 | UCI | No options advertised; `setoption` silently ignored; TT hardcoded 512MB | CONFIRMED |
| BOOK-001 | book | `decode_move` returning None exits the selection loop instead of continuing | CONFIRMED |
| BOOK-002 | book | Unseeded `rand::rng()` — non-deterministic UCI and gauntlet play | CONFIRMED (analysis API unaffected) |
| ANALYSIS-002 | analysis | `classify` mate-boundary arm returns `cp_loss: None` with no grounding | CONFIRMED |
| WASM-001 | wasm | `set_position_startpos` returns `()`; JS cannot detect move-loop truncation | CONFIRMED |
| HYGIENE-002 | tooling | Gauntlet TC `20+0.2` vs roadmap's stated `10+0.1` | CONFIRMED |
| HYGIENE-004 | tooling | Gauntlet hardcodes Docker bind-mount paths; unusable outside the container | CONFIRMED |
| HYGIENE-008 | tooling | `graphify-out/` built 2026-07-14, predates `src/analysis/`; CLAUDE.md tells agents to trust it | CONFIRMED |
| CLAIM-002, CLAIM-006 | docs | Stale MCP present-tense claims; PV truncation already documented as a known hazard | CONFIRMED |

---

## P3 — hygiene

`EXEC-001` (execute.rs debug assertions dead in release — but Zobrist *is* validated by `perft_fuzz`, see corrections below) · `EXEC-002` (`paranoid_hash` double-dead and only `eprintln!`s, so it could never fail a test) · `EXEC-003` / `DRAW-004` (history window off-by-one; cannot cause a false repetition) · `BOARD-004` / `DRAW-005` (KBB same-colour not flagged; fifty-move rule ends it anyway) · `BOARD-005` (double `compute_zobrist_full` in `set_fen`) · `MOVEGEN-002` / `202` (queenside castling unfiltered in the pseudo-legal path; all callers re-filter) · `MOVEGEN-102` (Kiwipete d5 commented out rather than `#[ignore]`d, so `--include-ignored` cannot reach it) · `ORDER-004` (`order_moves` dead, tests-only, already diverging from `MovePicker`) · `ORDER-005` (dead `_depth`/`_alpha`/`_beta`/`_ply` params) · `CONTRACT-002` (bool return dropped; no user-supplied FEN path exists) · `CONTRACT-008` (`apply_move` export dead) · `ANALYSIS-004/005/006` · `BUILD-003` (release built with `deterministic_zobrist`) · `BUILD-005/011/012/013/014/015` · `BUILD-102/103/104` · `EVAL-004` · `HYGIENE-007/009` · `TEST-009/011` · `UCI-004/108/109/111` · `CLAIM-004/005/007/008`

---

## REJECTED during verification

Raised by an area agent, disproved on re-attack. Recorded so they are not re-raised later.

| ID | Claim | Why rejected |
|---|---|---|
| ANALYSIS-001 | `Score::from_raw(i32::MIN)` panics or wraps | `INF = 32000`, not `i32::MIN`. All search scores bounded to +/-32000; `last_completed_best_score` starts at 0 and updates only on completed depths. Structurally unreachable. |
| ORDER-001 | SEE x-ray refresh keys on the wrong square | Traced the swap loop step by step: the one-iteration-delayed refresh yields identical gain arrays. Hidden piece and blocker are same-side; the revealed piece becomes selectable exactly when that side acts next. |
| DRAW-002 | Search treats 2nd repetition as a draw, throwing wins | Twofold-inside-the-tree is standard deliberate engine practice. History is a proper push/pop stack and cannot fire falsely for non-cycle positions. |
| EXEC-004 / DRAW-003 | Repetition blind to history before the search root | `handle_position` applies every move via `make_move_basic`, which pushes to `board.history`. Game history *is* present at the root. |
| ANALYSIS-003 | Analysis API order-dependent via shared TT | `evaluate` clears at `mod.rs:67`; `analyze_move` clears at 173 and 188. Order-independence holds. The real gap is test coverage only (P3). |
| HYGIENE-010 | Tracked `git-config-docker.template` beside an ignored instantiation | That is the conventional template pattern, not a defect. |

## Corrected overstatements

| ID | Original claim | Correction |
|---|---|---|
| EXEC-001 | "No hash invariants validated in release" | `perft_fuzz` (`perft_tests.rs:232`) runs unconditionally in release, asserting `board.zobrist == compute_zobrist_full()` before and after every make/undo across 5 seeds x 4 positions x 200 plies. Zobrist **is** validated. Only the in-`execute.rs` assertions are dead. P2 to P3. |
| SEARCH-202 | "PV data is fine, only the display is broken" | Half right. The print bug is real *and separate*, but the data is also truncated (ORDER-202). Two distinct defects. |
| TEST-002 | 11 dark magic tests | 5. The 11 `cfg` gates include non-test helpers. |
| MOVEGEN-004 vs TEST | Conflicting perft coverage claims | Settled: CI covers startpos d1-d5 and Kiwipete d1-d4. Positions 3-6 have zero CI coverage. |
| Test count | 300+ / 445 / 449 / 457 / 461 / 468 all quoted by different agents | Authoritative: `load_magic` = 445 pass + 4 ignored = **449**. `deterministic_magic` = 457 + 4 = **461**. The delta is the dark magic tests. |
