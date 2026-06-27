use std::str::FromStr;
use vantage::board::Board;
use vantage::moves::execute::{generate_legal, make_move_basic};
use vantage::moves::magic::loader::load_magic_tables;
use vantage::search::search::{SearchLimits, SearchResult, search};
use vantage::search::tt::TranspositionTable;

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
const MIDDLEGAME: &str = "r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8";

fn run(fen: &str, depth: i32, node_limit: Option<u64>) -> SearchResult {
    let tables = load_magic_tables();
    let mut board = Board::from_str(fen).unwrap();
    let mut tt = TranspositionTable::new(512);
    search(
        &mut board,
        &tables,
        &mut tt,
        SearchLimits {
            max_depth: depth,
            node_limit,
            time_limit: None,
        },
    )
}

#[test]
fn startpos_test() {
    let result = run(STARTPOS, 5, None);
    assert!(!result.pv.is_empty());
    assert_eq!(result.pv[0], result.best_move.unwrap());
    assert!(result.pv.len() <= 5);

    let mut board = Board::from_str(STARTPOS).unwrap();
    let tables = load_magic_tables();
    for mv in &result.pv {
        let mut moves = vec![];
        let mut scratch = vec![];
        generate_legal(&mut board, &tables, &mut moves, &mut scratch);
        assert!(moves.contains(mv));
        make_move_basic(&mut board, *mv);
    }
}

#[test]
fn kiwipete_test() {
    let result = run(KIWIPETE, 5, None);
    assert!(!result.pv.is_empty());
    assert_eq!(result.pv[0], result.best_move.unwrap());
    assert!(result.pv.len() <= 5);

    let mut board = Board::from_str(KIWIPETE).unwrap();
    let tables = load_magic_tables();
    for mv in &result.pv {
        let mut moves = vec![];
        let mut scratch = vec![];
        generate_legal(&mut board, &tables, &mut moves, &mut scratch);
        assert!(moves.contains(mv));
        make_move_basic(&mut board, *mv);
    }
}

#[test]
fn middlegame_test() {
    let result = run(MIDDLEGAME, 5, None);
    assert!(!result.pv.is_empty());
    assert_eq!(result.pv[0], result.best_move.unwrap());
    assert!(result.pv.len() <= 5);

    let mut board = Board::from_str(MIDDLEGAME).unwrap();
    let tables = load_magic_tables();
    for mv in &result.pv {
        let mut moves = vec![];
        let mut scratch = vec![];
        generate_legal(&mut board, &tables, &mut moves, &mut scratch);
        assert!(moves.contains(mv));
        make_move_basic(&mut board, *mv);
    }
}

#[test]
fn determinism_startpos() {
    let a = run(STARTPOS, 5, None);
    let b = run(STARTPOS, 5, None);
    // test 1: score, nodes, best_move byte-identical
    assert_eq!(a.score, b.score);
    assert_eq!(a.nodes, b.nodes);
    assert_eq!(a.best_move, b.best_move);
    // test 2: pv identical
    assert_eq!(a.pv, b.pv);
}

#[test]
fn determinism_kiwipete() {
    let a = run(KIWIPETE, 5, None);
    let b = run(KIWIPETE, 5, None);
    // test 1: score, nodes, best_move byte-identical
    assert_eq!(a.score, b.score);
    assert_eq!(a.nodes, b.nodes);
    assert_eq!(a.best_move, b.best_move);
    // test 2: pv identical
    assert_eq!(a.pv, b.pv);
}

#[test]
fn determinism_middlegame() {
    let a = run(MIDDLEGAME, 5, None);
    let b = run(MIDDLEGAME, 5, None);
    // test 1: score, nodes, best_move byte-identical
    assert_eq!(a.score, b.score);
    assert_eq!(a.nodes, b.nodes);
    assert_eq!(a.best_move, b.best_move);
    // test 2: pv identical
    assert_eq!(a.pv, b.pv);
}

#[test]
fn node_limit_returns_last_completed_depth_for_startpos() {
    // 1. depth d, no limit -> baseline R_d + N_d
    let rd = run(STARTPOS, 4, None);
    // 2. depth d+1, no limit -> N_d+1
    let rd1 = run(STARTPOS, 5, None);
    // 3. pick L between
    let l = (rd.nodes + rd1.nodes) / 2; // midpoint, sits N_d < L < N_d+1
    // 4. search depth d+1 BUT cap at L
    let capped = run(STARTPOS, 5, Some(l));
    // 5. capped must equal depth-d result
    assert!(capped.depth < 5);
    assert_eq!(capped.depth, rd.depth);
    assert_eq!(capped.score, rd.score);
    assert_eq!(capped.best_move, rd.best_move);
    assert_eq!(capped.pv, rd.pv);
}

#[test]
fn node_limit_returns_last_completed_depth_for_kiwipete() {
    // 1. depth d, no limit -> baseline R_d + N_d
    let rd = run(KIWIPETE, 4, None);
    // 2. depth d+1, no limit -> N_d+1
    let rd1 = run(KIWIPETE, 5, None);
    // 3. pick L between
    let l = (rd.nodes + rd1.nodes) / 2; // midpoint, sits N_d < L < N_d+1
    // 4. search depth d+1 BUT cap at L
    let capped = run(KIWIPETE, 5, Some(l));
    // 5. capped must equal depth-d result
    assert!(capped.depth < 5);
    assert_eq!(capped.depth, rd.depth);
    assert_eq!(capped.score, rd.score);
    assert_eq!(capped.best_move, rd.best_move);
    assert_eq!(capped.pv, rd.pv);
}

#[test]
fn node_limit_returns_last_completed_depth_for_middlegame() {
    // 1. depth d, no limit -> baseline R_d + N_d
    let rd = run(MIDDLEGAME, 4, None);
    // 2. depth d+1, no limit -> N_d+1
    let rd1 = run(MIDDLEGAME, 5, None);
    // 3. pick L between
    let l = (rd.nodes + rd1.nodes) / 2; // midpoint, sits N_d < L < N_d+1
    // 4. search depth d+1 BUT cap at L
    let capped = run(MIDDLEGAME, 5, Some(l));
    // 5. capped must equal depth-d result
    assert!(capped.depth < 5);
    assert_eq!(capped.depth, rd.depth);
    assert_eq!(capped.score, rd.score);
    assert_eq!(capped.best_move, rd.best_move);
    assert_eq!(capped.pv, rd.pv);
}

#[test]
fn node_limit_depth_one_always_completes_startpos() {
    let r = run(STARTPOS, 5, Some(1));
    assert!(r.depth >= 1);
    assert!(r.best_move.is_some());
    assert!(!r.pv.is_empty());
    assert_eq!(r.pv[0], r.best_move.unwrap());
}

#[test]
fn node_limit_depth_one_always_completes_kiwipete() {
    let r = run(KIWIPETE, 5, Some(1));
    assert!(r.depth >= 1);
    assert!(r.best_move.is_some());
    assert!(!r.pv.is_empty());
    assert_eq!(r.pv[0], r.best_move.unwrap());
}

#[test]
fn node_limit_depth_one_always_completes_middlegame() {
    let r = run(MIDDLEGAME, 5, Some(1));
    assert!(r.depth >= 1);
    assert!(r.best_move.is_some());
    assert!(!r.pv.is_empty());
    assert_eq!(r.pv[0], r.best_move.unwrap());
}

#[test]
fn node_limit_deterministic_startpos() {
    let baseline = run(STARTPOS, 5, None);
    let limit = baseline.nodes / 2;

    let a = run(STARTPOS, 5, Some(limit));
    assert!(a.depth < baseline.depth);
    let b = run(STARTPOS, 5, Some(limit));
    assert_eq!(a.score, b.score);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.pv, b.pv);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn node_limit_deterministic_kiwipete() {
    let baseline = run(KIWIPETE, 5, None);
    let limit = baseline.nodes / 2;

    let a = run(KIWIPETE, 5, Some(limit));
    assert!(a.depth < baseline.depth);
    let b = run(KIWIPETE, 5, Some(limit));
    assert_eq!(a.score, b.score);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.pv, b.pv);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn node_limit_deterministic_middlegame() {
    let baseline = run(MIDDLEGAME, 5, None);
    let limit = baseline.nodes / 2;

    let a = run(MIDDLEGAME, 5, Some(limit));
    assert!(a.depth < baseline.depth);
    let b = run(MIDDLEGAME, 5, Some(limit));
    assert_eq!(a.score, b.score);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.pv, b.pv);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn tt_clear_makes_queries_independent() {
    let tables = load_magic_tables();
    let limits = SearchLimits {
        max_depth: 5,
        node_limit: None,
        time_limit: None,
    };

    let b_fresh = run(MIDDLEGAME, 5, None);

    let mut tt = TranspositionTable::new(512);
    let mut ba = Board::from_str(KIWIPETE).unwrap();
    let _ = search(&mut ba, &tables, &mut tt, limits);
    tt.clear();
    let mut bb = Board::from_str(MIDDLEGAME).unwrap();
    let b_cleared = search(&mut bb, &tables, &mut tt, limits);

    assert_eq!(b_cleared.score, b_fresh.score);
    assert_eq!(b_cleared.best_move, b_fresh.best_move);
    assert_eq!(b_cleared.pv, b_fresh.pv);
    assert_eq!(b_cleared.nodes, b_fresh.nodes);
}
