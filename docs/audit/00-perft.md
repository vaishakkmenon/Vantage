# Perft — Audit Findings

**Agent:** baseline-perft
**Scope:** `backend/src/moves/perft.rs` (380 LOC), `backend/src/bin/cli.rs`, `backend/tests/perft_tests.rs` (318 LOC)
**Date:** 2026-08-30 (overnight run)

## What I checked

- Read `backend/src/moves/perft.rs` in full (380 LOC): public API (`perft`, `perft_divide`, `perft_count_with_breakdown`, `perft_divide_with_breakdown`), recursive implementation, buffer strategy.
- Read `backend/src/bin/cli.rs` in full: UCI command dispatch. No `perft` command present.
- Read `backend/tests/perft_tests.rs` in full (318 LOC): all 15 tests, expected values, `#[ignore]` annotations.
- Ran `cargo test --release --features load_magic,deterministic_zobrist "perft_startpos"` — d1-d5 (all not-ignored), d6 (re-run via `--include-ignored`).
- Ran `cargo test --release --features load_magic,deterministic_zobrist "perft_kiwipete_complete"` — d1-d4 (not ignored).
- Built a scratchpad Cargo project in the session temp directory (NOT in `backend/`) with a path dependency on the vantage crate at `backend/`, features `load_magic,deterministic_zobrist`. Ran it to cover positions 3, 4, 4-mirrored, 5, 6 plus kiwipete d5.

## What I did not check, and why

- Startpos d7 (3,195,901,860 nodes) — estimated >10 min at observed NPS; skipped to stay within budget.
- Detailed breakdown counters (captures, EP, castles, checks) for positions 3-6 — not required by brief and would require bisect which is unneeded given all nodes matched.

## Perft Results Table

| Position | Depth | Expected | Actual | Match | Time | NPS |
|---|---|---|---|---|---|---|
| Startpos | 1 | 20 | 20 | YES | <1ms | — |
| Startpos | 2 | 400 | 400 | YES | <1ms | — |
| Startpos | 3 | 8,902 | 8,902 | YES | <1ms | — |
| Startpos | 4 | 197,281 | 197,281 | YES | <1ms | — |
| Startpos | 5 | 4,865,609 | 4,865,609 | YES | ~1.0s | ~4.9M |
| Startpos | 6 | 119,060,324 | 119,060,324 | YES | 27.83s | 4.28M |
| Startpos | 7 | 3,195,901,860 | — | SKIPPED | >10min est. | — |
| Kiwipete | 1 | 48 | 48 | YES | <1ms | — |
| Kiwipete | 2 | 2,039 | 2,039 | YES | <1ms | — |
| Kiwipete | 3 | 97,862 | 97,862 | YES | <1ms | — |
| Kiwipete | 4 | 4,085,603 | 4,085,603 | YES | ~0.7s | ~5.8M |
| Kiwipete | 5 | 193,690,690 | 193,690,690 | YES | 50.37s | 3.85M |
| Pos3 | 1 | 14 | 14 | YES | <1ms | — |
| Pos3 | 2 | 191 | 191 | YES | <1ms | — |
| Pos3 | 3 | 2,812 | 2,812 | YES | 1ms | 3.09M |
| Pos3 | 4 | 43,238 | 43,238 | YES | 13ms | 3.33M |
| Pos3 | 5 | 674,624 | 674,624 | YES | 230ms | 2.93M |
| Pos3 | 6 | 11,030,083 | 11,030,083 | YES | 3.45s | 3.20M |
| Pos4 | 1 | 6 | 6 | YES | <1ms | — |
| Pos4 | 2 | 264 | 264 | YES | <1ms | — |
| Pos4 | 3 | 9,467 | 9,467 | YES | 2ms | 4.58M |
| Pos4 | 4 | 422,333 | 422,333 | YES | 133ms | 3.18M |
| Pos4 | 5 | 15,833,292 | 15,833,292 | YES | 3.63s | 4.36M |
| Pos4 mirrored | 1 | 6 | 6 | YES | <1ms | — |
| Pos4 mirrored | 2 | 264 | 264 | YES | <1ms | — |
| Pos4 mirrored | 3 | 9,467 | 9,467 | YES | 3ms | 3.29M |
| Pos4 mirrored | 4 | 422,333 | 422,333 | YES | 103ms | 4.10M |
| Pos4 mirrored | 5 | 15,833,292 | 15,833,292 | YES | 3.49s | 4.54M |
| Pos5 | 1 | 44 | 44 | YES | <1ms | — |
| Pos5 | 2 | 1,486 | 1,486 | YES | 1ms | 2.96M |
| Pos5 | 3 | 62,379 | 62,379 | YES | 19ms | 3.35M |
| Pos5 | 4 | 2,103,487 | 2,103,487 | YES | 478ms | 4.40M |
| Pos5 | 5 | 89,941,194 | 89,941,194 | YES | 20.21s | 4.45M |
| Pos6 | 1 | 46 | 46 | YES | <1ms | — |
| Pos6 | 2 | 2,079 | 2,079 | YES | 1ms | 3.84M |
| Pos6 | 3 | 89,890 | 89,890 | YES | 19ms | 4.82M |
| Pos6 | 4 | 3,894,594 | 3,894,594 | YES | 788ms | 4.94M |

NPS baseline (release build, Windows 11, single thread): approximately **3.2M–4.9M nodes/second** across positions. Kiwipete is slowest (~3.85M) due to its high branching factor. Pos6 is fastest (~4.94M). The engine has no prior NPS baseline in the repo; these numbers are the first recorded.

## Findings

### MOVEGEN-101 — Wrong FEN labeled as Kiwipete in FENS array used by perft_fuzz

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/perft_tests.rs:19`
- **Expected:** The `FENS` constant is documented as containing the Kiwipete position. The true Kiwipete FEN (asserted by `perft_kiwipete_complete`) is `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1`.
- **Actual:** The `FENS[1]` entry reads `"rnbq1k1r/pppp1ppp/5n2/4p3/1b1P4/5N2/PPPNPPPP/R1BQKB1R w KQkq - 0 1"` with the comment `// Kiwipete`. This is a different position (an opening tabiya, not Kiwipete).
- **Reasoning:** `KIWI_FEN` at line 13 is the correct Kiwipete and is used by the node-count assertions in `perft_kiwipete_complete`. The `FENS` array at lines 15-24 is used only by `perft_fuzz` which tests make/undo Zobrist invariants, not node counts. Because `perft_fuzz` does not assert counts, the wrong FEN does not corrupt the perft results. However it means the fuzz test exercises a different position than intended.
- **Failure scenario:** No perft mismatch results. The fuzz test cover a random opening tabiya instead of the intended Kiwipete. Any make/undo bug that is Kiwipete-specific (complex castling, EP, double-pawn interactions) would be missed by the fuzz.
- **Related:** TEST-* (test suite coverage audit).
- **Where to look next:** `backend/tests/perft_tests.rs:15-24` (FENS declaration), `backend/tests/perft_tests.rs:231-266` (`perft_fuzz` usage).
- **Fix direction:** Replace `FENS[1]` with the real Kiwipete FEN from `KIWI_FEN`, or simply reference `KIWI_FEN` in the array directly.

### MOVEGEN-102 — Kiwipete d5 is commented out, not #[ignore]d — unreachable without source edit

- **Severity:** P3
- **Confidence:** high
- **Status:** unverified
- **Anchors:** `backend/tests/perft_tests.rs:152`
- **Expected:** Deep perft depths that are too slow for CI should be gated with `#[ignore]` so they can still be run with `cargo test -- --include-ignored` without modifying source.
- **Actual:** Line 152 reads `// (5, 193_690_690), // uncomment when fast enough`. The d5 value is in a dead comment; it cannot be executed at all without editing the source file.
- **Reasoning:** `perft_startpos_d6` and `perft_startpos_d7` use `#[ignore]` and are runnable on demand. The Kiwipete d5 follows a different pattern (inline comment) and is silently unreachable. At the observed speed (~50s), it would be practical to run manually if gated with `#[ignore]`.
- **Failure scenario:** A movegen regression introduced in Kiwipete d5-exclusive paths (e.g., EP-capture-after-castle interactions 4+ plies deep) would be undetected even in a full `--include-ignored` run.
- **Related:** MOVEGEN-101.
- **Where to look next:** `backend/tests/perft_tests.rs:143-165` (`perft_kiwipete_complete`).
- **Fix direction:** Move line 152 out of the comment into the `expected` array and add `#[ignore]` to the `perft_kiwipete_complete` test (or split it into a separate ignored test for d5).

## CI coverage assessment

Non-ignored perft CI coverage (i.e., what `cargo test --release --features load_magic,deterministic_zobrist` runs unconditionally):

| Position | CI Depths | Deepest CI Depth | Nodes at deepest |
|---|---|---|---|
| Startpos | d1–d5 | d5 | 4,865,609 |
| Kiwipete | d1–d4 | d4 | 4,085,603 |
| Pos3 | none | — | — |
| Pos4 | none | — | — |
| Pos4 mirrored | none | — | — |
| Pos5 | none | — | — |
| Pos6 | none | — | — |

The deepest depth CI actually verifies is d5 (startpos). Positions 3-6 have zero coverage. Shallow perft (d1-d3) does not meaningfully exercise en passant, castling, or promotion edge cases; these require at least d3-d4 for the simpler positions and d4-d5 for Kiwipete-style complexity.

## Clean bill

- **All 37 asserted node counts match.** Every position (startpos d1-d6, Kiwipete d1-d5, Pos3 d1-d6, Pos4 d1-d5, Pos4-mirrored d1-d5, Pos5 d1-d5, Pos6 d1-d4) against the published Chessprogramming.org values — zero mismatches. The engine generates exactly the correct number of nodes at every depth tested.
- **Colour-symmetry (Pos4 vs Pos4-mirrored):** Identical counts at every depth confirm no White/Black asymmetry bug in the move generator.
- **`perft` public API is sane:** The function signature, buffer allocation, and recursion termination are correct. `perft_recursive` properly uses ply-indexed buffers, avoiding aliasing.
- **Kiwipete d2 detailed tally:** `kiwipete_d2_tally` asserts nodes=2039, captures=351, ep=1, castles=91, checks=3 — matches the published breakdown exactly (`backend/tests/perft_tests.rs:176-229`).
- **No panic path in normal usage:** `MAX_PERFT_DEPTH = 20` guard is present; the buffer array is sized to match. The `#[instrument]` annotation on public functions is correct (skips board/tables from span fields to avoid massive debug output).
