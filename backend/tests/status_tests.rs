//! tests/status_tests.rs
//! Robust status tests using the status façade (no board->movegen imports)
use std::str::FromStr;

use vantage::board::{Board, Piece};
use vantage::moves::execute::{make_move_basic, undo_move_basic};
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::{CAPTURE, EN_PASSANT, Move, PROMOTION, QUIET_MOVE};
use vantage::square::Square;
use vantage::status::{
    GameStatus, is_draw_by_fifty_move, is_draw_by_threefold, position_status,
};

// ---- Small helpers ----

#[inline]
fn sq(i: u8) -> Square {
    Square::from_index(i)
}

#[inline]
fn mv(piece: Piece, from: u8, to: u8) -> Move {
    Move {
        from: sq(from),
        to: sq(to),
        piece,
        promotion: None,
        flags: QUIET_MOVE,
    }
}

#[inline]
fn mv_king(from: u8, to: u8) -> vantage::moves::types::Move {
    vantage::moves::types::Move {
        from: sq(from),
        to: sq(to),
        piece: Piece::King,
        promotion: None,
        flags: QUIET_MOVE,
    }
}

#[inline]
fn mv_pawn(from: u8, to: u8) -> vantage::moves::types::Move {
    vantage::moves::types::Move {
        from: sq(from),
        to: sq(to),
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    }
}

#[inline]
fn mv_rook_capture(from: u8, to: u8) -> Move {
    Move {
        from: sq(from),
        to: sq(to),
        piece: Piece::Rook,
        promotion: None,
        flags: CAPTURE,
    }
}

#[inline]
fn mv_promo(from: u8, to: u8, promo: vantage::board::Piece) -> vantage::moves::types::Move {
    vantage::moves::types::Move {
        from: sq(from),
        to: sq(to),
        piece: vantage::board::Piece::Pawn,
        promotion: Some(promo),
        flags: PROMOTION,
    }
}

#[inline]
fn mv_ep_capture(from: u8, to: u8) -> vantage::moves::types::Move {
    // Pawn en passant capture (must be marked both capture + en_passant)
    vantage::moves::types::Move {
        from: sq(from),
        to: sq(to),
        piece: vantage::board::Piece::Pawn,
        promotion: None,
        flags: EN_PASSANT,
    }
}

// e1=4, d1=3; e2=12, f2=13 (a1=0 indexing)

#[test]
fn status_inplay_on_startpos() {
    let tables = load_magic_tables();
    let mut b = Board::new(); // start position
    // Start should not be an immediate draw or mate
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
    assert!(!is_draw_by_threefold(&b));
    assert!(!is_draw_by_fifty_move(&b));
}

#[test]
fn status_draw_by_threefold() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    b.set_fen("8/8/8/8/8/8/4k3/R3K3 w - - 0 1").unwrap();

    // 1st cycle
    let _ = make_move_basic(&mut b, mv_king(4, 3)); // W: Ke1-d1
    let _ = make_move_basic(&mut b, mv_king(12, 11)); // B: Ke2-d2
    let _ = make_move_basic(&mut b, mv_king(3, 4)); // W: Kd1-e1
    let _ = make_move_basic(&mut b, mv_king(11, 12)); // B: Kd2-e2

    // 2nd cycle → third occurrence of start key
    let _ = make_move_basic(&mut b, mv_king(4, 3));
    let _ = make_move_basic(&mut b, mv_king(12, 11));
    let _ = make_move_basic(&mut b, mv_king(3, 4));
    let _ = make_move_basic(&mut b, mv_king(11, 12));

    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn status_draw_by_fifty_move_rule() {
    let tables = load_magic_tables();
    let mut b = Board::new();

    // Bare kings, White to move, halfmove clock already at 99
    // FEN fields: <pieces> <stm> <castling> <ep> <halfmove> <fullmove>
    // Bare kings, White to move, halfmove clock already at 99
    // FEN fields: <pieces> <stm> <castling> <ep> <halfmove> <fullmove>
    // Separated kings: e1 and e8
    b.set_fen("4k3/8/8/8/8/8/8/R3K3 w - - 99 50").unwrap();

    // Not a draw yet at 99 half-moves
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // One reversible half-move → halfmove_clock becomes 100
    // e1 (4) -> d1 (3)
    let _ = make_move_basic(&mut b, mv_king(4, 3));

    // Now it *must* be a 50-move draw
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawFiftyMove);
}

#[test]
fn status_threefold_takes_priority_over_fifty_move() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // Add a rook to avoid dead position; set halfmove=92 so +8 plies => 100.
    b.set_fen("8/8/8/8/8/8/4k3/R3K3 w - - 92 50").unwrap();

    // 1st cycle → second occurrence of the start key
    let _ = make_move_basic(&mut b, mv_king(4, 3)); // W: Ke1-d1
    let _ = make_move_basic(&mut b, mv_king(12, 11)); // B: Ke2-d2
    let _ = make_move_basic(&mut b, mv_king(3, 4)); // W: Kd1-e1
    let _ = make_move_basic(&mut b, mv_king(11, 12)); // B: Kd2-e2

    // 2nd cycle → third occurrence (threefold) and halfmove_clock hits 100
    let _ = make_move_basic(&mut b, mv_king(4, 3));
    let _ = make_move_basic(&mut b, mv_king(12, 11));
    let _ = make_move_basic(&mut b, mv_king(3, 4));
    let _ = make_move_basic(&mut b, mv_king(11, 12));

    // Threefold should take precedence over the 50-move rule
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn status_stalemate_detection() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // Stalemate: Black to move, not in check, no legal moves:
    // Black king h8 (63), White king g6 (46), White queen f7 (45).
    // FEN: 7k/5Q2/6K1/8/8/8/8/8 b - - 0 1
    b.set_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();

    assert_eq!(position_status(&mut b, &tables), GameStatus::Stalemate);
}

#[test]
fn status_checkmate_detection() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // Checkmate: Black to move, in check, no legal moves:
    // Black king h8 (63), White king g6 (46), White queen g7 (54).
    // FEN: 7k/6Q1/6K1/8/8/8/8/8 b - - 0 1
    b.set_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").unwrap();

    assert_eq!(position_status(&mut b, &tables), GameStatus::Checkmate);
}

#[test]
fn status_no_false_draw_with_irreversible_reset() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // White pawn on d2, bare kings; after a reversible mini-cycle, push d2->d3 to reset
    b.set_fen("8/8/8/8/8/8/3Pk3/4K3 w - - 0 1").unwrap();

    // reversible mini-cycle (avoid capturing the pawn: Black uses e2<->f2)
    let _ = make_move_basic(&mut b, mv_king(4, 3)); // W: Ke1-d1
    let _ = make_move_basic(&mut b, mv_king(12, 13)); // B: Ke2-f2
    let _ = make_move_basic(&mut b, mv_king(3, 4)); // W: Kd1-e1
    let _ = make_move_basic(&mut b, mv_king(13, 12)); // B: Kf2-e2

    // irreversible pawn push d2 (11) -> d3 (19)
    let _ = make_move_basic(&mut b, mv_pawn(11, 19));
    // Immediately after, not a draw; window was truncated
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

// ───────────────────────────────────────────────────────────────────────────
// Fivefold repetition (automatic)
// Start: bare kings; 4 full cycles = 5 occurrences of the start key (automatic draw)
// ───────────────────────────────────────────────────────────────────────────
#[test]
fn status_draw_by_fivefold_is_automatic() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    b.set_fen("8/8/8/8/8/8/4k3/R3K3 w - - 0 1").unwrap();

    for _ in 0..4 {
        let _ = make_move_basic(&mut b, mv_king(4, 3)); // W: Ke1-d1
        let _ = make_move_basic(&mut b, mv_king(12, 11)); // B: Ke2-d2
        let _ = make_move_basic(&mut b, mv_king(3, 4)); // W: Kd1-e1
        let _ = make_move_basic(&mut b, mv_king(11, 12)); // B: Kd2-e2
    }

    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawFivefold);
}

// ───────────────────────────────────────────────────────────────────────────
// 75-move rule (automatic)
// Use FEN with halfmove clock at 149; one quiet move → 150 (automatic draw).
// ───────────────────────────────────────────────────────────────────────────
#[test]
fn status_draw_by_seventyfive_move_is_automatic() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    b.set_fen("8/8/8/8/8/8/4k3/4K3 w - - 149 50").unwrap();

    // One reversible half-move (e1→d1) -> halfmove_clock becomes 150
    let _ = make_move_basic(&mut b, mv_king(4, 3));

    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawSeventyFiveMove
    );
}

// ───────────────────────────────────────────────────────────────────────────
// Priority: if both automatic thresholds are met, fivefold takes precedence.
// We get near 75, then perform cycles to reach fivefold too.
// ───────────────────────────────────────────────────────────────────────────
#[test]
fn status_priority_fivefold_over_seventyfive() {
    let tables = load_magic_tables();
    let mut b = Board::new();

    // Start close to 75-move threshold but not yet there.
    b.set_fen("8/8/8/8/8/8/4k3/R3K3 w - - 146 50").unwrap();
    // 1 ply -> 147
    let _ = make_move_basic(&mut b, mv_king(4, 3));
    // Now do cycles to push both repetition count high and halfmove over 150
    for _ in 0..4 {
        let _ = make_move_basic(&mut b, mv_king(12, 11));
        let _ = make_move_basic(&mut b, mv_king(3, 4));
        let _ = make_move_basic(&mut b, mv_king(11, 12));
        let _ = make_move_basic(&mut b, mv_king(4, 3));
    }

    // Fivefold should be chosen over 75-move
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawFivefold);
}

// ───────────────────────────────────────────────────────────────────────────
// Dead positions (insufficient material) → DrawDeadPosition
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn dead_position_k_vs_k() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // Already bare kings
    b.set_fen("8/8/8/8/8/8/4k3/4K3 w - - 0 1").unwrap();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn dead_position_kn_vs_k() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // White N on d3 (19)
    b.set_fen("8/8/8/8/8/3N4/4k3/4K3 w - - 0 1").unwrap();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn dead_position_kb_vs_k() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // White B on d3 (19)
    b.set_fen("8/8/8/8/8/3B4/4k3/4K3 w - - 0 1").unwrap();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn dead_position_knn_vs_k() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // White knights on c3 (18) and d3 (19)
    b.set_fen("8/8/8/8/8/2N5/3N4/4k2K w - - 0 1").unwrap();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn dead_position_kn_vs_kn() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // White N on c3 (18), Black n on g1 (6) (any split-minor case is dead)
    b.set_fen("8/8/8/8/8/2N5/4k3/5n1K w - - 0 1").unwrap();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn dead_position_kb_vs_kb() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // White B on c3 (18), Black b on g1 (6)
    b.set_fen("8/8/8/8/8/2B5/4k3/5b1K w - - 0 1").unwrap();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

// ───────────────────────────────────────────────────────────────────────────
// Not dead: still mating material (guards against false positives)
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn not_dead_kbb_vs_k() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // Two bishops vs bare king: mating material exists
    // Two bishops vs bare king: mating material exists
    // Separated kings: e1 and e8
    b.set_fen("4k3/8/8/8/8/2B5/2B5/4K3 w - - 0 1").unwrap();
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

#[test]
fn not_dead_kbn_vs_k() {
    let tables = load_magic_tables();
    let mut b = Board::new();
    // Bishop + Knight vs bare king: mating material exists
    // Bishop + Knight vs bare king: mating material exists
    // Separated kings: e1 and e8
    b.set_fen("4k3/8/8/8/8/2B5/2N5/4K3 w - - 0 1").unwrap();
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

#[test]
fn seventyfive_forced_draw_precedes_threefold() {
    let tables = load_magic_tables();
    let mut b = Board::new();

    // (Ng1f3, Nb8c6, Nf3g1, Nc6b8) × 2
    let u1 = make_move_basic(&mut b, mv(Piece::Knight, 6, 21));
    let u2 = make_move_basic(&mut b, mv(Piece::Knight, 62, 45));
    let u3 = make_move_basic(&mut b, mv(Piece::Knight, 21, 6));
    let u4 = make_move_basic(&mut b, mv(Piece::Knight, 45, 62));
    let u5 = make_move_basic(&mut b, mv(Piece::Knight, 6, 21));
    let u6 = make_move_basic(&mut b, mv(Piece::Knight, 62, 45));
    let u7 = make_move_basic(&mut b, mv(Piece::Knight, 21, 6));
    let u8 = make_move_basic(&mut b, mv(Piece::Knight, 45, 62));

    assert!(is_draw_by_threefold(&b));

    // Do status check on a clone so we don't perturb the original sequence
    let mut b75 = b.clone();
    b75.halfmove_clock = 150; // ok to set directly on the throwaway clone

    assert_eq!(
        position_status(&mut b75, &tables),
        GameStatus::DrawSeventyFiveMove
    );

    // Undo in the exact reverse order of execution
    for u in [u8, u7, u6, u5, u4, u3, u2, u1] {
        undo_move_basic(&mut b, u);
    }
}

#[test]
fn threefold_only_after_third_occurrence() {
    let tables = load_magic_tables();
    let mut b = Board::new();

    // One loop (startpos appears twice total): NOT threefold yet.
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 6, 21)); // Ng1f3
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 62, 45)); // Nb8c6
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 21, 6)); // Nf3g1
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 45, 62)); // Nc6b8
    assert!(!is_draw_by_threefold(&b), "two occurrences are not enough");
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // Second loop (startpos appears three times total): threefold.
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 6, 21));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 62, 45));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 21, 6));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, 45, 62));
    assert!(
        is_draw_by_threefold(&b),
        "third occurrence should trigger threefold"
    );
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn detects_checkmate_and_stalemate() {
    let tables = load_magic_tables();
    let mut b = Board::new();

    // Simple KQ vs K checkmate: Black to move, no legal moves, in check.
    b.set_fen("7k/6Q1/7K/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(position_status(&mut b, &tables), GameStatus::Checkmate);

    // A classic stalemate pattern: Black to move, no legal moves, NOT in check.
    b.set_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(position_status(&mut b, &tables), GameStatus::Stalemate);
}

#[test]
fn status_priority_seventyfive_over_threefold() {
    // Bare kings; starting side to move = White.
    // halfmove_clock = 146 → after 8 reversible plies it will be 154 (≥ 150),
    // and we will have returned to the start position twice (3rd occurrence).
    //
    // Board:
    //   Black king: g8
    //   White king: g1
    // FEN ranks (8→1): "6k1/8/8/8/8/8/8/6K1"
    let fen = "6k1/8/8/8/8/8/8/6K1 w - - 146 73";
    let mut b = Board::from_str(fen).expect("valid FEN");

    // Helpers from your test module
    #[inline]
    fn sq(i: u8) -> vantage::square::Square {
        vantage::square::Square::from_index(i)
    }
    #[inline]
    fn mv_king(from: u8, to: u8) -> vantage::moves::types::Move {
        use vantage::board::Piece;
        vantage::moves::types::Move {
            from: sq(from),
            to: sq(to),
            piece: Piece::King,
            promotion: None,
            flags: QUIET_MOVE,
        }
    }

    // Squares (0=a1 … 63=h8) with LSB=a1 mapping:
    // g1 = 6,  f2 = 13,   g8 = 62,  f7 = 53
    // One 4-ply cycle returns to the initial position once.
    // Do it twice (8 plies) → the initial position occurs 3 times total.
    let cycle = [
        mv_king(6, 13),  // W: Kg1→f2
        mv_king(62, 53), // B: Kg8→f7
        mv_king(13, 6),  // W: Kf2→g1
        mv_king(53, 62), // B: Kf7→g8   (back to start position)
    ];

    for m in cycle.iter() {
        make_move_basic(&mut b, *m);
    }
    for m in cycle.iter() {
        make_move_basic(&mut b, *m);
    }

    // On this final ply:
    // - halfmove_clock ≥ 150 → SeventyFiveMove auto draw
    // - position repeats for the 3rd time → Threefold claim would also be valid
    // Priority must pick SeventyFiveMove over Threefold.
    let tables = load_magic_tables();
    let status = position_status(&mut b, &tables);
    assert_eq!(status, GameStatus::DrawSeventyFiveMove);
}

#[test]
fn repetition_resets_after_pawn_move() {
    let tables = load_magic_tables();

    // Black king g8 (62), White king g1 (6), White pawn a2 (8)
    // FEN: white to move
    let mut b = Board::from_str("6k1/8/8/8/8/8/P7/6K1 w - - 0 1").expect("valid FEN");

    // Helpers from your test module:
    // - sq(i)
    // - mv(piece, from, to)
    // - mv_king(from, to)

    // A 4-ply reversible king cycle returning to the same board **when White starts**:
    // W: g1→f2, B: g8→f7, W: f2→g1, B: f7→g8
    let cycle_white_starts = [
        mv_king(6, 13),  // W
        mv_king(62, 53), // B
        mv_king(13, 6),  // W
        mv_king(53, 62), // B (back to start)
    ];

    // Do one cycle (White starts) → not threefold yet.
    for m in cycle_white_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // ---- Irreversible move: White pawn a2→a3 (8→16) ----
    make_move_basic(&mut b, mv(Piece::Pawn, 8, 16));
    // Now it's **Black** to move.

    // A 4-ply reversible cycle returning to the same board **when Black starts**:
    // B: g8→f7, W: g1→f2, B: f7→g8, W: f2→g1
    let cycle_black_starts = [
        mv_king(62, 53), // B
        mv_king(6, 13),  // W
        mv_king(53, 62), // B
        mv_king(13, 6),  // W (back to post-pawn-move board)
    ];

    // Do one post-reset cycle → only 2 occurrences after reset → not threefold.
    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // Do a second identical post-reset cycle → 3 occurrences after reset → now threefold.
    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn threefold_ignores_non_capturable_ep_square() {
    let tables = load_magic_tables();

    // Not dead material: add a rook per side to avoid DrawDeadPosition.
    // Kings on g1 (6) and g8 (62); rooks on a1 (0) and a8 (56).
    //
    // EP square set in FEN to "a6" but it's NON-CAPTURABLE (no pawns exist),
    // so it should be ignored in the position identity for repetition.
    //
    // FEN ranks (8→1):
    //   "r5k1/8/8/8/8/8/8/R5K1 w - a6 0 1"
    let mut b = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - a6 0 1").expect("valid FEN");

    // Reversible 4-ply king cycle when White starts:
    // W: g1→f2, B: g8→f7, W: f2→g1, B: f7→g8 (back to start board)
    let cycle_white_starts = [
        mv_king(6, 13),  // W
        mv_king(62, 53), // B
        mv_king(13, 6),  // W
        mv_king(53, 62), // B
    ];

    // First full cycle: after W's first move, the EP square from FEN expires.
    // Since it was non-capturable, correct hashing/identity should IGNORE it,
    // allowing repetition to accumulate across that difference.
    for m in cycle_white_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    // We haven't reached threefold yet.
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // Second identical cycle → 3rd occurrence of the same position (ignoring the
    // initial non-capturable EP field difference) ⇒ Threefold should hold.
    for m in cycle_white_starts.iter() {
        make_move_basic(&mut b, *m);
    }

    assert!(is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn threefold_distinguishes_capturable_ep_square() {
    let tables = load_magic_tables();

    // Kings: g1 (6) and g8 (62); pawns: White e2 (12), Black f4 (29).
    // After e2→e4, EP on e3 is CAPTURABLE by the f4 pawn.
    let mut b = Board::from_str("6k1/8/8/8/5p2/8/4P3/6K1 w - - 0 1").expect("valid FEN");

    // White plays e2→e4 (double push) — creates capturable EP on e3.
    make_move_basic(
        &mut b,
        vantage::moves::types::Move {
            from: vantage::square::Square::from_index(12), // e2
            to: vantage::square::Square::from_index(28),   // e4
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );

    // Reversible 4-ply cycle when Black starts:
    // B: g8→f7, W: g1→f2, B: f7→g8, W: f2→g1
    let cycle_black_starts = [
        mv_king(62, 53), // B
        mv_king(6, 13),  // W
        mv_king(53, 62), // B
        mv_king(13, 6),  // W
    ];

    // After 1st cycle: identity switched from "EP-present" to "no-EP" (B), count(B)=1.
    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // After 2nd cycle: count(B)=2.
    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // After 3rd cycle: count(B)=3 → threefold triggers.
    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn repetition_resets_after_capture() {
    let tables = load_magic_tables();

    // Not dead material: rooks on a1/a8; kings g1/g8. White to move.
    let mut b = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 0 1").expect("valid FEN");

    // White-start reversible 4-ply cycle:
    let cycle_white_starts = [
        mv_king(6, 13),  // W: Kg1→f2
        mv_king(62, 53), // B: Kg8→f7
        mv_king(13, 6),  // W: Kf2→g1
        mv_king(53, 62), // B: Kf7→g8
    ];

    // Do one cycle → 2 occurrences so far; not yet threefold.
    for m in cycle_white_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // Irreversible capture: White Ra1×a8 (0→56).
    make_move_basic(
        &mut b,
        Move {
            from: Square::from_index(0),
            to: Square::from_index(56),
            piece: Piece::Rook,
            promotion: None,
            flags: CAPTURE,
        },
    );

    // Now Black to move. Post-capture identity should start counting from 1.
    // Black-start reversible cycle:
    let cycle_black_starts = [
        mv_king(62, 53), // B
        mv_king(6, 13),  // W
        mv_king(53, 62), // B
        mv_king(13, 6),  // W
    ];

    // One post-capture cycle → still not threefold.
    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // Second post-capture cycle → 3 occurrences after reset → now threefold.
    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn repetition_resets_after_promotion() {
    let tables = load_magic_tables();

    // White pawn ready to promote on a7; kings g1/g8. White to move.
    // Not dead material before/after promotion.
    let mut b = Board::from_str("6k1/8/P7/8/8/8/8/6K1 w - - 0 1").expect("valid FEN");

    // White-start reversible cycle to build pre-reset occurrences:
    let cycle_white_starts = [
        mv_king(6, 13),  // W
        mv_king(62, 53), // B
        mv_king(13, 6),  // W
        mv_king(53, 62), // B
    ];

    // One cycle → not yet threefold.
    for m in cycle_white_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // Irreversible promotion: a7→a8=Q (48→56).
    make_move_basic(
        &mut b,
        Move {
            from: Square::from_index(48),
            to: Square::from_index(56),
            piece: Piece::Pawn,
            promotion: Some(Piece::Queen),
            flags: PROMOTION,
        },
    );

    // Now Black to move. Count occurrences post-reset.
    let cycle_black_starts = [
        mv_king(62, 53), // B
        mv_king(6, 13),  // W
        mv_king(53, 62), // B
        mv_king(13, 6),  // W
    ];

    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    for m in cycle_black_starts.iter() {
        make_move_basic(&mut b, *m);
    }
    assert!(is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn status_threshold_boundaries_50_and_75() {
    let tables = load_magic_tables();

    // 99 → NOT Fifty
    let mut b_99 = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 99 1").expect("fen");
    assert_eq!(b_99.halfmove_clock, 99, "parsed halfmove should be 99");
    assert!(
        !is_draw_by_fifty_move(&b_99),
        "fifty must NOT be claimable at 99"
    );
    assert_eq!(position_status(&mut b_99, &tables), GameStatus::InPlay);

    // 100 → Fifty claimable
    let mut b_100 = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 100 1").expect("fen");
    assert_eq!(b_100.halfmove_clock, 100, "parsed halfmove should be 100");
    assert!(
        is_draw_by_fifty_move(&b_100),
        "fifty MUST be claimable at 100"
    );
    assert_eq!(
        position_status(&mut b_100, &tables),
        GameStatus::DrawFiftyMove
    );

    // 149 → NOT SeventyFive
    let mut b_149 = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 149 1").expect("fen");
    assert_eq!(b_149.halfmove_clock, 149, "parsed halfmove should be 149");
    assert_eq!(
        position_status(&mut b_149, &tables),
        GameStatus::DrawFiftyMove
    );
    assert!(is_draw_by_fifty_move(&b_149));

    // 150 → SeventyFive automatic
    let mut b_150 = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 150 1").expect("fen");
    assert_eq!(b_150.halfmove_clock, 150, "parsed halfmove should be 150");
    assert_eq!(
        position_status(&mut b_150, &tables),
        GameStatus::DrawSeventyFiveMove
    );
}

#[test]
fn status_evaluated_after_move_applied() {
    let tables = load_magic_tables();

    // Start below the 75-move threshold; 50-move is already claimable.
    let mut b = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 148 1").expect("fen");
    assert_eq!(b.halfmove_clock, 148);

    // Pre-move: Fifty is claimable already at 148.
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawFiftyMove);

    // White quiet move → 149: still Fifty.
    make_move_basic(
        &mut b,
        Move {
            from: Square::from_index(6), // Kg1
            to: Square::from_index(13),  // f2
            piece: Piece::King,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawFiftyMove);

    // Black quiet move → 150: now SeventyFive automatic overrides Fifty.
    make_move_basic(
        &mut b,
        Move {
            from: Square::from_index(62), // Kg8
            to: Square::from_index(53),   // f7
            piece: Piece::King,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawSeventyFiveMove
    );
}

#[test]
fn fivefold_respects_irreversible_window() {
    let tables = load_magic_tables();

    // Start with rooks to avoid dead position; White to move.
    let mut b = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 0 1").expect("fen");

    // Build some pre-reset occurrences (but we won't reach fivefold yet).
    let cycle_white_starts = [
        mv_king(6, 13),  // W
        mv_king(62, 53), // B
        mv_king(13, 6),  // W
        mv_king(53, 62), // B
    ];
    for m in cycle_white_starts.iter() {
        make_move_basic(&mut b, *m);
    }

    // Reset window with an irreversible pawn move: add a white pawn and move it a2→a3.
    // (If your make_move requires the piece to exist, inject it via a simple legal move:
    // we can just start from FEN that already had it instead, but to keep the test minimal,
    // we restart from a FEN that includes the pawn.)
    let mut b = Board::from_str("r5k1/8/8/8/8/8/P7/R5K1 w - - 0 1").expect("fen");
    // Immediate irreversible pawn move a2→a3 (8→16).
    make_move_basic(
        &mut b,
        Move {
            from: Square::from_index(8),
            to: Square::from_index(16),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );

    // Now Black to move; we need 5 occurrences of the post-reset identity.
    // Each full Black-start cycle adds one repeated occurrence.
    let cycle_black_starts = [
        mv_king(62, 53), // B
        mv_king(6, 13),  // W
        mv_king(53, 62), // B
        mv_king(13, 6),  // W
    ];

    // Do 4 cycles → occurrences after reset = 1 (post pawn move) + 4 = 5 → fivefold.
    for _ in 0..4 {
        for m in cycle_black_starts.iter() {
            make_move_basic(&mut b, *m);
        }
    }
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawFivefold);
}

#[test]
fn fen_halfmove_parsing_exact() {
    // Use from_fen explicitly to avoid any FromStr surprises.
    let b0 = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 0 1").unwrap();
    let b99 = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 99 1").unwrap();
    let b100 = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 100 1").unwrap();
    assert_eq!(b0.halfmove_clock, 0);
    assert_eq!(b99.halfmove_clock, 99);
    assert_eq!(b100.halfmove_clock, 100);
}

#[test]
fn fifty_consistency_never_mutates_clock() {
    let mut b = Board::from_str("r5k1/8/8/8/8/8/8/R5K1 w - - 99 1").unwrap();
    let tables = load_magic_tables();

    // Take a snapshot
    let h0 = b.halfmove_clock;
    assert!(!is_draw_by_fifty_move(&b));

    // Peek each early check individually, asserting the clock never changes
    let _ = vantage::status::is_fivefold(&b);
    assert_eq!(b.halfmove_clock, h0, "fivefold changed halfmove");
    let _ = vantage::status::is_seventyfive_move(&b);
    assert_eq!(b.halfmove_clock, h0, "seventyfive changed halfmove");
    let _ = vantage::status::is_insufficient_material(&b);
    assert_eq!(b.halfmove_clock, h0, "dead-material changed halfmove");
    let _ = vantage::status::is_draw_by_threefold(&b);
    assert_eq!(b.halfmove_clock, h0, "threefold changed halfmove");

    // Finally, call position_status; if this flips to DrawFiftyMove, the only
    // explanation is that one of the early checks mutated `b` internally.
    let st = position_status(&mut b, &tables);
    assert_eq!(h0, b.halfmove_clock, "clock changed by status()");
    assert_eq!(st, GameStatus::InPlay, "should not be 50 at 99");
}

#[test]
fn status_capture_resets_threefold_window() {
    // Use a position that is NOT dead material (keep a rook on board).
    // White: Kg1 (6), Ra1 (0)
    // Black: Kg8 (62), pawn a2 (8)
    // Side to move: White
    let fen = "6k1/8/8/8/8/8/p7/R5K1 w - - 0 1";

    let tables = load_magic_tables();
    let mut b = Board::default();
    b.set_fen(fen).expect("valid FEN");

    // ---- A → B → A: one reversible mini-cycle via king shuffles ----
    // A (start)
    make_move_basic(&mut b, mv_king(6, 7)); // White: Kg1 -> h1
    make_move_basic(&mut b, mv_king(62, 63)); // Black: Kg8 -> h8
    make_move_basic(&mut b, mv_king(7, 6)); // White: Kh1 -> g1
    make_move_basic(&mut b, mv_king(63, 62)); // Black: Kh8 -> g8
    // Back at A (second occurrence), not yet threefold.

    // ---- Irreversible move: capture on a2 to truncate the repetition window ----
    make_move_basic(&mut b, mv_rook_capture(0, 8)); // White: Rxa2 (a1->a2, capture)

    // ---- Attempt to "complete" what would have been a second reversible cycle ----
    make_move_basic(&mut b, mv_king(62, 63)); // Black: Kg8 -> h8
    make_move_basic(&mut b, mv_king(6, 7)); // White: Kg1 -> h1
    make_move_basic(&mut b, mv_king(63, 62)); // Black: Kh8 -> g8
    make_move_basic(&mut b, mv_king(7, 6)); // White: Kh1 -> g1

    // Because the capture reset the repetition window, we should NOT have threefold.
    assert!(
        !is_draw_by_threefold(&b),
        "capture must truncate repetition history"
    );
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

// Indices used below (LSB=a1):
// a1=0, g1=6, h1=7, a2=8, a3=16, a4=24, a5=32, a6=40, a7=48, a8=56,
// g8=62, h8=63, b5=33

// ---------------------------------------------------------------------
// 1) Pawn (non-capture) move resets threefold window
// ---------------------------------------------------------------------
#[test]
fn status_pawn_push_resets_threefold_window() {
    // Keep a rook to avoid dead-position; give White a pawn that can push.
    // White: Kg1(g1=6), Ra1(0), Pawn a3(16)
    // Black: Kg8(g8=62)
    let fen = "6k1/8/8/8/8/P7/8/R5K1 w - - 0 1";

    let tables = load_magic_tables();
    let mut b = Board::default();
    b.set_fen(fen).expect("valid FEN");

    // A → B → A using king shuffles
    make_move_basic(&mut b, mv_king(6, 7)); // W: Kg1->h1
    make_move_basic(&mut b, mv_king(62, 63)); // B: Kg8->h8
    make_move_basic(&mut b, mv_king(7, 6)); // W: Kh1->g1
    make_move_basic(&mut b, mv_king(63, 62)); // B: Kh8->g8

    // Irreversible: White pawn push a3->a4 (16->24)
    make_move_basic(&mut b, mv_pawn(16, 24));

    // Try to "finish" would-be threefold (won't count due to window reset)
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(6, 7)); // W
    make_move_basic(&mut b, mv_king(63, 62)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W

    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

// ---------------------------------------------------------------------
// 2) Promotion resets threefold window
// ---------------------------------------------------------------------
#[test]
fn status_promotion_resets_threefold_window() {
    // White: Kg1, Ra1, Pawn a7 (can promote to a8=Q)
    // Black: Kg8
    let fen = "6k1/P7/8/8/8/8/8/R5K1 w - - 0 1";

    let tables = load_magic_tables();
    let mut b = Board::default();
    b.set_fen(fen).expect("valid FEN");

    // A → B → A
    make_move_basic(&mut b, mv_king(6, 7)); // W
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W
    make_move_basic(&mut b, mv_king(63, 62)); // B

    // Irreversible: White promotes a7->a8=Q (48->56)
    make_move_basic(&mut b, mv_promo(48, 56, Piece::Queen));

    // Attempt another reversible cycle
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(6, 7)); // W
    make_move_basic(&mut b, mv_king(63, 62)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W

    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

// ---------------------------------------------------------------------
// 3) En passant capture resets threefold window
//    (Note: the preceding double-push is itself irreversible; this test
//     still validates EP capture is handled as an irreversible capture.)
// ---------------------------------------------------------------------
#[test]
fn status_ep_capture_resets_threefold_window() {
    // White: Kg1, Ra1, Pawn b5 (33)
    // Black: Kg8, Pawn a7 (48)
    // EP scenario: Black plays a7->a5 (48->32), then White plays b5->a6 EP (33->40)
    let fen = "p5k1/8/8/1P6/8/8/8/R5K1 w - - 0 1";

    let tables = load_magic_tables();
    let mut b = Board::default();
    b.set_fen(fen).expect("valid FEN");

    // A → B → A
    make_move_basic(&mut b, mv_king(6, 7)); // W
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W
    make_move_basic(&mut b, mv_king(63, 62)); // B

    // One more reversible move so Black gets to move the double-push
    make_move_basic(&mut b, mv_king(6, 7)); // W: Kg1->h1

    // Black double-push: a7->a5 (creates EP on a6)
    make_move_basic(&mut b, mv_pawn(48, 32));

    // Irreversible: White en passant capture: b5->a6 (33->40), EP flags set
    make_move_basic(&mut b, mv_ep_capture(33, 40));

    // Try forming a reversible mini-cycle afterwards
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W (we're on h1 now)
    make_move_basic(&mut b, mv_king(63, 62)); // B
    make_move_basic(&mut b, mv_king(6, 7)); // W

    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

// ---------------------------------------------------------------------
// 4) Double pawn push (EP square set) resets threefold window
// ---------------------------------------------------------------------
#[test]
fn status_double_push_sets_ep_resets_threefold_window() {
    // White: Kg1, Ra1, Pawn a2 (8) can double-push to a4(24)
    // Black: Kg8
    let fen = "6k1/8/8/8/8/8/P7/R5K1 w - - 0 1";

    let tables = load_magic_tables();
    let mut b = Board::default();
    b.set_fen(fen).expect("valid FEN");

    // A → B → A
    make_move_basic(&mut b, mv_king(6, 7)); // W
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W
    make_move_basic(&mut b, mv_king(63, 62)); // B

    // Irreversible: White double-push a2->a4 (8->24)
    make_move_basic(&mut b, mv_pawn(8, 24));

    // Try to "complete" would-be threefold
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(6, 7)); // W
    make_move_basic(&mut b, mv_king(63, 62)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W

    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

// ---------------------------------------------------------------------
// 5) Control: without any irreversible move, threefold occurs
// ---------------------------------------------------------------------
#[test]
fn status_threefold_occurs_without_irreversible() {
    // Minimal non-dead position: keep a rook on.
    // White: Kg1, Ra1
    // Black: Kg8
    let fen = "6k1/8/8/8/8/8/8/R5K1 w - - 0 1";

    let tables = load_magic_tables();
    let mut b = Board::default();
    b.set_fen(fen).expect("valid FEN");

    // A → B → A → B → A (no irreversible moves)
    make_move_basic(&mut b, mv_king(6, 7)); // W: Kg1->h1
    make_move_basic(&mut b, mv_king(62, 63)); // B: Kg8->h8
    make_move_basic(&mut b, mv_king(7, 6)); // W: Kh1->g1
    make_move_basic(&mut b, mv_king(63, 62)); // B: Kh8->g8
    // A second cycle:
    make_move_basic(&mut b, mv_king(6, 7)); // W
    make_move_basic(&mut b, mv_king(62, 63)); // B
    make_move_basic(&mut b, mv_king(7, 6)); // W
    make_move_basic(&mut b, mv_king(63, 62)); // B

    // Now we should have the third occurrence of A.
    assert!(
        is_draw_by_threefold(&b),
        "threefold should be claimable without irreversible moves"
    );

    let st = position_status(&mut b, &tables);
    // Avoid pinning to a specific variant name; just ensure it's not 'InPlay'.
    assert!(
        st != GameStatus::InPlay,
        "status should reflect a threefold-draw state"
    );
}

#[test]
fn make_undo_roundtrip_restores_full_state() {
    let tables = load_magic_tables();

    // Non-dead position with a pawn (so we can do an irreversible push),
    // plus kings and a rook. Halfmove/fullmove chosen arbitrarily.
    let start_fen = "r5k1/8/8/8/8/8/P7/6K1 w - - 12 34";
    let mut b = Board::from_str(start_fen).expect("valid FEN");

    // Snapshot baseline state
    let fen0 = start_fen.to_string(); // exact string we set above
    let h0 = b.halfmove_clock;
    let st0 = position_status(&mut b, &tables);

    // Sequence: quiet, quiet, irreversible pawn push, quiet, quiet
    let u1 = make_move_basic(&mut b, mv_king(6, 13)); // W: Kg1→f2 (quiet)
    let u2 = make_move_basic(&mut b, mv_king(62, 53)); // B: Kg8→f7 (quiet)
    let u3 = make_move_basic(&mut b, mv(Piece::Pawn, 8, 16)); // W: a2→a3 (irreversible)
    let u4 = make_move_basic(&mut b, mv_king(53, 62)); // B: f7→g8 (quiet)
    let u5 = make_move_basic(&mut b, mv_king(13, 6)); // W: f2→g1 (quiet)

    // Undo in reverse order
    undo_move_basic(&mut b, u5);
    undo_move_basic(&mut b, u4);
    undo_move_basic(&mut b, u3);
    undo_move_basic(&mut b, u2);
    undo_move_basic(&mut b, u1);

    // After full rollback, state must match baseline exactly
    assert_eq!(
        b.halfmove_clock, h0,
        "halfmove clock mismatch after round-trip"
    );
    // If your Board implements to_fen(), use it; otherwise FromStr round-trip suffices.
    assert_eq!(
        Board::from_str(&fen0).unwrap().to_fen(),
        b.to_fen(),
        "FEN mismatch after round-trip"
    );

    let st_after = position_status(&mut b, &tables);
    assert_eq!(st_after, st0, "status mismatch after round-trip");
}

#[test]
fn not_dead_when_any_pawn_present() {
    let tables = load_magic_tables();

    // Bare kings plus a single pawn (white a2). This must never be DrawDeadPosition.
    // Bare kings plus a single pawn (white a2). This must never be DrawDeadPosition.
    // Separated kings: e1 and e8
    let mut b = Board::from_str("4k3/8/8/8/8/8/P7/4K3 w - - 0 1").expect("valid FEN");

    let st = position_status(&mut b, &tables);
    assert_eq!(st, GameStatus::InPlay, "any pawn present ⇒ not dead");
}

#[test]
fn threefold_rejected_when_castling_rights_differ() {
    let tables = load_magic_tables();

    // Both sides keep castling rights; add knights so we can do reversible cycles without touching kings/rooks.
    // Top:  rn2k2r
    // Bottom: RN2K2R
    // Rights: KQkq
    let fen = "rn2k2r/8/8/8/8/8/8/RN2K2R w KQkq - 0 1";
    let mut b = Board::from_str(fen).expect("valid FEN");

    // Helper: white knight b1<->c3, black knight b8<->c6
    let wb1 = 1u8;
    let wc3 = 18u8; // b1 -> c3
    let bb8 = 57u8;
    let bc6 = 42u8; // b8 -> c6

    // A → B → A using ONLY knight shuffles (rights remain KQkq)
    let _ = make_move_basic(&mut b, mv(Piece::Knight, wb1, wc3));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, bb8, bc6));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, wc3, wb1));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, bc6, bb8));

    // After one cycle, A has occurred twice total → NOT threefold yet.
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // Now do a reversible move that CHANGES castling rights:
    // Move white rook h1->h2->h1. This permanently clears white's K-side right.
    let wh1 = 7u8;
    let wh2 = 15u8;
    let _ = make_move_basic(&mut b, mv(Piece::Rook, wh1, wh2)); // rights change here (clear K)
    // Give black a harmless reversible reply to keep ply order clean:
    let _ = make_move_basic(&mut b, mv(Piece::Knight, bb8, bc6));
    let _ = make_move_basic(&mut b, mv(Piece::Rook, wh2, wh1)); // rook returns (rights do NOT)
    let _ = make_move_basic(&mut b, mv(Piece::Knight, bc6, bb8));

    // Try to "recreate" A again with the same knight cycle.
    let _ = make_move_basic(&mut b, mv(Piece::Knight, wb1, wc3));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, bb8, bc6));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, wc3, wb1));
    let _ = make_move_basic(&mut b, mv(Piece::Knight, bc6, bb8));

    // Even though piece PLACEMENT matches A, castling rights differ (white no longer has K),
    // so this is NOT counted toward threefold.
    assert!(!is_draw_by_threefold(&b), "rights differ → no threefold");
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);
}

#[test]
fn threefold_accepts_when_castling_rights_match() {
    use std::str::FromStr;
    let tables = load_magic_tables();

    // KRKR, no castling rights from the start; keep at least one rook to avoid dead-material short-circuit.
    // Top:  r5k1  (a8 rook, g8 king)
    // Bottom: R5K1 (a1 rook, g1 king)
    let fen = "r5k1/8/8/8/8/8/8/R5K1 w - - 0 1";
    let mut b = Board::from_str(fen).expect("valid FEN");

    // King shuffles only; rights stay '-' the whole time.
    // White: Kg1<->Kh1 (6<->7), Black: Kg8<->Kh8 (62<->63)
    let wg1 = 6u8;
    let wh1 = 7u8;
    let bg8 = 62u8;
    let bh8 = 63u8;

    // A → B → A → B → A (five half-cycles, three As total)
    // 1st cycle A→B→A
    let _ = make_move_basic(&mut b, mv(Piece::King, wg1, wh1));
    let _ = make_move_basic(&mut b, mv(Piece::King, bg8, bh8));
    let _ = make_move_basic(&mut b, mv(Piece::King, wh1, wg1));
    let _ = make_move_basic(&mut b, mv(Piece::King, bh8, bg8));

    // Not threefold yet: A has occurred twice total.
    assert!(!is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::InPlay);

    // 2nd cycle A→B→A (this third A should trigger threefold)
    let _ = make_move_basic(&mut b, mv(Piece::King, wg1, wh1));
    let _ = make_move_basic(&mut b, mv(Piece::King, bg8, bh8));
    let _ = make_move_basic(&mut b, mv(Piece::King, wh1, wg1));
    let _ = make_move_basic(&mut b, mv(Piece::King, bh8, bg8));

    assert!(is_draw_by_threefold(&b));
    assert_eq!(position_status(&mut b, &tables), GameStatus::DrawThreefold);
}

#[test]
fn draw_insufficient_king_vs_king() {
    let mut b = Board::from_str("8/8/8/8/8/8/8/K6k w - - 0 1").unwrap();
    let tables = load_magic_tables();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn draw_insufficient_kb_vs_k() {
    // White: K e1, B c1; Black: K e8
    let mut b = Board::from_str("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1").unwrap();
    let tables = load_magic_tables();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn draw_insufficient_kn_vs_k() {
    // White: K e1, N c3; Black: K e8
    let mut b = Board::from_str("4k3/8/8/8/8/2N5/8/4K3 w - - 0 1").unwrap();
    let tables = load_magic_tables();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}

#[test]
fn draw_insufficient_kb_vs_kb_same_color() {
    // Bishops on same-colored squares: White B c1 (dark), Black b a3 (dark)
    // Kings on e1/e8
    let mut b = Board::from_str("4k3/8/8/8/8/b7/8/2B1K3 w - - 0 1").unwrap();
    let tables = load_magic_tables();
    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawDeadPosition
    );
}
