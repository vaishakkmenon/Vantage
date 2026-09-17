# Deep Frontend State — Audit Findings

**Agent:** deep-frontend-state
**Scope:** `main:frontend/src/app/chess/hooks/useChessGame.ts`, `main:frontend/src/app/chess/engine/useEngine.ts`, `main:frontend/src/app/chess/engine/adapter.ts`, `main:frontend/src/app/chess/components/ChessGame.tsx`, `main:frontend/src/app/chess/components/EvalBar.tsx`, `main:frontend/src/app/chess/components/LoadingScreen.tsx`, `main:frontend/src/app/chess/components/GameControls.tsx`, `main:frontend/src/app/chess/components/NewGameDialog.tsx`, `main:frontend/public/wasm/vantage.worker.js`, `main:frontend/src/app/layout.tsx`
**Date:** 2026-08-30 (overnight run)

> **Note:** This audit was conducted on `main` via `git show main:<path>`. The live site runs files from that branch. All line references reflect the files as read in this session. This agent cannot execute code; conclusions are code-path traces only — where runtime observation would settle ambiguity, that is noted explicitly.

## What I checked

- `git show main:frontend/src/app/chess/hooks/useChessGame.ts` — 127 LOC, read in full
- `git show main:frontend/src/app/chess/engine/useEngine.ts` — 86 LOC, read in full
- `git show main:frontend/src/app/chess/engine/adapter.ts` — 21 LOC, read in full
- `git show main:frontend/src/app/chess/components/ChessGame.tsx` — 185 LOC, read in full
- `git show main:frontend/src/app/chess/components/EvalBar.tsx` — 40 LOC, read in full
- `git show main:frontend/src/app/chess/components/LoadingScreen.tsx` — 7 LOC, read in full
- `git show main:frontend/src/app/chess/components/GameControls.tsx` — 24 LOC, read in full
- `git show main:frontend/src/app/chess/components/NewGameDialog.tsx` — 60 LOC, read in full
- `git show main:frontend/public/wasm/vantage.worker.js` — 98 LOC, read in full
- `git show main:frontend/src/app/layout.tsx` — 47 LOC, read in full (StrictMode check)
- `docs/audit/09-frontend.md` — read existing FE-001..FE-006 findings in full
- `docs/audit/10-wasm-js-contract.md` — read existing CONTRACT-001..007 findings in full

## What I did not check, and why

- Board.tsx, MoveHistory.tsx, GameOverModal.tsx, useBoardSize.ts — not required to trace the specific promise/state scenarios assigned; covered by prior agent
- `package.json` version pins — already covered in 09-frontend.md
- `vantage.js` (generated glue) and `vantage_bg.wasm` (binary) — no tooling to disassemble; used `vantage.d.ts` as proxy (already flagged in CONTRACT-005)
- Runtime execution — not possible without running the app. Every verdict below is a static trace; scenarios marked UNCLEAR-NEEDS-RUNTIME are those where the outcome depends on OS scheduling or browser microtask ordering that cannot be determined by reading code alone.

---

## FE-002 / CONTRACT-001 Unified Account

**The same defect, two views.** FE-002 names it from the React side; CONTRACT-001 names it from the worker side. Merging them here with the full trace.

### Promise lifecycle

**Where created:** `useEngine.ts:17-22` — `send()` creates a `new Promise`, assigns `++requestIdRef.current` as the key, stores `{ resolve, reject }` in `pendingRef.current`, and calls `workerRef.current?.postMessage(...)`.

**Where stored:** `pendingRef.current` — a `Map<number, {resolve, reject}>` on a ref (persists across renders).

**Normal settlement path:** `useEngine.ts:27-35` — `worker.onmessage` fires, finds the entry by `id`, deletes it from the map, then calls either `reject(new Error(error))` (if the message carries an `error` field) or `resolve(payload)`.

**Every path that CAN settle the promise:**
1. Worker responds with `{ id, payload }` — `onmessage` resolves (normal path).
2. Worker responds with `{ id, error: "..." }` — `onmessage` rejects. The worker's try/catch does this for any exception thrown inside the `onmessage` handler (worker.js line ~90 catch block).
3. Component unmounts — cleanup (`useEffect` return, `useEngine.ts:55-58`) terminates the worker, clears the map, but does **not** call `reject` on any entries. The Promise objects become permanently pending and garbage-collected whenever the caller's closure also becomes unreachable.

**Every path that CANNOT settle the promise:**
- `worker.onerror` (`useEngine.ts:32-37`) — calls `setError(...)` and `setIsLoading(false)`, never touches `pendingRef.current`. Pending promises are NOT rejected.
- Worker crash / WASM fatal trap that escapes the try/catch — triggers `worker.onerror`, same gap as above.
- WASM init hang (worker never calls `respond`) — triggers neither `onmessage` nor `onerror` (no error thrown, just silence). The `send('init')` promise hangs indefinitely.

**Effect on `isThinking`:** `runEngineTurn` (`ChessGame.tsx:73-91`) calls `chess.setThinking(true)` then awaits `engine.search()`. If `engine.search()` never settles (pending promise leaked), the `finally` block never executes, `setThinking(false)` is never called. `isThinking` is stuck `true` while the component is mounted.

**Effect on UI when `worker.onerror` fires during search:**
The `error` state is set. On the next React render, `ChessGame.tsx:161` short-circuits to `<div className="p-6 text-destructive">Error: {error}</div>`. The chess board is unmounted. `isThinking` being stuck does not affect the user-visible state once the error view renders. However: there is no Retry or Restart button in the error view or in `LoadingScreen.tsx`. The only recovery is a page reload.

**The single place a fix belongs:** `useEngine.ts` `worker.onerror` handler (line 32). It should iterate `pendingRef.current`, call `.reject(new Error(e.message))` on each entry, then clear the map. This allows every in-flight `await engine.x()` to propagate through its caller's `catch`/`finally` and reset state cleanly. `workerRef.current` should also be nulled so subsequent `send()` calls fail immediately rather than silently enqueuing.

---

## Scenario State-Machine Table

All scenarios are traced against: `useEngine.ts`, `ChessGame.tsx` `executeMove`/`runEngineTurn`/`handleStartGame`, and `vantage.worker.js`. Column definitions: **Board state** = chess.js `chessRef` + `displayFen`; **isThinking** = `chess.state.isThinking`; **Pending map** = `pendingRef.current` entries.

| # | Scenario / Trigger | Board state | isThinking | Pending map | User-visible | Verdict |
|---|---|---|---|---|---|---|
| 1 | User plays move 1; before `isThinking` set, plays move 2 | Both moves applied to chessRef synchronously; two ADD_MOVE dispatches queued | false during the window (SET_THINKING never called in executeMove, only inside runEngineTurn .then()) | 2× makeMove promises; then 2× search promises | Engine replies twice; board shows two engine responses; game state corrupted | **BUG P1** — see FE-001 CONFIRMED |
| 2 | Engine thinking; user clicks New Game, chooses black | stale applyLocalMove fires before gameKey increments; chessRef advanced then reset by startNewGame | Reset to false by START_NEW_GAME; finally block skips due to key mismatch | stale makeMove promise settles; new runEngineTurn's search promise added | Engine searches from startpos+stale_move; engine's "first move" as white is from wrong position | **BUG P1** — see FE-201 |
| 2b | Same as 2, chooses white | stale applyLocalMove fires then chessRef reset | Reset to false by START_NEW_GAME | stale makeMove promise settles and returns early (key mismatch) | Fresh game, player to move. Correct — no engine turn triggered | **SAFE** (because no runEngineTurn on player's first turn) |
| 3 | Engine thinking; user clicks Undo | N/A — no Undo button in GameControls.tsx | N/A | N/A | No undo UI exists | **N/A** |
| 4 | Engine thinking; component unmounts | Unchanged (effect on orphan closure; component gone) | Irrelevant (component gone) | clear() removes map entries; Promise resolve/reject functions become unreachable; GC'd | User left the page; no visible defect | **MEMORY LEAK, not user-visible** |
| 5a | Engine thinking; WASM throws (caught by worker try/catch) | Unchanged | setThinking(false) called in finally ✓ | Promise rejected via onmessage error path; deleted from map | Unhandled rejection propagates (no catch in runEngineTurn); board potentially stuck (finally runs, isThinking reset) | **SAFE** for isThinking; unhandled rejection may log to console |
| 5b | Engine thinking; worker.onerror fires (fatal crash / script load failure) | Unchanged | Stuck true while mounted; irrelevant once error UI renders | Promises NOT rejected; map entries leaked | Error UI shown; no Retry button; page reload only recovery | **BUG P1** — FE-002/CONTRACT-001 CONFIRMED |
| 6 | Worker returns response with unknown id | No state change | No change | No change (entry not in map; silently dropped) | None | **SAFE** |
| 7 | Two searches in flight (consequence of scenario 1) | Both engine moves applied; board shows N+2 plies when N+1 expected | true during both; false after both finally blocks | Two search promises, settled in worker's serial order | Board corrupted: two consecutive engine moves played | **BUG P1** — consequence of FE-001 |
| 8 | React 18 StrictMode double-mount | N/A | N/A | N/A | layout.tsx has no `<React.StrictMode>` wrapper; condition does not apply in production | **N/A** |
| 9 | Engine returns a move rejected by chess.js | `applyLocalMove` returns null; chessRef NOT advanced. But `engine.makeMove` was already sent. Worker applies the move; client did not. Desync. | finally block runs; setThinking(false) ✓ | makeMove promise settles, then check `!local` returns early | Board does not show engine move; engine position is one ply ahead of client position; future moves corrupted silently | **BUG P2** — silent desync; requires CONTRACT-002 (invalid FEN path) to be real |
| 10 | WASM init hangs (no resolve, no throw) | N/A (never reaches chess board) | Never set (no init before isLoading clears) | send('init') promise hangs indefinitely; no timeout | LoadingScreen shown indefinitely; no timeout, no retry button | **BUG P2** — FE-005 CONFIRMED |

---

## Findings

### FE-201 — New Game while engine thinks passes gameKey check; worker applies stale move to fresh game position

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `frontend/src/app/chess/components/ChessGame.tsx` lines ~57-68 (`handleStartGame`), lines ~73-91 (`runEngineTurn`), lines ~93-115 (`executeMove`)
- **Expected:** Clicking New Game while the engine is thinking completely resets the engine and the board; the new game starts from startpos with no contamination from the prior search.
- **Actual:** The following microtask-level race can occur:
  1. Engine is mid-search (`runEngineTurn` awaiting `engine.search()`).
  2. User clicks New Game → `handleStartGame` awaits `engine.newGame()` (sends 'new_game' to worker; worker queues it behind the in-flight 'search').
  3. Worker finishes search, sends search result; then processes 'new_game', sends OK.
  4. Main thread receives search result. `engine.search()` promise resolves. `runEngineTurn` continues. At this moment `chess.startNewGame()` has NOT yet been called (we are still awaiting `engine.newGame()`). `gameKeyRef.current` equals `keyAtStart`. **Check #1 passes.**
  5. `chess.applyLocalMove(searchResult.bestmove)` runs — chessRef advanced (then reset in step 6, so no lasting effect).
  6. `engine.makeMove(searchResult.bestmove)` is called — sends 'apply_move' to worker.
  7. Main thread receives 'new_game' OK → `engine.newGame()` resolves → `chess.startNewGame()` called → `chessRef.reset()`, `gameKey` incremented.
  8. Worker processes 'apply_move' (the stale move, sent after 'new_game' was already processed) → applies the stale move to the freshly-reset engine. Engine state is now `startpos + stale_move`.
  9. If player chose black, `handleStartGame` calls `runEngineTurn` with the new `keyAtStart`. `engine.search()` sends 'search' to the worker, which processes it from the corrupted position. The engine's first move of the new game is based on startpos+stale_move, not startpos.
- **Failure scenario:** Player is white, at move 15. Engine is searching. Player clicks New Game and chooses black. The new game's engine "first move as white" is computed from startpos plus one arbitrary move — the engine effectively starts the new game from a non-startpos position. Player sees the board show an incorrect opening position.
- **Related:** FE-001 (both stem from gameKey guard not covering all interleaving windows), CONTRACT-001 (no position reset call after the stale makeMove)
- **Where to look next:** `ChessGame.tsx:57-68` (`handleStartGame`), `ChessGame.tsx:73-91` (`runEngineTurn` try block), `useEngine.ts:17-22` (send / message ordering)
- **Fix direction:** Before `chess.startNewGame()` (and before triggering a new `runEngineTurn`), call `engine.setPosition` with startpos (or `engine.newGame()` a second time after the stale apply_move can no longer arrive). Alternatively, capture gameKey and compare before calling `engine.makeMove` inside `runEngineTurn` the same way the outer check works — but this requires a synchronous guard, not an async one. The safest fix is to send `engine.newGame()` again inside `handleStartGame` after `chess.startNewGame()` to re-reset the worker's engine state unconditionally.

---

## Verdicts on Previously Filed Findings

### FE-001 — CONFIRMED

The race window between `executeMove` returning and `isThinking` being set is real. `executeMove` dispatches `ADD_MOVE` (which does not set `isThinking`) then awaits `engine.makeMove()` inside a `.then()`. `runEngineTurn` is called from that `.then()` callback and sets `isThinking(true)` as its first statement. Between the `ADD_MOVE` dispatch and `runEngineTurn`'s `setThinking(true)` dispatch, `interactive` evaluates to `true` (status active, isThinking false, not browsing history). React 18 auto-batching does not close this window: batching applies within a single synchronous event handler; the `.then()` callback fires in a separate microtask after the event handler returns, so React will have re-rendered with `isThinking=false` before the callback runs. A second click event in that re-rendered state triggers a second `executeMove`. Scenario 7 (two concurrent searches) is the concrete consequence.

### FE-002 — CONFIRMED (code defect) / OVERSTATED (UX claim)

The code defect is real: `worker.onerror` does not reject pending promises; `isThinking` is stuck if `onerror` fires mid-search. However, the claim of "UI permanently dead" is overstated: `ChessGame.tsx:161` renders `<div>Error: {error}</div>` when `error` is set, replacing the chess board entirely. The user sees an error view, not a frozen board. The practical severity is: no in-game recovery (no Retry button), page reload required. The fix point is `useEngine.ts:32` as described in the unified account above.

### FE-003 — CONFIRMED

`EvalBar.tsx:26-29`: `MATE_SCORE_TO_MOVES` returns `movesToMate * (score > 0 ? 1 : -1)`. When `score < 0` (white is being mated), this is a negative integer, e.g., `-3`. Line 20: `` `M${movesToMate}` `` renders as `"M-3"`. Standard chess UIs conventionally show `"-M3"` (opponent mates in 3) or just `"M3"` with the bar position indicating which side. The raw `"M-3"` is unambiguous but non-standard and visually confusing in a narrow eval bar.

### FE-005 — CONFIRMED

`useEngine.ts:40-48`: `send('init')` has no `Promise.race` timeout. `LoadingScreen.tsx` (7 LOC) has no timeout state, no retry button, no fallback message. If `module.default()` in the worker (`vantage.worker.js:9`) blocks without throwing, `isLoading` stays `true` forever. Only recovery: page reload. Confirmed by reading both files in full.

### CONTRACT-001 — CONFIRMED (same defect as FE-002, different view)

See unified FE-002/CONTRACT-001 account above. The anchor is `useEngine.ts:32-37`; the fix belongs there.

---

## Root Cause Reduction

The six FE/CONTRACT findings reduce to **three distinct root causes**:

**Root Cause A — No synchronous interactivity guard in `executeMove`**
Covers: FE-001, Scenario 7 (double engine reply), and partially FE-201.
`executeMove` applies the move to local state (synchronous) but does not set `isThinking=true` until inside an async callback. The fix is one `setThinking(true)` call at the top of `executeMove` (before the async work), matching the pattern already used in `runEngineTurn`.

**Root Cause B — Worker error channel does not drain the pending promise map**
Covers: FE-002, CONTRACT-001, Scenario 5b, Scenario 10 (partially — onerror vs. silence are different).
`worker.onerror` is the only non-message path for worker failure, and it does not call `.reject()` on any pending entries. All three consequences (leaked promises, stuck `isThinking`, no in-game recovery) flow from this single omission. The fix is one loop in the onerror handler.

**Root Cause C — New Game flow does not re-establish engine position after potential stale-move contamination**
Covers: FE-201 (new finding). `handleStartGame` sends `engine.newGame()` before `chess.startNewGame()`, but a stale `engine.makeMove` can be sent to the worker after the new_game message and corrupt the fresh position. The fix is a second `engine.newGame()` call (or `engine.setPositionStartpos()`) after `chess.startNewGame()` to unconditionally re-reset the worker's engine state, eliminating the dependency on message ordering.

Root causes A and C together cover all scenarios where the `gameKey` guard is insufficient: A covers the window within a single game; C covers the window across the new-game transition.

---

## Clean Bill

- **StrictMode double-mount:** `layout.tsx` has no `<React.StrictMode>` wrapper. Scenario 8 is N/A in production.
- **Unknown-id worker response (Scenario 6):** Silently dropped; no state corruption. `useEngine.ts:27`: `const pending = pendingRef.current.get(id); if (pending) { ... }` — the else branch is absent and correct.
- **Unmount cleanup (Scenario 4):** `useEngine.ts:55-58` terminates worker, clears map. In-flight promises become permanently pending but the component is gone; no user-visible effect. Memory leak is bounded by the number of promises in flight at unmount time (typically 1).
- **`finally` in `runEngineTurn`:** Correctly guards `setThinking(false)` behind `gameKeyRef.current === keyAtStart`, preventing a stale engine turn from clearing `isThinking` for a new game. This works correctly for the non-FE-201 new-game path (player chooses white).
- **Caught WASM exception path (Scenario 5a):** Worker's try/catch at worker.js line ~8 catches WASM exceptions and calls `respond(id, 'error', {}, err.message)`. The `onmessage` handler rejects the pending promise. The `finally` block in `runEngineTurn` correctly calls `setThinking(false)`. This path is safe.
- **`react-chessboard` drop outside interactive window:** `onPieceDrop` checks `interactive` before calling `executeMove` and returns `false` otherwise. The library reverts the piece drag on `false`. Correctly handled — but only as effective as the `interactive` flag, which has the FE-001 gap.
