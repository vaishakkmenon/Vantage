use std::str::FromStr;
use vantage::board::{Board, Piece};
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::{CAPTURE, KINGSIDE_CASTLE, Move, PROMOTION_CAPTURE, QUIET_MOVE};
use vantage::search::ordering::order_moves;
use vantage::square::Square;

fn make_move(from: &str, to: &str, flags: u8, promo: Option<Piece>, piece: Piece) -> Move {
    Move {
        from: Square::from_str(from).unwrap(),
        to: Square::from_str(to).unwrap(),
        piece,
        promotion: promo,
        flags,
    }
}

#[test]
fn test_ordering_priorities() {
    // Setup board:
    // White Pawn on a7. Black Queen on a8. (Capture + Promo possible)
    // White Pawn on e4. Black Pawn on d5. (Capture possible)
    // White Pawn on h2. (Quiet move possible)
    let b = Board::from_str("q7/P7/8/3p4/4P3/8/7P/R3K2R w KQ - 0 1").unwrap();

    let mv_promo_queen = make_move(
        "a7",
        "a8",
        PROMOTION_CAPTURE,
        Some(Piece::Queen),
        Piece::Pawn,
    ); // high val promo
    let mv_promo_rook = make_move(
        "a7",
        "a8",
        PROMOTION_CAPTURE,
        Some(Piece::Rook),
        Piece::Pawn,
    ); // low val promo

    let mv_capture = make_move("e4", "d5", CAPTURE, None, Piece::Pawn); // Capture Pawn (10) by Pawn(1) -> Score ~10009

    let mv_quiet_killer1 = make_move("h2", "h3", QUIET_MOVE, None, Piece::Pawn);
    let mv_quiet_killer2 = make_move("h2", "h4", QUIET_MOVE, None, Piece::Pawn);
    let mv_quiet_history = make_move("e1", "f1", QUIET_MOVE, None, Piece::Pawn);

    let mut moves = vec![
        mv_quiet_history,
        mv_capture,
        mv_promo_rook,
        mv_quiet_killer2,
        mv_promo_queen,
        mv_quiet_killer1,
    ];

    let killers = [Some(mv_quiet_killer1), Some(mv_quiet_killer2)];
    let history = [[0; 64]; 64]; // default 0
    let hash_move = None;

    let tables = load_magic_tables();

    order_moves(&mut moves, &b, &killers, &history, hash_move, &tables);

    // EXPECTED ORDER:
    // 1. Promo Queen (Score ~20900)
    // 2. Promo Rook  (Score ~20500)
    // 3. Capture     (Score ~10000 + MVV)
    // 4. Killer 1    (9000)
    // 5. Killer 2    (8000)
    // 6. History     (0)

    assert_eq!(moves[0], mv_promo_queen, "Queen Promotion should be first");
    assert_eq!(moves[1], mv_promo_rook, "Rook Promotion should be second");
    assert_eq!(moves[2], mv_capture, "Capture should be third");
    assert_eq!(moves[3], mv_quiet_killer1, "Killer 1 should be fourth");
    assert_eq!(moves[4], mv_quiet_killer2, "Killer 2 should be fifth");
    assert_eq!(moves[5], mv_quiet_history, "History move should be last");
}

#[test]
fn test_pv_override() {
    let b = Board::new();
    let mv_quiet = make_move("e2", "e4", QUIET_MOVE, None, Piece::Pawn);
    let mv_capture = make_move("e2", "d3", CAPTURE, None, Piece::Pawn); // Dummy capture

    let mut moves = vec![mv_capture, mv_quiet];

    // Set Quiet move as Hash Move (PV)
    // Normally Capture > Quiet, but PV should override EVERYTHING.
    let tables = load_magic_tables();
    order_moves(
        &mut moves,
        &b,
        &[None, None],
        &[[0; 64]; 64],
        Some(mv_quiet),
        &tables,
    );

    assert_eq!(moves[0], mv_quiet, "PV Move should always be first");
}

#[test]
fn test_edge_cases_ordering() {
    use vantage::moves::types::EN_PASSANT;

    // 1. En Passant Capture (Pawn x Pawn = 100 * 10 - 1 = 999)
    // Needs a board with piece at 'd5' to work correctly with mvv_lva_score
    // We can't easily mock Board content here without parsing FEN.
    // Let's use a custom FEN.
    let b_complex =
        Board::from_str("rnbq1bnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQ d6 0 1").unwrap();

    // e5xd6 (EP)
    let mv_ep = make_move("e5", "d6", EN_PASSANT, None, Piece::Pawn);

    // e4xd5 (Normal capture if we had a pawn there, but let's fake it or assume logic works)
    // Actually best to test MVV-LVA logic directly.

    // 2. Castling (Quiet)
    let mv_castle = make_move("e1", "g1", KINGSIDE_CASTLE, None, Piece::King);

    // 4. Underpromotion (N) vs Capture
    // Promotion (N) score = 20000 + 320 = 20320
    // Capture (Q x P) = 100 * 10 - 5 = 995 + 10000 = 10995.
    // So Underpromotion > Capture. This is INTENTIONAL based on 'Promotions > Captures' rule.
    let mv_promo_n = make_move(
        "a7",
        "a8",
        PROMOTION_CAPTURE,
        Some(Piece::Knight),
        Piece::Pawn,
    );

    // 5. Killer Move
    let mv_killer = make_move("h2", "h3", QUIET_MOVE, None, Piece::Pawn);

    let mut moves = vec![mv_castle, mv_killer, mv_ep, mv_promo_n];

    let killers = [Some(mv_killer), None];
    let history = [[0; 64]; 64];
    let tables = load_magic_tables();

    order_moves(&mut moves, &b_complex, &killers, &history, None, &tables);

    // Expected:
    // 1. Promo N (20320)
    // 2. EP Capture (10999)
    // 3. Killer (9000)
    // 4. Castle (Quiet/History 0)

    assert_eq!(moves[0], mv_promo_n, "Underpromotion (N) > Capture/Killer");
    assert_eq!(moves[1], mv_ep, "En Passant should be treated as Capture"); // Was failing before fix
    assert_eq!(moves[2], mv_killer, "Killer > History");
    assert_eq!(moves[3], mv_castle, "Castling is just a quiet move");
}

#[test]
fn test_complex_capture_ordering() {
    // Board with Queen on d5, Rook on d5 (ghost), Pawn on d5 (ghost).
    // We'll use FEN to put high value targets.
    // White P on e4, N on f3, Q on d1.
    // Target d5 has a Queen.
    let b =
        Board::from_str("r1b1kbnr/ppp1pppp/8/3q4/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1").unwrap();

    // PxQ (Pawn captures Queen) -> Val 900*10 - 1 = 8999
    let mv_pxq = make_move("e4", "d5", CAPTURE, None, Piece::Pawn);

    // NxQ (Knight captures Queen) -> Val 900*10 - 2 = 8998
    let mv_nxq = make_move("f3", "d5", CAPTURE, None, Piece::Knight);

    // Let's assume we had QxQ. Val 9000 - 5 = 8995.

    let mut moves = vec![mv_nxq, mv_pxq];
    let tables = load_magic_tables();

    order_moves(&mut moves, &b, &[None, None], &[[0; 64]; 64], None, &tables);

    // PxQ should be > NxQ (Least Valuable Attacker for same victim)
    assert_eq!(
        moves[0], mv_pxq,
        "Pawn capturing Queen should be improved over Knight capturing Queen"
    );
    assert_eq!(moves[1], mv_nxq);
}

#[test]
fn test_mvv_victim_priority() {
    // Board Setup:
    // White Pawn at a2. Black Queen at b3.
    // White Pawn at h2. Black Rook at g3.
    // FEN: 8/8/8/8/8/1q4r1/P6P/4K3 w - - 0 1
    let b = Board::from_str("8/8/8/8/8/1q4r1/P6P/4K3 w - - 0 1").unwrap();

    // PxQ (a2xb3) -> Victim Queen(900), Attacker Pawn(1) -> 9000-1 = 8999
    let mv_pxq = make_move("a2", "b3", CAPTURE, None, Piece::Pawn);

    // PxR (h2xg3) -> Victim Rook(500), Attacker Pawn(1) -> 5000-1 = 4999
    let mv_pxr = make_move("h2", "g3", CAPTURE, None, Piece::Pawn);

    let mut moves = vec![mv_pxr, mv_pxq];
    let tables = load_magic_tables();

    order_moves(&mut moves, &b, &[None, None], &[[0; 64]; 64], None, &tables);

    assert_eq!(moves[0], mv_pxq, "PxQ should be ranked higher than PxR");
    assert_eq!(moves[1], mv_pxr);
}

#[test]
fn test_history_sorting() {
    let b = Board::new();

    // Two quiet moves
    let mv_a = make_move("a2", "a3", QUIET_MOVE, None, Piece::Pawn);
    let mv_h = make_move("h2", "h3", QUIET_MOVE, None, Piece::Pawn);

    let mut moves = vec![mv_a, mv_h];

    // Setup history table: mv_h has score 500, mv_a has score 100
    let mut history = [[0; 64]; 64];
    let from_h = Square::from_str("h2").unwrap().index() as usize;
    let to_h = Square::from_str("h3").unwrap().index() as usize;
    history[from_h][to_h] = 500;

    let from_a = Square::from_str("a2").unwrap().index() as usize;
    let to_a = Square::from_str("a3").unwrap().index() as usize;
    history[from_a][to_a] = 100;

    history[from_a][to_a] = 100;

    let tables = load_magic_tables();

    order_moves(&mut moves, &b, &[None, None], &history, None, &tables);

    // Expect mv_h (500) > mv_a (100)
    assert_eq!(
        moves[0], mv_h,
        "History move with higher score should be first"
    );
    assert_eq!(moves[1], mv_a);
}

#[test]
fn test_hash_vs_promo() {
    let b = Board::new();

    // Hash move is a simple quiet move
    let mv_hash = make_move("e2", "e3", QUIET_MOVE, None, Piece::Pawn);

    // Promotion move (Queen) - normally super high priority
    let mv_promo = make_move(
        "a7",
        "a8",
        PROMOTION_CAPTURE,
        Some(Piece::Queen),
        Piece::Pawn,
    );

    let mut moves = vec![mv_promo, mv_hash];

    let tables = load_magic_tables();

    // Hash Move should ALWAYS override everything, even promotions
    order_moves(
        &mut moves,
        &b,
        &[None, None],
        &[[0; 64]; 64],
        Some(mv_hash),
        &tables,
    );

    assert_eq!(moves[0], mv_hash, "PV/Hash move must override Promotions");
}
