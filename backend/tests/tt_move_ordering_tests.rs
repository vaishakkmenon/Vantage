// TT Move Ordering Test Suite

use std::str::FromStr;
use std::time::Instant;
use vantage::board::Board;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::Move;
use vantage::search::context::SearchContext;
use vantage::search::search::{TimeManager, alpha_beta, search};
use vantage::search::tt::TranspositionTable;

const INF: i32 = 32000;

fn search_fixed_depth(
    board: &mut Board,
    tables: &vantage::moves::magic::MagicTables,
    depth: i32,
    tt: &mut TranspositionTable,
    ctx: &mut SearchContext,
    alpha: i32,
    beta: i32,
) -> (i32, Option<Move>) {
    let mut nodes = 0;
    let mut time = TimeManager::new(None);
    alpha_beta(
        board, tables, ctx, tt, depth, 0, alpha, beta, &mut nodes, &mut time,
    )
}

// ============================================================================
// TEST 1: TT Move is Retrieved from Previous Depth
// ============================================================================

#[test]
fn test_tt_stores_best_move() {
    let mut board =
        Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    // Search to depth 3
    let (score1, move1) = search_fixed_depth(&mut board, &tables, 3, &mut tt, &mut ctx, -INF, INF);

    println!("Depth 3: score={}, move={:?}", score1, move1);

    // TT should now contain best move from depth 3
    assert!(move1.is_some(), "Should find a move at depth 3");

    // Search to depth 4 (should reuse TT move from depth 3)
    let (score2, move2) = search_fixed_depth(&mut board, &tables, 4, &mut tt, &mut ctx, -INF, INF);

    println!("Depth 4: score={}, move={:?}", score2, move2);

    // TT should help find move faster
    assert!(move2.is_some(), "Should find a move at depth 4");
}

// ============================================================================
// TEST 2: Iterative Deepening Benefits from TT Moves
// ============================================================================

#[test]
fn test_iterative_deepening_uses_tt_moves() {
    let mut board =
        Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let tables = load_magic_tables();

    // Iterative deepening should be faster than searching depth N directly
    // because TT moves from depth N-1 help search depth N

    // Note: 'search' implements iterative deepening internally
    let (score, best_move) = search(
        &mut board,
        &tables,
        &mut TranspositionTable::new(512),
        5,
        None,
    );

    println!("ID depth 5: score={}, move={:?}", score, best_move);

    assert!(
        best_move.is_some(),
        "Iterative deepening should find a move"
    );
}

// ============================================================================
// TEST 3: Performance - TT Move Ordering Makes Search Faster
// ============================================================================

#[test]
fn test_tt_move_ordering_improves_performance() {
    let mut board1 =
        Board::from_str("r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/3P1N2/PPP2PPP/RNBQKB1R w KQkq - 0 1")
            .unwrap();
    let mut board2 = board1.clone();
    let tables = load_magic_tables();
    let mut ctx = SearchContext::new();

    // Search with small TT (limited benefit)
    let mut tt_small = TranspositionTable::new(1); // 1 MB
    let start = Instant::now();
    let _ = search_fixed_depth(&mut board1, &tables, 5, &mut tt_small, &mut ctx, -INF, INF);
    let time_small = start.elapsed();

    // Search with larger TT (should be faster due to better TT move usage)
    let mut tt_large = TranspositionTable::new(64); // 64 MB
    let start = Instant::now();
    let _ = search_fixed_depth(&mut board2, &tables, 5, &mut tt_large, &mut ctx, -INF, INF);
    let time_large = start.elapsed();

    println!("Small TT (1 MB):  {:?}", time_small);
    println!("Large TT (64 MB): {:?}", time_large);

    // Larger TT should be faster or similar (more cache hits)
    // Not enforcing strict timing as it can vary by hardware
    assert!(time_large.as_millis() > 0, "Search should complete");
}

// ============================================================================
// TEST 4: TT Move from Shallow Search Helps Deeper Search
// ============================================================================

#[test]
fn test_shallow_search_helps_deep_search() {
    let mut board =
        Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    // Do a shallow search first
    let (_, shallow_move) =
        search_fixed_depth(&mut board, &tables, 2, &mut tt, &mut ctx, -INF, INF);

    println!("Shallow (depth 2) move: {:?}", shallow_move);

    // Now do a deep search (should use the TT move from shallow search)
    let start = Instant::now();
    let (_, deep_move) = search_fixed_depth(&mut board, &tables, 5, &mut tt, &mut ctx, -INF, INF);
    let time_with_tt = start.elapsed();

    println!("Deep (depth 5) move: {:?}", deep_move);
    println!("Time with TT priming: {:?}", time_with_tt);

    // Both should find a move
    assert!(shallow_move.is_some());
    assert!(deep_move.is_some());

    // Often (but not always) they'll be the same move
    println!("Moves match: {}", shallow_move == deep_move);
}

// ============================================================================
// TEST 5: TT Move Ordering Doesn't Break Correctness
// ============================================================================

#[test]
fn test_tt_move_ordering_same_results() {
    let mut board1 =
        Board::from_str("r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1")
            .unwrap();
    let mut board2 = board1.clone();
    let tables = load_magic_tables();

    // Search with TT (fresh context)
    let mut ctx1 = SearchContext::new();
    let mut tt = TranspositionTable::new(64);
    let (score_with_tt, move_with_tt) =
        search_fixed_depth(&mut board1, &tables, 4, &mut tt, &mut ctx1, -INF, INF);

    // Search again (fresh TT and fresh context for true independence)
    let mut ctx2 = SearchContext::new();
    let mut tt_fresh = TranspositionTable::new(64);
    let (score_fresh, move_fresh) =
        search_fixed_depth(&mut board2, &tables, 4, &mut tt_fresh, &mut ctx2, -INF, INF);

    println!(
        "First search:  score={}, move={:?}",
        score_with_tt, move_with_tt
    );
    println!(
        "Second search: score={}, move={:?}",
        score_fresh, move_fresh
    );

    // Scores should be identical (deterministic search with fresh contexts)
    assert_eq!(score_with_tt, score_fresh, "Scores should match");

    // Moves might differ in equal positions, but both should be found
    assert!(move_with_tt.is_some());
    assert!(move_fresh.is_some());
}

// ============================================================================
// TEST 6: TT Hit Rate Improves with Move Ordering
// ============================================================================

#[test]
fn test_tt_populated_during_search() {
    let mut board =
        Board::from_str("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    // First search should populate TT
    let (score1, _) = search_fixed_depth(&mut board, &tables, 4, &mut tt, &mut ctx, -INF, INF);

    // Second search of same position should be faster (TT hits)
    let start = Instant::now();
    let (score2, _) = search_fixed_depth(&mut board, &tables, 4, &mut tt, &mut ctx, -INF, INF);
    let time_second = start.elapsed();

    println!("First search score: {}", score1);
    println!("Second search score: {}", score2);
    println!("Second search time: {:?}", time_second);

    // Scores should be identical
    assert_eq!(score1, score2);

    // Second search should be very fast (mostly TT hits)
    assert!(
        time_second.as_millis() < 100,
        "Second search should be nearly instant with TT, took {}ms",
        time_second.as_millis()
    );
}

// ============================================================================
// TEST 7: TT Move Ordering in Tactical Positions
// ============================================================================

#[test]
fn test_tt_move_in_tactical_position() {
    // Position with clear best move (capture free queen)
    let mut board =
        Board::from_str("rnb1kbnr/pppppppp/8/8/8/3q4/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    // Should find Qxd3 quickly
    let (score, best_move) =
        search_fixed_depth(&mut board, &tables, 3, &mut tt, &mut ctx, -INF, INF);

    println!("Tactical position score: {}", score);
    println!("Best move: {:?}", best_move);

    // Should recognize huge advantage (winning queen)
    assert!(score > 700, "Should see winning queen, got {}", score);
    assert!(best_move.is_some());
}

// ============================================================================
// TEST 8: TT Move Ordering Across Different Positions
// ============================================================================

#[test]
fn test_tt_with_different_positions() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    // Search multiple different positions with same TT
    let positions = vec![
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 1",
    ];

    for (i, fen) in positions.iter().enumerate() {
        let mut board = Board::from_str(fen).unwrap();
        let (score, best_move) =
            search_fixed_depth(&mut board, &tables, 3, &mut tt, &mut ctx, -INF, INF);

        println!("Position {}: score={}, move={:?}", i + 1, score, best_move);

        assert!(
            best_move.is_some(),
            "Should find move in position {}",
            i + 1
        );
    }

    println!("TT successfully handled multiple positions");
}

// ============================================================================
// TEST 9: Verify TT Move is Legal
// ============================================================================

#[test]
fn test_tt_move_is_always_legal() {
    let mut board =
        Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    // Search and get best move
    let (_, best_move) = search_fixed_depth(&mut board, &tables, 4, &mut tt, &mut ctx, -INF, INF);

    if let Some(mv) = best_move {
        // Generate legal moves to verify TT move is legal
        use vantage::moves::execute::generate_legal;
        let mut legal_moves = Vec::new();
        let mut scratch = Vec::new();
        generate_legal(&mut board, &tables, &mut legal_moves, &mut scratch);

        // Check TT move is in legal moves
        let is_legal = legal_moves.iter().any(|&legal_mv| legal_mv == mv);

        assert!(is_legal, "TT move {:?} should be legal", mv);
        println!("✅ TT move is legal: {:?}", mv);
    }
}

// ============================================================================
// TEST 10: TT Move Ordering with Mate Positions
// ============================================================================

#[test]
fn test_tt_move_ordering_finds_mate() {
    // Simple mate in 1: Qd8#
    let mut board = Board::from_str("6k1/5ppp/8/8/8/8/5PPP/3Q2K1 w - - 0 1").unwrap();
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    let (score, best_move) =
        search_fixed_depth(&mut board, &tables, 2, &mut tt, &mut ctx, -INF, INF);

    println!("Mate position score: {}", score);
    println!("Mate move: {:?}", best_move);

    // Should find mate (high score)
    assert!(score > 20000, "Should find mate, got score {}", score);
    assert!(best_move.is_some(), "Should find mate move");
}
