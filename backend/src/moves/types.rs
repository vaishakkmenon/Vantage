use crate::board::{Color, Piece};
use crate::square::Square;
use arrayvec::ArrayVec;
use std::fmt;
use std::ops::{Deref, DerefMut};

pub trait MoveBuffer: Deref<Target = [Move]> + DerefMut {
    fn push(&mut self, mv: Move);
    fn clear(&mut self);
}

impl MoveBuffer for Vec<Move> {
    fn push(&mut self, mv: Move) {
        self.push(mv);
    }
    fn clear(&mut self) {
        self.clear();
    }
}

impl<const N: usize> MoveBuffer for ArrayVec<Move, N> {
    fn push(&mut self, mv: Move) {
        self.push(mv);
    }
    fn clear(&mut self) {
        self.clear();
    }
}

// Move flag encoding (4 bits)
// Bits 0-1: Special move type (00=quiet, 01=double pawn, 10=kingside castle, 11=queenside castle)
// Bit 2: Capture flag
// Bit 3: Promotion flag
pub const QUIET_MOVE: u8 = 0b0000;
pub const DOUBLE_PAWN_PUSH: u8 = 0b0001;
pub const KINGSIDE_CASTLE: u8 = 0b0010;
pub const QUEENSIDE_CASTLE: u8 = 0b0011;
pub const CAPTURE: u8 = 0b0100;
pub const EN_PASSANT: u8 = 0b0101;
pub const PROMOTION: u8 = 0b1000;
pub const PROMOTION_CAPTURE: u8 = 0b1100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub piece: Piece,
    pub promotion: Option<Piece>,
    pub flags: u8,
}

impl Move {
    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        (self.flags & CAPTURE) != 0
    }

    #[inline(always)]
    pub fn is_en_passant(&self) -> bool {
        self.flags == EN_PASSANT
    }

    #[inline(always)]
    pub fn is_castling(&self) -> bool {
        self.flags == KINGSIDE_CASTLE || self.flags == QUEENSIDE_CASTLE
    }

    #[inline(always)]
    pub fn is_kingside_castle(&self) -> bool {
        self.flags == KINGSIDE_CASTLE
    }

    #[inline(always)]
    pub fn is_queenside_castle(&self) -> bool {
        self.flags == QUEENSIDE_CASTLE
    }

    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        (self.flags & PROMOTION) != 0
    }

    #[inline(always)]
    pub fn is_double_pawn_push(&self) -> bool {
        self.flags == DOUBLE_PAWN_PUSH
    }

    #[inline(always)]
    pub fn is_quiet(&self) -> bool {
        self.flags == QUIET_MOVE
    }

    pub fn to_uci(&self) -> String {
        let promo = if let Some(p) = self.promotion {
            match p {
                Piece::Queen => "q",
                Piece::Rook => "r",
                Piece::Bishop => "b",
                Piece::Knight => "n",
                _ => "",
            }
        } else {
            ""
        };

        format!("{}{}{}", self.from, self.to, promo)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undo {
    pub from: Square,
    pub to: Square,
    pub piece: Piece,
    pub color: Color,
    pub prev_side: Color,
    pub capture: Option<(Color, Piece, Square)>,
    pub castling_rook: Option<(Square /*rook_from*/, Square /*rook_to*/)>,

    pub prev_castling_rights: u8,
    pub promotion: Option<Piece>,
    pub prev_en_passant: Option<Square>,

    pub prev_halfmove_clock: u32,
    pub prev_fullmove_number: u32,
    pub prev_history: Option<Vec<u64>>,
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Start with from->to like e2e4
        let mut s = format!("{}{}", self.from, self.to);

        // Add promotion piece if applicable (lowercase for UCI style)
        if let Some(promo) = self.promotion {
            let c = match promo {
                Piece::Queen => 'q',
                Piece::Rook => 'r',
                Piece::Bishop => 'b',
                Piece::Knight => 'n',
                _ => '?', // Should never happen
            };
            s.push(c);
        }

        // If verbose mode requested, we could add special tags
        if f.alternate() {
            // like "{:#}" formatting
            if self.is_castling() {
                s.push_str(" (castle)");
            } else if self.is_en_passant() {
                s.push_str(" (ep)");
            } else if self.is_capture() {
                s.push_str(" (x)");
            }
        }

        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullMoveUndo {
    pub prev_en_passant: Option<Square>,
    pub prev_halfmove_clock: u32,
    pub prev_side: Color,
}
