# Repo Hygiene & Python Tooling — Audit Findings

**Agent:** audit-hygiene
**Scope:** Root strays, tracked test files, data files, magic tables, book.bin, pkg/, graphify-out/, branch hygiene, uncommitted work, scripts/run_gauntlet.py, scripts/check_engines.py
**Date:** 2026-08-30 (overnight run)

## What I checked

- `git ls-files --error-unmatch` and `git check-ignore -v` on every listed stray file
- `git log --diff-filter=A -- data/books/UHO_Lichess_4852_v1.epd` (was it ever committed?)
- `git branch --merged main`; `git log --oneline feature/frontend`; `git log --oneline HEAD ^main`
- Read `backend/test.html` (49 lines), `backend/test-worker.js` (61 lines), `frontend/public/wasm/vantage.worker.js` (first 30 lines)
- Read `backend/src/moves/magic/loader.rs` (first 50 lines) for `include_bytes!` path
- Read `backend/src/book/polyglot_book.rs` (first 50 lines) for book loading
- Read `backend/src/bin/cli.rs` (first 30 lines) for `book.bin` path
- `grep -rn "book.bin\|PolyglotBook::load"` across `backend/src/`
- Read `backend/src/wasm.rs:35` for `include_bytes!("../book.bin")`
- Read `scripts/run_gauntlet.py` (315 lines) fully
- Read `scripts/check_engines.py` (54 lines) fully
- Read `VANTAGE_ROADMAP.md:130-155` for gauntlet protocol
- Checked `ls graphify-out/` for snapshot directories
- Checked `.gitignore` fully; `.claude/` directory listing
- `git ls-files git-config-docker.template`

## What I did not check, and why

- `.aider.tags.cache.v4/` directory contents — it's a directory; `git check-ignore` confirmed `.aider*` ignore rule covers it. Contents are irrelevant.
- `graphify-out/` node/wiki content — stale by design; reading it would violate hard rule 8.
- `scripts/run_gauntlet.py` runtime behavior — cutechess-cli is not installed locally.
- `data/magic_tables.bin` binary content — format correctness is not assessable from a hex dump.

## Findings

### HYGIENE-001 — run_gauntlet.py has no SPRT implementation; 50% threshold is not SPRT

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `scripts/run_gauntlet.py:153-154`, `VANTAGE_ROADMAP.md:143`
- **Expected:** `VANTAGE_ROADMAP.md:143` states "SPRT bounds: H0=0, H1=10 for small features; H0=0, H1=20 for large features" and the roadmap calls this the acceptance gate for every engine change.
- **Actual:** `run_gauntlet.py` contains no LLR calculation, no sequential probability ratio, and no SPRT boundary. The verdict logic at line 153 is `if percentage >= 50: verdict = "PASS"`, which means any result ≥ 50% passes regardless of sample size. 11 wins out of 20 games (55%) against a much stronger engine would produce "PASS".
- **Reasoning:** I searched `run_gauntlet.py` for `SPRT`, `elo0`, `elo1`, `H0`, `H1`, `alpha`, `beta`, `sprt`, `llr`, `LLR` — zero hits. The save_summary function computes only raw win percentage. The VANTAGE_ROADMAP.md is explicit that SPRT is the acceptance gate. The gap is complete: no SPRT.
- **Failure scenario:** A developer runs the gauntlet with 20 games, gets 55% against Crafty (who is rated 200 ELO above the engine), sees "PASS", and ships a regression. With 20 games, the confidence interval for 55% spans roughly [33%, 75%] — statistically inconclusive.
- **Related:** None — this is a standalone tooling defect.
- **Where to look next:** `scripts/run_gauntlet.py:140-160` (save_summary/verdict block), `VANTAGE_ROADMAP.md:139-148`.
- **Fix direction:** Implement proper SPRT using the standard formula: compute LLR from wins/draws/losses at each game, stop early when LLR crosses ln(β/1-α) or ln(1-β/α) with the chosen bounds. Tools like `python-chess` or `cutechess-cli --sprt` flags can do this natively; the roadmap's stated bounds should drive the parameters.

---

### HYGIENE-002 — run_gauntlet.py time control 20+0.2 mismatches roadmap's stated 10+0.1

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `scripts/run_gauntlet.py:15`, `VANTAGE_ROADMAP.md:141`
- **Expected:** `VANTAGE_ROADMAP.md:141` says "Short tests: 1000 games at 10+0.1 for quick validation".
- **Actual:** `TIME_CONTROL = "20+0.2"` at `run_gauntlet.py:15` — 2× slower. A 20-game run takes twice as long as the roadmap envisions, and the games are played at a different strength point than stated.
- **Reasoning:** Direct textual mismatch between the script constant and the roadmap. Neither is wrong per se, but they disagree, meaning every result recorded is not directly comparable to the roadmap's baseline.
- **Failure scenario:** A developer reads VANTAGE_ROADMAP.md, runs the gauntlet expecting 10+0.1 results, gets 20+0.2 results. ELO estimates at different time controls are not equivalent.
- **Related:** HYGIENE-001 (also a gauntlet correctness issue).
- **Where to look next:** `scripts/run_gauntlet.py:15`, `VANTAGE_ROADMAP.md:141`.
- **Fix direction:** Align the constant with the roadmap, or update the roadmap to reflect the actual time control in use.

---

### HYGIENE-003 — WASM build fails on fresh clone without backend/book.bin

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/src/wasm.rs:35`, `.gitignore:7`
- **Expected:** WASM build succeeds from a fresh clone.
- **Actual:** `backend/src/wasm.rs:35` has `include_bytes!("../book.bin")` which resolves at compile time to `backend/book.bin`. The file is gitignored by `.gitignore:7` (`book.bin`) and is therefore absent from a fresh clone. `cargo build --target wasm32-unknown-unknown` will fail with a file-not-found compile error.
- **Reasoning:** `include_bytes!` is a compile-time macro. The path `../book.bin` is relative to `backend/src/wasm.rs`, resolving to `backend/book.bin`. `.gitignore` line 7 is `book.bin` (unanchored, matches anywhere). `git ls-files backend/book.bin` returns nothing. A fresh clone will not have the file, so WASM compilation fails.
- **Failure scenario:** Any developer cloning the repo and running `wasm-pack build --target web --no-default-features --features psqt` from a fresh checkout gets a compile error. CI would fail the same way if it were added.
- **Related:** Known context item 3 (stale WASM on main). This is the underlying reason it cannot be easily rebuilt.
- **Where to look next:** `backend/src/wasm.rs:35`, `.gitignore:7`, `backend/src/book/polyglot_book.rs:17` (the `from_bytes` API that `include_bytes!` feeds).
- **Fix direction:** Either track `backend/book.bin` in git (small at 1.49 MB, precedent: `data/magic_tables.bin` is tracked), or gate the book inclusion behind a feature flag so WASM can build without it, or embed from `data/book.bin` if the book is canonical. The CLI's `PolyglotBook::load("book.bin")` uses a runtime path (relative to CWD of the running binary) and degrades gracefully, so only WASM is broken by absence.

---

### HYGIENE-004 — run_gauntlet.py opening book path is a Docker bind-mount; script is unusable outside container

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `scripts/run_gauntlet.py:14,17,25-35`
- **Expected:** Script should be runnable in the documented development environment.
- **Actual:** All paths in the script are hardcoded to Docker bind-mount locations: `HERO_ENGINE = "/workspace/backend/target/release/vantage"` (line 14), `BASE_RESULTS_DIR = "/results"` (line 15), `OPENING_BOOK = "/openings/UHO_Lichess_4852_v1.epd"` (line 16), all engine paths `/engines/tscp`, `/engines/fairymax` etc. (lines 25-35). The 167 MB EPD file referenced is not in the repo (gitignored). Attempting to run outside Docker silently fails with missing-engine errors.
- **Reasoning:** I also note that `data/books/UHO.epd` (the zero-byte file in the repo) is NOT referenced by the script; the script references `UHO_Lichess_4852_v1.epd` at a container path. `data/books/2moves_v2.pgn` and `data/books/UHO.epd` are zero-byte placeholders that serve no purpose.
- **Failure scenario:** Any developer outside Docker environment runs the gauntlet, gets misleading errors.
- **Related:** HYGIENE-001, HYGIENE-002.
- **Where to look next:** `scripts/run_gauntlet.py:14-16`, `data/books/`.
- **Fix direction:** Add a README or `--help` note that the script requires the Docker dev container. Alternatively, make paths configurable via CLI arguments or environment variables so the script is usable natively.

---

### HYGIENE-005 — Uncommitted analysis/ module creates high loss risk

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** git status output, `backend/src/analysis/` (untracked `??`), `backend/tests/analysis_tests.rs` (untracked `??`)
- **Expected:** All source changes are committed or at minimum their commit intent is visible. Known context item 8 says MCP Phase 0 + 1.1 are done.
- **Actual:** `backend/src/analysis/` and `backend/tests/analysis_tests.rs` are entirely untracked (`??` in git status). The deletion of `backend/src/mcp/mod.rs` and `backend/tests/mcp_tests.rs` are unstaged (` D`). This means git does not model the rename; if the working tree is cleaned or the branch is checked out elsewhere, the `analysis/` directory disappears silently.
- **Reasoning:** Untracked files are not protected by git. `git checkout --` or `git clean -fd` would destroy them. The deletions being unstaged means `git diff HEAD` shows nothing for the deleted files, creating a false impression of minimal changes.
- **Failure scenario:** A tool or developer runs `git clean -fd` to clean up build artifacts. `backend/src/analysis/` is deleted with no recovery path. The entire MCP Phase 0+1 implementation is gone.
- **Related:** Known context items 8 and 9 (analysis module state).
- **Where to look next:** `git status` output, `backend/src/analysis/`, `backend/tests/analysis_tests.rs`.
- **Fix direction:** Stage and commit all changes: stage the deletions of `mcp/mod.rs` and `mcp_tests.rs`, stage `analysis/` and `analysis_tests.rs`, stage `Cargo.toml`/`Cargo.lock`/`lib.rs` modifications, and commit as a single "refactor: rename mcp → analysis" commit. Git should auto-detect the rename once both sides are staged.

---

### HYGIENE-006 — feature/backend_agentic_updates and main hold different halves of the product with no single integrated branch

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `git log --oneline HEAD ^main` (13 commits), known context item 3
- **Expected:** A project has one branch that represents the current deployable state.
- **Actual:** The feature branch has 13 backend commits not in main. Main has the most recent frontend work (commit `84517eb feat: highlight king square translucent red when in check`) and the committed WASM binary — neither of which is on this branch. Known context item 3 confirms the live WASM on main predates fix `e7cc241` (which is only here). There is no branch containing the live frontend + the corrected WASM + the backend search improvements.
- **Reasoning:** `git branch --merged main` confirms `feature/frontend` and `feature/wasm` are fully merged. `git log HEAD ^main` shows 13 commits unique to this branch. Main's `84517eb` highlight-check commit is not on this branch. Neither branch is the "current state of the project" — they hold different halves.
- **Failure scenario:** A developer deploys from main: site runs stale WASM with the illegal-move bug (known context 3). A developer deploys from this branch: loses the check-highlight frontend feature. A merge of main into this branch likely conflicts on `backend/src/wasm.rs` (the WASM binary, wasm module) and possibly search integration files, given 13 backend-only commits have diverged.
- **Related:** Known context items 1, 2, 3.
- **Where to look next:** `git log --oneline main..HEAD`, `git log --oneline HEAD..main`.
- **Fix direction:** Merge this branch's backend changes into main (or vice versa) before any further feature work. The merge should be done in a dedicated integration step: rebase or merge, resolve wasm.rs conflicts (pick the fix from this branch), then rebuild and recommit the WASM binary.

---

### HYGIENE-007 — backend/test.html and backend/test-worker.js are tracked dead scaffolding with stale import paths

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/test.html:9`, `backend/test-worker.js:2`, `frontend/public/wasm/vantage.worker.js:1-10`
- **Expected:** These files, being tracked, would be maintained in sync with the real worker or serve a documented purpose.
- **Actual:** Both import from `./pkg/vantage.js` (relative to `backend/`). `backend/pkg/` contents are gitignored (`backend/pkg/.gitignore` contains `*`), so the referenced file does not exist in a fresh clone. The frontend production worker (`frontend/public/wasm/vantage.worker.js`) uses `/wasm/vantage.js` (absolute URL) and includes a `respond()` wrapper with message IDs for request correlation — `test-worker.js` has none of this. These are PoC/dev-harness files from initial WASM bring-up that were never removed.
- **Reasoning:** The path `./pkg/vantage.js` in test files vs `/wasm/vantage.js` in the production worker confirms these are different vintages. `backend/pkg/` contents are not tracked, so the import target doesn't exist post-clone. There is no documentation or CI usage of `test.html` or `test-worker.js`.
- **Failure scenario:** A future developer opening `test.html` in a browser sees immediate import error.
- **Related:** None.
- **Where to look next:** `backend/test.html:9`, `backend/test-worker.js:2`, `frontend/public/wasm/vantage.worker.js`.
- **Fix direction:** Delete both files from the repo (they are tracked, so `git rm` is needed). They serve no purpose the frontend worker doesn't serve better.

---

### HYGIENE-008 — graphify-out snapshots predate src/analysis/ module; CLAUDE.md instructs agents to consult first

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `CLAUDE.md:55-62` ("For codebase questions, first run `graphify query`"), `graphify-out/` (last snapshot 2026-07-14), `backend/src/analysis/` (created on this branch, which has commits after 2026-07-14)
- **Expected:** Agents following CLAUDE.md would consult graphify first and get accurate module structure.
- **Actual:** The most recent snapshot is 2026-07-14. This branch added `src/analysis/` (replacing `src/mcp/`) after that date. A graphify query for "analysis module" or "MCP tools" would return stale or absent results. The deleted `src/mcp/mod.rs` may still appear in the graph, pointing agents to non-existent code. 8 snapshot directories accumulate without pruning.
- **Reasoning:** `ls graphify-out/` shows snapshot dirs `2026-06-26` through `2026-07-14` — 8 total. CLAUDE.md instructs agents to run `graphify query` first. The known context (items 8, 9) confirms analysis/ is new. Any agent following CLAUDE.md would be misled about the module structure.
- **Failure scenario:** A future agent queries `graphify explain "analysis"`, gets no result or a stale `mcp` result, concludes the module doesn't exist, and reinvents work already done.
- **Related:** Known context items 8, 9.
- **Where to look next:** `graphify-out/manifest.json`, `CLAUDE.md:55-62`.
- **Fix direction:** Run `graphify update .` after committing the analysis/ module (hard rule 3 prevents running it now). Add a CI or hook step to warn when `graphify-out/` is more than N days stale. Prune old snapshot directories (retain the most recent 1-2).

---

### HYGIENE-009 — check_engines.py treats subprocess timeout as OK, masking hung engines

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `scripts/check_engines.py:43-45`
- **Expected:** A liveliness check distinguishes "engine started and quit cleanly" from "engine hung".
- **Actual:** `except subprocess.TimeoutExpired: print(f"... OK (Slow) ...")` at lines 43-45. An engine that crashes with an infinite loop or blocks waiting for input is reported as `OK (Slow)`. A `timeout=1` second is also short for engines that need startup time.
- **Reasoning:** `subprocess.run([path], input=b"quit\n", ..., timeout=1)` sends "quit" and waits 1 second. For xboard engines, "quit" is a valid command, but for engines that start slowly (e.g., Crafty loads book files), 1 second may not be enough. A truly broken engine that ignores all input and loops would time out and be marked `OK (Slow)`.
- **Failure scenario:** A corrupt `crafty` binary that never exits is marked `OK (Slow)`, and a developer running `check_engines.py` before a gauntlet is given false confidence.
- **Related:** HYGIENE-004 (Docker-only paths; script not runnable outside container anyway).
- **Where to look next:** `scripts/check_engines.py:43-45`.
- **Fix direction:** On `TimeoutExpired`, kill the process and mark `HUNG` rather than `OK (Slow)`. Consider increasing timeout to 3-5 seconds.

---

### HYGIENE-010 — git-config-docker.template is tracked without a corresponding .gitignore entry

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `git ls-files git-config-docker.template` (confirmed tracked), `.gitignore:9` (`git-config-docker` is ignored but not `git-config-docker.template`)
- **Expected:** Either both `git-config-docker` and `git-config-docker.template` are tracked (as a pair), or both are ignored.
- **Actual:** `git-config-docker` is gitignored (`.gitignore:9`) but its companion template file `git-config-docker.template` is tracked in the repo. The non-template (which would contain actual credentials or personal git config) is correctly ignored, but the template is committed, creating an inconsistency.
- **Reasoning:** The pattern suggests `git-config-docker` is a personal Docker git config (correctly ignored), and `git-config-docker.template` is a committed placeholder. This is a conventional pattern and probably intentional, but worth confirming — if the template contains sensitive defaults it should be reviewed.
- **Failure scenario:** Low — this is largely cosmetic. The only risk is if the template contains sensitive default values that were committed.
- **Related:** None.
- **Where to look next:** `git-config-docker.template` contents (not read in this audit — only 47 bytes).
- **Fix direction:** Confirm the template contains only placeholder values (no real credentials). If intentional, this is fine. If it should be ignored, add `git-config-docker.template` to `.gitignore`.

## Clean bill

- **data/magic_tables.bin path**: `include_bytes!("../../../../data/magic_tables.bin")` in `backend/src/moves/magic/loader.rs:27` resolves correctly at compile time: four levels up from `backend/src/moves/magic/` lands at repo root, then `data/magic_tables.bin`. File is tracked. CWD does not matter for `include_bytes!`. Path is correct.
- **data/books/UHO_Lichess_4852_v1.epd was never committed**: `git log --diff-filter=A` returned empty. The 167 MB file is local-only and gitignored. Clone size is unaffected.
- **feature/frontend and feature/wasm are fully merged**: `git branch --merged main` lists both. Safe to delete with `git branch -d`.
- **Root strays are properly gitignored**: `game.001`, `log.001`, `Modelfile`, `.aider.*` files are all covered by existing `.gitignore` rules. They are untracked and ignored.
- **book.bin absence at runtime is graceful**: CLI uses `PolyglotBook::load("book.bin").ok()` — the `.ok()` discards the error. If the file is absent, the engine prints a warning and continues without the book. No crash.
- **run_gauntlet.py result regex**: `re.search(r"(\d+) - (\d+) - (\d+)", line)` on the "Score of Vantage vs" line is correct for standard cutechess-cli output format. No silent-null-match risk in the happy path.
- **.claude/settings.json.bak**: `.gitignore` line 8 ignores `.claude/`, so this file is not tracked. It is a local stray with no impact on clone integrity.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `VANTAGE_ROADMAP.md:143` | "SPRT bounds: H0=0, H1=10 for small features; H0=0, H1=20 for large features" — implies SPRT as acceptance gate | `run_gauntlet.py` has a 50%-threshold verdict, no LLR/SPRT | Every engine change decision since the gauntlet was written has been validated with the wrong test | Yes — every strength measurement is statistically invalid |
| `VANTAGE_ROADMAP.md:141` | "Short tests: 1000 games at 10+0.1 for quick validation" | `run_gauntlet.py:15` uses `TIME_CONTROL = "20+0.2"` and `DEFAULT_GAMES_PER_ROUND = 20` | All results are at the wrong time control and too few games | No, but results are non-comparable to roadmap baselines |
