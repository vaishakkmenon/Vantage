mod fen;

use crate::bitboard::BitboardExt;
use crate::square::Square;
use std::fmt;
use std::str::FromStr;

pub mod castle_bits;
mod fen_tables;
pub use castle_bits::*;

/// Starting position constants
// ———————— White side (ranks 1 & 2) ————————
// Pawns on rank 2: bits 8–15
const WHITE_PAWN_MASK: u64 = 0x0000_0000_0000_FF00;
// Individual back-rank pieces on rank 1:
// Rooks on a1 (bit 0) and h1 (bit 7)
const WHITE_ROOK_MASK: u64 = (1 << 0) | (1 << 7); // 0x0000_0000_0000_0081
// Knights on b1 (bit 1) and g1 (bit 6)
const WHITE_KNIGHT_MASK: u64 = (1 << 1) | (1 << 6); // 0x0000_0000_0000_0042
// Bishops on c1 (bit 2) and f1 (bit 5)
const WHITE_BISHOP_MASK: u64 = (1 << 2) | (1 << 5); // 0x0000_0000_0000_0024
// Queen on d1 (bit 3)
const WHITE_QUEEN_MASK: u64 = 1 << 3; // 0x0000_0000_0000_0008
// King on e1 (bit 4)
const WHITE_KING_MASK: u64 = 1 << 4; // 0x0000_0000_0000_0010

// ———————— Black side (ranks 7 & 8) ————————
// Pawns on rank 7: bits 48–55
const BLACK_PAWN_MASK: u64 = 0x00FF_0000_0000_0000; // a7–h7
// Individual back-rank pieces on rank 8:
// Rooks on a8 (bit 56) and h8 (bit 63)
const BLACK_ROOK_MASK: u64 = (1 << 56) | (1 << 63); // 0x8100_0000_0000_0000
// Knights on b8 (bit 57) and g8 (bit 62)
const BLACK_KNIGHT_MASK: u64 = (1 << 57) | (1 << 62); // 0x4200_0000_0000_0000
// Bishops on c8 (bit 58) and f8 (bit 61)
const BLACK_BISHOP_MASK: u64 = (1 << 58) | (1 << 61); // 0x2400_0000_0000_0000
// Queen on d8 (bit 59)
const BLACK_QUEEN_MASK: u64 = 1 << 59; // 0x0800_0000_0000_0000
// King on e8 (bit 60)
const BLACK_KING_MASK: u64 = 1 << 60; // 0x1000_0000_0000_0000

// Empty square value, no piece 0-11 will coincide with 255
pub(crate) const EMPTY_SQ: u8 = 0xFF;

/// Which side is to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

/// Piece enum to hold all types of pieces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

/// Core board representation using bitboards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    /// White Pieces
    pub piece_bb: [[u64; 6]; 2],
    /// Occupancy fields
    pub occ_white: u64,
    pub occ_black: u64,
    pub occ_all: u64,
    /// Lookup table for each square
    pub piece_on_sq: [u8; 64], // new table: 0xFF = empty, 0–11 = (color<<3)|piece
    /// White or Black to move
    pub side_to_move: Color,
    /// Castling rights: bit 0=White kingside, 1=White queenside, 2=Black kingside, 3=Black queenside
    pub castling_rights: u8,
    /// En passant target square, as 0–63, or None if not available.
    pub en_passant: Option<Square>,
    /// Halfmove clock (for fifty-move draw rule).
    pub halfmove_clock: u32,
    /// Fullmove number (starts at 1 and increments after Black’s move).
    pub fullmove_number: u32,
    // Zobrist hash for each board.
    pub zobrist: u64,
    // History for zobrist hashing
    pub history: Vec<u64>,
}

impl Board {
    /// Recompute from current state and store into `self.zobrist`.
    #[inline]
    pub fn refresh_zobrist(&mut self) {
        self.zobrist = self.compute_zobrist_full();
    }

    #[inline(always)]
    pub(crate) fn bb(&self, color: Color, piece: Piece) -> u64 {
        self.piece_bb[color as usize][piece as usize]
    }

    #[inline(always)]
    pub(crate) fn set_bb(&mut self, color: Color, piece: Piece, new_bb: u64) {
        use crate::hash::zobrist::zobrist_keys;
        let ci = color as usize;
        let pi = piece as usize;

        let old_bb = self.piece_bb[ci][pi];
        let delta = old_bb ^ new_bb;
        if delta == 0 {
            return;
        }

        // store new bitboard
        self.piece_bb[ci][pi] = new_bb;

        // side occupancies
        if color == Color::White {
            self.occ_white ^= delta;
        } else {
            self.occ_black ^= delta;
        }
        self.occ_all = self.occ_white | self.occ_black;

        // --- ZOBRIST: toggle piece keys for all squares that changed ---
        let keys = zobrist_keys();

        let mut bits_to_update = delta;
        while bits_to_update != 0 {
            // isolate one toggled square
            let single_bit = bits_to_update & (!bits_to_update + 1);
            let sq_idx = single_bit.trailing_zeros() as usize;

            // update piece_on_sq table
            if new_bb & single_bit != 0 {
                self.place_piece_at_sq(color, piece, Square::from_index(sq_idx as u8));
            } else {
                self.clear_square(Square::from_index(sq_idx as u8));
            }

            // Zobrist: XOR the piece key (works for both add and remove)
            self.zobrist ^= keys.piece[ci][pi][sq_idx];

            bits_to_update &= bits_to_update - 1;
        }
    }

    #[inline(always)]
    pub(crate) fn clear_square(&mut self, sq: Square) {
        let i = sq.index() as usize;
        self.piece_on_sq[i] = EMPTY_SQ;
    }

    #[inline(always)]
    pub(crate) fn place_piece_at_sq(&mut self, color: Color, piece: Piece, sq: Square) {
        let i = sq.index() as usize;
        self.piece_on_sq[i] = (color as u8) << 3 | (piece as u8);
    }

    /// Create an empty board (all bitboards zero, White to move).
    pub fn new_empty() -> Self {
        let mut b = Board {
            piece_bb: [[0u64; 6]; 2],
            occ_white: 0,
            occ_black: 0,
            occ_all: 0,
            piece_on_sq: [EMPTY_SQ; 64],
            side_to_move: Color::White,
            castling_rights: 0,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            zobrist: 0,
            history: Vec::new(),
        };
        b.refresh_zobrist();
        b
    }

    pub fn new() -> Self {
        let mut b = Board::new_empty();
        // Set up white pieces
        b.set_bb(Color::White, Piece::Pawn, WHITE_PAWN_MASK);
        b.set_bb(Color::White, Piece::Bishop, WHITE_BISHOP_MASK);
        b.set_bb(Color::White, Piece::Knight, WHITE_KNIGHT_MASK);
        b.set_bb(Color::White, Piece::Rook, WHITE_ROOK_MASK);
        b.set_bb(Color::White, Piece::Queen, WHITE_QUEEN_MASK);
        b.set_bb(Color::White, Piece::King, WHITE_KING_MASK);

        // Set up black pieces
        b.set_bb(Color::Black, Piece::Pawn, BLACK_PAWN_MASK);
        b.set_bb(Color::Black, Piece::Bishop, BLACK_BISHOP_MASK);
        b.set_bb(Color::Black, Piece::Knight, BLACK_KNIGHT_MASK);
        b.set_bb(Color::Black, Piece::Rook, BLACK_ROOK_MASK);
        b.set_bb(Color::Black, Piece::Queen, BLACK_QUEEN_MASK);
        b.set_bb(Color::Black, Piece::King, BLACK_KING_MASK);

        // Setup side to move and other important information
        b.side_to_move = Color::White;
        b.castling_rights = CASTLE_WK | CASTLE_WQ | CASTLE_BK | CASTLE_BQ;
        b.en_passant = None;
        b.halfmove_clock = 0;
        b.fullmove_number = 1;
        b.refresh_zobrist();
        b.history.clear();
        // With the new logic we don't necessarily push the initial state here if we consider make_move pushes PRE-move.
        // BUT for consistency with fen.rs and standard practice:
        // If we treat history as "previous positions", initially there are none.
        // However, if we want `is_repetition` to work for positions reached via transposition,
        // we usually just scan the history stack.
        // The user request says: "Before you update... push the current board.zobrist".
        // So `new()` should probably start empty or consistent with fen.
        // Let's leave it empty here? Or push it?
        // Wait, standard `new()` is the start of the game.
        // If we make a move from here, we push THIS position.
        // So initially `history` should be empty or contain previous game states (none).
        // BUT `fen.rs` pushes it.
        // Let's see what I did in `fen.rs` planning... I said "initialize correctly".
        // If make_move pushes the *current* (pre-move) hash, then after 1 move, history has 1 entry (start pos).
        // If we revert to start pos, we pop. History empty.
        // So `new()` should imply empty history or unrelated to current pos.
        // BUT wait, `repetition_count` uses it.
        // Let's stick to: history contains *previous* positions since last irreversible.
        // So `new()` starts effectively empty or with just itself if we consider it "visited".
        // Actually, if we are at root, and we haven't made moves, history is empty.
        // If we reach this position again via search, we compare against history.
        // The User said: "In search, finding the current position *once* in the history means we have reached a cycle".
        // So `history` should contain the ancestors.
        // If `new()` creates a board, it has no ancestors (unless FEN says so).
        // So `b.history.clear()` is correct. remove the push.
        b
    }

    #[inline(always)]
    /// Bitboard of all pieces (both colors).
    pub fn occupied(&self) -> u64 {
        self.occ_all
    }

    #[inline(always)]
    pub fn has_castling(&self, flag: u8) -> bool {
        self.castling_rights & flag != 0
    }

    /// Validate that no square is occupied by more than one piece.
    /// Returns Ok if valid, Err describing the overlap if invalid.
    pub fn validate(&self) -> Result<(), String> {
        let bitboards = [
            ("white_pawns", self.bb(Color::White, Piece::Pawn)),
            ("white_knights", self.bb(Color::White, Piece::Knight)),
            ("white_bishops", self.bb(Color::White, Piece::Bishop)),
            ("white_rooks", self.bb(Color::White, Piece::Rook)),
            ("white_queens", self.bb(Color::White, Piece::Queen)),
            ("white_king", self.bb(Color::White, Piece::King)),
            ("black_pawns", self.bb(Color::Black, Piece::Pawn)),
            ("black_knights", self.bb(Color::Black, Piece::Knight)),
            ("black_bishops", self.bb(Color::Black, Piece::Bishop)),
            ("black_rooks", self.bb(Color::Black, Piece::Rook)),
            ("black_queens", self.bb(Color::Black, Piece::Queen)),
            ("black_king", self.bb(Color::Black, Piece::King)),
        ];

        let mut seen: u64 = 0;
        for (name, bb) in &bitboards {
            if (seen & bb) != 0 {
                return Err(format!("Bitboard `{}` overlaps with another piece", name));
            }
            seen |= bb;
        }
        Ok(())
    }

    #[inline(always)]
    /// Bitboard of all pieces for one side.
    pub fn occupancy(&self, color: Color) -> u64 {
        match color {
            Color::White => self.occ_white,
            Color::Black => self.occ_black,
        }
    }

    /// Shorthand for the opponent’s occupancy.
    pub fn opponent_occupancy(&self, color: Color) -> u64 {
        self.occupancy(color.opposite())
    }

    #[inline(always)]
    /// Single‐slot accessor for a given piece & color.
    pub fn pieces(&self, piece: Piece, color: Color) -> u64 {
        self.bb(color, piece)
    }

    /// Returns the piece and color at a given square, or None if empty.
    #[inline(always)]
    pub fn piece_at(&self, sq: Square) -> Option<(Color, Piece)> {
        let val = self.piece_on_sq[sq.index() as usize];
        if val == EMPTY_SQ {
            None
        } else {
            let color = Color::from_u8((val >> 3) & 1);
            let piece = Piece::from_u8(val & 0b111);
            Some((color, piece))
        }
    }

    /// Returns just the piece at a given square (ignoring color), or None if empty.
    #[inline(always)]
    pub fn piece_type_at(&self, sq: Square) -> Option<Piece> {
        self.piece_at(sq).map(|(_, piece)| piece)
    }

    /// Returns just the color at a given square, or None if empty.
    #[inline(always)]
    pub fn color_at(&self, sq: Square) -> Option<Color> {
        self.piece_at(sq).map(|(color, _)| color)
    }

    // Utility Aliases
    #[inline(always)]
    pub fn en_passant_target(&self) -> Option<Square> {
        self.en_passant
    }

    #[inline(always)]
    pub fn has_kingside_castle(&self, color: Color) -> bool {
        match color {
            Color::White => self.castling_rights & CASTLE_WK != 0,
            Color::Black => self.castling_rights & CASTLE_BK != 0,
        }
    }

    #[inline(always)]
    pub fn has_queenside_castle(&self, color: Color) -> bool {
        match color {
            Color::White => self.castling_rights & CASTLE_WQ != 0,
            Color::Black => self.castling_rights & CASTLE_BQ != 0,
        }
    }

    /// Checks if a side has any non-pawn material (N, B, R, Q).
    /// Used for Null Move Pruning to avoid Zugzwang in pawn-only endgames.
    #[inline(always)]
    pub fn has_major_pieces(&self, color: Color) -> bool {
        let knights = self.bb(color, Piece::Knight);
        let bishops = self.bb(color, Piece::Bishop);
        let rooks = self.bb(color, Piece::Rook);
        let queens = self.bb(color, Piece::Queen);
        (knights | bishops | rooks | queens) != 0
    }

    /// Function to get exactly what square the king sits on
    #[inline(always)]
    pub fn king_square(&self, color: Color) -> Square {
        let king_bb = self.pieces(Piece::King, color);
        if king_bb == 0 {
            panic!(
                "King missing for {:?}! \nFEN: {}\nOcc: {:#x}",
                color,
                self.to_fen(),
                self.occupied()
            );
        }
        Square::try_from(king_bb.lsb()).expect("Invalid king bitboard")
    }

    /// Full recompute from current state. Must match the incremental hash at all times.
    pub fn compute_zobrist_full(&self) -> u64 {
        use crate::hash::zobrist::zobrist_keys;

        let keys = zobrist_keys();
        let mut board_hash: u64 = 0;

        // 1) Pieces by (color, piece)
        // Prefer iterating bitboards for speed; falls back nicely if you don’t have a helper.
        #[inline]
        fn idx_of(c: Color, p: Piece) -> (usize, usize) {
            let ci = match c {
                Color::White => 0,
                Color::Black => 1,
            };
            let pi = match p {
                Piece::Pawn => 0,
                Piece::Knight => 1,
                Piece::Bishop => 2,
                Piece::Rook => 3,
                Piece::Queen => 4,
                Piece::King => 5,
            };
            (ci, pi)
        }

        const COLORS: [Color; 2] = [Color::White, Color::Black];
        const PIECES: [Piece; 6] = [
            Piece::Pawn,
            Piece::Knight,
            Piece::Bishop,
            Piece::Rook,
            Piece::Queen,
            Piece::King,
        ];
        // Iterate all 12 piece bitboards.
        for &c in &COLORS {
            for &p in &PIECES {
                let (ci, pi) = idx_of(c, p);
                let mut bb = self.bb(c, p);
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    board_hash ^= keys.piece[ci][pi][sq];
                    bb &= bb - 1; // pop LSB
                }
            }
        }

        // 2) Side to move (only when Black to move)
        if self.side_to_move == Color::Black {
            board_hash ^= keys.side_to_move;
        }

        // 3) Castling rights in K,Q,k,q bit order (your bitfield matches this)
        let rights = self.castling_rights; // assume u8 with bits 0..3 = K,Q,k,q
        if (rights & CASTLE_WK) != 0 {
            board_hash ^= keys.castling[0];
        } // K
        if (rights & CASTLE_WQ) != 0 {
            board_hash ^= keys.castling[1];
        } // Q
        if (rights & CASTLE_BK) != 0 {
            board_hash ^= keys.castling[2];
        } // k
        if (rights & CASTLE_BQ) != 0 {
            board_hash ^= keys.castling[3];
        } // q

        // 4) En passant (only if capturable this ply)
        if let Some(file) = crate::hash::zobrist::ep_file_to_hash(self) {
            board_hash ^= keys.ep_file[file as usize];
        }

        board_hash
    }

    /// Counts occurrences of the *current* Zobrist in the history window.
    pub fn repetition_count(&self) -> u8 {
        let mut count: u8 = 0;
        let current = self.zobrist;
        // Check history
        for &k in &self.history {
            if k == current {
                count = count.saturating_add(1);
            }
        }
        // *Also* count the current position itself?
        // If history contains *ancestors*, they are prior occurrences.
        // If we are at the same position as an ancestor, that is a repetition.
        // So count is at least 1 (the current one, implicitly) plus any history matches?
        // Or does history include current? Logic says "push current before move".
        // So history contains: S0 -> S1 -> S2.
        // If we are at S3. current=S3. history=[S0, S1, S2].
        // If S3 == S1, then we have seen it before.
        // The *total* count of S3 is 1 (current) + 1 (S1) = 2.
        // So we should start count at 1.
        count = count.saturating_add(1);
        count
    }

    pub fn is_repetition(&self) -> bool {
        let current_hash = self.zobrist;
        for &past_hash in self.history.iter().rev() {
            if past_hash == current_hash {
                return true;
            }
        }
        false
    }

    /// True iff `repetition_count() >= 3`
    pub fn is_threefold(&self) -> bool {
        self.repetition_count() >= 3
    }

    #[cfg(debug_assertions)]
    #[inline]
    pub fn assert_hash(&self) {
        // Recompute using the same logic as compute_zobrist_full()
        let full = self.compute_zobrist_full();
        debug_assert_eq!(
            self.zobrist, full,
            "Zobrist parity mismatch: stored={:#018x}, full={:#018x}",
            self.zobrist, full
        );
    }
}

impl Color {
    pub fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    /// Decode a 0/1 value into a Color.
    #[inline(always)]
    pub(crate) fn from_u8(v: u8) -> Self {
        match v {
            0 => Color::White,
            1 => Color::Black,
            _ => panic!("Invalid Color encoding: {}", v),
        }
    }
}

impl std::ops::Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.opposite()
    }
}

impl Piece {
    /// Decode a 0–5 value into a Piece.
    #[inline(always)]
    pub(crate) fn from_u8(v: u8) -> Self {
        match v {
            0 => Piece::Pawn,
            1 => Piece::Knight,
            2 => Piece::Bishop,
            3 => Piece::Rook,
            4 => Piece::Queen,
            5 => Piece::King,
            _ => panic!("Invalid Piece encoding: {}", v),
        }
    }

    pub fn value(&self) -> i32 {
        match self {
            Piece::Pawn => 100,
            Piece::Knight => 320,
            Piece::Bishop => 330,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 0,
        }
    }

    pub fn attacker_value(&self) -> i32 {
        match self {
            Piece::Pawn => 1,
            Piece::Knight => 2,
            Piece::Bishop => 3,
            Piece::Rook => 4,
            Piece::Queen => 5,
            Piece::King => 6,
        }
    }
}

/// An all-zero board (no pieces) with White to move.
impl Default for Board {
    fn default() -> Self {
        Board::new_empty()
    }
}

impl FromStr for Board {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut board = Board::new_empty();
        board.set_fen(s)?;
        Ok(board)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fen = self.to_fen();
        write!(f, "{}", fen)
    }
}

#[cfg(test)]
mod tests;
