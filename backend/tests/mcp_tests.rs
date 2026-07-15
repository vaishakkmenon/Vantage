use vantage::mcp::{analyze_move, evaluate};
use vantage::moves::magic::loader::load_magic_tables;
use vantage::search::tt::TranspositionTable;

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
const MIDDLEGAME: &str = "r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 8";

// White queen on d4 is safe (not on any line from the black queen on a8).
// d4-d5 puts it on the a8-h1 diagonal, undefended -> Qxd5 for free.
const HANGS_QUEEN: &str = "q3k3/8/8/8/3Q4/8/8/4K3 w - - 0 1";

const NODES: u64 = 50_000;

#[test]
fn evaluate_is_deterministic_startpos() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let a = evaluate(STARTPOS, NODES, &tables, &mut tt).unwrap();
    let mut tt = TranspositionTable::new(64);
    let b = evaluate(STARTPOS, NODES, &tables, &mut tt).unwrap();

    assert_eq!(a.score, b.score);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.pv, b.pv);
    assert_eq!(a.depth, b.depth);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn evaluate_is_deterministic_kiwipete() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let a = evaluate(KIWIPETE, NODES, &tables, &mut tt).unwrap();
    let mut tt = TranspositionTable::new(64);
    let b = evaluate(KIWIPETE, NODES, &tables, &mut tt).unwrap();

    assert_eq!(a.score, b.score);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.pv, b.pv);
    assert_eq!(a.depth, b.depth);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn evaluate_is_deterministic_middlegame() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let a = evaluate(MIDDLEGAME, NODES, &tables, &mut tt).unwrap();
    let mut tt = TranspositionTable::new(64);
    let b = evaluate(MIDDLEGAME, NODES, &tables, &mut tt).unwrap();

    assert_eq!(a.score, b.score);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.pv, b.pv);
    assert_eq!(a.depth, b.depth);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn analyze_move_is_deterministic_startpos() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let a = analyze_move(STARTPOS, "e2e4", NODES, &tables, &mut tt).unwrap();
    let mut tt = TranspositionTable::new(64);
    let b = analyze_move(STARTPOS, "e2e4", NODES, &tables, &mut tt).unwrap();

    assert_eq!(a.legal, b.legal);
    assert_eq!(a.classification, b.classification);
    assert_eq!(a.cp_loss, b.cp_loss);
    assert_eq!(a.eval_before, b.eval_before);
    assert_eq!(a.eval_after, b.eval_after);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.better_line, b.better_line);
    assert_eq!(a.refutation, b.refutation);
}

#[test]
fn analyze_move_is_deterministic_kiwipete() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let a = analyze_move(KIWIPETE, "e1g1", NODES, &tables, &mut tt).unwrap();
    let mut tt = TranspositionTable::new(64);
    let b = analyze_move(KIWIPETE, "e1g1", NODES, &tables, &mut tt).unwrap();

    assert_eq!(a.legal, b.legal);
    assert_eq!(a.classification, b.classification);
    assert_eq!(a.cp_loss, b.cp_loss);
    assert_eq!(a.eval_before, b.eval_before);
    assert_eq!(a.eval_after, b.eval_after);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.better_line, b.better_line);
    assert_eq!(a.refutation, b.refutation);
}

#[test]
fn analyze_move_is_deterministic_middlegame() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let a = analyze_move(MIDDLEGAME, "d4d5", NODES, &tables, &mut tt).unwrap();
    let mut tt = TranspositionTable::new(64);
    let b = analyze_move(MIDDLEGAME, "d4d5", NODES, &tables, &mut tt).unwrap();

    assert_eq!(a.legal, b.legal);
    assert_eq!(a.classification, b.classification);
    assert_eq!(a.cp_loss, b.cp_loss);
    assert_eq!(a.eval_before, b.eval_before);
    assert_eq!(a.eval_after, b.eval_after);
    assert_eq!(a.best_move, b.best_move);
    assert_eq!(a.better_line, b.better_line);
    assert_eq!(a.refutation, b.refutation);
}

#[test]
fn illegal_move_returns_clean_result_not_panic() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    // pawn cannot jump three ranks from its start square, illegal under any ruleset
    let result = analyze_move(STARTPOS, "e2e5", NODES, &tables, &mut tt);

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.legal, false);
    assert_eq!(result.classification, None);
    assert_eq!(result.cp_loss, None);
    assert_eq!(result.eval_before, None);
    assert_eq!(result.eval_after, None);
    assert_eq!(result.best_move, None);
    assert!(result.better_line.is_empty());
    assert!(result.refutation.is_empty());
}

#[test]
fn malformed_move_string_returns_clean_result_not_panic() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let result = analyze_move(STARTPOS, "not-a-move", NODES, &tables, &mut tt);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().legal, false);
}

#[test]
fn hanging_queen_is_classified_blunder_with_correct_refutation() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let result = analyze_move(HANGS_QUEEN, "d4d5", NODES, &tables, &mut tt).unwrap();

    assert_eq!(result.legal, true);
    assert_eq!(result.classification, Some("blunder".to_string()));

    let cp_loss = result
        .cp_loss
        .expect("hanging a queen should be a Cp-scale swing, not a mate score");
    assert!(
        cp_loss > 500,
        "expected a queen-sized cp_loss (>500), got {cp_loss}"
    );

    // the refutation must actually be the queen capture, a8 takes d5
    assert_eq!(result.refutation.first(), Some(&"a8d5".to_string()));
}

#[test]
fn engines_own_best_move_is_not_penalized() {
    let tables = load_magic_tables();
    let mut tt = TranspositionTable::new(64);
    let baseline = evaluate(STARTPOS, NODES, &tables, &mut tt).unwrap();
    let best_uci = baseline.best_move.expect("startpos must have a best move");

    let mut tt = TranspositionTable::new(64);
    let result = analyze_move(STARTPOS, &best_uci, NODES, &tables, &mut tt).unwrap();

    assert_eq!(result.legal, true);
    let cp_loss = result.cp_loss.unwrap_or(0);
    assert!(
        cp_loss.abs() <= 50,
        "engine's own best move should have ~0 cp_loss, got {cp_loss}"
    );
    assert_ne!(result.classification, Some("blunder".to_string()));
    assert_ne!(result.classification, Some("mistake".to_string()));
}
