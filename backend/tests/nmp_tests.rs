use vantage::board::{Board, Color};
use std::str::FromStr;

#[test]
fn test_has_major_pieces_logic() {
    // 1. Startpos: White has pieces -> True
    let b_start = Board::new();
    assert!(
        b_start.has_major_pieces(Color::White),
        "Startpos White should have major pieces"
    );
    assert!(
        b_start.has_major_pieces(Color::Black),
        "Startpos Black should have major pieces"
    );

    // 2. Empty board + Kings + Pawns -> False
    // FEN: 4k3/pp6/8/8/8/8/6PP/4K3 w - - 0 1
    let b_pawns = Board::from_str("4k3/pp6/8/8/8/8/6PP/4K3 w - - 0 1").unwrap();
    assert!(
        !b_pawns.has_major_pieces(Color::White),
        "Pawn endgame should NOT have major pieces"
    );
    assert!(
        !b_pawns.has_major_pieces(Color::Black),
        "Pawn endgame should NOT have major pieces"
    );

    // 3. King + Pawn + Knight -> True
    // FEN: 4k3/pp6/8/8/8/8/6PP/4K1N1 w - - 0 1
    let b_knight = Board::from_str("4k3/pp6/8/8/8/8/6PP/4K1N1 w - - 0 1").unwrap();
    assert!(
        b_knight.has_major_pieces(Color::White),
        "Knight is a major piece"
    );

    // 4. King + Rook -> True
    let b_rook = Board::from_str("4k3/8/8/8/8/8/8/R3K3 w - - 0 1").unwrap();
    assert!(
        b_rook.has_major_pieces(Color::White),
        "Rook is a major piece"
    );
}
