use crate::board::{Board, Color, Piece};
use crate::moves::king::KING_ATTACKS;
use crate::moves::knight::KNIGHT_ATTACKS;
use crate::moves::magic::MagicTables;
use crate::moves::pawn::pawn_attacks;
use crate::moves::types::Move;
use crate::square::Square;

/// Bitboard file masks (a1 = bit 0 … h8 = bit 63).
pub const FILE_A: u64 = 0x0101_0101_0101_0101;
pub const FILE_H: u64 = 0x8080_8080_8080_8080;

/// Returns a bitboard showing all the squares that *piece* could attack from *square*
pub fn attacks_from(
    piece: Piece,
    color: Color,
    square: u8,
    blockers: u64,
    tables: &MagicTables,
) -> u64 {
    let sq = square as usize;

    match piece {
        Piece::Knight => KNIGHT_ATTACKS[sq],
        Piece::King => KING_ATTACKS[sq],
        Piece::Pawn => pawn_attacks(square, color),
        Piece::Bishop => tables.bishop.get_attacks(square as usize, blockers),
        Piece::Rook => tables.rook.get_attacks(sq, blockers),
        Piece::Queen => tables.queen_attacks(sq, blockers),
    }
}

pub fn is_square_attacked(
    board: &Board,
    square: Square,
    attacker: Color,
    tables: &MagicTables,
) -> bool {
    let index = square.index();
    let target = 1u64 << index;

    let pawn_attackers = match attacker {
        Color::White => ((target & !FILE_H) >> 7) | ((target & !FILE_A) >> 9),
        Color::Black => ((target & !FILE_A) << 7) | ((target & !FILE_H) << 9),
    };

    if pawn_attackers & board.pieces(Piece::Pawn, attacker) != 0 {
        return true;
    }
    if KNIGHT_ATTACKS[index as usize] & board.pieces(Piece::Knight, attacker) != 0 {
        return true;
    }
    if KING_ATTACKS[index as usize] & board.pieces(Piece::King, attacker) != 0 {
        return true;
    }

    let occupied = board.occupied();

    let rook_attacks = tables.rook.get_attacks(index as usize, occupied);
    if rook_attacks & board.pieces(Piece::Rook, attacker) != 0 {
        return true;
    }

    let bishop_attacks = tables.bishop.get_attacks(index as usize, occupied);
    if bishop_attacks & board.pieces(Piece::Bishop, attacker) != 0 {
        return true;
    }

    if (rook_attacks | bishop_attacks) & board.pieces(Piece::Queen, attacker) != 0 {
        return true;
    }

    false
}

#[inline(always)]
pub fn in_check(board: &Board, side: Color, tables: &MagicTables) -> bool {
    let king_sq = board.king_square(side); // you’ll need this helper if not already implemented
    is_square_attacked(board, king_sq, side.opposite(), tables)
}

pub fn is_legal_castling(board: &Board, mv: Move, tables: &MagicTables) -> bool {
    let color = board.side_to_move;

    // 1. King must not be in check
    if in_check(board, color, tables) {
        return false;
    }

    // 2. Check squares king passes through
    let (start_idx, middle_idx, end_idx) = match (color, mv.to.index()) {
        (Color::White, 6) => (4, 5, 6),     // White kingside
        (Color::White, 2) => (4, 3, 2),     // White queenside
        (Color::Black, 62) => (60, 61, 62), // Black kingside
        (Color::Black, 58) => (60, 59, 58), // Black queenside
        _ => return false,
    };

    // After computing (start_idx, middle_idx, end_idx):
    let opp = color.opposite();
    for &test_idx in &[start_idx, middle_idx, end_idx] {
        if is_square_attacked(board, Square::from_index(test_idx as u8), opp, tables) {
            return false;
        }
    }

    true
}

/// Test suite for the `attacks_from` function across all piece types
#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Color, Piece};
    use crate::moves::magic::loader::load_magic_tables;
    use crate::moves::{king, knight, pawn};

    fn tables() -> MagicTables {
        load_magic_tables()
    }

    #[test]
    fn knight_attacks_center() {
        let t = tables();
        let d4 = 3 + 8 * 3;
        assert_eq!(
            attacks_from(Piece::Knight, Color::White, d4, 0, &t),
            knight::KNIGHT_ATTACKS[d4 as usize]
        );
    }

    #[test]
    fn knight_attacks_corner() {
        let t = tables();
        let a1 = 0;
        assert_eq!(
            attacks_from(Piece::Knight, Color::Black, a1, 0, &t),
            knight::KNIGHT_ATTACKS[a1 as usize]
        );
    }

    #[test]
    fn king_attacks_center() {
        let t = tables();
        let d4 = 3 + 8 * 3;
        assert_eq!(
            attacks_from(Piece::King, Color::White, d4, 0, &t),
            king::KING_ATTACKS[d4 as usize]
        );
    }

    #[test]
    fn king_attacks_corner() {
        let t = tables();
        let h8 = 7 + 8 * 7;
        assert_eq!(
            attacks_from(Piece::King, Color::White, h8, 0, &t),
            king::KING_ATTACKS[h8 as usize]
        );
    }

    #[test]
    fn white_pawn_attacks_center() {
        let t = tables();
        let e4 = 4 + 8 * 3;
        assert_eq!(
            attacks_from(Piece::Pawn, Color::White, e4, 0, &t),
            pawn::pawn_attacks(e4, Color::White)
        );
    }

    #[test]
    fn black_pawn_attacks_edge() {
        let t = tables();
        let a5 = 8 * 4;
        assert_eq!(
            attacks_from(Piece::Pawn, Color::Black, a5, 0, &t),
            pawn::pawn_attacks(a5, Color::Black)
        );
    }

    #[test]
    fn bishop_attacks_empty_board() {
        let t = tables();
        let c1 = 2;
        let blockers = 0;
        let expected = t.bishop.get_attacks(c1, blockers);
        assert_eq!(
            attacks_from(Piece::Bishop, Color::White, c1 as u8, blockers, &t),
            expected
        );
    }

    #[test]
    fn bishop_attacks_with_blockers() {
        let t = tables();
        let d4 = 3 + 8 * 3;
        let blockers = (1 << (d4 + 9)) | (1 << (d4 - 9));
        let expected = t.bishop.get_attacks(d4, blockers);
        assert_eq!(
            attacks_from(Piece::Bishop, Color::White, d4 as u8, blockers, &t),
            expected
        );
    }

    #[test]
    fn rook_attacks_empty_board() {
        let t = tables();
        let a1 = 0;
        let blockers = 0;
        let expected = t.rook.get_attacks(a1, blockers);
        assert_eq!(
            attacks_from(Piece::Rook, Color::Black, a1 as u8, blockers, &t),
            expected
        );
    }

    #[test]
    fn rook_attacks_with_blockers() {
        let t = tables();
        let e5 = 4 + 8 * 4;
        let blockers = (1 << (e5 + 8)) | (1 << (e5 - 1));
        let expected = t.rook.get_attacks(e5, blockers);
        assert_eq!(
            attacks_from(Piece::Rook, Color::Black, e5 as u8, blockers, &t),
            expected
        );
    }

    #[test]
    fn queen_attacks_combined() {
        let t = tables();
        let e4 = 4 + 8 * 3;
        let blockers = (1 << (e4 + 8)) | (1 << (e4 - 7));
        let expected = t.bishop.get_attacks(e4, blockers) | t.rook.get_attacks(e4, blockers);
        assert_eq!(
            attacks_from(Piece::Queen, Color::White, e4 as u8, blockers, &t),
            expected
        );
    }
}
