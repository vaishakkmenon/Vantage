# Frontend — Audit Findings

**Agent:** audit-frontend
**Scope:** `frontend/` on branch `main` (read via `git show main:<path>`)
**Date:** 2026-08-30 (overnight run)

> **IMPORTANT:** This audit was conducted on `main`, NOT on the working tree (`feature/backend_agentic_updates`).
> The working tree contains stale `.jsx` files; the deployed frontend is on `main`.

## What I checked

- `git ls-tree -r --name-only main -- frontend` — enumerated all 42 tracked frontend files
- `frontend/package.json` (23 LOC) — confirmed chess.js ^1.4.0, react-chessboard ^4.7.3, Next.js ^16.2.9, React 19.2.3
- `frontend/src/app/chess/hooks/useChessGame.ts` (127 LOC)
- `frontend/src/app/chess/engine/useEngine.ts` (86 LOC)
- `frontend/src/app/chess/components/ChessGame.tsx` (185 LOC) — main orchestrator
- `frontend/src/app/chess/hooks/useBoardSize.ts` (28 LOC)
- `frontend/src/app/chess/lib/gameStatus.ts` (22 LOC)
- `frontend/src/app/chess/lib/difficulty.ts` (23 LOC)
- `frontend/src/app/chess/engine/adapter.ts` (21 LOC)
- `frontend/src/app/chess/types/index.ts` (36 LOC)
- `frontend/next.config.ts` (35 LOC)
- `frontend/netlify.toml` (20 LOC)
- `frontend/tsconfig.json` (32 LOC)
- `frontend/public/wasm/vantage.worker.js` (76 LOC)
- `frontend/src/app/chess/components/Board.tsx` (38 LOC)
- `frontend/src/app/chess/components/EvalBar.tsx` (40 LOC)
- `frontend/src/app/chess/components/LoadingScreen.tsx` (7 LOC)
- `frontend/src/app/chess/components/GameOverModal.tsx` (27 LOC)
- `frontend/src/app/chess/components/GameControls.tsx` (24 LOC)
- `frontend/src/app/chess/components/MoveHistory.tsx` (38 LOC)
- `frontend/src/app/chess/components/NewGameDialog.tsx` (60 LOC)
- `frontend/src/app/layout.tsx` (47 LOC)
- `frontend/src/app/page.tsx` (5 LOC)
- `frontend/src/app/chess/lib/theme.ts` (20 LOC)
- `phase-1-frontend.md` — intent document (906 LOC), skimmed head/middle/tail

## What I did not check, and why

- `frontend/src/components/ui/*.tsx` (shadcn primitives) — downstream of Radix, out-of-scope for engine correctness
- `frontend/src/lib/utils.ts` — one-line `clsx` wrapper, no logic
- `frontend/src/app/globals.css` — styling only
- `frontend/public/wasm/vantage.js` and `vantage_bg.wasm.d.ts` — generated files; wasm binary staleness already noted as Known Context #3
- `frontend/eslint.config.mjs`, `postcss.config.mjs` — build config, no runtime impact
- `package-lock.json` — dep tree audit out of scope

## Findings

### FE-001 — Double-move race: board stays interactive between `executeMove` return and `isThinking` being set

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `frontend/src/app/chess/components/ChessGame.tsx` lines ~93-115 (`executeMove`), lines ~147-148 (`interactive` flag)
- **Expected:** After a user makes a move, the board becomes non-interactive until the engine responds and the engine's move is applied. This prevents a second user move from being sent while the engine is thinking.
- **Actual:** The `interactive` flag is `chess.state.status === 'active' && !chess.state.isThinking && !chess.isBrowsingHistory`. `isThinking` is not set to `true` synchronously in `executeMove`; it is set inside `runEngineTurn`, which is called inside the `.then()` callback of `engine.makeMove()`. `engine.makeMove()` is a Worker `postMessage` round-trip — at minimum one event-loop tick. During that window, `isThinking` is still `false`, the board is still interactive, and a second drag or click triggers a second `executeMove`.
- **Reasoning:** `executeMove` calls `chess.addMove()` (synchronous dispatch) then `engine.makeMove(uci).then(result => { ...; runEngineTurn(...); })`. `runEngineTurn` calls `chess.setThinking(true)` as its first statement. The React re-render that flips `isThinking` cannot happen until after the `.then()` callback fires and React processes the dispatch. All of this is at least one event-loop tick after `executeMove` returns `true`. During that tick(s), `onPieceDrop` and `onSquareClick` both check `interactive`, which is still true.
- **Failure scenario:** User drags a piece; before the Worker responds, user drags a second piece. Both `executeMove` calls succeed. The engine's state and the local chess.js state diverge: chess.js has two moves applied, but the engine's `apply_move` was called for move-1 before `engine.search()` runs, so engine searches from the post-move-1 position. Then `engine.apply_move` for move-2 is sent and the subsequent `runEngineTurn` runs from move-2's position. Two engine responses can both pass the `gameKeyRef` guard (same gameKey), resulting in the engine replying twice and the board showing an impossible position.
- **Related:** none identified
- **Where to look next:** `ChessGame.tsx` `executeMove` function (~line 93), `interactive` const (~line 147), `runEngineTurn` function (~line 73).
- **Fix direction:** Set `isThinking` to `true` synchronously inside `executeMove` before calling `engine.makeMove()`, not inside `runEngineTurn`. Alternatively, introduce a `ref`-based "pending move" flag that is checked at the top of `onPieceDrop` and `onSquareClick` and cleared in the `finally` block.

---

### FE-002 — Worker `onerror` mid-game leaves pending promises permanently unresolved; `isThinking` stuck true

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `frontend/src/app/chess/engine/useEngine.ts` lines ~28-33 (`worker.onerror`), lines ~14-17 (`pendingRef`), `ChessGame.tsx` `runEngineTurn` `finally` block (~line 87)
- **Expected:** If the worker errors mid-search, the pending `engine.search()` promise should be rejected so `runEngineTurn`'s `finally` block runs, `isThinking` is reset, and the error is surfaced.
- **Actual:** `worker.onerror` only calls `setError(...)` and `setIsLoading(false)`. It does NOT iterate `pendingRef.current` to reject outstanding promises. Any promise from `send()` that was in flight when the error fired never settles. `runEngineTurn`'s `finally` block (`if (gameKeyRef.current === keyAtStart) chess.setThinking(false)`) never executes.
- **Reasoning:** Looked at `useEngine.ts` `worker.onerror` handler — no `pendingRef` interaction. Cleanup function does call `pendingRef.current.clear()` but that only runs on unmount, not on error. If the WASM engine OOMs during a deep search (Advanced/Expert depth), the worker thread can terminate with an error, triggering `onerror` while `runEngineTurn` is awaiting `engine.search()`.
- **Failure scenario:** User starts an Expert game (5000ms movetime), WASM allocates aggressively and throws a runtime error mid-search. `onerror` fires. `setError` is called; on next render `ChessGame` returns `<div>Error: {error}</div>` (the chess UI disappears). The game is lost; no recovery path (no "restart" button in the error view). The `isThinking` leak is secondary since the UI is replaced — but if the component re-mounts later, the state is clean.
- **Related:** FE-001 (same finally-block path)
- **Where to look next:** `useEngine.ts:28-33`, `runEngineTurn` finally block in `ChessGame.tsx:~85-90`.
- **Fix direction:** In `worker.onerror`, iterate `pendingRef.current`, call `reject` on each pending entry with an `Error`, then clear the map. This allows any awaiting `runEngineTurn` to propagate through `catch`/`finally` and reset `isThinking`. Also add a "Restart" button to the error view so the user can recover without a page reload.

---

### FE-003 — `MATE_SCORE_TO_MOVES` returns a signed integer; eval bar displays "M-3" for opponent mates

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `frontend/src/app/chess/components/EvalBar.tsx` lines 19-20 (`evalLabel`), lines 26-29 (`MATE_SCORE_TO_MOVES`)
- **Expected:** When the engine is being mated (score < -MATE_THRESHOLD), the eval label should show something legible like "-M3" (standard notation) or be styled red with "M3".
- **Actual:** `MATE_SCORE_TO_MOVES` returns `movesToMate * (score > 0 ? 1 : -1)`, so for a losing position it returns a negative integer (e.g., -3). `evalLabel` then returns the template string `` `M${-3}` `` which renders as the literal string `"M-3"` in the eval bar.
- **Reasoning:** Read `EvalBar.tsx` line 27: `return movesToMate * (score > 0 ? 1 : -1)`. Line 20: `` return `M${movesToMate}`; `` with no absolute-value guard.
- **Failure scenario:** User is being mated in 3. Eval bar shows "M-3" instead of "-M3" or a negated pawn value. Minor but visually confusing.
- **Related:** none
- **Where to look next:** `EvalBar.tsx:19-20`, `EvalBar.tsx:26-29`.
- **Fix direction:** In `evalLabel`, use `Math.abs(MATE_SCORE_TO_MOVES(score))` for the move count and prepend a minus sign when score < 0, e.g., `` `${score < 0 ? '-' : ''}M${Math.abs(movesToMate)}` ``. Alternatively, pass the sign-information separately as a display hint.

---

### FE-004 — Promotion hardcoded to queen; underpromotion impossible

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `frontend/src/app/chess/components/ChessGame.tsx` line ~97 (`executeMove`)
- **Expected:** When a pawn reaches the promotion rank, the user is offered a choice of queen, rook, bishop, or knight.
- **Actual:** `const uci = source + target + (isPromotion ? 'q' : '');` — always promotes to queen regardless of user intent. The `react-chessboard` library supports a `onPromotionPieceSelect` callback for a promotion dialog; it is not wired up.
- **Reasoning:** Read `executeMove` in `ChessGame.tsx`. No promotion dialog state, no `onPromotionPieceSelect` handler passed to `<Board>`, no `Chessboard` prop in `Board.tsx` for promotion UI.
- **Failure scenario:** User reaches a position where underpromotion to knight is the only move avoiding stalemate or winning; the UI auto-promotes to queen. Known limitation acknowledged in `phase-1-frontend.md` "Known Limitations" section, but not flagged as incomplete in the implementation.
- **Related:** none
- **Where to look next:** `ChessGame.tsx:~97`, `Board.tsx` (no `onPromotionPieceSelect` prop), `react-chessboard` docs for promotion UI.
- **Fix direction:** Add a promotion piece state variable and wire up `react-chessboard`'s built-in promotion dialog via `onPromotionPieceSelect`. Intercept the move before sending it to the engine and wait for the user's piece choice.

---

### FE-005 — No timeout on WASM init; `LoadingScreen` can persist indefinitely if worker hangs

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `frontend/src/app/chess/engine/useEngine.ts` lines ~40-48 (`send('init')` call), `frontend/src/app/chess/components/LoadingScreen.tsx` (entire file, 7 LOC)
- **Expected:** If the WASM binary fails to load (e.g., hangs in `module.default()` without throwing), the user eventually sees an error or a retry prompt.
- **Actual:** `send('init')` sends a message to the worker and awaits the response. If `module.default()` inside the worker hangs (no exception, no timeout), the `send()` promise never resolves. `isLoading` stays `true`. `LoadingScreen` displays indefinitely with no actionable UI.
- **Reasoning:** Worker's `init` case calls `await module.default()` then `respond(id, ...)`. If `module.default()` blocks (e.g., WASM fetch times out silently, or the Netlify edge returns a stalled response), the `respond()` is never called. The worker's try/catch would only catch thrown errors, not indefinite blocking. The `useEngine` `send('init')` has no `Promise.race` with a timeout. `LoadingScreen` has no timeout state or fallback message.
- **Failure scenario:** User on a slow connection where `/wasm/vantage_bg.wasm` (2.5 MB per known context #3) download stalls. The spinner shows forever. No error message, no retry button.
- **Related:** FE-002 (worker unreachability pattern)
- **Where to look next:** `useEngine.ts:40-48`, `vantage.worker.js:7-12`, `LoadingScreen.tsx`.
- **Fix direction:** Wrap `send('init')` in a `Promise.race` with a `setTimeout` reject (e.g., 30s). On timeout, set `error` state. Add a "Retry" button to the `LoadingScreen` (or error view) that terminates and recreates the worker.

---

### FE-006 — COOP/COEP headers scoped to `/wasm/*` only; cross-origin isolation is a document-level property

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `frontend/next.config.ts` lines 22-33 (headers function), `frontend/netlify.toml` lines 6-18 (`[[headers]]`)
- **Expected:** If the intent is cross-origin isolation (required for `SharedArrayBuffer`), the document's own response must carry `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. Setting them only on `/wasm/*` paths isolates those responses, not the top-level document.
- **Actual:** `next.config.ts` sets COOP/COEP only on `source: '/wasm/:path*'`. The Netlify `[[headers]]` for `/*` does not include COOP/COEP. The worker script (`vantage.worker.js`) does not use `SharedArrayBuffer` or `Atomics` anywhere in its visible code — making isolation unnecessary for the current implementation.
- **Reasoning:** Confirmed no `SharedArrayBuffer` usage in `vantage.worker.js`. WASM loaded via standard dynamic import and `fetch`. The current headers config is therefore dead config — it doesn't harm functionality, but it misleads maintainers into thinking cross-origin isolation is in place (it is not, for the document). A future change that adds `SharedArrayBuffer` would silently fail if the document headers are missing.
- **Failure scenario:** A developer adds `SharedArrayBuffer`-based WASM threading, tests locally (possibly with the `--disable-web-security` flag), and ships — only to find it fails in production because the document lacks isolation.
- **Related:** none
- **Where to look next:** `next.config.ts:22-33`, `netlify.toml:6-18`, `vantage.worker.js` (no SharedArrayBuffer calls).
- **Fix direction:** Either remove the COOP/COEP headers from `next.config.ts` entirely (since they serve no current purpose), or extend them to include the document path (`/`) and also add them to `netlify.toml`'s `/*` block if SharedArrayBuffer is a future goal. Add a comment explaining the intent.

## Clean bill

- **Worker single-instance on double-mount (StrictMode):** `useEngine`'s `useEffect` cleanup terminates the worker via `worker.terminate()` and nulls the ref. React 18 StrictMode double-invoke creates, destroys, and recreates one worker — only one survives. Correctly handled.
- **Game-key guard for stale engine responses:** `runEngineTurn` checks `gameKeyRef.current !== keyAtStart` immediately after `engine.search()` resolves, before applying the move. This correctly drops responses from a previous game. The `gameKeyRef` is updated via a `useEffect` on `chess.state.gameKey`, and the prediction logic in `handleStartGame` (`chess.state.gameKey + 1`) accounts for the pre-render timing correctly (verified by reading the dispatch flow).
- **Two sources of truth (chess.js vs engine):** The game-over status is driven exclusively by the engine's `makeMove` response (`result.status`). The local chess.js (`chessRef`) is used only for SAN generation and `hasPieceAtSquare` — it is never queried for draw/checkmate status. No disagreement is possible.
- **ResizeObserver leak in `useBoardSize`:** The `useEffect` returns `() => observer.disconnect()` correctly. The `node` state is set via a `useCallback` ref that never changes identity. One observer per node, cleaned up on `node` change or unmount.
- **Pending map cleanup on unmount:** `useEngine` cleanup clears `pendingRef.current` and terminates the worker. In-flight promises from before unmount are dropped (acceptable; the component is gone).
- **`keydown` handler in ChessGame:** Registered on `document` with a `return () => document.removeEventListener(...)` cleanup. The `chess` dependency causes the effect to re-run on every render (since `chess` is a new object each render), which is slightly wasteful but functionally correct — the old listener is removed before the new one is registered.
- **`aria-label` on interactive buttons:** `GameControls.tsx` has `aria-label` on all three buttons ("Start new game", "Flip board orientation", "Resign game"). `NewGameDialog` uses `<label htmlFor>` pairings correctly.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `phase-1-frontend.md:~480` (1b.3) | "Engine info display" — depth, nodes, time display during search | Not implemented; `SearchResult` type has no `depth`/`nodes` fields; `EvalBar` only shows final score | Cosmetic; users can't see search progress | No |
| `phase-1-frontend.md:~490` (1b.6) | "Clock display + useClock" — per-player clocks | Not implemented; no `useClock` hook exists; `types/index.ts` has no clock types | Cosmetic/feature gap | No |
| `phase-1-frontend.md:~500` (1c.1) | "Analysis mode toggle" — analyze any position | Not implemented; no mode toggle in `GameControls` or `ChessGame` | Feature gap | No |
| `phase-1-frontend.md:~503` (1c.2) | "FEN import/export" | Not implemented; no FEN input field anywhere | Feature gap | No |
| `phase-1-frontend.md:~505` (1c.3) | "PGN export" | Not implemented | Feature gap | No |
| `phase-1-frontend.md:~507` (1c.4) | "PV line display" | Not implemented; `SearchResult` carries only `bestmove`+`score` | Requires engine API change too | No |
| `phase-1-frontend.md:Component graph` | `EngineInfo`, `ClockDisplay`, `DifficultySelector`, `ModeToggle`, `FenInput`, `PgnExport`, `PVLines` components | None of these files exist in `main` | Feature gap (Phase 1b/1c) | No |
