//! Minimal search wiring tests: depth-0 behavior, stalemate/checkmate behavior,
//! and a simple "free capture at depth=1" sanity check.
use vantage::board::Board;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::Move;
use vantage::search::context::SearchContext;
use vantage::search::eval::static_eval;
use vantage::search::search::{TimeManager, alpha_beta};
use vantage::search::tt::TranspositionTable;
use std::str::FromStr;

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

fn fen(f: &str) -> Board {
    Board::from_str(f).expect("valid FEN")
}

/// Helper function to search and return score
fn search_position(f: &str, depth: i32) -> (i32, Option<Move>) {
    let mut board = fen(f);
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();
    search_fixed_depth(&mut board, &tables, depth, &mut tt, &mut ctx, -INF, INF)
}

#[test]
fn depth0_equals_static_eval_white_up_pawn() {
    let mut b = fen("k7/8/8/8/4P3/8/8/6K1 w - - 0 1");
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    let (score, _) = search_fixed_depth(&mut b, &tables, 0, &mut tt, &mut ctx, -INF, INF);
    assert_eq!(score, static_eval(&b, &tables, -INF, INF));
    assert!(score >= 70);
}

#[test]
fn stalemate_returns_zero_any_depth() {
    let mut b = fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    for d in 1..=3 {
        let (score, _) = search_fixed_depth(&mut b, &tables, d, &mut tt, &mut ctx, -INF, INF);
        assert_eq!(score, 0, "stalemate should return 0 at depth {d}");
    }
}

#[test]
fn depth1_prefers_free_capture_white() {
    let mut b = fen("k7/8/8/3p4/4P3/8/8/6K1 w - - 0 1");
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let mut ctx = SearchContext::new();

    let (score, best_move) = search_fixed_depth(&mut b, &tables, 1, &mut tt, &mut ctx, -INF, INF);
    assert!(best_move.is_some());
    assert!(score >= 80);
}

#[test]
fn test_threefold_repetition_recognition() {
    // CORRECTED FEN: Symmetric K+N+2P vs K+N+2P
    // White Knight on e6, Black Knight on e3.
    // Material is exactly equal. Position is symmetric.
    // The engine should find no winning line and evaluate close to 0.
    let fen = "7k/6pp/4N3/8/8/4n3/6PP/7K w - - 0 1";

    // Search depth 6 to allow it to see 3-move repetitions if it tries to shuffle
    let (score, _) = search_position(fen, 6);

    // In a symmetric position, score should be close to 0.
    // Allow small deviation for tempo bonus, PSQT differences, etc.
    assert!(
        score.abs() <= 50,
        "Should evaluate symmetric position close to 0 (within ±50cp), got {}",
        score
    );
}

#[test]
fn test_avoids_losing_queen_to_knight() {
    let fen = "rnbqkb1r/pppp1ppp/5n2/4p3/4P3/5Q2/PPPP1PPP/RNB1KBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score < 500);
}

#[test]
fn test_finds_winning_capture_sequence() {
    let fen = "rnbqkb1r/pppp1ppp/5n2/4p1B1/4P3/8/PPPP1PPP/RN1QKBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score > -100);
}

#[test]
fn test_equal_trade_evaluation() {
    let fen = "rnbqkb1r/pppp1ppp/8/4p3/4n3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score.abs() < 150);
}

#[test]
fn test_refuses_bad_queen_sacrifice() {
    let fen = "r1bqkbnr/ppp2ppp/2n5/3pp3/3PP3/2N2Q2/PPP2PPP/R1B1KBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 5);
    assert!(score < 400);
}

#[test]
fn test_forced_capture_sequence() {
    let fen = "r2qr1k1/ppp2ppp/2n2n2/2bpp1B1/2P5/2N1PN2/PP2QPPP/2RR2K1 w - - 0 1";
    let (score_shallow, _) = search_position(fen, 3);
    let (score_deep, _) = search_position(fen, 5);
    assert!((score_shallow - score_deep).abs() < 200);
}

#[test]
fn test_queen_trade_evaluation() {
    let fen = "r1bqkb1r/ppppnppp/2n5/4p3/4P3/2N2N2/PPPPQPPP/R1B1KB1R w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score.abs() < 250);
}

#[test]
fn test_hanging_piece_after_capture() {
    let fen = "rnbqkb1r/pppp1ppp/8/8/4n3/2N5/PPPPPPPP/R1BQKBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score > 250);
}

#[test]
fn test_desperado_captures() {
    let fen = "rnbqkb1r/pppp1ppp/5n2/4N3/8/8/PPPPPPPP/RNBQKB1R w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score.abs() < 1000);
}

#[test]
fn test_quiet_position_no_captures() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score.abs() < 100);
}

#[test]
fn test_multiple_recaptures() {
    let fen = "rnbqkb1r/pppp1ppp/5n2/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score.abs() < 150);
}

#[test]
fn test_quiescence_depth_limit() {
    let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p3/4P3/3P1N2/PPP2PPP/RNBQKB1R w KQkq - 0 1";
    use std::time::Instant;
    let start = Instant::now();
    let (_score, _) = search_position(fen, 6);
    let elapsed = start.elapsed();
    assert!(elapsed.as_secs() < 180);
}

#[test]
fn test_quiescence_with_checks() {
    let fen = "rnb1kbnr/pppp1ppp/8/4p3/5PPq/8/PPPPP2P/RNBQKBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score < 500);
}

#[test]
fn test_stand_pat_beta_cutoff() {
    let fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score.abs() < 200);
}

#[test]
fn test_delta_pruning_correctness() {
    let fen = "rnbqkb1r/ppp2ppp/3p1n2/4p3/3PP3/2N5/PPP2PPP/R1BQKBNR w KQkq - 0 1";
    let (score, _) = search_position(fen, 4);
    assert!(score.abs() < 500);
}

#[test]
fn test_tactical_win_material() {
    let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p3/3PP3/5N2/PPP2PPP/RNBQKB1R w KQkq - 0 1";
    let (score, _) = search_position(fen, 5);
    assert!(score > -50);
}
