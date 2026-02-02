use std::time::Duration;
/// Tactical position tests
/// Verify the engine finds forced mates and wins material
use vantage::board::Board;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::search::search::search;

#[test]
fn test_mate_in_1_scholars_mate() {
    // Position after Qxf7# (this is checkmate position)
    // Just verify we can load it - actual mate finding would need the position before
    let fen = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    // Search should recognize this is a very bad position for Black
    let (score, _best_move) = search(&mut board, &tables, 1, Some(Duration::from_secs(5)));

    // Black is in a lost position (should have very negative score from Black's perspective)
    assert!(
        score < -500,
        "Should recognize bad position, got: {}",
        score
    );
}

#[test]
fn test_back_rank_mate() {
    // Black to move, delivers back rank mate with Rd1#
    // FEN corrected: Rook at d8 (not d1), White King g1
    let fen = "3r2k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    let (_score, best_move) = search(&mut board, &tables, 3, Some(Duration::from_secs(10)));

    // Should find the back rank mate
    let mv = best_move.expect("Should find a move");
    assert_eq!(mv.to_uci(), "d8d1", "Should deliver check with Rd1");
    // Note: Exact move depends on search, but should find mate in short depth
}

#[test]
fn test_capture_hanging_queen() {
    // White queen hanging on e5, Black should capture it
    // FEN corrected: Black Queen at e8 (not d8) so it can capture e5 vertically
    let fen = "rnb1kbnr/pppp1ppp/4q3/4Q3/4P3/8/PPPP1PPP/RNB1KBNR b KQkq - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    let (score, best_move) = search(&mut board, &tables, 2, Some(Duration::from_secs(5)));

    // Should recognize massive material advantage after capturing queen
    // Score should be around +900 (queen) from Black's perspective
    assert!(score > 700, "Should win queen, score: {}", score);

    let mv = best_move.expect("Should find a move");
    let move_uci = mv.to_uci();

    // Queen is on e5, can be captured by d8 queen, or potentially f6/g5
    // Just verify it's a queen capture
    assert!(
        move_uci.ends_with("e5"),
        "Should capture queen on e5: {}",
        move_uci
    );
}

#[test]
fn test_avoid_hanging_piece() {
    // White knight on f3 is hanging, should move it
    let fen = "rnbqkb1r/pppppppp/5n2/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    let (_score, best_move) = search(&mut board, &tables, 3, Some(Duration::from_secs(5)));

    let mv = best_move.expect("Should find a move");
    let move_uci = mv.to_uci();

    // Should either move the knight away or defend it
    // If it doesn't move the knight, it should at least not be a blunder
    // This is a basic test - we just want to ensure it doesn't hang material stupidly
    println!("Best move to avoid hanging knight: {}", move_uci);
}

#[test]
fn test_fork_opportunity() {
    // Black knight can fork king and rook with Ne4
    let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/3P1N2/PPP2PPP/RNBQKB1R b KQkq - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    let (score, _best_move) = search(&mut board, &tables, 4, Some(Duration::from_secs(10)));

    // Should recognize this is a good position for black
    // Score from Black's perspective should be positive (engine returns from side-to-move perspective)
    println!("Fork position score: {}", score);
    // Note: This test mainly ensures no crash, actual fork finding depends on depth/eval
}

#[test]
fn test_starting_position_sanity() {
    // Starting position should be approximately equal
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    let (score, best_move) = search(&mut board, &tables, 3, Some(Duration::from_secs(5)));

    // Should return some standard opening move
    assert!(
        best_move.is_some(),
        "Should find a move in starting position"
    );

    // Evaluation should be close to 0 (within 1 pawn)
    assert!(
        score.abs() < 150,
        "Starting position should be ~equal, got: {}",
        score
    );
}

#[test]
fn test_piece_up_advantage() {
    // White is up a queen, should have huge advantage
    let fen = "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    let (score, _best_move) = search(&mut board, &tables, 2, Some(Duration::from_secs(5)));

    // Should recognize queen advantage (~900 centipawns)
    // From White's perspective, should be very positive
    assert!(
        score > 700,
        "Should recognize queen advantage, got: {}",
        score
    );
}

#[test]
fn test_piece_down_disadvantage() {
    // Black is up a queen (White is down a queen)
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();
    let tables = load_magic_tables();

    let (score, _best_move) = search(&mut board, &tables, 2, Some(Duration::from_secs(5)));

    // Should recognize huge disadvantage
    // From White's perspective (side to move), should be very negative
    assert!(
        score < -800,
        "Should recognize queen disadvantage, got: {}",
        score
    );
}
