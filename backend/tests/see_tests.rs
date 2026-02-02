use vantage::board::Board;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::Move;
use vantage::search::see::SeeExt;
use vantage::square::Square;
use std::str::FromStr;

use vantage::moves::magic::MagicTables;

// Helper now returns (Move, MagicTables) so we can reuse tables in test assertions
fn find_move(board: &mut Board, from_str: &str, to_str: &str) -> (Move, MagicTables) {
    let mut moves = Vec::new();
    let mut scratch = Vec::new();

    let tables = load_magic_tables();
    vantage::moves::execute::generate_legal(board, &tables, &mut moves, &mut scratch);

    let from = Square::from_str(from_str).unwrap();
    let to = Square::from_str(to_str).unwrap();

    let m = *moves
        .iter()
        .find(|m| m.from == from && m.to == to)
        .expect(&format!(
            "Move {}{} not found or illegal in pos: {}",
            from_str,
            to_str,
            board.to_fen()
        ));

    (m, tables)
}

#[test]
fn test_see_exchange_losing() {
    // White Rook takes protected Pawn. (100 - 500 = -400)
    // FIX: Replaced deep King with a Black Rook on d8 which actually defends d4
    let fen = "3r4/8/8/8/3p4/8/8/3RK3 w - - 0 1";
    let mut board = Board::from_str(fen).expect("Invalid FEN");

    let (m, tables) = find_move(&mut board, "d1", "d4");

    // R(d1)xP(d4) [+100] -> R(d8)xR(d4) [-500]. Net -400.
    assert_eq!(
        board.static_exchange_eval(m, 0, &tables),
        false,
        "RxP (protected by Rook) should be bad"
    );
}

#[test]
fn test_see_exchange_winning_battery() {
    // White Battery (Q+R) attacks a8.
    // 1. QxR (+500), 2. KxQ (-900), 3. RxQ (+900). Net: +500.
    let fen = "r6k/8/8/8/8/8/Q7/R6K w - - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    let (m, tables) = find_move(&mut board, "a2", "a8");

    assert_eq!(
        board.static_exchange_eval(m, 0, &tables),
        true,
        "Battery capture should be good"
    );
}

#[test]
fn test_see_pruning_threshold() {
    // Knight takes protected Pawn (+100 - 320 = -220).
    // FIX: Moved Black King to e5 so it protects d4
    let fen = "8/8/8/4k3/3p4/8/4N3/3K4 w - - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    let (m, tables) = find_move(&mut board, "e2", "d4");

    // N(e2)xP(d4) [+100] -> K(e5)xN(d4) [-320]. Net -220.
    assert_eq!(
        board.static_exchange_eval(m, 0, &tables),
        false,
        "NxP should be bad at threshold 0"
    );

    // If we accept losing material (threshold -300), it should be true (-220 > -300)
    assert_eq!(
        board.static_exchange_eval(m, -300, &tables),
        true,
        "NxP should be 'good' if we accept losing material"
    );
}

#[test]
fn test_see_en_passant_capture() {
    // White pawn on e5, Black pawn moves d7-d5. White captures e5xd6 e.p.
    // The victim (Black pawn) is on d5, but the move is to d6.
    // SEE must correctly identify the victim on d5.
    let fen = "rnbqkbnr/ppppp1pp/8/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 1";
    let mut board = Board::from_str(fen).unwrap();

    // e5 -> f6 (en passant)
    let (m, tables) = find_move(&mut board, "e5", "f6");

    // Pawn (100) captures Pawn (100). Score should be >= 0.
    // If buggy, it sees "capture to empty square" and returns false/0.
    assert_eq!(board.static_exchange_eval(m, 0, &tables), true);
}

#[test]
fn test_see_promotion_capture() {
    // White pawn on a7 captures rook on b8 and promotes to Queen.
    // Gain: Rook (500) + Queen_Diff (800) = Huge.
    let fen = "1r6/P7/8/8/8/8/8/K7 w - - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    // a7 -> b8 (promotion capture)
    // Note: You might need to adjust find_move to handle promotion flags if your parser needs it
    let (m, tables) = find_move(&mut board, "a7", "b8");

    // Even with a huge threshold, this should pass
    assert_eq!(board.static_exchange_eval(m, 1000, &tables), true);
}
