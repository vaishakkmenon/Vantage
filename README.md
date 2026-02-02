# Vantage ♟️

![Vantage CI](https://github.com/vaishakkmenon/Vantage/actions/workflows/ci.yml/badge.svg)

**Vantage** is a high-performance chess engine developed in **Rust**, designed to explore the intersection of memory safety and raw computational speed. It implements the Universal Chess Interface (UCI) protocol and is compatible with any modern GUI (CuteChess, Arena, Banksia).

### 🚀 Key Features
* **Move Generation:** 64-bit Magic Bitboards for ultra-fast sliding piece attack lookups.
* **Search:** Alpha-Beta Pruning with Iterative Deepening, Principal Variation Search (PVS), and Null Move Pruning.
* **Evaluation:** Custom PeSTO-based static evaluation with tapered phases (opening -> endgame).
* **Opening Book:** Integrated Polyglot (.bin) book support for instant (<1ms) opening responses.
* **Optimization:** Hash-efficient transposition tables using Zobrist Hashing.

### 🛠️ Quick Start

**Build from source:**
```bash
git clone [https://github.com/vaishakkmenon/Vantage-chess.git](https://github.com/vaishakkmenon/Vantage-chess.git)
cd Vantage/backend
cargo run --release