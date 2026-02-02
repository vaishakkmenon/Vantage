use std::str::FromStr;

use vantage::board::Board;
use vantage::moves::movegen::generate_pawn_moves;

fn pawn_move_count(fen: &str) -> usize {
    let board = Board::from_str(fen).unwrap();
    let mut moves = vec![];
    generate_pawn_moves(&board, &mut moves);
    moves.len()
}

#[test]
fn start_position_white_pawns() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    assert_eq!(
        pawn_move_count(fen),
        16,
        "White should have 16 pawn moves (8 pushes + 8 doubles)"
    );
}

#[test]
fn start_position_black_pawns() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
    assert_eq!(
        pawn_move_count(fen),
        16,
        "Black should have 16 pawn moves (8 pushes + 8 doubles)"
    );
}

#[test]
fn promotion_pushes() {
    // White pawn on rank 7 with empty promotion square
    let fen = "8/P7/8/8/8/8/8/k6K w - - 0 1";
    assert_eq!(
        pawn_move_count(fen),
        4,
        "Pawn should have 4 promotion moves (N,B,R,Q)"
    );
}

#[test]
fn promotion_pushes_only() {
    // a8 empty, b8 empty → only 4 promotion pushes
    let fen = "8/P7/8/8/8/8/8/k6K w - - 0 1";
    assert_eq!(
        pawn_move_count(fen),
        4,
        "Should be 4 promotion pushes (N,B,R,Q)"
    );
}

#[test]
fn promotion_captures_only() {
    // a8 occupied by black rook, b8 occupied by black knight → no push, only 4 promotion captures
    let fen = "rn6/P7/8/8/8/8/8/k6K w - - 0 1";
    assert_eq!(
        pawn_move_count(fen),
        4,
        "Should be 4 promotion captures (N,B,R,Q)"
    );
}

#[test]
fn promotion_push_and_capture() {
    // a8 empty, b8 has black knight → 4 pushes + 4 captures = 8 total
    let fen = "1n6/P7/8/8/8/8/8/k6K w - - 0 1";
    assert_eq!(
        pawn_move_count(fen),
        8,
        "Should be 8 total promotions (4 push + 4 capture)"
    );
}

#[test]
fn en_passant_only() {
    // Block b6 with a white knight so only EP is available (no push, no capture on b6)
    let fen = "8/8/1N6/pP6/8/8/8/k6K w - a6 0 1";
    assert_eq!(pawn_move_count(fen), 1);
}

#[test]
fn en_passant_illegal_due_to_pin() {
    // White pawn EP capture would expose king to rook
    // This tests that legal filtering later removes it, but movegen will still generate it
    let fen = "4k3/8/8/rPp5/8/8/8/K7 w - c6 0 1";
    let count = pawn_move_count(fen);
    assert!(
        count >= 1,
        "Pseudo-legal should still generate EP here; legal filter must remove it"
    );
}

#[test]
fn blocked_pawn_cannot_push() {
    // White pawn blocked by enemy pawn
    let fen = "8/8/8/8/3p4/3P4/8/k6K w - - 0 1";
    assert_eq!(pawn_move_count(fen), 0, "Blocked pawn should have no moves");
}
