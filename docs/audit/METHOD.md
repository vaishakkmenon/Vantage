# Method — Agent Roster and Coverage

How the audit was run, who looked at what, and what was executed. Use this to retrace
any finding, or to judge how much weight a given report deserves.

Each area file also contains its own `## What I checked` and `## What I did not check`
sections listing every file opened with LOC and every grep run — that is the line-level
record. This page is the index over them.

---

## Roster

### Wave A — baseline + backend subsystems (9 agents, parallel)

| Agent | Output | Scope | Executed anything? |
|---|---|---|---|
| `baseline-build` | `00-baseline.md` | fmt, clippy, test, build across 3 feature configurations | Yes — the only agent permitted to run cargo builds |
| `audit-board` | `01-board-fen.md` | `board/` (1,461 LOC), `square.rs`, `status.rs`, `bitboard.rs`, `utils.rs` | No |
| `audit-movegen` | `02-movegen.md` | `movegen.rs` (810), `pawn/king/knight.rs`, `square_control.rs`, `types.rs`, `magic/` (718), `perft.rs` | No |
| `audit-exec` | `03-exec-zobrist.md` | `execute.rs` (503), `hash/zobrist.rs`, `book/polyglot_hash.rs` (as a cross-check) | No |
| `audit-search` | `04-search.md` | `search/search.rs` (779), `context.rs` | No |
| `audit-eval` | `05-eval.md` | `eval.rs` (759), `pesto.rs` | No |
| `audit-ordering` | `06-ordering-tt-see.md` | `picker.rs` (632), `ordering.rs`, `see.rs`, `tt.rs` | No |
| `audit-book-cli` | `07-book-uci-cli.md` | `book/` (1,118), `bin/cli.rs` (515), `logger.rs`, `output.rs` | No |
| `audit-wasm-analysis` | `08-wasm-analysis.md` | `wasm.rs` (304), `analysis/mod.rs` (205), `analysis_tests.rs` | No |

### Wave B — cross-cutting (6 agents, parallel)

| Agent | Output | Scope |
|---|---|---|
| `audit-frontend` | `09-frontend.md` | 43 files from `main` via `git show`; `phase-1-frontend.md` as the intent doc |
| `audit-contract` | `10-wasm-js-contract.md` | Three-way diff: `wasm.rs` exports ↔ `vantage.d.ts` + `vantage.worker.js` ↔ `useEngine.ts` + `adapter.ts` |
| `audit-build` | `11-build-ci-deploy.md` | `ci.yml`, `docker/*`, `docker-compose*.yml`, `Cargo.toml`/`.lock`, `.gitattributes`, `.gitignore`, `netlify.toml` |
| `audit-tests` | `12-test-suite.md` | All 25 integration test files + 14 inline test modules; per-file coverage map |
| `audit-claims` | `13-doc-claims.md`, `INCOMPLETE.md` | Every claim in `VANTAGE_ROADMAP.md`, `vantage-engine-pass.md`, `MCP_BUILD_PLAN.md`, `CLAUDE.md`, `README.md` verified against source |
| `audit-hygiene` | `14-repo-hygiene.md` | Repo strays, branch topology, `scripts/*.py` reviewed as code |

### Wave C — live execution (2 agents, needed the built binary)

| Agent | Output | What it ran |
|---|---|---|
| `baseline-perft` | `00-perft.md` | 37 perft counts across 6 CPW positions + the Position-4 mirror; timings and NPS |
| `baseline-uci` | `00-uci-live.md` | 12 scenario families driving `vantage-cli.exe` over stdin, including malformed-input abuse |

### Wave D — deep dives (5 agents, seeded by A and B)

| Agent | Output | Hypotheses attacked |
|---|---|---|
| `deep-pv` | `15-deep-pv.md` | SEARCH-001 ply overflow, SEARCH-004 PVS scout PV write |
| `deep-draws` | `16-deep-draws.md` | BOARD-002, BOARD-004, EXEC-003, EXEC-004 + three-implementation draw comparison |
| `deep-edges` | `17-deep-edges.md` | MOVEGEN-001 ArrayVec, MOVEGEN-002 castling filter, movegen/Zobrist seam |
| `deep-frontend-state` | `18-deep-frontend-state.md` | 10 interleavings; reduced 6 FE symptoms to 3 root causes |
| `deep-tt` | `19-deep-tt.md` | ORDER-001/002/003, ANALYSIS-003; separated the two PV-truncation mechanisms |

### Wave E — adversarial verification (4 agents)

| Agent | Output | Cluster |
|---|---|---|
| `verify-search` | `VERIFY-search.md` | Search, PV, eval. Resolved the ORDER-202 vs SEARCH-202 contradiction |
| `verify-engine` | `VERIFY-engine.md` | Board, movegen, exec, draws. Resolved the EXEC-001 vs MOVEGEN-203 contradiction |
| `verify-infra` | `VERIFY-infra.md` | Build, CI, tests, docs. Produced the authoritative test-count reconciliation |
| `verify-client` | `VERIFY-client.md` | Frontend, wasm, analysis, UCI, book |

---

## Commands executed

Only three agents ran anything. Everything else was reading.

**`baseline-build`**, from `backend/`:
```
cargo fmt -- --check
cargo build --release --features load_magic,deterministic_zobrist
cargo clippy --release --features "load_magic deterministic_zobrist" -- -D warnings
cargo test --release --features load_magic,deterministic_zobrist -- --test-threads=1 --no-fail-fast
cargo clippy --release --features "deterministic_magic deterministic_zobrist" -- -D warnings
cargo test --release --features deterministic_magic,deterministic_zobrist -- --test-threads=1 --no-fail-fast
cargo build --release           # default features
cargo tree --duplicates
```
Verbatim output is in `00-baseline.md`.

**`baseline-perft`** and **`baseline-uci`**, plus the deep-dive agents, drove
`backend/target/release/vantage-cli.exe` over piped stdin with explicit timeouts.
No agent ran an interactive command.

---

## Constraints every agent operated under

1. Read-only on source. No `Edit`/`Write` outside `docs/audit/` and the scratchpad.
2. No git write commands of any kind — no `checkout`, `worktree`, `stash`, `add`, `commit`, `restore`, `reset`, `clean`, `merge`, `rebase`. Read-only git only.
3. No `cargo fmt` without `--check`, no `cargo fix`, no `graphify update`.
4. Always write the output file, even on zero findings or tool failure.
5. Return at most 30 lines to the orchestrator — detail lives in the file, which keeps the orchestrator's context from filling before the verification wave.
6. A `file:line` anchor personally read for every finding. No speculation about unopened files.
7. Fix directions in prose only. No code, no diffs.

Constraint 7 follows `CLAUDE.md`'s teaching-mode section for `src/analysis/` and
`src/bin/vantage-mcp.rs`, and the standing preference that you write all project code
yourself. Reviewing those paths and pointing at problems is exactly what that section
asks for; every suggestion in this audit stays prose.

---

## Coverage honesty

- **Read in full**: every backend `src/` file except `book/polyglot_keys.rs` (783 lines of generated constants, spot-checked only); all 43 frontend files on `main`; all CI/Docker/config files; both Python scripts; all six intent documents.
- **Skimmed rather than read line by line**: the three largest test files (`zobrist_hash_tests.rs` 1,774 LOC, `status_tests.rs` 1,346, `movegen_tests.rs` 1,117) were skimmed for structure and assertion quality; the thin high-risk test files were read in full.
- **Not read**: `graphify-out/` (deliberately — stale), `Cargo.lock` line by line (queried via `cargo tree`), `data/books/*.epd`.
