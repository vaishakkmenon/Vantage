use crate::bitboard::BitboardExt;
use crate::board::{Board, Color, Piece};
use crate::moves::magic::MagicTables;
use crate::moves::types::Move;

pub trait SeeExt {
    fn static_exchange_eval(&self, m: Move, threshold: i32, tables: &MagicTables) -> bool;
    fn get_attackers_to_square_see(&self, square: u8, occupancy: u64, tables: &MagicTables) -> u64;
    fn get_lva_square(&self, attackers: u64, side: Color, occ: u64) -> u8;
}

impl SeeExt for Board {
    fn get_attackers_to_square_see(&self, square: u8, occupancy: u64, tables: &MagicTables) -> u64 {
        let sq_usize = square as usize;

        // 1. Pawns
        let white_pawns = crate::moves::pawn::pawn_attacks(square, Color::Black)
            & self.pieces(Piece::Pawn, Color::White);
        let black_pawns = crate::moves::pawn::pawn_attacks(square, Color::White)
            & self.pieces(Piece::Pawn, Color::Black);

        // 2. Knights
        let knights =
            self.pieces(Piece::Knight, Color::White) | self.pieces(Piece::Knight, Color::Black);
        let knight_attacks = crate::moves::knight::KNIGHT_ATTACKS[sq_usize] & knights;

        // 3. Kings
        let kings = self.pieces(Piece::King, Color::White) | self.pieces(Piece::King, Color::Black);
        let king_attacks = crate::moves::king::KING_ATTACKS[sq_usize] & kings;

        // 4. Sliders
        let bishop_queens = self.pieces(Piece::Bishop, Color::White)
            | self.pieces(Piece::Bishop, Color::Black)
            | self.pieces(Piece::Queen, Color::White)
            | self.pieces(Piece::Queen, Color::Black);
        let rook_queens = self.pieces(Piece::Rook, Color::White)
            | self.pieces(Piece::Rook, Color::Black)
            | self.pieces(Piece::Queen, Color::White)
            | self.pieces(Piece::Queen, Color::Black);

        let diag = tables.bishop.get_attacks(sq_usize, occupancy) & bishop_queens;
        let orth = tables.rook.get_attacks(sq_usize, occupancy) & rook_queens;

        (white_pawns | black_pawns | knight_attacks | king_attacks | diag | orth) & occupancy
    }

    fn static_exchange_eval(&self, m: Move, threshold: i32, tables: &MagicTables) -> bool {
        let to_sq = m.to.index();
        let from_sq = m.from.index();

        let piece_value = |p: Piece| match p {
            Piece::Pawn => 100,
            Piece::Knight => 320,
            Piece::Bishop => 330,
            Piece::Rook => 500,
            Piece::Queen => 900,
            Piece::King => 20000,
        };

        // 1. Initial Exchange
        let next_victim_piece = if m.is_en_passant() {
            Piece::Pawn
        } else {
            match self.piece_type_at(m.to) {
                Some(p) => p,
                None => return threshold <= 0,
            }
        };

        let mut value = piece_value(next_victim_piece);

        // FIX 1: Add promotion value (e.g., +800 if promoting to Queen)
        if let Some(p) = m.promotion {
            value += piece_value(p) - piece_value(Piece::Pawn);
        }

        if value < threshold {
            return false;
        }

        // FIX 2: The attacker is now the PROMOTED piece (Queen), not the original Pawn
        let mut next_victim = if let Some(p) = m.promotion {
            p
        } else {
            self.piece_type_at(m.from).unwrap()
        };

        let mut gain = [0; 32];
        let mut d = 0;
        gain[d] = value;

        // 2. Occupancy Simulation
        let mut occupancy = self.occupied();
        occupancy &= !(1u64 << from_sq);

        let mut attackers = self.get_attackers_to_square_see(to_sq, occupancy, tables);
        let mut side_to_move = self.side_to_move.opposite();

        // 3. Swap Loop
        loop {
            d += 1;
            let attacker_sq = self.get_lva_square(attackers, side_to_move, occupancy);
            if attacker_sq == 64 {
                break;
            }

            attackers &= !(1u64 << attacker_sq);
            occupancy &= !(1u64 << attacker_sq);

            if matches!(next_victim, Piece::Bishop | Piece::Rook | Piece::Queen) {
                attackers = self.get_attackers_to_square_see(to_sq, occupancy, tables);
                attackers &= !(1u64 << attacker_sq);
            }

            if d >= 31 {
                break;
            }

            gain[d] = piece_value(next_victim) - gain[d - 1];
            next_victim = self
                .piece_at(crate::square::Square::from_index(attacker_sq))
                .unwrap()
                .1;
            side_to_move = side_to_move.opposite();
        }

        // 4. Back-propagate
        while d > 1 {
            d -= 1;
            gain[d - 1] = -std::cmp::max(-gain[d - 1], gain[d]);
        }
        gain[0] >= threshold
    }

    fn get_lva_square(&self, attackers: u64, side: Color, _occ: u64) -> u8 {
        let side_attackers = attackers & self.occupancy(side);
        if side_attackers == 0 {
            return 64;
        }

        for piece in [
            Piece::Pawn,
            Piece::Knight,
            Piece::Bishop,
            Piece::Rook,
            Piece::Queen,
            Piece::King,
        ] {
            let subset = side_attackers & self.pieces(piece, side);
            if subset != 0 {
                return subset.lsb();
            }
        }
        64
    }
}
