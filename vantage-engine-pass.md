# Vantage Engine Pass — Work Plan

Two enhancements + a determinism harness that acts as the acceptance gate. Build and test everything with `--features load_magic,deterministic_zobrist`, book disabled. Stop when the 6 harness tests are green — nothing more.

A good first Claude-Code-driven unit: self-contained Rust, with the harness as objective verification.

---

## Task 0 — Signature refactor (do first, it touches the others)

You're adding `node_limit` (input) and `pv` (output) to search. Rather than grow the tuple/param list twice, introduce:

- `SearchLimits { max_depth: i32, node_limit: Option<u64>, time_limit: Option<Duration> }`
- `SearchResult { score: Score, best_move: Option<Move>, pv: Vec<Move>, depth: i32, nodes: u64 }`

`search(board, tables, tt, limits) -> SearchResult`. Get the shape right once — the MCP server consumes this directly.

**Ripple to update (keep them compiling/working):** `handle_go` in `cli.rs`, and `go_depth`/`go_movetime` in `wasm.rs`. WASM just passes `node_limit: None` and ignores `pv` — chess.vaishakmenon.com behavior must be unchanged.

---

## Task 1 — Node budget

- Plumb `node_limit` into the search; check it alongside the existing `check_time()` (`search.rs:87-101`), on the node counter, never time-based (stays deterministic).
- **Semantics (confirmed):** on cap, return the result from the **last fully completed iterative-deepening depth**, not the partial current one. So the ID loop must hold the last completed `SearchResult` and, if a depth is aborted by the cap, discard that depth's partial result and return the previous one.
- Implement the abort as a flag that propagates up; the ID loop commits a depth's result only if it completed.
- Define the degenerate case (cap below depth-1 completion): return depth-1's best-so-far, or error — pick one and test it.
- Optional: add a `nodes` arm to the `go` parser in `cli.rs` for manual binary testing. The MCP server uses the lib, so this is convenience only.

---

## Task 2 — PV extraction

- **Triangular PV table** — collect the line during search, prepend the current move to the child's PV when a move becomes the new best at a node. **Not** TT-walking (unreliable, and you clear the TT per query).
- **PVS gotcha (Vantage uses PVS):** only update the PV on a full-window result (a true PV node), never on a null-window scout search — otherwise the PV comes out truncated or wrong. This is the classic PV+PVS bug; budget time for it.
- **Travels as a unit:** `pv`, `score`, and `best_move` must all come from the *same* completed depth. When the node cap aborts a depth, you return the last completed depth's `pv` too — never a deeper partial PV with a shallower score.
- **Phase-1 decision:** collect PV through the main search only (ends at the depth horizon). Quiescence-PV (extending the line through qsearch captures) is a later refinement — skip it now.
- **Invariants:** `pv[0] == best_move`; every PV move is legal in sequence when replayed from the root FEN; `pv.len() <= depth`.

---

## Task 3 — Determinism harness (the gate)

Tests run via the **lib API** (the real code path the server uses), deterministic build, small TT (64MB), book off, `tt.clear()` between queries. Positions: startpos, kiwipete, and the middlegame FEN `r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8`.

1. **Reproducibility:** same `(FEN, depth)` → byte-identical `score`, `nodes`, `best_move` across two runs.
2. **PV reproducibility:** same `(FEN, depth)` → byte-identical `pv` across two runs.
3. **PV consistency:** `pv[0] == best_move`; replay the PV from the FEN and assert every move is legal; `pv.len() <= depth`.
4. **Node-cap reproducibility:** same `(FEN, node_limit)` → byte-identical `score`, `best_move`, `pv`, and nodes-at-stop across two runs.
5. **Node-cap semantics:** record `N_d` (nodes to complete depth *d*) and `N_{d+1}`. Run a capped search with `node_limit` between them; assert its result equals the plain depth-*d* result. Confirms "last completed depth."
6. **Purity / order-independence:** search A then B gives the same B result as searching B alone (validates per-query `tt.clear()`).

**Definition of done:** all 6 green in the deterministic build. That's the signal it's safe to build the MCP server on top.

---

## Explicitly out of scope

MultiPV, quiescence-PV, UCI `setoption`, any engine-strength work. If you find yourself "in here anyway," stop — that's the rabbit hole.

---

## Downstream note (for when you're back)

These lock the tool contract: the MCP tools will take a `nodes` budget (not `depth`) and `evaluate`/`analyze_move` will return `pv`. We wire those into the tool schemas when we build M2 (the MCP server).
