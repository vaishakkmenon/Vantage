use crate::utils::enumerate_subsets;
use crate::utils::square_index;

/// Returns the rook blocker mask for a square, excluding edge squares.
///
/// Used for magic bitboards. Only includes squares along the same rank and file
/// that could contain blockers between the piece and the board edge.
pub fn rook_vision_mask(square: usize) -> u64 {
    let rank = square / 8;
    let file = square % 8;
    let mut mask = 0u64;

    for r in (rank + 1)..7 {
        let sq = square_index(r, file);
        mask |= 1u64 << sq;
    }

    for r in (1..rank).rev() {
        let sq = square_index(r, file);
        mask |= 1u64 << sq;
    }

    for f in (file + 1)..7 {
        let sq = square_index(rank, f);
        mask |= 1u64 << sq;
    }

    for f in (1..file).rev() {
        let sq = square_index(rank, f);
        mask |= 1u64 << sq;
    }

    mask
}

/// Returns the bishop blocker mask for a square, excluding edge squares.
///
/// Diagonal squares are included up to (but not including) the edge. This avoids
/// redundant blocker configs in magic bitboard generation.
pub fn bishop_vision_mask(square: usize) -> u64 {
    let rank = square / 8;
    let file = square % 8;
    let mut mask = 0u64;

    // NE
    let mut r = rank + 1;
    let mut f = file + 1;
    while r <= 6 && f <= 6 {
        let sq = square_index(r, f);
        mask |= 1u64 << sq;
        r += 1;
        f += 1;
    }

    // SW
    if let Some(mut r) = rank.checked_sub(1)
        && let Some(mut f) = file.checked_sub(1)
    {
        while r >= 1 && f >= 1 {
            let sq = square_index(r, f);
            mask |= 1u64 << sq;
            r -= 1;
            f -= 1;
        }
    }

    // NW
    let mut r = rank + 1;
    if let Some(mut f) = file.checked_sub(1) {
        while r <= 6 && f >= 1 {
            let sq = square_index(r, f);
            mask |= 1u64 << sq;
            r += 1;
            f -= 1;
        }
    }

    // SE
    if let Some(mut r) = rank.checked_sub(1) {
        let mut f = file + 1;
        while r >= 1 && f <= 6 {
            let sq = square_index(r, f);
            mask |= 1u64 << sq;
            r -= 1;
            f += 1;
        }
    }

    mask
}

pub fn generate_rook_blockers(square: usize) -> Vec<u64> {
    let mask = rook_vision_mask(square);
    let mut configs = Vec::new();

    enumerate_subsets(mask, |subset| {
        configs.push(subset);
    });

    configs
}

pub fn generate_bishop_blockers(square: usize) -> Vec<u64> {
    let mask = bishop_vision_mask(square);
    let mut configs = Vec::new();

    enumerate_subsets(mask, |subset| {
        configs.push(subset);
    });

    configs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::bitboard_to_string;

    #[test]
    fn test_rook_vision_d4() {
        let idx = 3 * 8 + 3; // d4
        let mask = rook_vision_mask(idx);
        println!("Rook vision mask for d4:\n{}", bitboard_to_string(mask));
        assert_eq!(mask.count_ones(), 10);
    }

    #[test]
    fn test_bishop_vision_d4() {
        let idx = 3 * 8 + 3; // d4
        let mask = bishop_vision_mask(idx);
        println!("Bishop vision mask for d4:\n{}", bitboard_to_string(mask));
        assert_eq!(mask.count_ones(), 9);
    }

    #[test]
    fn test_rook_vision_a1() {
        let idx = 0; // a1
        let mask = rook_vision_mask(idx);
        println!("Rook vision mask for a1:\n{}", bitboard_to_string(mask));
        assert_eq!(mask.count_ones(), 12);
    }

    #[test]
    fn test_bishop_vision_h8() {
        let idx = 63; // h8
        let mask = bishop_vision_mask(idx);
        println!("Bishop vision mask for h8:\n{}", bitboard_to_string(mask));
        assert_eq!(mask.count_ones(), 6);
    }
}
