use wasm_bindgen::prelude::*;

use crate::board::{Board, Color, Piece};
use crate::book::PolyglotBook;
use crate::moves::execute::{generate_legal, make_move_basic};
use crate::moves::magic::MagicTables;
use crate::moves::magic::loader::load_magic_tables;
use crate::moves::types::Move;
use crate::search::search::search;
use crate::search::tt::TranspositionTable;
use std::str::FromStr;
use std::time::Duration;

#[wasm_bindgen]
pub struct VantageEngine {
    magic_tables: MagicTables,
    board: Board,
    tt: TranspositionTable,
    book: Option<PolyglotBook>,
}

#[wasm_bindgen]
impl VantageEngine {
    /// Initialize the engine. This is expensive due to magic table generation.
    /// Call once.
    #[wasm_bindgen(constructor)]
    pub fn new() -> VantageEngine {
        console_error_panic_hook::set_once();

        let magic_tables = load_magic_tables();
        let tt = TranspositionTable::new(64); // 64MB for browser

        let book = {
            let book_bytes = include_bytes!("../book.bin");
            Some(PolyglotBook::from_bytes(book_bytes))
        };

        VantageEngine {
            magic_tables,
            board: Board::new(),
            tt,
            book,
        }
    }

    /// Reset to a new game (starting position, clear TT)
    pub fn new_game(&mut self) {
        self.board = Board::new();
        self.tt.clear();
    }

    /// Set position from FEN string. Returns true on success.
    pub fn set_position_fen(&mut self, fen: &str) -> bool {
        match Board::from_str(fen) {
            Ok(board) => {
                self.board = board;
                true
            }
            Err(_) => false,
        }
    }

    /// Set starting position and apply a sequence of UCI moves.
    /// `moves_str` is space-separated: "e2e4 e7e5 g1f3"
    pub fn set_position_startpos(&mut self, moves_str: &str) {
        self.board = Board::new();
        if !moves_str.is_empty() {
            for move_str in moves_str.split_whitespace() {
                self.apply_move(move_str);
            }
        }
    }

    /// Apply a single UCI move (e.g., "e2e4", "e7e8q"). Returns true if legal.
    pub fn apply_move(&mut self, uci_move: &str) -> bool {
        if let Some(mv) = self.parse_uci_move(uci_move) {
            make_move_basic(&mut self.board, mv);
            true
        } else {
            false
        }
    }

    /// Search to a fixed depth. Returns JSON:
    /// { "bestmove": "e2e4", "score": 35, "from_book": false }
    pub fn go_depth(&mut self, depth: i32) -> String {
        self.run_search(depth, None)
    }

    /// Search for a fixed time in milliseconds. Returns same JSON as go_depth.
    pub fn go_movetime(&mut self, ms: u32) -> String {
        self.run_search(64, Some(Duration::from_millis(ms as u64)))
    }

    /// Get the current board position as a FEN string.
    pub fn get_fen(&self) -> String {
        self.board.to_fen()
    }

    /// Get all legal moves as a JSON array: ["e2e4", "d2d4", ...]
    pub fn get_legal_moves(&mut self) -> String {
        let mut moves: Vec<Move> = Vec::with_capacity(256);
        let mut scratch: Vec<Move> = Vec::with_capacity(256);
        generate_legal(
            &mut self.board,
            &self.magic_tables,
            &mut moves,
            &mut scratch,
        );

        let uci_moves: Vec<String> = moves.iter().map(|m| m.to_uci()).collect();
        format!(
            "[{}]",
            uci_moves
                .iter()
                .map(|m| format!("\"{}\"", m))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Get whose turn it is: "white" or "black"
    pub fn side_to_move(&self) -> String {
        match self.board.side_to_move {
            Color::White => "white".to_string(),
            Color::Black => "black".to_string(),
        }
    }
}

// Private helper methods (not exposed to JS)
impl VantageEngine {
    fn run_search(&mut self, depth: i32, time_limit: Option<Duration>) -> String {
        // Check opening book first
        if let Some(ref book) = self.book {
            if let Some(book_move) = book.probe(&self.board) {
                return format!(
                    r#"{{"bestmove":"{}","score":0,"from_book":true}}"#,
                    book_move.to_uci()
                );
            }
        }

        let (score, best_move) = search(
            &mut self.board,
            &self.magic_tables,
            &mut self.tt,
            depth,
            time_limit,
        );

        let move_str = best_move
            .map(|m| m.to_uci())
            .unwrap_or_else(|| "0000".to_string());

        format!(
            r#"{{"bestmove":"{}","score":{},"from_book":false}}"#,
            move_str, score
        )
    }

    fn parse_uci_move(&self, move_str: &str) -> Option<Move> {
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
                'q' => Some(Piece::Queen),
                'r' => Some(Piece::Rook),
                'b' => Some(Piece::Bishop),
                'n' => Some(Piece::Knight),
                _ => None,
            }
        } else {
            None
        };

        let mut moves: Vec<Move> = Vec::with_capacity(256);
        let mut scratch: Vec<Move> = Vec::with_capacity(256);
        let mut board_copy = self.board.clone();
        generate_legal(
            &mut board_copy,
            &self.magic_tables,
            &mut moves,
            &mut scratch,
        );

        for mv in moves {
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
}
