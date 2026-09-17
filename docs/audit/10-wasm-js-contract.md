# WASM–JS Contract — Audit Findings

**Agent:** audit-contract
**Scope:** `backend/src/wasm.rs`, `main:frontend/public/wasm/vantage.d.ts`, `main:frontend/public/wasm/vantage.worker.js`, `main:frontend/src/app/chess/engine/useEngine.ts`, `main:frontend/src/app/chess/engine/adapter.ts`, `main:frontend/src/app/chess/types/index.ts`, `backend/pkg/vantage.d.ts`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `backend/src/wasm.rs` (304 LOC) — read in full; enumerated all `#[wasm_bindgen]` exports
- `git show main:frontend/public/wasm/vantage.d.ts` — committed TypeScript declarations (proxy for deployed binary)
- `git show main:frontend/public/wasm/vantage.worker.js` — hand-written worker (~98 LOC)
- `git show main:frontend/src/app/chess/engine/useEngine.ts` — React hook (~80 LOC)
- `git show main:frontend/src/app/chess/engine/adapter.ts` — adapter interface (~15 LOC)
- `git show main:frontend/src/app/chess/types/index.ts` — type definitions including `SearchOptions`, `GameStatus`
- `backend/pkg/vantage.d.ts` — local wasm-pack output (first 80 lines)
- `git diff main HEAD -- backend/src/wasm.rs` — 12 insertions / 5 deletions (commit e7cc241)

## What I did not check, and why

- Binary byte-level inspection of `vantage_bg.wasm` — `wasm-objdump` not available; used `vantage.d.ts` as proxy
- Full `backend/pkg/vantage.d.ts` beyond line 80 — the first 80 lines showed it is byte-identical to main's d.ts in the JS surface portion; truncated read is sufficient for contract purposes
- DifficultyConfig usage — not needed to establish contract correctness

## Three-Way Contract Table

| Rust export (wasm.rs) | vantage.d.ts (deployed proxy) | worker.js case / WASM call | TS caller | Status |
|---|---|---|---|---|
| `constructor()` | `constructor()` | `init` → `new VantageEngine()` | `send('init')` | OK |
| `new_game() → void` | `new_game(): void` | `new_game` → `.new_game()` | `send('new_game')` | OK |
| `set_position_fen(&str) → bool` | `set_position_fen(s): boolean` | `set_position` → `.set_position_fen()` *(return bool dropped)* | `send('set_position', {fen})` | **MISMATCH — see CONTRACT-002** |
| `set_position_startpos(&str) → void` | `set_position_startpos(s): void` | `set_position` → `.set_position_startpos()` | `send('set_position', {moves})` | OK |
| `apply_move(&str) → bool` | `apply_move(s): boolean` | **NEVER CALLED** | — | **UNUSED EXPORT — see CONTRACT-006** |
| `make_move(&str) → String` | `make_move(s): string` | `apply_move` → `JSON.parse(.make_move())` | `send('apply_move', {move})` | OK (intentional naming split) |
| `go_depth(i32) → String` | `go_depth(n): string` | `search` → `JSON.parse(.go_depth(d))` | `send('search', {depth})` | **PARTIAL — see CONTRACT-003** |
| `go_movetime(u32) → String` | `go_movetime(n): string` | `search` → `JSON.parse(.go_movetime(t))` | `send('search', {movetime})` | **PARTIAL — see CONTRACT-004** |
| `get_fen() → String` | `get_fen(): string` | `get_fen` → `.get_fen()` | `send('get_fen')` | OK |
| `get_legal_moves() → String` | `get_legal_moves(): string` | `get_legal_moves` → `JSON.parse(.get_legal_moves())` | `send('get_legal_moves')` | OK |
| `get_legal_moves_for_square(&str) → String` | `get_legal_moves_for_square(s): string` | `get_legal_moves_for_square` → `JSON.parse(.get_legal_moves_for_square(s))` | `send('get_legal_moves_for_square', {square})` | OK |
| `get_game_status() → String` | `get_game_status(): string` | `get_game_status` → `.get_game_status()` | `send('get_game_status')` | OK |
| `side_to_move() → String` | `side_to_move(): string` | `side_to_move` → `.side_to_move()` | `send('side_to_move')` | OK |
| `is_move_legal(&str) → bool` | `is_move_legal(s): boolean` | `is_move_legal` → `.is_move_legal()` | `send('is_move_legal', {move})` | OK |

## Findings

### CONTRACT-001 — worker.onerror does not reject pending promises; UI hangs after worker crash

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `main:frontend/public/wasm/vantage.worker.js:6-93` (catch block line ~90), `main:frontend/src/app/chess/engine/useEngine.ts:32-38` (onerror handler)
- **Expected:** When the worker becomes unresponsive or crashes, all pending promise entries in `pendingRef` should be rejected so callers unblock.
- **Actual:** `worker.onerror` calls `setError(e.message)` and `setIsLoading(false)` but never iterates over `pendingRef.current` to call `.reject()` on in-flight requests. `workerRef.current` is also not nulled out so future `send()` calls still enqueue messages to the dead worker.
- **Reasoning:** `useEngine.ts:32` sets up `worker.onerror`. At line 35-38 it sets error state but the `pendingRef.current` Map is untouched. Any request that was in-flight at crash time has its `{resolve, reject}` functions permanently stored in the Map with no path to settlement. The worker's `onmessage` try/catch (worker.js line ~8) only catches errors *inside* handler execution; it does not fire for true worker crashes (WASM OOM, script-load failure). In the most reachable case — the `vantage.worker.js` URL returning 404 or a network error — the `init` promise hangs while `setIsLoading(false)` is called, leaving the UI unlocked but the pending init promise leaking indefinitely.
- **Failure scenario:** Deploy with a broken `/wasm/vantage.js` path. `onerror` fires, UI shows no spinner, but the `send('init')` promise never settles. Any subsequent `send('search', ...)` called by parent component also hangs. In a longer session: if the worker OOMs during a deep search, the game freezes with the engine-thinking indicator permanent.
- **Related:** CONTRACT-005 (manual rebuild gap means stale/missing wasm files are a real risk)
- **Where to look next:** `useEngine.ts:32-38` (onerror handler), `useEngine.ts:16-19` (pendingRef Map), `useEngine.ts:25-29` (send function)
- **Fix direction:** In `onerror`, iterate `pendingRef.current` and call `reject(new Error(...))` on every entry, then clear the Map. Optionally null out `workerRef.current` so future `send` calls reject immediately instead of silently enqueuing to a dead worker. Consider the same cleanup in the `useEffect` cleanup function to guard against double-unmount in React StrictMode (currently it calls `worker.terminate()` + `pendingRef.current.clear()` but skips rejecting).

---

### CONTRACT-002 — set_position_fen bool return silently dropped; invalid FEN accepted without error

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `main:frontend/public/wasm/vantage.worker.js:27-32` (set_position case), `backend/src/wasm.rs:54-60` (set_position_fen returns bool)
- **Expected:** If `set_position_fen` returns `false` (invalid FEN string), the caller should receive an error so it can reject the UI action or alert the user.
- **Actual:** Worker line 28 calls `engine.set_position_fen(payload.fen)` without capturing the boolean return value. It then responds `ok` with the current (unchanged) FEN from `engine.get_fen()`. The TS caller `useEngine.setPosition` resolves successfully.
- **Reasoning:** `wasm.rs:54-60`: on bad FEN, `Board::from_str` returns `Err`, `set_position_fen` returns `false` and board is unchanged. Worker.js line 28 discards this false. Line 31 calls `engine.get_fen()` which returns the previous position's FEN. The `respond(id, 'ok', { fen: ... })` call carries no error field. In `useEngine.ts`, `worker.onmessage` checks `if (error)` at line 30 — since `error` is undefined, it resolves. The caller gets a resolved promise containing the old FEN, with no indication anything went wrong.
- **Failure scenario:** `engine.setPosition('not a valid fen')` resolves without throwing. If UI code trusts the resolved position, the displayed board and the engine's internal board diverge silently. Any subsequent moves are played from the old position.
- **Related:** CONTRACT-001 (both are error-channel gaps)
- **Where to look next:** `vantage.worker.js:27-32`, `wasm.rs:54-60`, `useEngine.ts:60`
- **Fix direction:** Capture the bool from `set_position_fen`. If false, call `respond(id, 'error', {}, 'Invalid FEN string')` and `break` instead of `respond(id, 'ok', ...)`.

---

### CONTRACT-003 — search payload.depth truthy check silently upgrades depth=0 to depth=10

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `main:frontend/public/wasm/vantage.worker.js:48-57` (search case), `main:frontend/src/app/chess/types/index.ts:6-9` (SearchOptions)
- **Expected:** `send('search', {depth: 0})` should invoke `engine.go_depth(0)` (quiescence-only search or immediate evaluation). `send('search', {depth: 0, movetime: 0})` should invoke `engine.go_depth(0)`.
- **Actual:** Worker line 49: `if (payload.depth)` is a JS truthiness check. `0` is falsy. The branch falls through to `else if (payload.movetime)` which is also `0` (falsy), then falls through to `engine.go_depth(10)`. Caller receives a depth-10 result when it requested depth-0 or no options at all.
- **Reasoning:** `SearchOptions` in `types/index.ts:6` defines both fields as optional numbers with no minimum. A caller can legitimately send `{depth: 0}`. The truthy check means the intended value is silently ignored. Furthermore, if a caller sends `{depth: 6, movetime: 5000}` intending depth-limited play with a time safety net, only `depth` is used (truthy), which is probably the intended behavior — but `movetime` is completely ignored rather than being used as a ceiling, which is another expectation gap.
- **Failure scenario:** Difficulty mode configured to `{depth: 1}` works correctly. If a future difficulty mode uses `{movetime: 0}` (instant reply), it silently falls back to depth-10 instead.
- **Related:** CONTRACT-004
- **Where to look next:** `vantage.worker.js:48-57`, `types/index.ts:6-9`
- **Fix direction:** Replace truthy checks with explicit `payload.depth !== undefined` / `payload.movetime !== undefined` guards. Document whether `depth` and `movetime` are mutually exclusive or combinable.

---

### CONTRACT-004 — go_movetime receives u32; negative JS movetime wraps to ~4 billion ms

- **Severity:** P2
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `backend/src/wasm.rs:94` (`pub fn go_movetime(&mut self, ms: u32)`), `main:frontend/public/wasm/vantage.d.ts` (`go_movetime(ms: number): string`), `main:frontend/public/wasm/vantage.worker.js:53`
- **Expected:** Passing a negative movetime should either be rejected or treated as 0.
- **Actual:** wasm-bindgen maps TypeScript `number` to Rust `u32` using JavaScript's `ToUint32` algorithm. `ToUint32(-1)` = 4294967295. A caller passing `movetime: -1` would cause the engine to search for ~49 days of wall-clock time (or until the browser kills the worker).
- **Reasoning:** `wasm.rs:94` declares the parameter as `u32`. The generated `vantage.d.ts` declares it as `number` (no sign annotation). The wasm-bindgen ABI for u32 does NOT add a > 0 runtime JS-side guard in the generated glue for this wasm-bindgen version (confirmed by reading the `InitOutput` ABI in `vantage.d.ts` — the raw Wasm export `vantageengine_go_movetime` takes `(a: number, b: number)` without bounds checking). A JS caller can pass any JS number, which wasm-bindgen feeds through `ToUint32` bit-reinterpretation. `SearchOptions.movetime` in `types/index.ts` is typed as `number?` with no runtime guard.
- **Failure scenario:** A caller with a bug sends `movetime: -1`. The engine hangs for effectively forever, the worker stops responding, and pending promises from CONTRACT-001's gap hang too.
- **Where to look next:** `wasm.rs:94-96`, `types/index.ts:8`, `vantage.worker.js:53`
- **Fix direction:** Add a `> 0` guard in the worker before calling `go_movetime`. Alternatively, clamp at the Rust side before constructing `Duration::from_millis`.

---

### CONTRACT-005 — No copy script: manual rebuild gap is root cause of staleness

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `CLAUDE.md` lines referencing `wasm-pack build`, `backend/pkg/` (wasm-pack output dir), `main:frontend/public/wasm/` (runtime dir)
- **Expected:** A documented or scripted step to propagate new WASM builds to the frontend.
- **Actual:** wasm-pack writes to `backend/pkg/`. The frontend serves from `frontend/public/wasm/`. There is no npm script, Makefile target, or CI step that copies between the two. The rebuild sequence is entirely manual and undocumented.
- **Reasoning:** `CLAUDE.md` documents the wasm-pack build command as `cd backend && wasm-pack build --target web --no-default-features --features psqt`. The output lands in `backend/pkg/` (wasm-pack default). The committed runtime files are in `frontend/public/wasm/` (confirmed by `git show main:frontend/public/wasm/vantage.d.ts`). `backend/pkg/vantage.d.ts` exists and is identical to the committed d.ts — meaning someone did copy it manually but there is no scripted path. The behavioral fix in commit `e7cc241` (stop continuing after illegal move in `set_position_startpos`) exists only in `backend/src/wasm.rs` on this branch; `main:frontend/public/wasm/vantage_bg.wasm` was last updated at commit `f5eb24e` (2026-07-10) per the known context. The fix has never been deployed.
- **Correct rebuild sequence** (derived from CLAUDE.md + file layout):
  1. `cd backend && wasm-pack build --target web --no-default-features --features psqt`
  2. Copy `backend/pkg/vantage.js`, `backend/pkg/vantage_bg.wasm`, `backend/pkg/vantage.d.ts` → `frontend/public/wasm/`
  3. Do NOT overwrite `frontend/public/wasm/vantage.worker.js` — it is hand-written and not generated by wasm-pack.
- **Note on staleness gap:** The deployed `vantage_bg.wasm` (inferred from `vantage.d.ts` on main) exports exactly the same API surface as the current `backend/pkg/vantage.d.ts`. The worker calls no method that is missing from the deployed binary. There is **no live P0 contract mismatch** — the staleness is behavioral (old buggy `set_position_startpos`), not structural. This assessment is made by treating `vantage.d.ts` as the proxy for binary exports, as explicitly flagged.
- **Related:** Known context item 3 (committed wasm predates e7cc241)
- **Where to look next:** `backend/pkg/` directory listing, `frontend/public/wasm/` directory listing, `package.json` scripts section
- **Fix direction:** Add a `build:wasm` npm/shell script (or Makefile target) that runs wasm-pack and copies the three files. Add it to whatever CI runs frontend build so the deployed site always uses the binary built from HEAD of the branch being deployed.

---

### CONTRACT-006 — Module Worker { type: 'module' } compatibility gap: Firefox <114 and some Safari

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `main:frontend/src/app/chess/engine/useEngine.ts:22` (`new Worker(..., { type: 'module' })`), `main:frontend/public/wasm/vantage.worker.js:13` (`await import('/wasm/vantage.js')`)
- **Expected:** The worker is usable on all browsers the site targets.
- **Actual:** `{ type: 'module' }` workers require Firefox 114+ (released June 2023), Chrome 80+, and Safari 15+. Dynamic `await import()` inside a module worker adds a further requirement. There is no fallback.
- **Reasoning:** `useEngine.ts:22` uses `new Worker('/wasm/vantage.worker.js', { type: 'module' })`. Firefox added module worker support in 114 (Bugzilla 1247687). Safari added it in 15.0 (2021). A user on Firefox ESR 102 (EOL June 2024 but still in enterprise deployments) or any browser before these versions will hit a DOM exception when constructing the Worker. The `onerror` handler will fire, setting the error state. Combined with CONTRACT-001, there is no fallback search path — the chess UI becomes completely non-functional. No `canuse`/feature-detect guard is present.
- **Failure scenario:** User with Firefox 102 ESR navigates to chess.vaishakmenon.com. Worker construction throws, `onerror` fires, error message appears. Engine never initializes. User cannot play.
- **Where to look next:** `useEngine.ts:22`, `vantage.worker.js:13`
- **Fix direction:** Either add a feature-detect (`'module' in Worker.prototype` is not reliable; instead try constructing a trivial module worker in a try/catch) and show a meaningful "requires a modern browser" message, or rewrite the worker as a classic (non-module) worker using `importScripts`. The latter requires that `vantage.js` be adapted for `importScripts` import style, which wasm-pack supports via `--target no-modules`.

---

### CONTRACT-007 — Error channel end-to-end trace: bad FEN in go_depth path

- **Severity:** P2 (analysis/documentation finding — reveals one P1 already reported)
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/wasm.rs:89`, `main:frontend/public/wasm/vantage.worker.js:8-92`, `main:frontend/src/app/chess/engine/useEngine.ts:29-34`
- **Expected:** A clear, settled promise rejection with a descriptive error when the caller provides a bad FEN and searches.
- **Actual:** Traced path for "caller sets bad FEN then calls search":
  1. `useEngine.setPosition('bad fen')` → `send('set_position', {fen: 'bad fen'})`
  2. Worker `set_position` case: `set_position_fen` returns `false` (silently dropped per CONTRACT-002), responds `ok` with unchanged FEN.
  3. `useEngine.setPosition` promise resolves — caller gets no error signal.
  4. `useEngine.search({depth: 6})` → `send('search', {depth: 6})`
  5. Worker `search` case: `payload.depth = 6` (truthy), calls `engine.go_depth(6)`.
  6. `go_depth` runs on the old (unchanged) board position — legal behavior, returns valid JSON.
  7. Promise resolves with a bestmove for the original position, not the intended one.
  8. **Result:** No exception is thrown anywhere. All promises settle normally. The engine plays moves for the wrong position.
- **Error channel for Rust panics:** `wasm.rs:29` registers `console_error_panic_hook`. A panic in the Wasm module surfaces as a JS exception inside the worker. The `onmessage` try/catch at `vantage.worker.js:8` catches it and calls `respond(id, 'error', {}, err.message)`. `useEngine.ts:30-32`: `if (error) { pending.reject(new Error(error)) }` — the promise rejects. **This path works correctly for panics.**
- **Conclusion:** Panics are properly propagated. Silent failures (wrong-result paths without panic) are the real risk, all stemming from CONTRACT-002.
- **Where to look next:** `vantage.worker.js:8` (catch), `useEngine.ts:29-34` (message handler)
- **Fix direction:** See CONTRACT-002 fix direction. Once the bool is checked, the error channel is already wired up correctly.

---

### CONTRACT-008 — apply_move Wasm export unused; dead export

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/wasm.rs:78-85` (`apply_move` returns `bool`), `main:frontend/public/wasm/vantage.worker.js:39-42` (worker `apply_move` case calls `make_move` instead)
- **Expected:** Every exported method is called from JS, or is intentionally part of a public API.
- **Actual:** The worker's `apply_move` message case maps to `engine.make_move()` (which returns richer JSON). The `apply_move` Wasm method (which returns only a bool) is never called from any JS file on main. It is exported (binary overhead, d.ts surface) but has zero callers.
- **Reasoning:** `vantage.worker.js:40`: `const result = JSON.parse(engine.make_move(payload.move))`. The Wasm `apply_move` method is listed in `vantage.d.ts` and the InitOutput ABI (`vantageengine_apply_move`) but no worker case calls it. The naming split (worker message type `apply_move` → WASM method `make_move`) is intentional and correct for the worker; but it leaves `apply_move` as a dead export.
- **Where to look next:** `wasm.rs:78-85`, `vantage.d.ts` (apply_move listed), `vantage.worker.js:39`
- **Fix direction:** If `apply_move` has no intended external callers, remove the `#[wasm_bindgen]` annotation to keep the Wasm binary surface minimal. If it is intended as a lower-level utility for future use, add a code comment documenting that.

## Clean bill

- **Protocol envelope is complete and symmetric.** Every `type` the TS side sends has a corresponding `case` in the worker. The `default` case responds with `{error: 'Unknown message type: ...'}`, so the promise rejects (does not hang silently). Verified by reading both `useEngine.ts` and `vantage.worker.js` in full.
- **JSON return shapes are correct.** `go_depth`, `go_movetime`, `make_move` all return JSON strings via `format!(r#"..."#, ...)` in Rust (`wasm.rs:209-235`, `147-149`). The worker JSON.parses these. Every branch in `run_search` (book hit and search hit) returns valid JSON. `make_move` on invalid input returns `{"valid":false,"fen":"","status":""}` — also valid JSON. No branch returns a plain error string that would cause `JSON.parse` to throw unexpectedly.
- **GameStatus TS type vs Rust output.** `types/index.ts:3` includes `'resigned'` in the `GameStatus` union. `wasm.rs:290-303` never emits `'resigned'` — all 8 variants are a subset of the TS union. The `'resigned'` value is set by UI logic outside the engine, not a contract violation.
- **Deployed wasm exports everything the current worker calls.** Using `vantage.d.ts` on main as the proxy for what `vantage_bg.wasm` exports: all 13 method names the worker invokes are present in the committed d.ts. There is no live P0 missing-export mismatch on production. (The staleness is behavioral only — the set_position_startpos bug fix from e7cc241 is not deployed.)
- **wasm-bindgen naming convention.** No `#[wasm_bindgen(js_name = ...)]` overrides are used in `wasm.rs`. All methods are snake_case in both Rust and the generated TypeScript. The worker and callers use snake_case consistently. No camelCase/snake_case mismatch.
- **Message ID tracking.** `pendingRef` is keyed by incrementing integer IDs sent with every message. Worker always echoes `id` in the response. The `id` lookup in `useEngine.ts:26-27` ensures responses are matched correctly even if they arrive out of order.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `CLAUDE.md` wasm-pack command | Implies `frontend/public/wasm/` is kept current with engine source | No copy script; `e7cc241` fix never propagated to deployed binary | Live site runs stale engine (behavioral regression on invalid-move sequences) | No (site is functional, just wrong behavior) |
