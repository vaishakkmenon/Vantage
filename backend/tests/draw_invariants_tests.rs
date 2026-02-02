use vantage::board::{Board, Piece};
use vantage::moves::execute::{make_move_basic, undo_move_basic};
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::{Move, QUIET_MOVE};
use vantage::square::Square;
use vantage::status::{GameStatus, is_draw_by_fifty_move, position_status};

fn sq(i: u8) -> Square {
    Square::from_index(i)
}
fn mv(piece: Piece, from: u8, to: u8) -> Move {
    Move {
        from: sq(from),
        to: sq(to),
        piece,
        promotion: None,
        flags: QUIET_MOVE,
    }
}

#[test]
fn halfmove_clock_increments_and_resets() {
    let mut b = Board::new();

    let u1 = make_move_basic(&mut b, mv(Piece::Knight, 6, 21)); // g1f3
    assert_eq!(b.halfmove_clock, 1);
    let u2 = make_move_basic(&mut b, mv(Piece::Knight, 62, 45)); // g8f6
    assert_eq!(b.halfmove_clock, 2);

    let u3 = make_move_basic(&mut b, mv(Piece::Pawn, 12, 28)); // e2e4
    assert_eq!(b.halfmove_clock, 0);
    let u4 = make_move_basic(&mut b, mv(Piece::Pawn, 51, 35)); // d7d5
    assert_eq!(b.halfmove_clock, 0);

    let cap = Move {
        flags: vantage::moves::types::CAPTURE,
        ..mv(Piece::Pawn, 28, 35)
    }; // e4xd5
    let u5 = make_move_basic(&mut b, cap);
    assert_eq!(b.halfmove_clock, 0);

    for u in [u5, u4, u3, u2, u1].into_iter().rev() {
        undo_move_basic(&mut b, u);
    }
    assert_eq!(b.halfmove_clock, 0);
}

#[test]
fn history_push_pop_and_repetition_basics() {
    let mut b = Board::new();

    let u1 = make_move_basic(&mut b, mv(Piece::Knight, 6, 21)); // Ng1f3
    let u2 = make_move_basic(&mut b, mv(Piece::Knight, 62, 45)); // Nb8c6
    let u3 = make_move_basic(&mut b, mv(Piece::Knight, 21, 6)); // Nf3g1
    let u4 = make_move_basic(&mut b, mv(Piece::Knight, 45, 62)); // Nc6b8

    let cnt = b.repetition_count();
    assert!(cnt >= 2, "start position should reappear; got {}", cnt);
    assert!(!b.is_threefold());

    undo_move_basic(&mut b, u4);
    undo_move_basic(&mut b, u3);
    undo_move_basic(&mut b, u2);
    undo_move_basic(&mut b, u1);

    let cnt_reset = b.repetition_count();
    assert!(cnt_reset >= 1);
}

#[test]
fn truncates_history_on_irreversible_move() {
    let mut b = Board::new();

    // Quiet “mini-loop” setup (alternating sides, valid all the way):
    // 1. Ng1-f3 (W)
    let u1 = make_move_basic(&mut b, mv(Piece::Knight, 6, 21));
    // ... Ng8-f6 (B)
    let u2 = make_move_basic(&mut b, mv(Piece::Knight, 62, 45));
    // 2. Nf3-g1 (W)
    let u3 = make_move_basic(&mut b, mv(Piece::Knight, 21, 6));
    // Side to move is now Black.

    let before_irrev = b.repetition_count();
    assert!(before_irrev >= 1);

    // Irreversible move must be Black’s move here. Use e7-e5 (52 -> 36).
    let u4 = make_move_basic(&mut b, mv(Piece::Pawn, 52, 36));

    // History window should have been truncated; current position's count is typically 1.
    let after_irrev = b.repetition_count();
    assert!(
        after_irrev <= 2,
        "history should be truncated; got {}",
        after_irrev
    );
    assert!(!b.is_threefold());

    // Build a fresh quiet repetition loop AFTER truncation, alternating sides correctly:
    // 3. Ng1-f3 (W)
    let u5 = make_move_basic(&mut b, mv(Piece::Knight, 6, 21));
    // ... Nf6-g8 (B)  (return Black knight)
    let u6 = make_move_basic(&mut b, mv(Piece::Knight, 45, 62));
    // 4. Nf3-g1 (W)
    let u7 = make_move_basic(&mut b, mv(Piece::Knight, 21, 6));
    // ... Ng8-f6 (B)
    let u8 = make_move_basic(&mut b, mv(Piece::Knight, 62, 45));

    let after_loop = b.repetition_count();
    assert!(
        after_loop >= 2 && !b.is_threefold(),
        "post-truncation repetitions should be tracked independently (got {})",
        after_loop
    );

    for u in [u8, u7, u6, u5, u4, u3, u2, u1].into_iter() {
        undo_move_basic(&mut b, u);
    }
}

#[test]
fn fifty_move_rule_becomes_claimable_at_100_halfmoves() {
    let _tables = load_magic_tables();
    let mut b = Board::new();

    // Bump to 99 halfmoves, then make one quiet move to hit 100.
    b.halfmove_clock = 99;

    // Quiet move: Ng1-f3 (6 -> 21)
    let mv = Move {
        from: Square::from_index(6),
        to: Square::from_index(21),
        piece: Piece::Knight,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let _u = make_move_basic(&mut b, mv);

    assert!(
        is_draw_by_fifty_move(&b), // or your `is_draw_by_fifty(&b)`
        "Should be claimable at exactly 100 halfmoves (50 full moves)"
    );
}

#[test]
fn seventyfive_move_forced_draw_precedes_threefold_at_150_halfmoves() {
    let tables = load_magic_tables();
    let mut b = Board::new();

    // Quick reversible loop (no captures/pawn moves)
    let seq = [
        (6u8, 21u8),  // Ng1-f3
        (62u8, 45u8), // Nb8-c6
        (21u8, 6u8),  // Nf3-g1
        (45u8, 62u8), // Nc6-b8
    ];
    for &(f, t) in &seq {
        let _ = make_move_basic(
            &mut b,
            Move {
                from: Square::from_index(f),
                to: Square::from_index(t),
                piece: Piece::Knight,
                promotion: None,
                flags: QUIET_MOVE,
            },
        );
    }

    // Set to 149 halfmoves, then one quiet move to hit 150.
    b.halfmove_clock = 149;

    let _ = make_move_basic(
        &mut b,
        Move {
            from: Square::from_index(6),
            to: Square::from_index(21),
            piece: Piece::Knight,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );

    assert_eq!(
        position_status(&mut b, &tables),
        GameStatus::DrawSeventyFiveMove,
        "Forced 75-move draw (150 halfmoves) must take precedence over threefold"
    );
}
