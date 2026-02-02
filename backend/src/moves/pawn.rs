use crate::board::Color;

pub const WHITE_PAWN_ATTACKS: [u64; 64] = generate_pawn_attack_table(Color::White);
pub const BLACK_PAWN_ATTACKS: [u64; 64] = generate_pawn_attack_table(Color::Black);

const fn single_pawn_attack_mask(square: u8, color: Color) -> u64 {
    let rank = (square / 8) as i8;
    let file = (square % 8) as i8;
    let mut attacks = 0u64;

    let deltas: [(i8, i8); 2] = match color {
        Color::White => [(1, -1), (1, 1)],
        Color::Black => [(-1, -1), (-1, 1)],
    };

    let mut i = 0;
    while i < 2 {
        let dr = deltas[i].0;
        let df = deltas[i].1;

        let r = rank + dr;
        let f = file + df;
        if r >= 0 && r < 8 && f >= 0 && f < 8 {
            let dest = (r * 8 + f) as u64;
            attacks |= 1u64 << dest;
        }

        i += 1;
    }

    attacks
}

const fn generate_pawn_attack_table(color: Color) -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut square = 0;
    while square < 64 {
        table[square] = single_pawn_attack_mask(square as u8, color);
        square += 1;
    }
    table
}

/// Returns the pawn attack bitboard for a given square and color, or None if the square is invalid.
pub fn pawn_attacks_checked(square: u8, color: Color) -> Option<u64> {
    if square >= 64 {
        return None;
    }
    Some(pawn_attacks(square, color))
}

#[inline(always)]
pub fn pawn_attacks(square: u8, color: Color) -> u64 {
    match color {
        Color::White => WHITE_PAWN_ATTACKS[square as usize],
        Color::Black => BLACK_PAWN_ATTACKS[square as usize],
    }
}

#[cfg(test)]
fn pawn_attack_mask(square: u8, color: Color) -> u64 {
    let rank = square / 8;
    let file = square % 8;
    let mut attacks = 0u64;

    let deltas: [(i8, i8); 2] = match color {
        Color::White => [(1, -1), (1, 1)],
        Color::Black => [(-1, -1), (-1, 1)],
    };

    for (dr, df) in deltas {
        let r = rank as i8 + dr;
        let f = file as i8 + df;
        if (0..8).contains(&r) && (0..8).contains(&f) {
            let dest = r * 8 + f;
            attacks |= 1u64 << dest;
        }
    }

    attacks
}

#[cfg(test)]
mod tests {

    use super::{BLACK_PAWN_ATTACKS, WHITE_PAWN_ATTACKS, pawn_attack_mask};
    use crate::board::Color;

    #[test]
    fn dump_white_pawn_attacks() {
        for sq in 0..64 {
            println!(
                "0x{:016X}, // {}",
                pawn_attack_mask(sq as u8, Color::White),
                sq
            );
        }
    }

    #[test]
    fn dump_black_pawn_attacks() {
        for sq in 0..64 {
            println!(
                "0x{:016X}, // {}",
                pawn_attack_mask(sq as u8, Color::Black),
                sq
            );
        }
    }

    #[test]
    fn test_checked_matches_const() {
        use crate::board::Color;

        for square in 0..64u8 {
            let w = super::pawn_attacks_checked(square, Color::White);
            let b = super::pawn_attacks_checked(square, Color::Black);

            assert_eq!(w, Some(super::WHITE_PAWN_ATTACKS[square as usize]));
            assert_eq!(b, Some(super::BLACK_PAWN_ATTACKS[square as usize]));
        }
    }

    #[test]
    fn test_white_pawn_attack_mask_d4() {
        let d4 = 3 + 8 * 3; // index 27
        let expected = (1u64 << (4 * 8 + 2)) | (1u64 << (4 * 8 + 4)); // c5 + e5
        assert_eq!(pawn_attack_mask(d4, Color::White), expected);
    }

    #[test]
    fn test_white_attacks_match_reference() {
        for (square, &attack) in WHITE_PAWN_ATTACKS.iter().enumerate() {
            assert_eq!(attack, pawn_attack_mask(square as u8, Color::White));
        }
    }

    #[test]
    fn test_black_attacks_match_reference() {
        for (square, &attack) in BLACK_PAWN_ATTACKS.iter().enumerate() {
            assert_eq!(attack, pawn_attack_mask(square as u8, Color::Black));
        }
    }

    #[test]
    fn test_pawn_attacks_checked() {
        use crate::board::Color;

        assert_eq!(
            super::pawn_attacks_checked(27, Color::White),
            Some(super::WHITE_PAWN_ATTACKS[27])
        );
        assert_eq!(
            super::pawn_attacks_checked(64, Color::Black),
            None,
            "Square 64 should return None"
        );
        assert_eq!(
            super::pawn_attacks_checked(255, Color::White),
            None,
            "Out-of-bounds square should return None"
        );
    }
}
