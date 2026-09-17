# Incomplete Implementation Register

**Agent:** audit-claims
**Date:** 2026-08-30 (overnight run)

## How this file was built

`backend/src/` contains zero `todo!()`, `unimplemented!()`, `TODO`, `FIXME`, `XXX`, or `HACK` markers. Every row here was found by comparing an intent document (VANTAGE_ROADMAP.md, vantage-engine-pass.md, MCP_BUILD_PLAN.md, CLAUDE.md, README.md) against the actual source files. No speculation — every "What exists" cell has a confirmed file:line anchor.

This file is the **human-curated index**. Per-area audit files (`docs/audit/0*.md`) may add additional rows within their domain. When reconciling, trust the most specific file for implementation details and this file for cross-cutting impact.

---

## Engine

| Intent source (file:line) | What it promises (quoted) | What exists (file:line, or "nothing") | Blast radius | Blocking downstream work? |
|---|---|---|---|---|
| `VANTAGE_ROADMAP.md:33-36` | Phase 1.1 PVS — "Search first move with full window / Remaining moves: null window search, re-search with full window only on fail-high" listed as FUTURE work | Already implemented at `search.rs:445-544` — PVS is live | Roadmap is actively misleading; developer will re-implement something that works | YES — re-implementation would corrupt a working search |
| `VANTAGE_ROADMAP.md:39-40` | Phase 1.2 — "Countermove heuristic: `[64][64]` table indexed by previous move's from-to squares" | Nothing in `backend/src/search/` — grep returns empty | ~20-30 ELO gain deferred | No blocking dependency |
| `VANTAGE_ROADMAP.md:44-53` | Phase 1.3 eval batch — bishop pair, rook on open file, rook on 7th, knight mobility, queen mobility, backward pawns | Nothing found in `backend/src/search/eval.rs` — grep returns empty for all six features | ~40-60 ELO gain deferred | No blocking dependency |
| `VANTAGE_ROADMAP.md:60-64` | Phase 2.1 — Singular extension | Nothing in `search.rs` — grep for "singular" returns empty | ~30-50 ELO gain deferred | No blocking dependency |
| `VANTAGE_ROADMAP.md:67-70` | Phase 2.2 — IIR: "if no_hash_move && depth >= 4 { depth -= 1 }" | Nothing in `search.rs` — grep for "iir" and "internal.*iter" returns empty | ~10-20 ELO gain deferred | No blocking dependency |
| `vantage-engine-pass.md:35` | PVS PV table bug guard — "only update the PV on a full-window result (a true PV node), never on a null-window scout search" | PV IS updated on null-window cutoff at `search.rs:560-566` — the guard was not added | `SearchResult.pv` may be truncated; `better_line`/`refutation` in `AnalyzeMoveResult` may be short | No (move quality correct; PV quality impaired) |
| `vantage-engine-pass.md:28` | Optional `nodes` arm in `go` parser in `cli.rs` | Not present — `handle_go` has no `"nodes"` branch; `node_limit: None` hardcoded at `cli.rs:310,374` | CLI cannot test node budget manually | No (optional per spec) |

---

## MCP

| Intent source (file:line) | What it promises (quoted) | What exists (file:line, or "nothing") | Blast radius | Blocking downstream work? |
|---|---|---|---|---|
| `MCP_BUILD_PLAN.md:47-49` | `pub fn legal_moves(fen: &str) -> Result<Vec<String>, AnalysisError>` | No `legal_moves` function in `analysis/mod.rs` (grep returns empty) | MCP Phase 4 cannot wire the tool; startpos→20-move check has no target | YES — blocks MCP Phase 4 |
| `MCP_BUILD_PLAN.md:119-122` | `src/bin/vantage-mcp.rs` skeleton — stdio MCP server | File does not exist (`ls backend/src/bin/` shows only `cli.rs`) | No server binary; `claude mcp add` at Phase 5 has nothing to point at | YES — blocks Phases 3-5 |
| `MCP_BUILD_PLAN.md:78-101` | `EngineConfig` struct with `depth`, `tt_size_mb`, `features`, `engine_version`; `EvaluateResult` with `fen` echo and `config` fingerprint | `EvaluateResult` in `analysis/mod.rs:20-27` has `score, best_move, pv, depth, nodes` — no `fen` echo, no `config` field | Every response lacks the reproducibility fingerprint; grounding claim unverifiable from payload alone | No (analysis functions work; contract is richer than the spec requires for now) |
| `MCP_BUILD_PLAN.md:104` | `AnalyzeMoveResult` echoes thresholds: `{"inaccuracy": 50, "mistake": 100, ...}` | `AnalyzeMoveResult` at `analysis/mod.rs:29-38` has no `thresholds` field; thresholds hardcoded in `classify()` at `analysis/mod.rs:137-141` | Model must invent severity language rather than reading it from the structured payload | No (analysis works; quality of grounding reduced) |
| `MCP_BUILD_PLAN.md:201-205` | Four DoD checkboxes: vantage-mcp binary registered, three tools callable, demo transcript saved, README/status doc updated | None checked — binary absent, only 2/3 tools implemented, no transcript, README unchanged | Sprint is not done per its own definition | YES — sprint gate not passed |
| `CLAUDE.md:23-25` | "`vantage-mcp` — stdio MCP server exposing `evaluate`, `analyze_move`, `legal_moves`" in present tense | Server binary absent; `legal_moves` unimplemented | Any Claude Code session reading CLAUDE.md will attempt to use nonexistent MCP tools | YES — CLAUDE.md is the primary session prompt; present-tense description misleads every session |

---

## Infrastructure / Build

| Intent source (file:line) | What it promises (quoted) | What exists (file:line, or "nothing") | Blast radius | Blocking downstream work? |
|---|---|---|---|---|
| `CLAUDE.md:38` | `echo "uci\nisready\nposition startpos\ngo depth 6\nquit" \| ./backend/target/release/vantage` — a working run command | Binary is `vantage-cli` (`Cargo.toml:29`); `echo` without `-e` in bash and without processing in PowerShell does not emit newlines; command fails on both binary name and shell syntax | Any developer copying this command gets an immediate error | No engine functionality blocked; documentation only |
| `README.md:54` | "The executable will be located at `target/release/vantage`." | Binary is `target/release/vantage-cli` (`Cargo.toml:29`) | Player / developer cannot find or load the binary using the documented path | No engine functionality blocked; documentation only |
| `CLAUDE.md:36` | `--features psqt` in the WASM build command | Feature `psqt` declared at `Cargo.toml:38` has zero `#[cfg(feature = "psqt")]` sites in `backend/src/` — flag is inert | WASM build produces identical output with or without this flag; developer may believe they are toggling PSQT evaluation | No — WASM builds work; the flag is just dead |
