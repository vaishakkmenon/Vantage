# Vantage

[![Vantage CI](https://github.com/vaishakkmenon/Vantage/actions/workflows/ci.yml/badge.svg)](https://github.com/vaishakkmenon/Vantage/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/vaishakkmenon/Vantage?color=green)](https://github.com/vaishakkmenon/Vantage/releases)

**Vantage** is a high-performance, UCI-compatible chess engine written in **Rust**. It utilizes magic bitboards for move generation and advanced search techniques to play at a competitive level.

> **Note:** Vantage is a command-line engine. To play against it, you need a chess GUI like **Arena**, **Cute Chess**, or **Banksia**.

---

## ⚡ Quick Start

**Just want to play?**

1. Download the [latest release](https://github.com/vaishakkmenon/Vantage/releases/latest) for your OS
2. Extract the archive
3. Load `vantage` (or `vantage.exe` on Windows) into your chess GUI
4. Start playing!

---

## 📥 Download & Install

### For Players (Windows & Linux)
You do not need to install Rust. Just download the latest engine release:

1. Go to the **[Releases Page](https://github.com/vaishakkmenon/Vantage/releases/latest)**.
2. Download the zip file for your OS (`vantage-windows-x86_64.zip` or `vantage-linux-x86_64.tar.gz`).
3. Extract the contents.
4. Load `vantage` into your favorite Chess GUI.

**Recommended GUIs:**
- [Cute Chess](https://cutechess.com/) (Cross-platform, actively maintained)
- [Arena](http://www.playwitharena.de/) (Windows)
- [Banksia GUI](https://banksiagui.com/) (Cross-platform)

### For Developers (Build from Source)
If you want to modify the code or run tests:

```bash
# Clone the repository
git clone https://github.com/vaishakkmenon/Vantage.git
cd Vantage/backend

# Run Unit Tests (300+ tests)
cargo test --release

# Build the Engine (Optimized)
cargo build --release
```

The executable will be located at `target/release/vantage`.

---

## 🧠 Engine Features

### Board Representation
* **Bitboards**: Full 64-bit representation for all pieces.
* **Magic Bitboards**: Ultra-fast sliding piece attack lookups (Rook/Bishop) using precomputed tables.
* **Zobrist Hashing**: Efficient position identification for the Transposition Table.

### Search & Evaluation
* **Alpha-Beta Pruning**: With Principal Variation Search (PVS) for optimal tree traversal.
* **Iterative Deepening**: Progressive search depth for better time management.
* **Transposition Table**: 512MB hash table caches search results to avoid re-calculating known positions.
* **Move Ordering**: Prioritizes captures (MVV-LVA) and killer moves to maximize pruning.
* **Opening Book**: Supports Polyglot (`.bin`) opening books for varied play.
* **PeSTO Evaluation**: Piece-square tables with tapered evaluation for midgame/endgame.

---

## 📊 Performance & Strength

| Metric | Status |
|--------|--------|
| **Estimated ELO** | TBD - Formal testing pending |
| **Nodes Per Second** | TBD - Benchmarking in progress |
| **Search Depth** | TBD - Performance testing underway |
| **Test Suites** | TBD - WAC/Bratko-Kopec results pending |

*Benchmarking and rating tests are planned for future releases.*

---

## ⚙️ Configuration

### UCI Protocol Support

Vantage implements the Universal Chess Interface (UCI) protocol with the following capabilities:

**Supported `go` Commands:**
- `go depth <n>` - Search to a fixed depth
- `go movetime <ms>` - Search for a fixed time in milliseconds
- `go wtime <ms> btime <ms>` - Time control for both sides
- `go winc <ms> binc <ms>` - Increment per move
- `go movestogo <n>` - Moves until next time control
- `go infinite` - Search until stopped

**Current Limitations:**
- No `setoption` support (hash size and other parameters are fixed)
- Single-threaded only (no SMP/multi-threading)
- Fixed 512MB transposition table

**Memory Usage:**
- Transposition Table: 512MB (power-of-2 aligned)
- Additional overhead: ~50-100MB for move generation and search data structures

---

## 🎮 How to Use

### Loading into a Chess GUI

#### Cute Chess
1. Open Cute Chess
2. Go to `Tools > Settings > Engines`
3. Click `Add` and browse to the `vantage` executable
4. Click `OK` to save
5. Start a new game and select Vantage as one of the players

#### Arena Chess
1. Open Arena
2. Go to `Engines > Install New Engine`
3. Browse to and select the `vantage.exe` file
4. Click `OK` to install
5. Select Vantage from the engine list to play

### Command Line Mode
You can also run Vantage directly in a terminal for debugging or scripting:

```bash
./vantage
uci
isready
position startpos moves e2e4 e7e5
go depth 6
```

**Example UCI Session:**
```
> uci
id name Vantage
id author Vaishak Menon
uciok

> isready
readyok

> position startpos moves e2e4
> go depth 8
info depth 1 score cp 50 nodes 20 time 1 pv e7e5
info depth 2 score cp 35 nodes 85 time 2 pv e7e5 g1f3
...
bestmove e7e5
```

---

## 🗺️ Roadmap

**Planned Features:**
- [ ] ELO rating through tournament testing
- [ ] Performance benchmarking suite
- [ ] Configurable hash table size via UCI options
- [ ] Multi-threading support (Lazy SMP)
- [ ] Endgame tablebase support (Syzygy)
- [ ] Neural network evaluation (NNUE)

**Known Limitations:**
- Single-threaded execution only
- No UCI `setoption` command support
- Fixed 512MB transposition table (not user-configurable)
- No time management tuning options

*Want to contribute? See the [Contributing](#-contributing) section below!*

---

## 🛠️ Development Status

* **Language**: Rust 🦀
* **Tests**: 300+ unit tests with 100% pass rate (enforced by CI)
* **Platform**: Windows & Linux (cross-platform support via GitHub Actions)
* **Architecture**: Single-threaded bitboard-based engine
* **Code Quality**: Clippy-approved, formatted with rustfmt

---

## 🤝 Contributing

Contributions are welcome! Whether you're fixing bugs, adding features, or improving documentation:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Run `cargo test --release` to ensure all tests pass
5. Run `cargo clippy` to check for issues
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

**Areas where contributions are especially welcome:**
- Performance optimizations
- Additional search techniques
- UCI option support
- Multi-threading implementation
- Documentation improvements
- Bug reports and testing

---

## 🙏 Acknowledgments

Vantage builds upon the work of the chess programming community:

- **[Chess Programming Wiki](https://www.chessprogramming.org/)** - Comprehensive reference for chess engine development
- **[PeSTO](https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function)** - Piece-square tables for evaluation
- **[Polyglot Book Format](http://hgm.nubati.net/book_format.html)** - Opening book implementation
- **Magic Bitboards** - Fast sliding piece move generation technique

Special thanks to the Rust chess programming community for their open-source contributions and discussions.

---

## 🐛 Troubleshooting

### Engine won't load in GUI
- **Solution**: Ensure you've downloaded the correct version for your OS (Windows vs Linux)
- **Solution**: Check that the file has execute permissions (Linux: `chmod +x vantage`)
- **Solution**: Try running from command line first to see error messages

### Engine plays very slowly
- **Check**: Are you using the release build? Debug builds are significantly slower
- **Check**: Is your GUI set to very long time controls?
- **Note**: Vantage is currently single-threaded, so it won't use all CPU cores

### "Unknown command" errors
- **Cause**: Vantage doesn't support all UCI commands yet
- **Workaround**: Avoid using `setoption` commands in your GUI configuration

**Still having issues?** [Open an issue](https://github.com/vaishakkmenon/Vantage/issues) on GitHub with:
- Your OS and version
- The GUI you're using
- Steps to reproduce the problem
- Any error messages

---

## 📄 License

This project is licensed under the **GPL-3.0 License** - see the [LICENSE](LICENSE) file for details.

This means you're free to:
- ✅ Use Vantage for personal or commercial purposes
- ✅ Modify the source code
- ✅ Distribute copies

With the requirement that:
- ⚠️ Derivative works must also be open-source under GPL-3.0
- ⚠️ You must include the original license and copyright notice

---

## 👨‍💻 Author

**Developed by Vaishak Menon**

- Portfolio: [vaishakmenon.com](https://vaishakmenon.com)
- GitHub: [@vaishakkmenon](https://github.com/vaishakkmenon)

*Star the repo to follow development and new releases!*
