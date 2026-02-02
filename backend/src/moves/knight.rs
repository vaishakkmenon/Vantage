pub const KNIGHT_ATTACKS: [u64; 64] = generate_knight_table();

/// Knight attack generation using const for compile time generation instead of hardcoding
#[inline(always)]
const fn knight_attacks(square: u8) -> u64 {
    let rank = square / 8;
    let file = square % 8;

    let mut attacks = 0u64;
    let mut i = 0;
    while i < 8 {
        let dr = [2, 2, -2, -2, 1, 1, -1, -1][i];
        let df = [1, -1, 1, -1, 2, -2, 2, -2][i];

        let r = rank as i8 + dr;
        let f = file as i8 + df;

        if r >= 0 && r < 8 && f >= 0 && f < 8 {
            attacks |= 1u64 << (r * 8 + f);
        }

        i += 1;
    }

    attacks
}

const fn generate_knight_table() -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut i = 0;

    while i < 64 {
        table[i] = knight_attacks(i as u8);
        i += 1;
    }

    table
}

/// Returns the knight attack bitboard for a given square and color, or None if the square is invalid.
pub fn knight_attacks_checked(square: u8) -> Option<u64> {
    if square >= 64 {
        return None;
    }
    Some(KNIGHT_ATTACKS[square as usize])
}

#[cfg(test)]
mod tests {
    use super::{KNIGHT_ATTACKS, knight_attacks};

    #[test]
    fn dump_knight_attacks() {
        for sq in 0..64 {
            println!("0x{:016X}, // {}", knight_attacks(sq), sq);
        }
    }

    fn assert_attacks(square: u8, expected: u64) {
        let generated = KNIGHT_ATTACKS[square as usize];
        assert_eq!(
            generated, expected,
            "Mismatch on square {}: expected {:016X}, got {:016X}",
            square, expected, generated
        );
    }

    #[test]
    fn test_knight_attacks_center() {
        let d4 = 3 + 8 * 3; // d4 = index 27
        let expected = 0x0000142200221400;
        assert_attacks(d4, expected);
    }

    #[test]
    fn test_knight_attacks_corner_a1() {
        let a1 = 0;
        let expected = 0x0000000000020400;
        assert_attacks(a1, expected);
    }

    #[test]
    fn test_knight_attacks_corner_h1() {
        let h1 = 7;
        let expected = 0x0000000000402000;
        assert_attacks(h1, expected);
    }

    #[test]
    fn test_knight_attacks_corner_a8() {
        let a8 = 56;
        let expected = 0x0004020000000000;
        assert_attacks(a8, expected);
    }

    #[test]
    fn test_knight_attacks_corner_h8() {
        let h8 = 63;
        let expected = 0x0020400000000000;
        assert_attacks(h8, expected);
    }

    #[test]
    fn test_knight_attacks_matches_reference_function() {
        for square in 0u8..64 {
            let expected = knight_attacks(square);
            let actual = KNIGHT_ATTACKS[square as usize];
            assert_eq!(
                expected, actual,
                "Mismatch at square {}: expected {:016X}, got {:016X}",
                square, expected, actual
            );
        }
    }

    #[test]
    fn test_knight_attacks_checked_bounds() {
        use super::knight_attacks_checked;

        assert_eq!(knight_attacks_checked(0), Some(KNIGHT_ATTACKS[0]));
        assert_eq!(knight_attacks_checked(63), Some(KNIGHT_ATTACKS[63]));
        assert_eq!(knight_attacks_checked(64), None);
        assert_eq!(knight_attacks_checked(u8::MAX), None);
    }
}
