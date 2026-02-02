pub trait BitboardExt {
    fn lsb(self) -> u8;
}

impl BitboardExt for u64 {
    #[inline(always)]
    fn lsb(self) -> u8 {
        debug_assert!(self != 0, "Called lsb() on empty bitboard");
        self.trailing_zeros() as u8
    }
}
