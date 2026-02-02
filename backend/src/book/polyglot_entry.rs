use crate::board::{Board, Piece};
use crate::moves::types::{
    CAPTURE, EN_PASSANT, KINGSIDE_CASTLE, Move, PROMOTION, PROMOTION_CAPTURE, QUEENSIDE_CASTLE,
    QUIET_MOVE,
};
use crate::square::Square;

#[derive(Debug, Clone, Copy)]
pub struct PolyglotEntry {
    pub key: u64,
    pub move_poly: u16,
    pub weight: u16,
    pub learn: u32,
}

impl PolyglotEntry {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let key = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let move_poly = u16::from_be_bytes(bytes[8..10].try_into().unwrap());
        let weight = u16::from_be_bytes(bytes[10..12].try_into().unwrap());
        let learn = u32::from_be_bytes(bytes[12..16].try_into().unwrap());

        Self {
            key,
            move_poly,
            weight,
            learn,
        }
    }

    /// Decode the Polyglot move encoding to engine Move.
    /// Requires the board to determine piece type, captures, etc.
    pub fn decode_move(&self, board: &Board) -> Option<Move> {
        let to_file = (self.move_poly & 0x7) as u8;
        let to_rank = ((self.move_poly >> 3) & 0x7) as u8;
        let from_file = ((self.move_poly >> 6) & 0x7) as u8;
        let from_rank = ((self.move_poly >> 9) & 0x7) as u8;
        let promo_bits = (self.move_poly >> 12) & 0x7;

        let from_sq = Square::from_file_rank(from_file, from_rank);
        let to_sq_raw = Square::from_file_rank(to_file, to_rank);

        let from_idx = from_sq.index();
        let to_idx_raw = to_sq_raw.index();

        // Get the piece at the from square
        let (_color, piece) = board.piece_at(from_sq)?;

        // --- Castling Fix ---
        // Polyglot encodes castling as king->rook, engine uses king->destination
        // E1=4, G1=6, C1=2, H1=7, A1=0
        // E8=60, G8=62, C8=58, H8=63, A8=56
        if piece == Piece::King {
            // White kingside: e1->h1 in book -> e1->g1 in engine
            if from_idx == 4 && to_idx_raw == 7 {
                return Some(Move {
                    from: from_sq,
                    to: Square::from_index(6),
                    piece: Piece::King,
                    promotion: None,
                    flags: KINGSIDE_CASTLE,
                });
            }
            // White queenside: e1->a1 in book -> e1->c1 in engine
            if from_idx == 4 && to_idx_raw == 0 {
                return Some(Move {
                    from: from_sq,
                    to: Square::from_index(2),
                    piece: Piece::King,
                    promotion: None,
                    flags: QUEENSIDE_CASTLE,
                });
            }
            // Black kingside: e8->h8 in book -> e8->g8 in engine
            if from_idx == 60 && to_idx_raw == 63 {
                return Some(Move {
                    from: from_sq,
                    to: Square::from_index(62),
                    piece: Piece::King,
                    promotion: None,
                    flags: KINGSIDE_CASTLE,
                });
            }
            // Black queenside: e8->a8 in book -> e8->c8 in engine
            if from_idx == 60 && to_idx_raw == 56 {
                return Some(Move {
                    from: from_sq,
                    to: Square::from_index(58),
                    piece: Piece::King,
                    promotion: None,
                    flags: QUEENSIDE_CASTLE,
                });
            }
        }

        let to_sq = to_sq_raw;

        // Check for promotion
        let promotion = match promo_bits {
            1 => Some(Piece::Knight),
            2 => Some(Piece::Bishop),
            3 => Some(Piece::Rook),
            4 => Some(Piece::Queen),
            _ => None,
        };

        // Determine if it's a capture
        let is_capture = board.piece_at(to_sq).is_some();

        // Check for en passant
        let is_en_passant = piece == Piece::Pawn
            && board.en_passant == Some(to_sq)
            && from_sq.file() != to_sq.file();

        // Determine flags
        let flags = if promotion.is_some() {
            if is_capture {
                PROMOTION_CAPTURE
            } else {
                PROMOTION
            }
        } else if is_en_passant {
            EN_PASSANT
        } else if is_capture {
            CAPTURE
        } else {
            QUIET_MOVE
        };

        Some(Move {
            from: from_sq,
            to: to_sq,
            piece,
            promotion,
            flags,
        })
    }
}
