# Unverified Claims

Findings whose reasoning holds but which could not be executed during the run — either
because the build was out of scope, or because they need a browser or a benchmark.
Each row gives the exact thing to run. None of these is speculation about unread code;
every one has a `file:line` anchor in its source report.

---

## Needs a WASM build

`wasm-pack` was excluded from this run by choice.

### HYGIENE-003 — WASM build fails on a fresh clone
Confirmed by reading (`backend/src/wasm.rs:35` is a bare `include_bytes!("../book.bin")`, no `cfg` guard, and `book.bin` is gitignored), but not executed. Your local build works only because `backend/book.bin` happens to exist on disk.

```
cd backend && wasm-pack build --target web --no-default-features --features psqt
```

To reproduce the actual failure, that must run against a tree without `backend/book.bin`. Do **not** delete your copy to test it — clone to a temp directory instead:

```
git clone . /tmp/vantage-clonetest && cd /tmp/vantage-clonetest/backend && wasm-pack build --target web
```

Note that `--features psqt` in the documented command is a dead flag (`CLAIM-004`); it gates nothing.

### CONTRACT-005 — whether a rebuilt `.wasm` still matches the worker's expectations
The deployed binary was verified to export everything `vantage.worker.js` calls, using `vantage.d.ts` as the proxy. After a rebuild, re-check the regenerated `.d.ts` against the worker's switch cases before copying `backend/pkg/` into `frontend/public/wasm/`. There is no script for that copy — that missing script is the finding.

---

## Needs an npm build

### Frontend type and lint health
`tsconfig.json` has `strict: true` and neither `ignoreBuildErrors` nor `ignoreDuringBuilds` is set, so a build is the real check. No frontend job exists in CI (`BUILD-006`), so this has likely never run in a clean environment.

```
cd frontend && npm ci && npm run build && npm run lint
```

Run it against `main`, not the working tree — the working tree's frontend is stale.

### BUILD-002 — Dockerfile.web
Confirmed by reading that it copies `/build/dist` while Next.js emits `.next/`. Executing `docker build -f docker/Dockerfile.web .` would demonstrate the failure, but the read is unambiguous.

---

## Needs a browser

### FE-201 (P1, PLAUSIBLE) — New Game mid-search applies a stale engine move
The race window was confirmed by code analysis: `gameKeyRef` has not updated when the stale search resolves during `await engine.newGame()`. Runtime confirmation takes about 60 seconds:

1. Start a game, set difficulty to Expert (longest search).
2. Play a move so the engine begins thinking.
3. While it is thinking, click **New Game**.
4. Choose **Black**.
5. Watch the engine's first move. If it is inconsistent with the fresh starting position, the race is real.

### CONTRACT-006 — `Worker { type: 'module' }` on older browsers
Unsupported in Firefox < 114, with no fallback. Needs a browser matrix check, or a decision that you do not support those versions.

### BUILD-008 — COOP/COEP headers in the Docker/nginx path
`nginx.conf` sets neither, and `next.config.ts`'s `headers()` does not apply to a static nginx serve. Whether this actually breaks WASM loading depends on the deployment. Check response headers on the deployed site and in a local `docker-compose.prod.yml` run. Related: whether `netlify.toml` needs its own `[[headers]]` block for the Netlify deployment.

---

## Needs a benchmark

### BUILD-010 — missing `[profile.release]`
No LTO, `codegen-units=16`. The improvement is real but unquantified. The perft agent established the comparison point: **3.2–4.9M nodes/sec**, single-threaded release, Windows 11 (recorded in `00-perft.md`). Re-run the same perft positions after adding `lto = true` and `codegen-units = 1` and compare. Do not claim an ELO figure from an NPS delta without a match.

### SEARCH-003 — NMP firing at PV nodes
The guard removal is deliberate (there is a comment at `search.rs:354`), and `has_major_pieces` limits zugzwang exposure. Whether it costs anything needs a tactical suite or an A/B match — which in turn needs `HYGIENE-001` fixed first, since the current gauntlet has no SPRT.

---

## Blocked behind another fix

### SEARCH-203 / SEARCH-004 — sub-PV contamination from null-window cutoffs
Cannot be observed empirically until the PV is actually printed in full. Both `ORDER-202` (data truncation) and `SEARCH-202` (print truncation) must be fixed first; then re-run PV replay-legality checks at depths 4–12 and compare below vs above depth 4, where aspiration windows engage.

### MOVEGEN-001 / MOVEGEN-201 — `good_captures` ArrayVec(64) overflow
Two agents tried and failed to construct a triggering position. Worst-case pawn-derived contribution is 44 into 64 slots, needing 21+ additional SEE-positive piece captures. It was neither triggered nor proved impossible. To settle it, extend `perft_fuzz` to assert `good_captures.len() < 64` across random promoted-piece-heavy positions, or switch `push` to `try_push` and observe. Until then it stays PLAUSIBLE P2.

### BOARD-002 / DRAW-001 — mate at halfmove clock 100
The code path is confirmed by reading (`status.rs:118` returns `DrawFiftyMove` before `generate_legal` is called). A live demonstration needs a status readout the CLI does not currently expose. A unit test on `position_status` with a constructed FEN would settle it in one assertion.

---

## Deliberately not run

- Engine strength / gauntlet matches — the harness is itself a finding (`HYGIENE-001`).
- `graphify update` — would have written to the repo; the staleness is documented as `HYGIENE-008`.
- Any Docker build — out of scope for the run.
