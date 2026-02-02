use vantage::board::{Board, Color, Piece};
use vantage::book::PolyglotBook;
use vantage::moves::execute::{generate_legal, make_move_basic};
use vantage::moves::magic::MagicTables;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::Move;
use vantage::search::search::search;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::str::FromStr;
use std::time::Duration;

fn main() {
    // Load magic tables once at startup
    let magic_tables = load_magic_tables();

    let mut board = Board::new(); // Start position

    let book = PolyglotBook::load("book.bin").ok();
    if book.is_some() {
        println!("info string Opening Book loaded successfully");
    } else {
        println!("info string No opening book found (book.bin), running engine only");
    }

    // Main UCI loop
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l.trim().to_string(),
            Err(_) => break,
        };

        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let command = parts[0];

        match command {
            "uci" => handle_uci(),
            "isready" => println!("readyok"),
            "ucinewgame" => {
                board = Board::new();
            }
            "position" => {
                if let Some(new_board) = handle_position(&parts, &magic_tables) {
                    board = new_board;
                }
            }
            "go" => {
                handle_go(&parts, &mut board, &magic_tables, &book);
            }
            "fen" => {
                println!("{}", board.to_fen());
            }
            "quit" => break,
            "d" | "display" => {
                println!("{}", board);
            }
            "test" | "bench" => {
                run_epd_tests("../bench_arena/bk.epd", &magic_tables);
            }
            _ => {}
        }
    }
}

fn handle_uci() {
    println!("id name Vantage 1.0");
    println!("id author Vaishak Menon");
    println!("uciok");
}

fn handle_position(parts: &[&str], tables: &MagicTables) -> Option<Board> {
    let mut board = if parts.len() > 1 && parts[1] == "startpos" {
        Board::new()
    } else if parts.len() > 1 && parts[1] == "fen" {
        let fen_start = 2;
        let mut fen_end = parts.len();
        for (i, &part) in parts.iter().enumerate().skip(fen_start) {
            if part == "moves" {
                fen_end = i;
                break;
            }
        }

        let fen_string = parts[fen_start..fen_end].join(" ");
        Board::from_str(&fen_string).ok()?
    } else {
        Board::new()
    };

    // Apply moves if any
    if let Some(moves_idx) = parts.iter().position(|&p| p == "moves") {
        for move_str in &parts[moves_idx + 1..] {
            if let Some(mv) = parse_uci_move(&board, move_str, tables) {
                make_move_basic(&mut board, mv);
            } else {
                eprintln!("Invalid move: {}", move_str);
                return None;
            }
        }
    }

    Some(board)
}

fn parse_uci_move(board: &Board, move_str: &str, tables: &MagicTables) -> Option<Move> {
    if move_str.len() < 4 {
        return None;
    }

    let chars: Vec<char> = move_str.chars().collect();

    let from_file = (chars[0] as u8).wrapping_sub(b'a');
    let from_rank = (chars[1] as u8).wrapping_sub(b'1');
    let to_file = (chars[2] as u8).wrapping_sub(b'a');
    let to_rank = (chars[3] as u8).wrapping_sub(b'1');

    if from_file > 7 || from_rank > 7 || to_file > 7 || to_rank > 7 {
        return None;
    }

    let from_square = (from_rank * 8 + from_file) as usize;
    let to_square = (to_rank * 8 + to_file) as usize;

    let promo_piece = if move_str.len() >= 5 {
        match chars[4] {
            'q' => Some(vantage::board::Piece::Queen),
            'r' => Some(vantage::board::Piece::Rook),
            'b' => Some(vantage::board::Piece::Bishop),
            'n' => Some(vantage::board::Piece::Knight),
            _ => None,
        }
    } else {
        None
    };

    let mut moves = Vec::with_capacity(256);
    let mut scratch = Vec::with_capacity(256);
    let mut board_copy = board.clone();
    generate_legal(&mut board_copy, tables, &mut moves, &mut scratch);

    for mv in moves {
        // FIXED: Cast index() to usize for comparison
        if (mv.from.index() as usize) == from_square && (mv.to.index() as usize) == to_square {
            if promo_piece.is_some() {
                if mv.promotion == promo_piece {
                    return Some(mv);
                }
            } else if mv.promotion.is_none() {
                return Some(mv);
            }
        }
    }
    None
}

fn handle_go(parts: &[&str], board: &mut Board, tables: &MagicTables, book: &Option<PolyglotBook>) {
    // --- STEP A: Check Opening Book First ---
    // If we have a book, and the board position is in it, play immediately.
    if let Some(b) = book {
        if let Some(book_move) = b.probe(board) {
            println!("info string Book move found");
            println!("bestmove {}", book_move.to_uci());
            return; // EXIT IMMEDIATELY - Do not search!
        }
    }
    // ----------------------------------------
    let mut depth = 64;
    let mut time_limit = None;

    // Time Control Variables
    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut winc: u64 = 0;
    let mut binc: u64 = 0;
    let mut movestogo: Option<u64> = None;
    let mut movetime: Option<u64> = None;

    let mut i = 1;
    while i < parts.len() {
        match parts[i] {
            "depth" => {
                if i + 1 < parts.len() {
                    depth = parts[i + 1].parse().unwrap_or(64);
                }
                i += 2;
            }
            "movetime" => {
                if i + 1 < parts.len() {
                    movetime = parts[i + 1].parse().ok();
                }
                i += 2;
            }
            "wtime" => {
                if i + 1 < parts.len() {
                    wtime = parts[i + 1].parse().ok();
                }
                i += 2;
            }
            "btime" => {
                if i + 1 < parts.len() {
                    btime = parts[i + 1].parse().ok();
                }
                i += 2;
            }
            "winc" => {
                if i + 1 < parts.len() {
                    winc = parts[i + 1].parse().unwrap_or(0);
                }
                i += 2;
            }
            "binc" => {
                if i + 1 < parts.len() {
                    binc = parts[i + 1].parse().unwrap_or(0);
                }
                i += 2;
            }
            "movestogo" => {
                if i + 1 < parts.len() {
                    movestogo = parts[i + 1].parse().ok();
                }
                i += 2;
            }
            "infinite" => {
                depth = 100;
                time_limit = None;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    if let Some(ms) = movetime {
        time_limit = Some(Duration::from_millis(ms));
    } else {
        let (my_time, my_inc) = if board.side_to_move == Color::White {
            (wtime, winc)
        } else {
            (btime, binc)
        };

        if let Some(t) = my_time {
            // --- SAFETY BUFFER (Hidden from engine) ---
            // Reserve 15% of time or 500ms (whichever is smaller) for lag/OS overhead.
            // This time is INVISIBLE to the search engine.
            let safety_buffer = (t * 15 / 100).min(500);
            let usable_time = t.saturating_sub(safety_buffer);

            let mut alloc: u64;

            if let Some(mtg) = movestogo {
                let moves_to_plan = mtg.max(2);
                alloc = usable_time / moves_to_plan;
                alloc += (my_inc * 3) / 4;
            } else {
                // VERY CONSERVATIVE tiered allocation
                // Assume ~40 moves remaining in a typical game
                if usable_time > 5000 {
                    // Normal game: 1/40th of time + most of increment
                    alloc = usable_time / 40 + (my_inc * 9) / 10;
                } else if usable_time > 2000 {
                    // Low time: 1/30th
                    alloc = usable_time / 30 + (my_inc * 3) / 4;
                } else if usable_time > 500 {
                    // Very low: 1/20th
                    alloc = usable_time / 20 + my_inc / 2;
                } else {
                    // CRITICAL: Just use increment + tiny bit
                    alloc = my_inc / 2 + 20;
                }
            }

            // HARD CAP: Never use more than 20% of remaining time on one move
            // This prevents catastrophic time loss
            let hard_cap = usable_time / 5;
            alloc = alloc.min(hard_cap);

            // Also cap at usable time
            alloc = alloc.min(usable_time);

            // Minimum: Ensure we at least calculate for a tiny bit (10ms)
            if alloc < 10 && usable_time >= 10 {
                alloc = 10;
            }

            time_limit = Some(Duration::from_millis(alloc));
        }
    }

    if let Some(limit) = time_limit {
        println!("info string Target time: {}ms", limit.as_millis());
    }
    let (_score, best_move) = search(board, tables, depth, time_limit);

    if let Some(m) = best_move {
        println!("bestmove {}", m.to_uci());
    } else {
        println!("bestmove 0000");
    }
}

// --- EPD Test Runner ---
fn run_epd_tests(path: &str, tables: &MagicTables) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => match File::open(format!("bench_arena/{}", path.split('/').last().unwrap())) {
            Ok(f) => f,
            Err(_) => {
                println!("Error: Could not find EPD file at '{}' or local.", path);
                return;
            }
        },
    };

    println!("Running Tactical Tests from {} (1s per position)...", path);
    println!("----------------------------------------------------");

    let reader = BufReader::new(file);
    let mut solved = 0;
    let mut total = 0;

    for (line_idx, line_res) in reader.lines().enumerate() {
        let line = line_res.unwrap_or_default();
        if line.trim().is_empty() {
            continue;
        }

        if let Some(bm_idx) = line.find(" bm ") {
            let fen = &line[..bm_idx].trim();
            let rest = &line[bm_idx + 4..];
            let move_end = rest.find(';').unwrap_or(rest.len());
            let san_move = rest[..move_end].trim();

            let mut board = match Board::from_str(fen) {
                Ok(b) => b,
                Err(_) => {
                    println!("Error parsing FEN on line {}", line_idx + 1);
                    continue;
                }
            };

            let expected_uci = san_to_uci(&mut board, san_move, tables);

            // Fixed 1.0s search for testing
            let time_limit = Some(Duration::from_millis(1000));
            let depth = 64;

            let (_score, best_move) = search(&mut board, tables, depth, time_limit);

            let result_str = match best_move {
                Some(m) => m.to_uci(),
                None => "none".to_string(),
            };

            let passed = if let Some(ref exp) = expected_uci {
                *exp == result_str
            } else {
                false
            };

            if passed {
                solved += 1;
            }
            total += 1;

            println!("Test #{}: {}", total, if passed { "PASS" } else { "FAIL" });
            if !passed {
                println!(
                    "   Expected: {} | Got: {}",
                    expected_uci.unwrap_or(san_move.to_string()),
                    result_str
                );
            }
        }
    }

    println!("----------------------------------------------------");
    println!("Result: {}/{} Solved", solved, total);
}

// --- Helper: Convert SAN to UCI ---
fn san_to_uci(board: &mut Board, san: &str, tables: &MagicTables) -> Option<String> {
    let mut moves = Vec::with_capacity(256);
    let mut scratch = Vec::with_capacity(256);
    generate_legal(board, tables, &mut moves, &mut scratch);

    let clean_san = san.replace("+", "").replace("#", "").replace("x", "");

    // Handle Castling
    if clean_san == "O-O" {
        return moves
            .iter()
            .find(|m| {
                let from = m.from.index() as i8;
                let to = m.to.index() as i8;
                (to - from).abs() == 2 && to > from
            })
            .map(|m| m.to_uci());
    }
    if clean_san == "O-O-O" {
        return moves
            .iter()
            .find(|m| {
                let from = m.from.index() as i8;
                let to = m.to.index() as i8;
                (to - from).abs() == 2 && to < from
            })
            .map(|m| m.to_uci());
    }

    if clean_san.len() < 2 {
        return None;
    }
    let target_str = &clean_san[clean_san.len() - 2..];

    let file = (target_str.chars().nth(0)? as u8).wrapping_sub(b'a');
    let rank = (target_str.chars().nth(1)? as u8).wrapping_sub(b'1');
    if file > 7 || rank > 7 {
        return None;
    }
    let target_sq = (rank * 8 + file) as usize;

    let first_char = clean_san.chars().next()?;
    let piece_type = match first_char {
        'N' => Piece::Knight,
        'B' => Piece::Bishop,
        'R' => Piece::Rook,
        'Q' => Piece::Queen,
        'K' => Piece::King,
        _ => Piece::Pawn,
    };

    let disambig_char = if piece_type == Piece::Pawn {
        if clean_san.len() > 2 && first_char.is_lowercase() {
            Some(first_char)
        } else {
            None
        }
    } else {
        let content = &clean_san[1..clean_san.len() - 2];
        if !content.is_empty() {
            content.chars().next()
        } else {
            None
        }
    };

    let candidates: Vec<&Move> = moves
        .iter()
        .filter(|m| {
            // FIXED: Cast index() to usize
            if (m.to.index() as usize) != target_sq {
                return false;
            }

            if let Some((_, p)) = board.piece_at(m.from) {
                if p != piece_type {
                    return false;
                }
            } else {
                return false;
            }

            if let Some(d) = disambig_char {
                let from_sq = m.from.index();
                let from_file = from_sq % 8;
                let from_rank = from_sq / 8;

                if d >= 'a' && d <= 'h' {
                    if from_file != (d as u8 - b'a') {
                        return false;
                    }
                } else if d >= '1' && d <= '8' {
                    if from_rank != (d as u8 - b'1') {
                        return false;
                    }
                }
            }
            true
        })
        .collect();

    if !candidates.is_empty() {
        Some(candidates[0].to_uci())
    } else {
        None
    }
}
