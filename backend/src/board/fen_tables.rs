// src/board/fen_tables.rs
// O(1) FEN glyph ↔ (piece,color) maps.

use super::{Color, Piece};

pub(super) const CHAR_TO_PC: [Option<(Piece, Color)>; 128] = {
    let mut table: [Option<(Piece, Color)>; 128] = [None; 128];

    // Uppercase = White
    table['P' as usize] = Some((Piece::Pawn, Color::White));
    table['N' as usize] = Some((Piece::Knight, Color::White));
    table['B' as usize] = Some((Piece::Bishop, Color::White));
    table['R' as usize] = Some((Piece::Rook, Color::White));
    table['Q' as usize] = Some((Piece::Queen, Color::White));
    table['K' as usize] = Some((Piece::King, Color::White));

    // Lowercase = Black
    table['p' as usize] = Some((Piece::Pawn, Color::Black));
    table['n' as usize] = Some((Piece::Knight, Color::Black));
    table['b' as usize] = Some((Piece::Bishop, Color::Black));
    table['r' as usize] = Some((Piece::Rook, Color::Black));
    table['q' as usize] = Some((Piece::Queen, Color::Black));
    table['k' as usize] = Some((Piece::King, Color::Black));

    table
};

#[inline]
const fn pc_index(piece: Piece, color: Color) -> usize {
    (color as usize) * 6 + (piece as usize)
}

pub(super) const PC_TO_CHAR: [char; 12] = {
    let mut t = ['\0'; 12];
    // White
    t[pc_index(Piece::Pawn, Color::White)] = 'P';
    t[pc_index(Piece::Knight, Color::White)] = 'N';
    t[pc_index(Piece::Bishop, Color::White)] = 'B';
    t[pc_index(Piece::Rook, Color::White)] = 'R';
    t[pc_index(Piece::Queen, Color::White)] = 'Q';
    t[pc_index(Piece::King, Color::White)] = 'K';
    // Black
    t[pc_index(Piece::Pawn, Color::Black)] = 'p';
    t[pc_index(Piece::Knight, Color::Black)] = 'n';
    t[pc_index(Piece::Bishop, Color::Black)] = 'b';
    t[pc_index(Piece::Rook, Color::Black)] = 'r';
    t[pc_index(Piece::Queen, Color::Black)] = 'q';
    t[pc_index(Piece::King, Color::Black)] = 'k';
    t
};

#[cfg(any(test, debug_assertions))]
mod debug_guards {
    use super::*;
    pub fn _assert_tables() {
        debug_assert_eq!(Color::White as u8, 0);
        debug_assert_eq!(Color::Black as u8, 1);

        // Round-trip the 12 canonical glyphs.
        const GLYPHS: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
        for &g in &GLYPHS {
            let (piece, color) = CHAR_TO_PC[g as usize].expect("glyph missing from CHAR_TO_PC");
            let idx = pc_index(piece, color);
            debug_assert_eq!(PC_TO_CHAR[idx], g, "pc→char mismatch for {}", g);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn _tables_guard() {
        super::debug_guards::_assert_tables();
    }
}
