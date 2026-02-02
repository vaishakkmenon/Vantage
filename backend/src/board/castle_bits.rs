// src/board/castle_bits.rs

/// Underlying bit type for castling rights.
/// Use the same width you already use across the engine.
pub type CastleBits = u8;

// IMPORTANT: keep your original bit positions/values.
// Replace these literals with your current constants if they differ.
pub const CASTLE_WK: CastleBits = 0b0001;
pub const CASTLE_WQ: CastleBits = 0b0010;
pub const CASTLE_BK: CastleBits = 0b0100;
pub const CASTLE_BQ: CastleBits = 0b1000;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn castle_bits_single_and_disjoint() {
        assert_eq!(CASTLE_WK.count_ones(), 1);
        assert_eq!(CASTLE_WQ.count_ones(), 1);
        assert_eq!(CASTLE_BK.count_ones(), 1);
        assert_eq!(CASTLE_BQ.count_ones(), 1);

        let all = CASTLE_WK | CASTLE_WQ | CASTLE_BK | CASTLE_BQ;
        assert_eq!(all.count_ones(), 4);
    }
}
