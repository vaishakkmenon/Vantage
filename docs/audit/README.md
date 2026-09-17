# Vantage Codebase Audit — Index

Overnight unattended audit run on 2026-08-30. Read-only: no source file was modified.

**Start here:** [`SUMMARY.md`](SUMMARY.md) — the eight things that matter, in fix order.
**Then:** [`TRIAGE.md`](TRIAGE.md) — every finding, one table, sorted by severity.

---

## The files

| File | What it holds |
|---|---|
| [`SUMMARY.md`](SUMMARY.md) | Executive summary, top 8 issues, clean bill, suggested work order |
| [`TRIAGE.md`](TRIAGE.md) | Full finding table + rejected findings + corrected overstatements |
| [`INCOMPLETE.md`](INCOMPLETE.md) | Incomplete-implementation register (14 rows) |
| [`UNVERIFIED.md`](UNVERIFIED.md) | Claims needing a build or browser, with the exact command |
| [`METHOD.md`](METHOD.md) | Agent roster, what each one read, what each one ran |
| [`00-baseline.md`](00-baseline.md) | fmt / clippy / test / build output, verbatim. **Ground truth.** |
| [`00-perft.md`](00-perft.md) | Perft vs published values, 37 counts, NPS baseline |
| [`00-uci-live.md`](00-uci-live.md) | Live UCI protocol driving, including abuse cases |
| [`01-board-fen.md`](01-board-fen.md) | Board representation, FEN, status/draw detection |
| [`02-movegen.md`](02-movegen.md) | Move generation, magic bitboards, attack detection |
| [`03-exec-zobrist.md`](03-exec-zobrist.md) | make/unmake, incremental Zobrist |
| [`04-search.md`](04-search.md) | Search core, PVS, pruning, node budget |
| [`05-eval.md`](05-eval.md) | Evaluation, all 15 roadmap-claimed terms verified |
| [`06-ordering-tt-see.md`](06-ordering-tt-see.md) | Move picker, killers/history, SEE, transposition table |
| [`07-book-uci-cli.md`](07-book-uci-cli.md) | Polyglot book, UCI/CLI (static read) |
| [`08-wasm-analysis.md`](08-wasm-analysis.md) | wasm-bindgen boundary, `analysis::` module |
| [`09-frontend.md`](09-frontend.md) | Next.js frontend (read from `main`) |
| [`10-wasm-js-contract.md`](10-wasm-js-contract.md) | Three-way Rust ↔ glue ↔ TS contract diff |
| [`11-build-ci-deploy.md`](11-build-ci-deploy.md) | CI, Docker, Cargo config, deployment |
| [`12-test-suite.md`](12-test-suite.md) | Test quality, coverage map, acceptance-gate check |
| [`13-doc-claims.md`](13-doc-claims.md) | Every documented claim verified against code |
| [`14-repo-hygiene.md`](14-repo-hygiene.md) | Repo state, branches, Python tooling |
| [`15-deep-pv.md`](15-deep-pv.md) | Deep dive: PV correctness, ply overflow |
| [`16-deep-draws.md`](16-deep-draws.md) | Deep dive: draw detection across three implementations |
| [`17-deep-edges.md`](17-deep-edges.md) | Deep dive: movegen/Zobrist seam, ArrayVec overflow |
| [`18-deep-frontend-state.md`](18-deep-frontend-state.md) | Deep dive: frontend state races |
| [`19-deep-tt.md`](19-deep-tt.md) | Deep dive: TT generation, PV truncation mechanism |
| [`VERIFY-search.md`](VERIFY-search.md) | Adversarial verification: search cluster |
| [`VERIFY-engine.md`](VERIFY-engine.md) | Adversarial verification: board/movegen/exec/draws |
| [`VERIFY-infra.md`](VERIFY-infra.md) | Adversarial verification: build/CI/tests/docs |
| [`VERIFY-client.md`](VERIFY-client.md) | Adversarial verification: frontend/wasm/analysis/UCI |

---

## Scope

**Backend** — audited from the working tree on `feature/backend_agentic_updates`: 46 files, 10,407 LOC in `src/`; 25 integration test files (9,324 LOC), plus 14 inline `#[cfg(test)]` modules.

**Frontend** — audited from `main`, read via `git show main:<path>`. The working tree's `frontend/` is stale (25 old `.jsx` files); `main` has the real 43-file `.tsx` rebuild that is deployed to chess.vaishakmenon.com. No checkout, worktree, or stash was used.

**Also covered** — `.github/workflows/ci.yml`, `docker/*`, `docker-compose*.yml`, `scripts/*.py`, `.gitattributes`, `.gitignore`, `data/`, and the intent documents (`VANTAGE_ROADMAP.md`, `vantage-engine-pass.md`, `MCP_BUILD_PLAN.md`, `phase-1-frontend.md`, `README.md`, `CLAUDE.md`).

### Why two branches

The project's newest code is split, and neither tree alone is the product:

| Area | Newest code on | Evidence |
|---|---|---|
| Backend engine | `feature/backend_agentic_updates` (13 ahead of main) | `SearchLimits`/`SearchResult`, PV table, node budget, `src/analysis/` |
| Frontend | `main` (9 commits this branch lacks) | 43 files: `.tsx`, shadcn/ui, `engine/adapter.ts`, `netlify.toml` |

That split is itself finding `HYGIENE-006`.

---

## Method

Five waves, 26 agents. Each wave was dispatched in parallel; completion notifications drove the next wave with no human input.

| Wave | Agents | Purpose |
|---|---|---|
| A | 9 | Baseline build/test/clippy + 8 backend subsystem sweeps |
| B | 6 | Cross-cutting: frontend, wasm/JS contract, build/CI, tests, doc claims, hygiene |
| C | 2 | Perft verification and live UCI driving (needed the built binary) |
| D | 5 | Targeted deep dives seeded by A and B findings |
| E | 4 | Adversarial verification — every finding re-attacked |

Every agent operated under a shared brief requiring: a `file:line` anchor personally read for each finding, an explicit expected-vs-actual, a step-by-step reasoning chain, a concrete reproducible failure scenario, and a prose-only fix direction (no code — you write the code).

### Why there is a verification wave

Wave E agents were instructed to **disprove** findings, not confirm them. The result: **6 findings rejected outright** and **13 severity changes**, including the run's only initial P0 (`SEARCH-001`, ply overflow) dropping to P2 after 12 test positions at depths 4–100 failed to trigger it. Two agents also reached directly contradictory conclusions about the PV bug; that contradiction was resolved with line-level evidence rather than averaged over. Both the rejections and the corrections are documented in `TRIAGE.md` rather than silently dropped.

### Where grep would have failed

`backend/src/` contains **zero** `todo!()`, `unimplemented!()`, `TODO`, `FIXME`, `XXX`, or `HACK` markers. Incomplete work in this repo is invisible to text search — it exists only as a gap between an intent document and the code. That is why `INCOMPLETE.md` was built by reading `MCP_BUILD_PLAN.md`, `vantage-engine-pass.md`, `VANTAGE_ROADMAP.md`, and `phase-1-frontend.md` against the source, claim by claim.

---

## What was NOT checked

- **`wasm-pack build`** and **`npm ci && next build`** were excluded by choice at the start of the run. Anything that can only be settled by running them is in `UNVERIFIED.md` with the command.
- **Browser runtime.** The frontend was audited by reading `main`, not by running it. `FE-201` in particular carries a PLAUSIBLE status for this reason, with 60-second manual repro steps.
- **Engine strength.** No gauntlet or SPRT run. (`HYGIENE-001` argues the existing harness could not have produced valid results anyway.)
- **`graphify-out/`** was deliberately ignored as a source — it was built 2026-07-14 and predates `src/analysis/` and this branch's 13 commits. Agents read source directly. The staleness is itself finding `HYGIENE-008`.
- **`book/polyglot_keys.rs`** (783 lines of generated constants) was spot-checked, not read line by line.
- **`data/books/UHO_Lichess_4852_v1.epd`** (167 MB) was checked for tracking status only.

## Gaps in the run

None. All 26 agents completed and wrote their files. No agent was retried, and no permission prompt blocked the run.
