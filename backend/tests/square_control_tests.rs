use std::str::FromStr;
use vantage::board::{Board, Color};

use vantage::moves::magic::MagicTables;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::square_control::{in_check, is_square_attacked};
use vantage::square::Square;

fn tables() -> MagicTables {
    load_magic_tables()
}

#[test]
fn in_check_detects_simple_rook_check() {
    // Black rook on e8 gives check to white king on e1. Black king exists on h8.
    let fen = "4r2k/8/8/8/8/8/8/4K3 w - - 0 1";
    let b = Board::from_str(fen).unwrap();
    let t = tables();
    assert!(in_check(&b, Color::White, &t)); // e8 rook checks e1 king
    assert!(!in_check(&b, Color::Black, &t)); // black king on h8 is safe
}

#[test]
fn is_square_attacked_handles_pawn_direction() {
    // White pawn on b5; a6 and c6 are attacked, a4 and c4 are not.
    let fen = "8/8/8/1P6/8/8/8/4k3 w - - 0 1";
    let b = Board::from_str(fen).unwrap();
    let t = tables();

    assert!(is_square_attacked(
        &b,
        Square::from_str("a6").unwrap(),
        Color::White,
        &t
    ));
    assert!(is_square_attacked(
        &b,
        Square::from_str("c6").unwrap(),
        Color::White,
        &t
    ));
    assert!(!is_square_attacked(
        &b,
        Square::from_str("a4").unwrap(),
        Color::White,
        &t
    ));
    assert!(!is_square_attacked(
        &b,
        Square::from_str("c4").unwrap(),
        Color::White,
        &t
    ));
}

#[test]
fn pawn_wraparound_edges_are_masked() {
    // White pawn on a5 should not "wrap" to h6/h4.
    let fen = "8/8/8/P7/8/8/8/4k3 w - - 0 1";
    let b = Board::from_str(fen).unwrap();
    let t = tables();

    assert!(is_square_attacked(
        &b,
        Square::from_str("b6").unwrap(),
        Color::White,
        &t
    )); // valid
    assert!(!is_square_attacked(
        &b,
        Square::from_str("h6").unwrap(),
        Color::White,
        &t
    )); // must be false
    assert!(!is_square_attacked(
        &b,
        Square::from_str("h4").unwrap(),
        Color::White,
        &t
    )); // must be false
}

#[test]
fn castling_attack_check_blocks_through_and_to() {
    let fen = "r3k2r/8/8/8/1b6/8/8/R3K2R w KQkq - 0 1";
    let b = Board::from_str(fen).unwrap();
    let t = tables();

    assert!(in_check(&b, Color::White, &t));

    // If you expose `is_legal_castling`, these should both be false:
    // let king_side = Move::castle_white_kingside(); // however you construct this in tests
    // let queen_side = Move::castle_white_queenside();
    // assert!(!is_legal_castling(&b, king_side, &t));
    // assert!(!is_legal_castling(&b, queen_side, &t));
}
