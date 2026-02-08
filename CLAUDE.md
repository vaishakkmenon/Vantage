# Vantage Chess Engine

## Quick Facts
- **Backend**: Rust (Edition 2024), built with `cargo build --release`
- **Frontend**: Next.js 15 + Tailwind CSS (in `frontend/`)
- **WASM**: Compiled via `wasm-pack build --target web --no-default-features --features psqt`
- **Tests**: `cargo test --release --features load_magic,deterministic_zobrist` (300+ tests, must all pass)
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`

## Architecture
- `backend/` — Rust chess engine (library + CLI binary)
  - `src/lib.rs` — Library root, re-exports all modules
  - `src/bin/cli.rs` — UCI stdin/stdout interface
  - `src/board/` — Board representation, FEN, piece types
  - `src/search/search.rs` — Main search function
  - `src/moves/magic/` — Magic bitboard tables
  - `src/book/` — Polyglot opening book
- `frontend/` — Next.js chess UI for chess.vaishakmenon.com (WIP)
- `data/` — Opening book (book.bin)

## Key Conventions
- Engine is single-threaded (no Lazy SMP)
- UCI protocol for engine communication
- WASM builds use 64MB TT (native uses 512MB)
- Feature `cli` gates terminal-only deps (indicatif, tracing-subscriber)
- Never break `cargo test --release`

## Common Tasks
- Build native: `cd backend && cargo build --release`
- Build WASM: `cd backend && wasm-pack build --target web --no-default-features --features psqt`
- Run tests: `cd backend && cargo test --release --features load_magic,deterministic_zobrist`
- Run engine: `echo "uci\nisready\nposition startpos\ngo depth 6\nquit" | ./backend/target/release/vantage`
