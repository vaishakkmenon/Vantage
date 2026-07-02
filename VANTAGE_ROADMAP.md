# Vantage Chess Engine — Development Roadmap

## Project Context

Vantage is a UCI-compliant chess engine written in Rust. The primary goal is to reach ~2800+ ELO and consistently beat the Crafty engine (~2700-2800). The long-term stretch goal is approaching 3000 ELO.

**Developer context:** Solo developer, 1-2 hours/day. All changes must be verifiable via short SPRT test matches. Favor correctness and testability over speed of implementation.

---

## Current Engine State (Baseline ~2200-2400 ELO)

### Implemented ✅

**Search:** Alpha-beta (negamax), iterative deepening with time prediction, aspiration windows (50cp, depth > 4), quiescence search (captures + promotions), null move pruning (R=2-3 dynamic), LMR (conservative formula), LMP (threshold: 3+6×depth), reverse futility pruning (depth < 9), futility pruning (depth < 7), delta pruning in QS (200cp), SEE pruning in QS, check extensions (+1 ply)

**Move Ordering:** Hash move priority, MVV-LVA captures, good/bad capture split (SEE-based), killer moves (2 slots/ply), history heuristic ([64][64] from-to), staged/lazy move generation

**Evaluation:** Tapered material (Pesto values), piece-square tables (MG/EG), phase calc (N=1,B=1,R=2,Q=4), bishop mobility (5cp/sq), rook mobility (3cp/sq), king zone attacks (3×3, tapered), king pawn shield, doubled pawns (-10cp), isolated pawns (-15cp), passed pawns (rank-based), blocked passers (-50cp), king-passer distance, mop-up endgame eval, lazy eval cutoff (±200cp)

**Infrastructure:** Zobrist hashing (64-bit), TT with bound types + depth-preferred replacement + generation, mate score adjustment, dynamic TT sizing, UCI protocol, Polyglot opening book, 3-fold repetition detection, FEN support

---

## Implementation Phases

### Phase 1: Quick Wins (~+80-130 ELO collectively)
**Target: 3-5 weeks**

Complete these in order. Batch SPRT testing where noted.

#### 1.1 Principal Variation Search (PVS)
- Search first move with full window
- Remaining moves: null window search, re-search with full window only on fail-high
- Test independently — this affects all subsequent search behavior
- **Expected: +20-40 ELO**

#### 1.2 Countermove Heuristic
- Add `[64][64]` table indexed by previous move's from-to squares
- Store the move that caused a beta cutoff as the countermove
- Use as additional move ordering signal after killers, before history
- **Expected: +20-30 ELO**

#### 1.3 Evaluation Batch (test together as a group)
Implement all of these, then run one SPRT test for the batch:

- **Bishop pair bonus:** If side has ≥2 bishops, add ~30cp bonus (tapered: more valuable in endgame)
- **Rook on open file:** No friendly pawns on file → +15-20cp. Semi-open (no friendly, has enemy) → +10cp
- **Rook on 7th rank:** Rook on rank 7 (rank 2 for black) → +20cp, especially with enemy king on 8th
- **Knight mobility:** Extend existing mobility framework. ~4cp/square, exclude squares attacked by enemy pawns
- **Queen mobility:** ~1-2cp/square (keep weight low to avoid queen wandering)
- **Backward pawns:** Pawn with no friendly pawns on adjacent files that could advance to support it, and the stop square is attacked by an enemy pawn → -12cp
- **Expected: +40-60 ELO combined**

---

### Phase 2: Search Depth & Accuracy (~+60-100 ELO)
**Target: 3-4 weeks**

#### 2.1 Singular Extension
- At TT nodes with sufficient depth, verify the TT move is singular (significantly better than alternatives)
- Reduce search depth and search with window [ttScore - margin, ttScore - margin + 1]
- If no other move beats the reduced threshold, extend the TT move by 1 ply
- Typical margin: ~50-75cp, only at depth ≥ 6
- **Expected: +30-50 ELO**

#### 2.2 Internal Iterative Reduction (IIR)
- When no hash move exists at a node, reduce depth by 1
- Simple: `if no_hash_move && depth >= 4 { depth -= 1; }`
- **Expected: +10-20 ELO**

#### 2.3 LMR Formula Tuning
- Current formula is noted as conservative
- Move to standard log-based: `reduction = base + ln(depth) * ln(move_number) / divisor`
- Typical starting values: base=0.75, divisor=2.25
- Can pre-compute in a 2D table at init for speed
- Fine-tune with SPRT tests (adjust base and divisor in 0.25 increments)
- **Expected: +10-20 ELO**

---

### Phase 3: NNUE Evaluation (~+150-250 ELO)
**Target: 2-4 months**

This is the single largest ELO gain possible. It replaces the entire hand-crafted evaluation.

#### 3.1 Data Generation Pipeline
- Generate training positions via self-play with current engine
- Use fixed-depth search (depth 8-10) for position scoring
- Target: 10-50 million scored positions minimum
- Output in a standard format (e.g., Marlinflow/Bullet-compatible)
- **This is CPU-bound — run overnight/in background**

#### 3.2 Network Architecture
- Start with HalfKP or HalfKA feature set
- Architecture: `(768 → 256)x2 → 1` (perspective network)
- Clipped ReLU activation
- Quantized weights (i16) for inference speed

#### 3.3 Rust Inference
- Implement incremental accumulator updates (only recompute changed features on make/unmake)
- Use SIMD intrinsics for dot products and activation (SSE2/AVX2)
- Profile and verify the NNUE eval is fast enough to not slow NPS significantly

#### 3.4 Training Loop
- Use Bullet trainer (Rust-based, designed for chess NNUE)
- Train, test in matches, regenerate data with improved engine, retrain
- First net may only gain +100 ELO over HCE — iterate
- **Expected after 2-3 training cycles: +150-250 ELO**

---

### Phase 4: Scaling & Polish (~+50-80 ELO)
**Target: 4-6 weeks**

#### 4.1 Lazy SMP (Parallel Search)
- Shared transposition table with atomic operations
- Each thread runs independent search with slight depth variation
- Rust's type system makes this safer than C++ implementations
- Start with 2-4 threads, scale up
- **Expected: +30-50 ELO at 4 threads**

#### 4.2 Time Management Improvements
- **Move stability:** If best move changes between iterations, allocate more time
- **Score stability:** If eval is volatile (>30cp swings), allocate more time
- Track best move changes across last 3-4 iterations
- **Expected: +15-30 ELO combined**

#### 4.3 Syzygy Tablebases
- Use an existing Rust crate for probing (e.g., `shakmern` or similar)
- Probe in search when piece count ≤ 6-7
- Use WDL probes in search, DTZ probes at root for best move
- **Expected: +20-40 ELO (mostly in endgames)**

---

## Testing Protocol

### SPRT Testing
- Use `cutechess-cli` for matches
- Short tests: 1000 games at 10+0.1 for quick validation
- Confirmation tests: 5000 games at 10+0.1 for final verification
- SPRT bounds: H0=0, H1=10 for small features; H0=0, H1=20 for large features

### Regression Checks
- After each feature, run a quick match against the previous version
- If ELO drops, revert and debug before moving on
- Keep tagged git versions for each stable improvement

### Benchmarks
- Fixed-depth search on standard positions (e.g., WAC, BK test suites) for NPS regression
- Profile periodically to catch performance regressions

---

## ELO Projection

```
Current baseline:           ~2300
After Phase 1:              ~2400-2430
After Phase 2:              ~2500-2530
After Phase 3 (NNUE):      ~2700-2780  ← Crafty range
After Phase 4:              ~2800-2850  ← Consistently beats Crafty
Tuning/iteration:           ~2900-3000  ← Stretch goal
```

---

## Quick Reference for Claude Code Sessions

When starting a session on Vantage, reference this file:
```
Read VANTAGE_ROADMAP.md and check which phase/feature I'm currently working on.
Look at recent git log to see what was last implemented.
Follow the implementation order strictly — don't skip ahead.
After implementing a feature, remind me to run SPRT tests before moving on.
```

### Rules for Claude Code
1. **Never break existing functionality.** Run `cargo test` after every change.
2. **Keep changes atomic.** One feature per PR/commit so regressions are easy to bisect.
3. **Follow Rust idioms.** No unsafe unless absolutely necessary (SIMD in Phase 3 is the exception).
4. **Respect the engine's architecture.** Read existing code before adding features — maintain consistency.
5. **When in doubt, be conservative.** A small, correct improvement beats an ambitious broken one.