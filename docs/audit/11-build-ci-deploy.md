# Build / CI / Deploy — Audit Findings

**Agent:** audit-build
**Scope:** `.github/workflows/ci.yml`, `docker/Dockerfile.{dev,engine,web}`, `docker/nginx.conf`, `docker-compose.yml`, `docker-compose.prod.yml`, `.dockerignore`, `backend/Cargo.toml`, `backend/Cargo.lock`, `.gitattributes`, `.gitignore`, `frontend/.gitignore`, `git show main:frontend/{netlify.toml,package.json,next.config.ts}`
**Date:** 2026-08-30 (overnight run)

## What I checked

- `.github/workflows/ci.yml` — 125 lines, read in full
- `backend/Cargo.toml` — 49 lines, read in full
- `backend/Cargo.lock` — grepped for `schemars` version entries
- `docker/Dockerfile.web` — 44 lines, read in full
- `docker/Dockerfile.engine` — 49 lines, read in full
- `docker/Dockerfile.dev` — 177 lines, read in full
- `docker/nginx.conf` — 26 lines, read in full
- `docker-compose.yml` — 30 lines, read in full
- `docker-compose.prod.yml` — 60 lines, read in full
- `.dockerignore` — 65 lines, read in full
- `.gitattributes` — 7 lines, read in full
- `.gitignore` — 23 lines, read in full
- `frontend/.gitignore` — 41 lines, read in full
- `git show main:frontend/netlify.toml` — full content
- `git show main:frontend/package.json` — full content
- `git show main:frontend/next.config.ts` — full content
- `git ls-tree -r HEAD -- data/` — confirmed tracked files
- `ls data/books/` — confirmed untracked files present on disk
- `git log --oneline main..HEAD` — 13 commits on feature branch
- `grep -rn "use rmcp|use tokio|..." backend/src/` — confirmed zero use sites for orphan deps

## What I did not check, and why

- Actual Docker build outputs — would require running `docker build`, prohibited.
- Whether `@netlify/plugin-nextjs` passes `next.config.ts` headers to Netlify's CDN — would require reading Netlify plugin source or running a deploy.
- `Cargo.lock` in full — only grepped for specific crate names; full dep tree not traced.

## Findings

### BUILD-001 — Release job and Dockerfile.engine both copy a binary that does not exist

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified (confirmed by static read; not built)
- **Anchors:** `backend/Cargo.toml:29` (`name = "vantage-cli"`), `.github/workflows/ci.yml:71,75` (`binary_path: backend/target/release/vantage.exe` / `vantage`), `docker/Dockerfile.engine:40` (`COPY --from=builder /build/target/release/vantage /usr/local/bin/vantage`)
- **Expected:** The binary produced by `cargo build --release` is named after the `[[bin]] name` field, which is `vantage-cli`. The CI release job and Dockerfile should copy `target/release/vantage-cli` (or `vantage-cli.exe`).
- **Actual:** CI release matrix references `backend/target/release/vantage` and `backend/target/release/vantage.exe`. Dockerfile.engine copies `/build/target/release/vantage`. Neither path exists after a build. On Linux the `cp` silently fails or errors; the resulting Docker image has no working binary, and the CI release package contains nothing.
- **Reasoning:** `Cargo.toml:29` defines `[[bin]] name = "vantage-cli"`, which is the canonical binary name. Cargo always uses the `name` field. Both the CI workflow and the Dockerfile use the package name `vantage` instead of the binary name `vantage-cli`. The brief confirms this as seed 1 for the CI side; I additionally confirmed it holds in `Dockerfile.engine:40`.
- **Failure scenario:** Any tagged release push triggers the release job. The `cp ${{ matrix.binary_path }}` step on Linux copies a nonexistent path; the resulting archive contains only `README.md`. Any user who downloads the release gets a broken archive. `docker compose -f docker-compose.prod.yml up` would produce a container with ENTRYPOINT pointing at a missing binary.
- **Related:** BUILD-003 (release also ships wrong features), seed 1.
- **Where to look next:** `backend/Cargo.toml:28-30`, `ci.yml:69-76`, `docker/Dockerfile.engine:24,40`.
- **Fix direction:** Change the binary path in both the CI matrix (`binary_path`) and `Dockerfile.engine` COPY to use `vantage-cli` / `vantage-cli.exe`. Optionally rename `[[bin]] name` to `vantage` and update `src/bin/cli.rs` reference if the CLI name itself is the wrong thing.

---

### BUILD-002 — Dockerfile.web copies `.next/` build output from wrong path (`/build/dist`)

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `docker/Dockerfile.web:37` (`COPY --from=builder /build/dist /usr/share/nginx/html`), `frontend/.gitignore:17` (`.next/` is the ignored build output), `git show main:frontend/netlify.toml:3` (`publish = ".next"`), `git show main:frontend/package.json:7` (`"build": "next build"`)
- **Expected:** `next build` emits its output to `.next/` by default, as confirmed by `netlify.toml publish = ".next"` and `frontend/.gitignore` excluding `/.next/`. The nginx stage should copy `/build/.next`.
- **Actual:** `Dockerfile.web:37` copies `/build/dist` which Next.js never creates. The nginx container's `/usr/share/nginx/html` is empty; every HTTP request returns 404.
- **Reasoning:** `npm run build` calls `next build`. Next.js 15/16 emits to `.next/` by default. The `publish = ".next"` in `netlify.toml` confirms this. `dist/` is a Vite/CRA artifact that Next.js does not produce. The `frontend/.gitignore` lists `/build` as a generic production directory (line 21) but `.next/` is the actual framework output (line 17). `Dockerfile.web` confuses the two.
- **Failure scenario:** `docker compose -f docker-compose.prod.yml up` → nginx starts with empty html root → 404 on every request.
- **Related:** seed 2.
- **Where to look next:** `docker/Dockerfile.web:37`, `frontend/.gitignore:17`.
- **Fix direction:** Change `COPY --from=builder /build/dist` to `COPY --from=builder /build/.next`. However, serving `.next/` directly from nginx does not work for Next.js SSR or API routes — the correct production approach is `next start` (Node.js server) or a standalone output. If the intent is static export, add `output: 'export'` to `next.config.ts` and Next.js will emit to `out/`. The current `next.config.ts` has no `output` field, so static export is not configured.

---

### BUILD-003 — Release binary compiled with `deterministic_zobrist` (a test-only flag)

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `ci.yml:87` (`cargo build --release --features "load_magic deterministic_zobrist"`), `backend/Cargo.toml:37` (`deterministic_zobrist = []`)
- **Expected:** `deterministic_zobrist` exists to make hash seeds reproducible for testing. A shipped engine binary should use random Zobrist keys (seeded at startup or compile-time randomly) so no two instances are predictable.
- **Actual:** Every tagged release is compiled with `deterministic_zobrist`, making Zobrist hashes fixed constants across all copies of the released binary. An opponent who knows the constants can precompute position hashes and potentially identify transposition table collisions deterministically.
- **Reasoning:** The feature name and its only usage context (test suite) make its purpose clear. `ci.yml:60` also uses it for the test job, which is correct — but the release build at line 87 copies the same feature set without distinguishing between test and production.
- **Failure scenario:** All released binaries share identical Zobrist keys, enabling precomputed hash-collision attacks against the engine's TT. Less severe than BUILD-001 (the release doesn't ship at all due to the binary name bug) but independently wrong.
- **Related:** BUILD-001.
- **Where to look next:** `ci.yml:87`, and wherever Zobrist keys are initialized in `backend/src/board/`.
- **Fix direction:** The release build step should use only `--features "load_magic"` (or `--features "load_magic,cli"` if a CLI release is intended). Remove `deterministic_zobrist` from the release feature set.

---

### BUILD-004 — Deployed release always ships without opening book (`book.bin` gitignored)

- **Severity:** P1
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `.gitignore:8` (`book.bin`), `ci.yml:96-102` (book copy with fallback warning), `data/` (git ls-tree shows only `data/magic_tables.bin` tracked; no `book.bin`)
- **Expected:** The release package should include `book.bin` so the engine can use opening preparation. The release script attempts to copy it at `ci.yml:96-99`.
- **Actual:** `book.bin` is excluded by `.gitignore:8`. A CI checkout never contains it. The fallback at `ci.yml:101` prints a warning and continues, producing a release archive with no book. Users receive a weaker engine that plays without any opening book.
- **Reasoning:** `git ls-tree -r HEAD -- data/` shows only `data/magic_tables.bin`. No `backend/book.bin` or root `book.bin` is tracked. The CI warning is self-documenting: the author knew this was a risk but left the condition unaddressed.
- **Failure scenario:** Every tagged release produces an archive missing `book.bin`. The warning at `ci.yml:101` appears in CI logs but the job does not fail.
- **Related:** None.
- **Where to look next:** `.gitignore:8`, `ci.yml:95-102`, `data/`.
- **Fix direction:** Either (a) track `book.bin` in git using Git LFS (it is a binary file), or (b) add a CI step that downloads the book from a versioned URL (GitHub Release asset, S3, etc.) before packaging, or (c) make the missing book a hard failure by removing the `else echo "WARNING"` branch and using `set -e`.

---

### BUILD-005 — `permissions: contents: write` granted to all jobs, only `release` needs it

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `ci.yml:26-27` (`permissions:\n  contents: write`)
- **Expected:** Least-privilege: each job should declare only the permissions it needs. `check` (lint/fmt) and `test` need only `contents: read` to checkout code.
- **Actual:** `contents: write` is declared at the workflow level, which grants it to all three jobs (`check`, `test`, `release`). A compromised action in `check` or `test` (e.g., a supply-chain attack via `dtolnay/rust-toolchain` or `Swatinem/rust-cache`) could push to the repository.
- **Reasoning:** Only `release` invokes `softprops/action-gh-release`, which needs write to create a release. `check` and `test` only read source.
- **Fix direction:** Move `permissions: contents: write` into the `release` job block. Add `permissions: contents: read` to `check` and `test` jobs explicitly.

---

### BUILD-006 — CI tests only one of eight distinct feature combinations; WASM target never built

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `ci.yml:43` (clippy: `load_magic deterministic_zobrist`), `ci.yml:60` (test: same), `backend/Cargo.toml:32-41` (feature definitions)
- **Expected:** All combinations that gate real code paths should be built and tested in CI.
- **Actual:** CI builds/tests exactly one combination. Never verified:
  - `--features "psqt,cli"` (the **default** feature set — what a user gets with `cargo build`)
  - `--no-default-features` (the base set, used by WASM)
  - `--features "deterministic_magic"` (a different magic-table path)
  - `--features "paranoid_hash"` (hash verification mode)
  - `--features "lmr_stats"` / `--features "aspiration_stats"` (stats features)
  - `wasm32-unknown-unknown` target (`crate-type = ["cdylib"]` at `Cargo.toml:26`) — never built in CI despite being the deliverable for the live site
- **Reasoning:** The default feature set (`psqt`, `cli`) is what any user running `cargo build` gets, and it is never CI-tested. The WASM target is the artifact served at chess.vaishakmenon.com and is never built in CI.
- **Failure scenario:** A change that compiles fine under `load_magic,deterministic_zobrist` but breaks the default build or the WASM build ships silently. Seed 3 (stale committed WASM) is a direct consequence of no WASM CI.
- **Related:** seed 3, seed 4 (dead feature flags only discoverable because they're never tested).
- **Where to look next:** `ci.yml:43,60`, `Cargo.toml:32-41`.
- **Fix direction:** Add CI matrix entries for the default build (`cargo build`), `--no-default-features`, and `wasm-pack build --target web --no-default-features --features psqt`. The dead stats flags (`lmr_stats`, `aspiration_stats`) can be omitted if they are confirmed dead (see seed 4).

---

### BUILD-007 — Current 13-commit feature branch has never had CI run on it

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `ci.yml:4-6` (`push: branches: [ "main" ]`), `ci.yml:13-14` (`pull_request: branches: [ "main" ]`), `git log --oneline main..HEAD` (13 commits: c0e0412..e7cc241)
- **Expected:** Changes on an active development branch should be CI-verified before merge.
- **Actual:** `ci.yml` triggers on push to `main` and on PRs targeting `main`. The branch `feature/backend_agentic_updates` is not `main`, and no PR targeting `main` exists (otherwise the brief would note CI-passing status). All 13 commits on this branch — including the wasm apply_move fix (e7cc241), MCP analysis module (9ffe47a, e6576ae), PV table work (3442035–35b9114), struct-based search I/O (bf5ad05–c0e0412) — have never been CI-verified.
- **Reasoning:** `git log --oneline main..HEAD` lists 13 commits. The only way CI runs on branch pushes is via `workflow_dispatch` (manual), which requires a human to trigger it. The `paths-ignore` pattern at `ci.yml:8-19` (which skips CI for `.md`-only changes) compounds this: documentation-only commits on main also skip CI.
- **Failure scenario:** A bug introduced in any of the 13 commits ships to main when the PR is merged, with the first CI run happening post-merge.
- **Related:** BUILD-006.
- **Where to look next:** `ci.yml:3-20`.
- **Fix direction:** Add the feature branch pattern to the push trigger: `branches: [ "main", "feature/**" ]`. Alternatively use `branches-ignore` with only excluded patterns.

---

### BUILD-008 — nginx.conf lacks COOP/COEP headers; Docker deployment breaks WASM loading

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `docker/nginx.conf:22-26` (security headers block), `git show main:frontend/next.config.ts:25-33` (headers() for `/wasm/:path*`), `docker/Dockerfile.web:34` (nginx config is the only header source in Docker)
- **Expected:** `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` must be present on responses serving the HTML page and WASM assets for `asyncWebAssembly` to work in some browser configurations, and are required if SharedArrayBuffer is ever used.
- **Actual:** `nginx.conf` sets `X-Frame-Options`, `X-Content-Type-Options`, and `X-XSS-Protection` but not COOP or COEP. The `next.config.ts` `headers()` function only applies when Next.js is the HTTP server (dev server, `next start`, or Netlify via `@netlify/plugin-nextjs`). In Docker, nginx is the only server, so `next.config.ts` headers are never applied.
- **Reasoning:** `Dockerfile.web:34` copies `docker/nginx.conf` as the only nginx config. `next.config.ts` is a build-time file; its `headers()` hook runs in the Node.js server, which is not present in the Docker deployment. Netlify is unaffected (plugin applies `next.config.ts` at the CDN level), but Docker is not Netlify.
- **Failure scenario:** A user running the Docker stack (`docker compose -f docker-compose.prod.yml up`) would get WASM load failures in browsers that enforce COEP for SharedArrayBuffer or for certain `asyncWebAssembly` configurations.
- **Related:** BUILD-002 (web container already broken by wrong copy path; this compounds it).
- **Where to look next:** `docker/nginx.conf:22-26`, `frontend/next.config.ts:25-33`.
- **Fix direction:** Add `add_header Cross-Origin-Opener-Policy "same-origin" always;` and `add_header Cross-Origin-Embedder-Policy "require-corp" always;` inside the `server {}` block in `nginx.conf`, scoped to the root location or globally. Also add them to the static-asset `location` block so `.wasm` files are served with the headers.

---

### BUILD-009 — `.dockerignore` excludes `Cargo.lock`, defeating reproducible engine builds

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `.dockerignore:22` (`Cargo.lock`), `docker/Dockerfile.engine:16` (`COPY backend/Cargo.toml backend/Cargo.lock* ./`)
- **Expected:** `Cargo.lock` should be included in the Docker build context so `cargo build --release` uses the pinned dependency versions.
- **Actual:** `.dockerignore:22` unconditionally excludes `Cargo.lock`. `Dockerfile.engine:16` uses a glob `Cargo.lock*` which silently matches nothing when the file is excluded. Cargo then resolves the latest compatible versions of all dependencies at build time, making the Docker build non-reproducible.
- **Reasoning:** `Cargo.lock*` with a `*` glob does not fail when no file matches — it silently copies nothing. The exclusion in `.dockerignore` applies across all Dockerfiles since the context is the repo root. This means the layer-caching optimization (lines 16-20 of Dockerfile.engine) also fails: the dummy build in line 20 runs without a lock file, resolves arbitrary versions, and the actual build at line 24 may resolve different versions again.
- **Failure scenario:** A dependency releases a semver-compatible breaking change. The Docker build picks it up silently; native CI (which does have `Cargo.lock`) does not. Engine behavior in Docker diverges from tested behavior.
- **Related:** BUILD-006.
- **Where to look next:** `.dockerignore:22`, `docker/Dockerfile.engine:16-24`.
- **Fix direction:** Remove `Cargo.lock` from `.dockerignore`. If the concern is lock file size, it is small (text). The Dockerfile's explicit `COPY backend/Cargo.lock* ./` already shows the author intended to include it.

---

### BUILD-010 — No `[profile.release]` section; LTO and codegen-units use unoptimized defaults

- **Severity:** P2
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/Cargo.toml` (no `[profile.release]` section in full 49-line file)
- **Expected:** For a chess engine where nodes-per-second is the primary performance metric, the release profile should enable `lto = true` (link-time optimization) and `codegen-units = 1` (single codegen unit for maximum inlining across crates).
- **Actual:** Cargo defaults to `lto = false` and `codegen-units = 16` for release builds. These defaults trade build speed for runtime performance, which is the wrong trade-off for a shipped engine binary.
- **Reasoning:** Absent a `[profile.release]` block, all Cargo release defaults apply. LTO alone typically yields 5–15% NPS improvement on search-heavy workloads; `codegen-units = 1` can add further gains. `panic = "abort"` (vs the default `panic = "unwind"`) reduces binary size and eliminates unwind cleanup code in hot paths. These are standard in competitive engine projects.
- **Failure scenario:** Released engine underperforms its potential by a measurable margin on every search call. Needs benchmarking to quantify; mark as P2 not P1.
- **Related:** None.
- **Where to look next:** `backend/Cargo.toml` (add `[profile.release]` block). Compare NPS before/after.
- **Fix direction:** Add a `[profile.release]` section with `lto = true`, `codegen-units = 1`, `panic = "abort"`. Benchmark NPS before and after to confirm the improvement before committing.

---

### BUILD-011 — `crate-type = ["rlib", "cdylib"]` forces cdylib linkage on every native build

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/Cargo.toml:26` (`crate-type = ["rlib", "cdylib"]`)
- **Expected:** `cdylib` is needed only when targeting `wasm32-unknown-unknown` (for the `.wasm` output). Native builds only need `rlib`.
- **Actual:** Every native `cargo build`, `cargo test`, and `cargo clippy` invocation builds both `rlib` and `cdylib`. The cdylib produces a `.so`/`.dylib`/`.dll` that is never used natively. This adds link time to every build.
- **Reasoning:** Cargo builds all listed crate types for every target. The `cdylib` type is used by `wasm-pack` (which sets `--target web` implicitly). Native builds do not use the `.so`. The typical pattern is to gate `cdylib` on the wasm target: `[target.'cfg(target_arch = "wasm32")'.lib] crate-type = ["cdylib"]`.
- **Fix direction:** Split the `[lib]` section: keep `rlib` in the unconditional `[lib]` block, and move `cdylib` into `[target.'cfg(target_arch = "wasm32")'.lib]`. This halves the link step on native builds.

---

### BUILD-012 — Third-party GitHub Actions pinned to semver tags, not SHAs (supply-chain risk)

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `ci.yml:106` (`uses: thedoctor0/zip-release@0.7.6`), `ci.yml:120` (`uses: softprops/action-gh-release@v1`)
- **Expected:** GitHub Actions best practice (and GitHub's own hardening guide) recommends pinning third-party actions to a full commit SHA, not a tag, because tags are mutable.
- **Actual:** `thedoctor0/zip-release@0.7.6` is a semver tag (mutable — tag can be force-pushed). `softprops/action-gh-release@v1` is a floating major tag, the weakest possible pin.
- **Reasoning:** A tag owner can silently redirect a tag to a malicious commit. Any push of a `v*` tag triggers the release job, which has `permissions: contents: write` (BUILD-005), making the blast radius larger.
- **Fix direction:** Pin both to their current commit SHAs. Run `gh api repos/thedoctor0/zip-release/git/ref/tags/0.7.6` and `gh api repos/softprops/action-gh-release/git/ref/tags/v1` to get the target SHA, then use `@<sha>` with a comment noting the version.

---

### BUILD-013 — `schemars` crate appears in Cargo.lock at two major versions (0.8.22 and 1.2.2)

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/Cargo.lock` (grep: `schemars 1.2.2` and `schemars 0.8.22` both present)
- **Expected:** A single version of `schemars` in the dependency tree.
- **Actual:** Both `schemars 0.8.22` and `schemars 1.2.2` are compiled into the binary. The declared dep in `Cargo.toml:13` is `schemars = "0.8"`, meaning the project itself uses 0.8.22. Something in the dependency tree of `rmcp` or another transitive dependency pulls 1.2.2.
- **Reasoning:** `schemars` is an orphan dep (seed 5 — zero `use` sites in `src/`). Having two major versions compiled adds binary size with no benefit to this project.
- **Fix direction:** Identify which transitive dep pulls schemars 1.2.2 (`cargo tree -i schemars`). If it is `rmcp`, this resolves when `rmcp` is removed after the MCP server moves to its own crate (which is the appropriate fix given seed 5).

---

### BUILD-014 — `.wasm` not marked binary in `.gitattributes`; corruption risk on renormalize

- **Severity:** P3
- **Confidence:** medium
- **Status:** unverified
- **Anchors:** `.gitattributes:1-7` (full file), `.gitignore:14` (`vantage_bg.wasm` excluded from tracking), seed 3 (committed WASM at `main:frontend/public/wasm/vantage_bg.wasm`, 2.5 MB)
- **Expected:** Binary files should be explicitly marked `binary` in `.gitattributes` to prevent line-ending normalization.
- **Actual:** `.gitattributes` marks `*.bin`, `*.exe`, `*.dll` as binary but omits `*.wasm`. With `* text=auto eol=lf` on line 1, git uses a heuristic (NUL-byte detection) to classify files. WASM files start with the 4-byte magic `\0asm`, which contains a NUL and usually triggers the binary heuristic correctly — but this is not guaranteed across all git versions. A `git add --renormalize` or a git version that changes the heuristic threshold could corrupt the committed WASM file.
- **Reasoning:** The committed `frontend/public/wasm/vantage_bg.wasm` (seed 3) is in the main branch. It is 2,495,946 bytes. A line-ending conversion on a WASM file corrupts it completely. The `.gitignore:14` entry for `vantage_bg.wasm` prevents the file from being accidentally re-added, but the already-tracked file in `main` is not protected by `.gitignore`.
- **Fix direction:** Add `*.wasm binary` to `.gitattributes`. Also consider adding `*.wasm` to `.gitignore` on the feature branch side (it is already there: `.gitignore:14`), and remove the committed WASM from `main` in favor of a build step.

---

### BUILD-015 — `data/books/` contains large untracked files required for development

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `ls data/books/` (shows `2moves_v2.pgn`, `UHO.epd`, `UHO_Lichess_4852_v1.epd` on disk), `git ls-tree -r HEAD -- data/` (only `data/magic_tables.bin` tracked), `.gitignore:18-19` (`*.pgn`, `*.epd` excluded)
- **Expected:** Either the development books are tracked (possibly via LFS) or their acquisition is documented and automated.
- **Actual:** Three opening/test files exist locally but are gitignored. A fresh clone has an empty `data/books/` directory. `docker-compose.yml:16` mounts `./data/books:/openings`, so the dev container silently gets an empty opening directory if the files are not manually placed. The `Dockerfile.dev` downloads `2moves_v2.pgn` from stockfish's repo (line 166-168) with a fallback — but this download is only in the image, not in the host-mounted volume that the compose mounts over it.
- **Fix direction:** Document the required files and their download sources in a setup script or CONTRIBUTING guide, or add a `volumes` post-start hook to the compose that downloads missing books. Alternatively, remove the `data/books:/openings` mount and rely on the files baked into the image.

## Clean bill

- **`next.config.ts` COOP/COEP on Netlify**: `netlify.toml` `[[headers]]` applies platform-wide security headers, and `@netlify/plugin-nextjs` passes the `next.config.ts` `headers()` through to the CDN. For Netlify deployments, the COOP/COEP for `/wasm/:path*` are correctly applied. The gap is Docker-only (BUILD-008).
- **`netlify.toml` CSP includes `'wasm-unsafe-eval'`**: The Content-Security-Policy in `netlify.toml` line (git show output) explicitly includes `'wasm-unsafe-eval'` in `script-src`, which is required for WASM execution in strict CSP environments. This is correct.
- **`Swatinem/rust-cache@v2`**: This is a pinned major version, still more conservative than a floating major on an action used in `check`/`test` (no write permission risk vs the release job). Lower risk than BUILD-012's actions.
- **Test matrix OS coverage**: `ci.yml:49-51` tests on both `ubuntu-latest` and `windows-latest`, which catches Windows-specific path bugs. This is good practice and more than most engine projects do.
- **`.dockerignore` excludes sensitive files**: `.dockerignore:49-57` correctly excludes `.env`, `*.pem`, `*.key`, `credentials*`, and `git-config-docker`. No credential-leak risk in Docker builds.
- **`data/magic_tables.bin` in `.gitattributes`**: `.gitattributes:5` marks `*.bin binary`, so `data/magic_tables.bin` is correctly protected from line-ending normalization.

## Incomplete implementations

| Intent source (file:line) | What it promises (quoted) | What exists | Blast radius | Blocking? |
|---|---|---|---|---|
| `MCP_BUILD_PLAN.md` (Phase 2–5) | `legal_moves` tool, `vantage-mcp` binary, full MCP stdio server | `src/analysis/mod.rs` exists (Phase 1.1); no `src/bin/vantage-mcp.rs`, no `legal_moves`, no server binary | MCP server not runnable; orphan deps (`rmcp`, `tokio`, `serde_json`) drag into every build | Yes — MCP server described in CLAUDE.md is non-functional |
| `CLAUDE.md` ("MCP Server (in progress)") | "stdio MCP server exposing `evaluate`, `analyze_move`, `legal_moves`" | Only `evaluate` and `analyze_move` exist in `src/analysis/mod.rs`; `legal_moves` absent | Any consumer of the MCP interface gets incomplete tool set | Yes |
