# Vantage MCP Server — Guided Build Plan

**Goal:** `vantage-mcp`, a stdio MCP server exposing `evaluate`, `analyze_move`, and `legal_moves`, registered in Claude Code, with one end-to-end grounded-analysis demo.

**Method:** You write all production code. Claude Code explains concepts, shows *toy* examples and signature-only stubs, and reviews your work. It does not edit your files.

**Estimated effort:** 5 sessions, one evening each.

---

## Phase 0 — Instructor mode + orientation (~1 hr)

### 0.1 Build the mental model (no code yet)

Ask Claude Code, in order:

1. *"Explain the MCP protocol lifecycle for a stdio server: initialize handshake, tools/list, tools/call. Show me what the raw JSON-RPC messages look like for a toy 'add two numbers' tool. Don't reference my project."*
2. *"Explain how the rmcp crate maps onto that lifecycle: ServerHandler, the #[tool] macro, ServiceExt::serve with a stdio transport. Toy example only."*
3. *"What does the #[tool] macro generate from a function signature? How do parameter schemas get derived (schemars), and where does the tool description string come from?"*

**Checkpoint — you can answer without notes:** What three things does a server tell the client during initialize? Where does a tool's JSON schema come from in rmcp? Why must a stdio server never print to stdout except protocol messages? (That last one *will* bite you — your engine logs go to stderr or a file, never stdout.)

### 0.2 Dependencies

Add to `Cargo.toml` yourself (ask Claude Code which rmcp features you need for a stdio server, and current version): `rmcp`, `tokio`, `serde`, `serde_json`, `schemars`.

**Done when:** deps compile, CLAUDE.md updated, checkpoint questions answered.

---

## Phase 1 — Rename the module + add `legal_moves` (~1 evening)

The module is named `mcp::` but it's about to have two consumers (MCP server now, batch labeling CLI later). Fix the name while it's cheap.

### 1.1 Rename `src/mcp/` → `src/analysis/`

Do it by hand (or `git mv` + fix imports). Update `tests/mcp_tests.rs` imports; consider renaming to `analysis_tests.rs`. This is deliberately mechanical — it re-familiarizes you with the module before you extend it.

### 1.2 Implement `legal_moves`

You have `find_legal_move` (UCI → validated Move). You need the enumeration counterpart. Stub to fill:

```rust
/// All legal moves for the given position, as UCI strings, sorted.
/// Errors on invalid/unparseable FEN (reuse the same FEN error path
/// evaluate() uses — one error type for the whole module).
pub fn legal_moves(fen: &str) -> Result<Vec<String>, AnalysisError> {
    todo!()
}
```

You already have movegen; this is a thin wrapper. The design questions worth thinking about (discuss with Claude Code before coding): output ordering (sort for determinism — everything this module returns should be byte-stable), promotion encoding (four UCI strings per promotion push?), and whether to reuse or refactor the FEN-parse error handling shared with `evaluate`.

### 1.3 Tests you write

- startpos → exactly 20 moves
- kiwipete → 48 moves (you know this number from perft)
- a stalemate FEN → empty vec, not an error
- a position with promotions → all four promotion pieces present
- malformed FEN → clean error, no panic
- determinism: two calls, identical vecs

Then ask: *"Review my legal_moves implementation and tests. What edge cases am I missing?"*

**Done when:** all tests green, module renamed, `VANTAGE_ROADMAP.md` untouched (engine work is out of scope — tripwire).

---

## Phase 2 — Structured output types (~1 evening)

The grounding claim ("every statement traces to a tool result") only works if tools return machine-checkable JSON, not prose. This phase is pure type design — the highest-leverage hour of the project.

### 2.1 Design the response types

Sketch these yourself first, on paper or in a scratch file, *then* compare notes with Claude Code. Target shape:

```rust
#[derive(Serialize, JsonSchema)]
pub struct EngineConfig {
    pub depth: u8,
    pub tt_size_mb: u32,
    pub features: String,      // e.g. "load_magic,deterministic_zobrist"
    pub engine_version: String,
}

#[derive(Serialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ScoreJson {
    Cp(i32),
    Mate(i32),   // negative = getting mated
}

#[derive(Serialize, JsonSchema)]
pub struct EvaluateResult {
    pub fen: String,           // echo input — makes transcripts self-contained
    pub score: ScoreJson,
    pub best_move: String,
    pub pv: Vec<String>,
    pub nodes: u64,
    pub config: EngineConfig,  // the reproducibility fingerprint, every response
}
```

Design `AnalyzeMoveResult` yourself: it needs the position eval, the played move, the post-move eval, cp_loss, the classification, the refutation PV, and the thresholds used (`{"inaccuracy": 50, "mistake": 100, ...}` — echoing thresholds in the payload means Claude never has to invent severity language).

Questions to work through with Claude Code (as discussion, not dictation): tagged vs untagged enum serialization for Score and why it matters for the JSON schema; whether classification should be a Rust enum serialized to snake_case; what happens to schema quality if you use `Option` fields.

### 2.2 Tests

Snapshot-style serialization tests: build a known result, `serde_json::to_string`, assert exact expected JSON. This locks the wire format — any future refactor that changes it fails loudly.

**Done when:** both result types serialize to stable, readable JSON; existing analysis functions refactored to return them (or thin conversion functions added); all Phase-1 tests still green.

---

## Phase 3 — Server binary + first tool (~1 evening)

### 3.1 Skeleton

Create `src/bin/vantage-mcp.rs` yourself. Ask: *"Show me a minimal rmcp stdio server with ONE tool ('echo': takes a string, returns it). Toy code, in chat. Explain each part: the handler struct, tool_router, the tokio main, serve(stdio())."*

Then write yours, adapting the pattern. Skeleton target:

```rust
// vantage-mcp: stdio MCP server over the analysis:: module.
// RULE: nothing writes to stdout except the protocol. Engine/debug
// output → stderr.

struct VantageServer {
    // decide: construct engine state per-call (your determinism
    // contract already clears TT per query — does per-call
    // construction make that redundant or simpler?)
}

#[tool(description = "…")]  // Phase 4 refines descriptions
async fn evaluate(&self, /* params struct */) -> /* result */ {
    todo!()
}
```

Design decision to reason through before coding (good discussion prompt): your engine functions are synchronous and CPU-heavy; rmcp handlers are async. Ask Claude Code about `tokio::task::spawn_blocking` vs just blocking the handler, and what actually matters for a single-user local server. (Spoiler: for your use case the simple answer is fine — but know *why*.)

### 3.2 Wire `evaluate` only

One tool end-to-end before adding the rest.

### 3.3 Verify without Claude Code first

Two levels, in order:

1. Raw: run the binary, paste an `initialize` request JSON into stdin by hand (you saw the shape in Phase 0), then `tools/list`, then a `tools/call` for evaluate on startpos. Watching the raw protocol once is worth an hour of debugging later.
2. Tooling: `npx @modelcontextprotocol/inspector -- path/to/vantage-mcp` (or `cargo run --bin vantage-mcp --features load_magic,deterministic_zobrist` as the command) and exercise the tool in the inspector UI.

**Done when:** inspector shows the server connecting, lists `evaluate` with a correct schema, and a call on startpos returns your Phase-2 JSON with the config fingerprint.

---

## Phase 4 — Remaining tools, descriptions, errors (~1 evening)

### 4.1 Wire `analyze_move` and `legal_moves`

Same pattern as evaluate. You'll move faster; budget the saved time for 4.2.

### 4.2 Tool descriptions are prompts — write them like it

Draft all three yourself, then ask Claude Code to critique *as the model that will read them*: *"Here are my three MCP tool descriptions. You're the LLM consuming them: what would you misunderstand? What would make you call the wrong tool or misreport a result?"*

Requirements for `analyze_move`'s description: states the cp-loss thresholds and category names verbatim; states that side-to-move perspective is handled (or how); tells the caller the refutation line is engine-best play, at what depth. For `evaluate`: what the score means (cp from side-to-move? white's view? — whatever your convention is, say it), and that results are deterministic at fixed depth.

### 4.3 Error handling

Every failure path returns a proper MCP tool error with a message useful to an LLM ("invalid FEN: rank 3 has 9 squares" beats "parse error"). Test by calling each tool through the inspector with: malformed FEN, illegal move, well-formed-but-impossible FEN if your parser distinguishes. Nothing panics; the server survives every bad call.

**Done when:** three tools callable in inspector, descriptions reviewed, all error paths exercised by hand.

---

## Phase 5 — Register, demo, document (~1 short evening)

### 5.1 Build a release binary and register it

```
cargo build --release --bin vantage-mcp --features load_magic,deterministic_zobrist
claude mcp add --transport stdio vantage -- <absolute-path-to>/vantage-mcp.exe
claude mcp list        # verify connected
```

Options before the server name, `--` before the command. On Windows, absolute path to the .exe avoids shell-resolution issues. If it shows disconnected: check nothing prints to stdout on startup, and ask Claude Code to help read the MCP logs.

### 5.2 The demo — this is the deliverable

In a fresh Claude Code session with the server connected, run one real analysis: give it a middlegame FEN and a dubious candidate move, ask for an assessment. Then audit the transcript: does every factual claim (eval, classification, refutation) trace to a tool result? Does the model quote your thresholds instead of inventing severity? Save the transcript (or record a short clip) — this artifact is the project's proof, for the portfolio and for future-you.

If the model freelances claims → your Phase-4 descriptions need tightening. That loop (observe → tighten description → rerun) is the actual craft here; expect 2-3 iterations.

### 5.3 Close it out

- `MCP_STATUS.md` (or README section): what exists, how to build/register, the demo transcript link, and this checklist marked done.
- README: a short "Vantage as MCP server" section with the three tools and the determinism contract.

**Definition of done (from the sprint agreement):**
- [ ] `vantage-mcp` binary speaks stdio MCP, registered in Claude Code
- [ ] `evaluate`, `analyze_move`, `legal_moves` callable, structured JSON out, config fingerprint in every response
- [ ] One end-to-end grounded demo transcript saved
- [ ] README/status doc updated

**Then stop.** No hosting, no HTTP transport, no batch endpoints, no extra tools. The batch labeling CLI is HRM week-one work, on the `analysis::` module you just finished naming correctly.

---

## Standing prompts (copy-paste as needed)

- *"Explain X with a toy example unrelated to my project."*
- *"Here's my implementation of X — review for correctness, idiom, and edge cases. Don't rewrite it; point at problems."*
- *"Quiz me on what we covered before I start coding."*
- *"What would break if I did X instead?"* (use whenever a design choice feels arbitrary)

## Anti-drift rules

1. One phase per session. Finishing early → review/quiz, not next phase.
2. Any engine-strength idea that surfaces goes in `VANTAGE_ROADMAP.md`, not this sprint.
3. Any HRM idea that surfaces goes in a `hrm-notes.md` parking lot, not this sprint.
4. If Claude Code writes project code, delete it, re-read CLAUDE.md teaching mode, retype the solution yourself if it was right.