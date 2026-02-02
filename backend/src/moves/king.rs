/// Precomputed king attack bitboards for each square.
pub const KING_ATTACKS: [u64; 64] = generate_king_table();

/// Knight attack generation using const for compile time generation instead of hardcoding
#[inline(always)]
const fn king_attacks(square: u8) -> u64 {
    let rank = square / 8;
    let file = square % 8;
    let mut attacks = 0u64;

    let mut dr = -1;
    while dr <= 1 {
        let mut df = -1;
        while df <= 1 {
            if dr != 0 || df != 0 {
                let r = rank as i8 + dr;
                let f = file as i8 + df;
                if r >= 0 && r < 8 && f >= 0 && f < 8 {
                    let dest = (r * 8 + f) as u8;
                    attacks |= 1u64 << dest;
                }
            }
            df += 1;
        }
        dr += 1;
    }

    attacks
}

const fn generate_king_table() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut i = 0;

    while i < 64 {
        table[i] = king_attacks(i as u8);
        i += 1;
    }

    table
}

/// Returns the king attack bitboard for a given square and color, or None if the square is invalid.
pub fn king_attacks_checked(square: u8) -> Option<u64> {
    if square >= 64 {
        return None;
    }
    Some(KING_ATTACKS[square as usize])
}

#[cfg(test)]
mod tests {
    // use super::{KING_ATTACKS, king_attacks};
    use super::{KING_ATTACKS, king_attacks, king_attacks_checked};

    #[test]
    fn dump_king_attacks() {
        for sq in 0..64 {
            println!("0x{:016X}, // {}", king_attacks(sq), sq);
        }
    }

    fn assert_attacks(square: u8, expected: u64) {
        let actual = KING_ATTACKS[square as usize];
        assert_eq!(
            actual, expected,
            "Mismatch at square {}: expected {:016X}, got {:016X}",
            square, expected, actual
        );
    }

    #[test]
    fn test_king_attacks_center() {
        let d4 = 3 + 8 * 3; // index 27
        let expected = 0x0000001C141C0000;
        assert_attacks(d4, expected);
    }

    #[test]
    fn test_king_attacks_corner_a1() {
        let a1 = 0;
        let expected = king_attacks(a1);
        assert_attacks(a1, expected);
    }

    #[test]
    fn test_king_attacks_corner_h1() {
        let h1 = 7;
        let expected = king_attacks(h1);
        assert_attacks(h1, expected);
    }

    #[test]
    fn test_king_attacks_corner_a8() {
        let a8 = 56;
        let expected = king_attacks(a8);
        assert_attacks(a8, expected);
    }

    #[test]
    fn test_king_attacks_corner_h8() {
        let h8 = 63;
        let expected = king_attacks(h8);
        assert_attacks(h8, expected);
    }

    #[test]
    fn test_king_attacks_matches_reference_function() {
        for square in 0u8..64 {
            let expected = king_attacks(square);
            let actual = KING_ATTACKS[square as usize];
            assert_eq!(
                expected, actual,
                "Mismatch at square {}: expected {:016X}, got {:016X}",
                square, expected, actual
            );
        }
    }

    #[test]
    fn test_king_attacks_checked_valid() {
        for sq in 0u8..64 {
            assert_eq!(
                king_attacks_checked(sq),
                Some(KING_ATTACKS[sq as usize]),
                "Expected valid king attack for square {}",
                sq
            );
        }
    }

    #[test]
    fn test_king_attacks_checked_invalid() {
        assert_eq!(king_attacks_checked(64), None);
        assert_eq!(king_attacks_checked(100), None);
        assert_eq!(king_attacks_checked(u8::MAX), None);
    }
}
