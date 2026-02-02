use crate::utils::square_index;

#[inline]
/// Scanning along an axis for available positions. Created a function to replace repeated logic.
fn scan_ray<F>(mut rank: isize, mut file: isize, step: F, mut on_square: impl FnMut(usize) -> bool)
where
    F: Fn(isize, isize) -> (isize, isize),
{
    while (0..=7).contains(&rank) && (0..=7).contains(&file) {
        let sq = square_index(rank as usize, file as usize);
        if !on_square(sq) {
            break;
        }
        let (new_rank, new_file) = step(rank, file);
        rank = new_rank;
        file = new_file;
    }
}

#[inline]
pub fn rook_attacks_per_square(square: usize, blockers: u64) -> u64 {
    let rank = square / 8;
    let file = square % 8;
    let mut attacks = 0u64;

    // Function to add possible attacks based blockers and current square
    // Created as a closure to keep the function local
    let mut add = |sq: usize| {
        attacks |= 1 << sq;
        (blockers >> sq) & 1 == 0 // stop if blocker found
    };

    scan_ray(
        rank as isize + 1,
        file as isize,
        |r, f| (r + 1, f),
        &mut add,
    ); // north

    scan_ray(
        rank as isize - 1,
        file as isize,
        |r, f| (r - 1, f),
        &mut add,
    ); // south

    scan_ray(
        rank as isize,
        file as isize + 1,
        |r, f| (r, f + 1),
        &mut add,
    ); // east

    scan_ray(
        rank as isize,
        file as isize - 1,
        |r, f| (r, f - 1),
        &mut add,
    ); // west

    attacks
}

#[inline]
pub fn bishop_attacks_per_square(square: usize, blockers: u64) -> u64 {
    let rank = square / 8;
    let file = square % 8;
    let mut attacks = 0u64;

    let mut add = |sq: usize| {
        attacks |= 1 << sq;
        (blockers >> sq) & 1 == 0
    };

    scan_ray(
        rank as isize + 1,
        file as isize + 1,
        |r, f| (r + 1, f + 1),
        &mut add,
    ); // NE

    scan_ray(
        rank as isize - 1,
        file as isize - 1,
        |r, f| (r - 1, f - 1),
        &mut add,
    ); // SW

    scan_ray(
        rank as isize + 1,
        file as isize - 1,
        |r, f| (r + 1, f - 1),
        &mut add,
    ); // NW

    scan_ray(
        rank as isize - 1,
        file as isize + 1,
        |r, f| (r - 1, f + 1),
        &mut add,
    ); // SE

    attacks
}

pub fn get_rook_attack_bitboards(square: usize, blockers: &[u64]) -> Vec<u64> {
    blockers
        .iter()
        .map(|&b| rook_attacks_per_square(square, b))
        .collect()
}

pub fn get_bishop_attack_bitboards(square: usize, blockers: &[u64]) -> Vec<u64> {
    blockers
        .iter()
        .map(|&b| bishop_attacks_per_square(square, b))
        .collect()
}

#[rustfmt::skip]
const KNIGHT_ATTACKS: [u64; 64] = [
    0x20400, 0x50800, 0xa1100, 0x142200, 0x284400, 0x508800, 0xa01000, 0x402000,
    0x2040004, 0x5080008, 0xa110011, 0x14220022, 0x28440044, 0x50880088, 0xa0100010, 0x40200020,
    0x204000402, 0x508000805, 0xa1100110a, 0x1422002214, 0x2844004428, 0x5088008850, 0xa0100010a0, 0x4020002040,
    0x20400040200, 0x50800080500, 0xa1100110a00, 0x142200221400, 0x284400442800, 0x508800885000, 0xa0100010a000, 0x402000204000,
    0x2040004020000, 0x5080008050000, 0xa1100110a0000, 0x14220022140000, 0x28440044280000, 0x50880088500000, 0xa0100010a00000, 0x40200020400000,
    0x204000402000000, 0x508000805000000, 0xa1100110a000000, 0x1422002214000000, 0x2844004428000000, 0x5088008850000000, 0xa0100010a0000000, 0x4020002040000000,
    0x400040200000000, 0x800805000000000, 0x1100110a00000000, 0x2200221400000000, 0x4400442800000000, 0x8800885000000000, 0x100010a000000000, 0x2000204000000000,
    0x4020000000000, 0x8050000000000, 0x110a0000000000, 0x22140000000000, 0x44280000000000, 0x88500000000000, 0x10a00000000000, 0x20400000000000,
];

#[rustfmt::skip]
const KING_ATTACKS: [u64; 64] = [
    0x30203, 0x70507, 0xe0a0e, 0x1c141c, 0x382838, 0x705070, 0xe0a0e0, 0xc040c0,
    0x3020300, 0x7050700, 0xe0a0e00, 0x1c141c00, 0x38283800, 0x70507000, 0xe0a0e000, 0xc040c000,
    0x302030000, 0x705070000, 0xe0a0e0000, 0x1c141c0000, 0x3828380000, 0x7050700000, 0xe0a0e00000, 0xc040c00000,
    0x30203000000, 0x70507000000, 0xe0a0e000000, 0x1c141c000000, 0x382838000000, 0x705070000000, 0xe0a0e0000000, 0xc040c0000000,
    0x3020300000000, 0x7050700000000, 0xe0a0e00000000, 0x1c141c00000000, 0x38283800000000, 0x70507000000000, 0xe0a0e000000000, 0xc040c000000000,
    0x302030000000000, 0x705070000000000, 0xe0a0e0000000000, 0x1c141c0000000000, 0x3828380000000000, 0x7050700000000000, 0xe0a0e00000000000, 0xc040c00000000000,
    0x302030000000000, 0x705070000000000, 0xe0a0e0000000000, 0x1c141c0000000000, 0x3828380000000000, 0x7050700000000000, 0xe0a0e00000000000, 0xc040c00000000000,
    0x203000000000000, 0x507000000000000, 0xa0e000000000000, 0x141c000000000000, 0x2838000000000000, 0x5070000000000000, 0xa0e0000000000000, 0x40c0000000000000
];

#[inline(always)]
pub fn get_knight_attacks(square: usize) -> u64 {
    KNIGHT_ATTACKS[square]
}

#[inline(always)]
pub fn get_king_attacks(square: usize) -> u64 {
    KING_ATTACKS[square]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bit(sq: usize) -> u64 {
        1u64 << sq
    }

    #[test]
    fn rook_attacks_from_d4_no_blockers() {
        let square = 27; // d4
        let blockers = 0;
        let result = rook_attacks_per_square(square, blockers);

        let expected = bit(3)   // d1
            | bit(11)           // d2
            | bit(19)           // d3
            | bit(35)           // d5
            | bit(43)           // d6
            | bit(51)           // d7
            | bit(59)           // d8
            | bit(24)           // a4
            | bit(25)           // b4
            | bit(26)           // c4
            | bit(28)           // e4
            | bit(29)           // f4
            | bit(30)           // g4
            | bit(31); // h4

        assert_eq!(result, expected);
    }

    #[test]
    fn bishop_attacks_from_d4_no_blockers() {
        let square = 27; // d4
        let blockers = 0;
        let result = bishop_attacks_per_square(square, blockers);

        let expected = bit(36) // e5
            | bit(45)          // f6
            | bit(54)          // g7
            | bit(63)          // h8 
            | bit(34)          // c5  NW
            | bit(41)          // b6  NW
            | bit(48)          // a7  NW
            | bit(20)          // e3
            | bit(13)          // f2
            | bit(6)           // g1
            | bit(18)          // c3
            | bit(9)           // b2
            | bit(0); // a1

        assert_eq!(result, expected);
    }

    #[test]
    fn rook_attacks_blocked_east() {
        let square = 27; // d4
        let blockers = bit(28); // e4 blocks east
        let result = rook_attacks_per_square(square, blockers);

        let expected = bit(3)   // d1
            | bit(11)           // d2
            | bit(19)           // d3
            | bit(35)           // d5
            | bit(43)           // d6
            | bit(51)           // d7
            | bit(59)           // d8
            | bit(24)           // a4
            | bit(25)           // b4
            | bit(26)           // c4
            | bit(28); // e4 (included)

        assert_eq!(result, expected);
    }

    #[test]
    fn bishop_attacks_blocked_ne() {
        let square = 27; // d4
        let blockers = bit(36); // e5 blocks NE
        let result = bishop_attacks_per_square(square, blockers);

        let expected = bit(36) // e5 (included)
            | bit(20)          // e3
            | bit(13)          // f2
            | bit(6)           // g1
            | bit(18)          // c3
            | bit(9)           // b2
            | bit(0)           // a1
            | bit(34)          // c5 
            | bit(41)          // b6 
            | bit(48); // a7 

        assert_eq!(result, expected);
    }
}
