use super::polyglot_keys::POLYGLOT_RANDOMS;
use crate::board::castle_bits::{CASTLE_BK, CASTLE_BQ, CASTLE_WK, CASTLE_WQ};
use crate::board::{Board, Color, Piece};
use crate::square::Square;

pub fn compute_polyglot_hash(board: &Board) -> u64 {
    let mut hash: u64 = 0;

    // 1. Hash Pieces
    for sq_idx in 0..64u8 {
        let sq = Square::from_index(sq_idx);
        // board.piece_at returns Option<(Color, Piece)>
        if let Some((color, piece_type)) = board.piece_at(sq) {
            let pt_idx = match piece_type {
                Piece::Pawn => 0,
                Piece::Knight => 1,
                Piece::Bishop => 2,
                Piece::Rook => 3,
                Piece::Queen => 4,
                Piece::King => 5,
            };

            let color_offset = if color == Color::White { 1 } else { 0 };
            let piece_kind = pt_idx * 2 + color_offset;

            let random_idx = (64 * piece_kind) + sq_idx as usize;
            hash ^= POLYGLOT_RANDOMS[random_idx];
        }
    }

    // 2. Hash Castling Rights (using bitmask operations)
    let rights = board.castling_rights;
    if (rights & CASTLE_WK) != 0 {
        hash ^= POLYGLOT_RANDOMS[768];
    }
    if (rights & CASTLE_WQ) != 0 {
        hash ^= POLYGLOT_RANDOMS[769];
    }
    if (rights & CASTLE_BK) != 0 {
        hash ^= POLYGLOT_RANDOMS[770];
    }
    if (rights & CASTLE_BQ) != 0 {
        hash ^= POLYGLOT_RANDOMS[771];
    }

    // 3. Hash En Passant
    if let Some(ep_sq) = board.en_passant {
        let ep_file = ep_sq.file();

        if is_pawn_adjacent(board, ep_sq, board.side_to_move) {
            hash ^= POLYGLOT_RANDOMS[772 + ep_file as usize];
        }
    }

    // 4. Hash Turn (Polyglot XORs when WHITE to move)
    if board.side_to_move == Color::White {
        hash ^= POLYGLOT_RANDOMS[780];
    }

    hash
}

fn is_pawn_adjacent(board: &Board, ep_sq: Square, side: Color) -> bool {
    let ep_rank = ep_sq.rank();
    let ep_file = ep_sq.file();

    // EP square rank check: rank 5 (index 5) for White capturing, rank 2 (index 2) for Black
    let capture_rank = if side == Color::White {
        if ep_rank != 5 {
            return false;
        }
        4 // White pawns that can capture are on rank 4
    } else {
        if ep_rank != 2 {
            return false;
        }
        3 // Black pawns that can capture are on rank 3
    };

    let left_file = if ep_file > 0 { Some(ep_file - 1) } else { None };
    let right_file = if ep_file < 7 { Some(ep_file + 1) } else { None };

    let check_sq = |f: u8| -> bool {
        let sq = Square::from_file_rank(f, capture_rank);
        if let Some((color, piece)) = board.piece_at(sq) {
            return piece == Piece::Pawn && color == side;
        }
        false
    };

    if let Some(f) = left_file
        && check_sq(f) {
            return true;
        }
    if let Some(f) = right_file
        && check_sq(f) {
            return true;
        }

    false
}
