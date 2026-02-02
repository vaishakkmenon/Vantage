use vantage::board::Board;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::search::search::search; // Updated to 'search'
use vantage::square::Square;
use std::str::FromStr;

#[test]
fn test_scholar_mate_position_analysis() {
    println!("\n=== Analyzing Scholar's Mate Position ===");
    for depth in 1..=6 {
        let mut board =
            Board::from_str("r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1")
                .unwrap();
        let tables = load_magic_tables();
        let (score, mv) = search(&mut board, &tables, depth, None);

        if let Some(m) = mv {
            println!("Depth {}: score {} - move {:?}", depth, score, m);
        }
    }
}

#[test]
fn test_simple_capture_is_best() {
    // 6k1/8/8/2q5/3P4/8/8/6K1 w - - 0 1 (White pawn on d4 captures Queen on c5)
    let mut board = Board::from_str("6k1/8/8/2q5/3P4/8/8/6K1 w - - 0 1").unwrap();
    let tables = load_magic_tables();

    let (score, best_move) = search(&mut board, &tables, 4, None);

    assert!(best_move.is_some(), "Should find a best move");
    let bm = best_move.unwrap();

    let c5 = Square::from_index(34);
    assert_eq!(bm.to, c5, "Should capture the free queen on c5");
    assert!(score > 0, "Should evaluate as winning");
}

#[test]
fn test_lmr_finds_tactical_move() {
    // Back rank mate: 6k1/5ppp/8/8/8/8/5PPP/4R1K1 w - - 0 1
    // White can play Re8#
    let mut board = Board::from_str("6k1/5ppp/8/8/8/8/5PPP/4R1K1 w - - 0 1").unwrap();
    let tables = load_magic_tables();

    // LMR should NOT prune mate
    let (best_score, best_move) = search(&mut board, &tables, 6, None);

    assert!(best_move.is_some(), "Should find a best move");
    let bm = best_move.unwrap();

    let e8 = Square::from_index(60);
    assert_eq!(bm.to, e8, "Should find Re8# (back rank mate)");
    assert!(best_score > 20000, "Should recognize this as mate");
}

#[test]
fn test_lmr_performance_improvement() {
    use std::time::Instant;
    let mut board =
        Board::from_str("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .unwrap();
    let tables = load_magic_tables();

    let start = Instant::now();
    let (_score, _mv) = search(&mut board, &tables, 6, None);
    let duration = start.elapsed();

    println!("Search to depth 6 took: {:?}", duration);
    assert!(
        duration.as_secs() < 30,
        "Search took too long: {:?}",
        duration
    );
}

#[test]
fn test_lmr_research_accuracy() {
    let mut board = Board::from_str("8/8/8/4k3/8/3K4/4P3/8 w - - 0 1").unwrap();
    let tables = load_magic_tables();

    let (best_score, best_move) = search(&mut board, &tables, 8, None);

    assert!(best_move.is_some(), "Should find a best move");
    assert!(
        best_score > 50,
        "Should evaluate as winning/advantage for white"
    );
}
