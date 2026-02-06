use std::str::FromStr;
use vantage::board::Board;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::search::search::search;
use vantage::search::tt::TranspositionTable;

#[test]
fn test_aspiration_finds_correct_move() {
    // Back rank mate
    let mut board = Board::from_str("6k1/5ppp/8/8/8/8/5PPP/4R1K1 w - - 0 1").unwrap();
    let tables = load_magic_tables();

    // Search depth 6 (triggers aspiration windows which start > depth 4)
    let (score, best_move) = search(
        &mut board,
        &tables,
        &mut TranspositionTable::new(512),
        6,
        None,
    );

    assert!(best_move.is_some(), "Should find a best move");
    assert!(score > 20000, "Should recognize mate, got score {}", score);
}

#[test]
fn test_aspiration_handles_score_drop() {
    // r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1
    let mut board =
        Board::from_str("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1")
            .unwrap();
    let tables = load_magic_tables();

    let (score, best_move) = search(
        &mut board,
        &tables,
        &mut TranspositionTable::new(512),
        6,
        None,
    );

    assert!(best_move.is_some());
    assert!(score.abs() < 500);
}

#[test]
fn test_aspiration_handles_score_jump() {
    // Tactical jump
    let mut board =
        Board::from_str("r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1")
            .unwrap();
    let tables = load_magic_tables();

    let (score, best_move) = search(
        &mut board,
        &tables,
        &mut TranspositionTable::new(512),
        6,
        None,
    );

    assert!(best_move.is_some());
    assert!(score > -500); // Loose check, just ensure it doesn't crash or return -INF
}

#[test]
fn test_aspiration_performance() {
    use std::time::Instant;
    let mut board =
        Board::from_str("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .unwrap();
    let tables = load_magic_tables();

    let start = Instant::now();
    let (_score, best_move) = search(
        &mut board,
        &tables,
        &mut TranspositionTable::new(512),
        7,
        None,
    );
    let duration = start.elapsed();

    println!("Aspiration Search to depth 7 took: {:?}", duration);
    assert!(best_move.is_some());
    assert!(duration.as_secs() < 30);
}
