use std::str::FromStr;
use vantage::board::{Board, Color, Piece};
use vantage::moves::execute::{make_move_basic, undo_move_basic};
use vantage::moves::types::{
    CAPTURE, EN_PASSANT, KINGSIDE_CASTLE, Move, PROMOTION, PROMOTION_CAPTURE, QUEENSIDE_CASTLE,
    QUIET_MOVE,
};
use vantage::square::Square;

pub(crate) const EMPTY_SQ: u8 = 0xFF;

#[test]
fn roundtrip_simple_move() {
    let mut b = Board::new();
    let before = b.clone();
    let mv = Move {
        from: Square::from_index(12),
        to: Square::from_index(20),
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let undo = make_move_basic(&mut b, mv);
    undo_move_basic(&mut b, undo);
    assert_eq!(b, before);
}

#[test]
fn roundtrip_pawn_capture() {
    let mut board = Board::new();
    let original = board.clone();

    // 1) White: e2 -> e4
    let mv1 = Move {
        from: Square::from_index(12), // e2
        to: Square::from_index(28),   // e4
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let u1 = make_move_basic(&mut board, mv1);

    // 2) Black: d7 -> d5
    let mv2 = Move {
        from: Square::from_index(51), // d7
        to: Square::from_index(35),   // d5
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let u2 = make_move_basic(&mut board, mv2);

    // 3) White captures: e4 -> d5
    let mv3 = Move {
        from: Square::from_index(28), // e4
        to: Square::from_index(35),   // d5
        piece: Piece::Pawn,
        promotion: None,
        flags: CAPTURE,
    };
    let u3 = make_move_basic(&mut board, mv3);

    // Immediately after capture:
    // - The black pawn bitboard for d5 should no longer contain that square
    let mask_d5 = 1u64 << 35;
    assert_eq!(
        board.pieces(Piece::Pawn, Color::Black) & mask_d5,
        0,
        "Black pawn at d5 should have been cleared"
    );

    // - The white pawn bitboard for d5 should now contain that square
    assert_ne!(
        board.pieces(Piece::Pawn, Color::White) & mask_d5,
        0,
        "White pawn should now be on d5"
    );

    // - And piece_on_sq should reflect a white pawn at index 35
    let occ = board.piece_on_sq[35];
    let expected = ((Color::White as u8) << 3) | (Piece::Pawn as u8);
    assert_eq!(occ, expected, "piece_on_sq[35] should encode a White Pawn");

    // Now undo in reverse and verify full restoration
    undo_move_basic(&mut board, u3);
    undo_move_basic(&mut board, u2);
    undo_move_basic(&mut board, u1);
    assert_eq!(
        board, original,
        "Board should be back to the starting position"
    );
}

#[test]
fn roundtrip_white_kingside_castle() {
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut b = Board::from_str(fen).unwrap();
    let original = b.clone();

    let mv = Move {
        from: Square::from_str("e1").unwrap(),
        to: Square::from_str("g1").unwrap(),
        piece: Piece::King,
        promotion: None,
        flags: KINGSIDE_CASTLE,
    };
    let undo = make_move_basic(&mut b, mv);
    assert_ne!(b.pieces(Piece::King, Color::White) & (1 << 6), 0); // g1
    assert_ne!(b.pieces(Piece::Rook, Color::White) & (1 << 5), 0); // f1

    undo_move_basic(&mut b, undo);
    assert_eq!(b, original);
}

#[test]
fn roundtrip_white_queenside_castle() {
    use std::str::FromStr;

    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();
    let original = board.clone();

    let mv = Move {
        from: Square::from_str("e1").unwrap(),
        to: Square::from_str("c1").unwrap(),
        piece: Piece::King,
        promotion: None,
        flags: QUEENSIDE_CASTLE,
    };

    let undo = make_move_basic(&mut board, mv);

    assert_ne!(board.pieces(Piece::King, Color::White) & (1 << 2), 0); // c1
    assert_ne!(board.pieces(Piece::Rook, Color::White) & (1 << 3), 0); // d1

    undo_move_basic(&mut board, undo);
    assert_eq!(board, original);
}

#[test]
fn roundtrip_black_kingside_castle() {
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1"; // note 'b'
    let mut board = Board::from_str(fen).unwrap();
    let original = board.clone();

    let mv = Move {
        from: Square::from_str("e8").unwrap(),
        to: Square::from_str("g8").unwrap(),
        piece: Piece::King,
        promotion: None,
        flags: KINGSIDE_CASTLE,
    };

    let undo = make_move_basic(&mut board, mv);
    assert_ne!(board.pieces(Piece::King, Color::Black) & (1 << 62), 0); // g8
    assert_ne!(board.pieces(Piece::Rook, Color::Black) & (1 << 61), 0); // f8

    undo_move_basic(&mut board, undo);
    assert_eq!(board, original);
}

#[test]
fn roundtrip_black_queenside_castle() {
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1"; // note 'b'
    let mut board = Board::from_str(fen).unwrap();
    let original = board.clone();

    let mv = Move {
        from: Square::from_str("e8").unwrap(),
        to: Square::from_str("c8").unwrap(),
        piece: Piece::King,
        promotion: None,
        flags: QUEENSIDE_CASTLE,
    };

    let undo = make_move_basic(&mut board, mv);
    assert_ne!(board.pieces(Piece::King, Color::Black) & (1 << 58), 0); // c8
    assert_ne!(board.pieces(Piece::Rook, Color::Black) & (1 << 59), 0); // d8

    undo_move_basic(&mut board, undo);
    assert_eq!(board, original);
}

#[test]
fn castling_rights_removed_on_king_move() {
    use std::str::FromStr;
    use vantage::board::{Board, Color};

    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    assert!(board.has_kingside_castle(Color::White));
    assert!(board.has_queenside_castle(Color::White));

    let mv = Move {
        from: Square::from_str("e1").unwrap(),
        to: Square::from_str("f1").unwrap(),
        piece: Piece::King,
        promotion: None,
        flags: QUIET_MOVE,
    };

    let undo = make_move_basic(&mut board, mv);

    assert!(!board.has_kingside_castle(Color::White));
    assert!(!board.has_queenside_castle(Color::White));

    undo_move_basic(&mut board, undo);

    assert!(board.has_kingside_castle(Color::White));
    assert!(board.has_queenside_castle(Color::White));
}

#[test]
fn castling_rights_removed_on_rook_move() {
    use std::str::FromStr;
    use vantage::board::{Board, Color};

    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    assert!(board.has_kingside_castle(Color::White));
    assert!(board.has_queenside_castle(Color::White));

    let mv = Move {
        from: Square::from_str("h1").unwrap(),
        to: Square::from_str("h2").unwrap(),
        piece: Piece::Rook,
        promotion: None,
        flags: QUIET_MOVE,
    };

    let undo = make_move_basic(&mut board, mv);

    assert!(!board.has_kingside_castle(Color::White));
    assert!(board.has_queenside_castle(Color::White));

    undo_move_basic(&mut board, undo);

    assert!(board.has_kingside_castle(Color::White));
    assert!(board.has_queenside_castle(Color::White));
}

#[test]
fn castling_rights_removed_on_rook_capture() {
    // Put a black bishop on c3 (rank 3: "2b5") and make it Black to move.
    let fen = "r3k2r/8/8/8/8/2b5/8/R3K2R b KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    assert!(board.has_queenside_castle(Color::White));

    let mv = Move {
        from: Square::from_str("c3").unwrap(),
        to: Square::from_str("a1").unwrap(),
        piece: Piece::Bishop,
        promotion: None,
        flags: CAPTURE,
    };

    let undo = make_move_basic(&mut board, mv);
    assert!(!board.has_queenside_castle(Color::White));

    undo_move_basic(&mut board, undo);
    assert!(board.has_queenside_castle(Color::White));
}

#[test]
fn roundtrip_en_passant_correct() {
    let mut board = Board::new();
    let original = board.clone();

    // 1) White: e2 -> e4
    let mv1 = Move {
        from: Square::from_str("e2").unwrap(),
        to: Square::from_str("e4").unwrap(),
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let u1 = make_move_basic(&mut board, mv1);

    // 2) Black: a7 -> a6 (dummy move so White can play e4->e5)
    let mv2 = Move {
        from: Square::from_str("a7").unwrap(),
        to: Square::from_str("a6").unwrap(),
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let u2 = make_move_basic(&mut board, mv2);

    // 3) White: e4 -> e5
    let mv3 = Move {
        from: Square::from_str("e4").unwrap(),
        to: Square::from_str("e5").unwrap(),
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let u3 = make_move_basic(&mut board, mv3);

    // 4) Black: d7 -> d5 (double push; EP target = d6)
    let mv4 = Move {
        from: Square::from_str("d7").unwrap(),
        to: Square::from_str("d5").unwrap(),
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    };
    let u4 = make_move_basic(&mut board, mv4);

    // 5) White: e5xd6 en passant (captures the pawn that moved d7->d5)
    let mv5 = Move {
        from: Square::from_str("e5").unwrap(),
        to: Square::from_str("d6").unwrap(),
        piece: Piece::Pawn,
        promotion: None,
        flags: EN_PASSANT,
    };
    let u5 = make_move_basic(&mut board, mv5);

    // After EP: white pawn on d6, black pawn removed from d5
    let d6 = Square::from_str("d6").unwrap().index();
    let d5 = Square::from_str("d5").unwrap().index();
    let mask_d6 = 1u64 << d6;
    let mask_d5 = 1u64 << d5;

    assert_ne!(
        board.pieces(Piece::Pawn, Color::White) & mask_d6,
        0,
        "white pawn should be on d6"
    );
    assert_eq!(
        board.pieces(Piece::Pawn, Color::Black) & mask_d5,
        0,
        "black pawn should be gone from d5"
    );

    // piece_on_sq: d6 has white pawn, d5 is empty
    let expected_white_pawn = ((Color::White as u8) << 3) | (Piece::Pawn as u8);
    assert_eq!(
        board.piece_on_sq[d6 as usize], expected_white_pawn,
        "d6 should encode a White Pawn"
    );
    assert_eq!(
        board.piece_on_sq[d5 as usize], EMPTY_SQ,
        "d5 should be empty after EP"
    );

    // Undo sequence
    undo_move_basic(&mut board, u5);
    undo_move_basic(&mut board, u4);
    undo_move_basic(&mut board, u3);
    undo_move_basic(&mut board, u2);
    undo_move_basic(&mut board, u1);

    assert_eq!(
        board, original,
        "Board should be back to start after EP roundtrip"
    );
}

#[test]
fn halfmove_and_fullmove_counters_with_ep() {
    let mut board = Board::new();
    let orig_half = board.halfmove_clock;
    let orig_full = board.fullmove_number;

    // White: e2->e4 (pawn move resets halfmove)
    let u1 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("e2").unwrap(),
            to: Square::from_str("e4").unwrap(),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.halfmove_clock, 0);
    assert_eq!(board.fullmove_number, orig_full); // increments only after Black's move

    // Black: a7->a6 (pawn move resets halfmove, and increments fullmove)
    let u2 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("a7").unwrap(),
            to: Square::from_str("a6").unwrap(),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.halfmove_clock, 0);
    assert_eq!(board.fullmove_number, orig_full + 1);

    // White: e4->e5 (pawn move resets halfmove)
    let u3 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("e4").unwrap(),
            to: Square::from_str("e5").unwrap(),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.halfmove_clock, 0);
    assert_eq!(board.fullmove_number, orig_full + 1);

    // Black: d7->d5 (pawn move resets halfmove, increments fullmove)
    let u4 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("d7").unwrap(),
            to: Square::from_str("d5").unwrap(),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.halfmove_clock, 0);
    assert_eq!(board.fullmove_number, orig_full + 2);

    // White: e5xd6 en passant (capture resets halfmove)
    let u5 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("e5").unwrap(),
            to: Square::from_str("d6").unwrap(),
            piece: Piece::Pawn,
            promotion: None,
            flags: EN_PASSANT,
        },
    );
    assert_eq!(board.halfmove_clock, 0);
    assert_eq!(board.fullmove_number, orig_full + 2); // still same; increments after Black moves

    // Undo all and verify counters restored
    undo_move_basic(&mut board, u5);
    undo_move_basic(&mut board, u4);
    undo_move_basic(&mut board, u3);
    undo_move_basic(&mut board, u2);
    undo_move_basic(&mut board, u1);

    assert_eq!(board.halfmove_clock, orig_half);
    assert_eq!(board.fullmove_number, orig_full);
}

#[test]
fn roundtrip_promotion_no_capture() {
    // Position: White pawn on e7 ready to promote, kings only otherwise
    // FEN: 4k3/4P3/8/8/8/8/8/4K3 w - - 0 1
    let fen = "4k3/4P3/8/8/8/8/8/4K3 w - - 0 1";
    let mut board = Board::from_str(fen).unwrap();
    let original = board.clone();

    // e7 -> e8 = Q
    let mv = Move {
        from: Square::from_str("e7").unwrap(),
        to: Square::from_str("e8").unwrap(),
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION,
    };
    let undo = make_move_basic(&mut board, mv);

    // After: White Queen on e8, no white pawn on e7/e8
    let e8 = Square::from_str("e8").unwrap().index();
    let e7 = Square::from_str("e7").unwrap().index();
    let mask_e8 = 1u64 << e8;

    assert_ne!(
        board.pieces(Piece::Queen, Color::White) & mask_e8,
        0,
        "White queen should be on e8 after promotion"
    );
    assert_eq!(
        board.piece_on_sq[e7 as usize], EMPTY_SQ,
        "e7 should be empty after promotion"
    );
    // Promotion is a pawn move → halfmove clock resets
    assert_eq!(
        board.halfmove_clock, 0,
        "Halfmove clock should reset on promotion (pawn move)"
    );

    // Undo
    undo_move_basic(&mut board, undo);
    assert_eq!(
        board, original,
        "Board should return to original after promotion undo"
    );
}

#[test]
fn roundtrip_promotion_with_capture() {
    // Position: Black king on a8, black rook on f8; white pawn on e7; white king on e1
    // FEN: k4r2/4P3/8/8/8/8/8/4K3 w - - 0 1
    let fen = "k4r2/4P3/8/8/8/8/8/4K3 w - - 0 1";
    let mut board = Board::from_str(fen).unwrap();
    let original = board.clone();

    // e7 x f8 = N   (capture the rook on f8, promote to knight)
    let mv = Move {
        from: Square::from_str("e7").unwrap(),
        to: Square::from_str("f8").unwrap(),
        piece: Piece::Pawn,
        promotion: Some(Piece::Knight),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    let f8 = Square::from_str("f8").unwrap().index();
    let mask_f8 = 1u64 << f8;

    // After: White knight on f8, black rook removed
    assert_ne!(
        board.pieces(Piece::Knight, Color::White) & mask_f8,
        0,
        "White knight should be on f8 after capture-promotion"
    );
    assert_eq!(
        board.pieces(Piece::Rook, Color::Black) & mask_f8,
        0,
        "Black rook should be gone from f8 after capture"
    );
    // Pawn move → halfmove clock resets
    assert_eq!(
        board.halfmove_clock, 0,
        "Halfmove clock should reset on capture-promotion"
    );

    // Undo
    undo_move_basic(&mut board, undo);
    assert_eq!(
        board, original,
        "Board should return to original after capture-promotion undo"
    );
}

#[test]
fn en_passant_lifecycle_set_clear_undo() {
    let mut board = Board::new();

    // White: e2->e4 sets EP = e3
    let u1 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("e2").unwrap(),
            to: Square::from_str("e4").unwrap(),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.en_passant, Some(Square::from_str("e3").unwrap()));

    // Black: g8->f6 (quiet) should CLEAR EP
    let u2 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("g8").unwrap(),
            to: Square::from_str("f6").unwrap(),
            piece: Piece::Knight,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.en_passant, None);

    // Undo black move → EP restored to e3
    undo_move_basic(&mut board, u2);
    assert_eq!(board.en_passant, Some(Square::from_str("e3").unwrap()));

    // Undo white move → EP cleared again
    undo_move_basic(&mut board, u1);
    assert_eq!(board.en_passant, None);
}

#[test]
fn castling_rights_removed_on_h1_rook_capture() {
    // Add a black bishop on e4 and make it Black to move.
    // Rank 4 FEN: "4b3" puts a bishop on e4.
    let fen = "r3k2r/8/8/8/4b3/8/8/R3K2R b KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    assert!(board.has_kingside_castle(Color::White));

    let mv = Move {
        from: Square::from_str("e4").unwrap(),
        to: Square::from_str("h1").unwrap(),
        piece: Piece::Bishop,
        promotion: None,
        flags: CAPTURE,
    };

    let undo = make_move_basic(&mut board, mv);

    assert!(
        !board.has_kingside_castle(Color::White),
        "Capturing rook on h1 should clear WK right"
    );

    undo_move_basic(&mut board, undo);
    assert!(board.has_kingside_castle(Color::White));
}

#[test]
fn castling_rights_removed_on_a8_and_h8_rook_capture() {
    // All rights present
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();

    // White captures a8 rook → clears Black queenside right
    let mv1 = Move {
        from: Square::from_str("e1").unwrap(), // treat as bishop/queen-like mover for the test
        to: Square::from_str("a8").unwrap(),
        piece: Piece::Bishop,
        promotion: None,
        flags: CAPTURE,
    };
    let u1 = make_move_basic(&mut board, mv1);
    assert!(
        !board.has_queenside_castle(Color::Black),
        "Capturing rook on a8 should clear BQ"
    );
    undo_move_basic(&mut board, u1);
    assert!(board.has_queenside_castle(Color::Black));

    // White captures h8 rook → clears Black kingside right
    let mv2 = Move {
        from: Square::from_str("e1").unwrap(),
        to: Square::from_str("h8").unwrap(),
        piece: Piece::Bishop,
        promotion: None,
        flags: CAPTURE,
    };
    let u2 = make_move_basic(&mut board, mv2);
    assert!(
        !board.has_kingside_castle(Color::Black),
        "Capturing rook on h8 should clear BK"
    );
    undo_move_basic(&mut board, u2);
    assert!(board.has_kingside_castle(Color::Black));
}

#[test]
fn castling_halfmove_fullmove_behavior() {
    // Rights available, clocks zero
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();
    let start_half = board.halfmove_clock;
    let start_full = board.fullmove_number;

    // White castles O-O (king move, no capture) → halfmove increments by 1, fullmove unchanged yet
    let u1 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("e1").unwrap(),
            to: Square::from_str("g1").unwrap(),
            piece: Piece::King,
            promotion: None,
            flags: KINGSIDE_CASTLE,
        },
    );
    assert_eq!(board.halfmove_clock, start_half + 1);
    assert_eq!(board.fullmove_number, start_full);

    // Black quiet move → halfmove increments again, fullmove increments after black
    let u2 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("b8").unwrap(),
            to: Square::from_str("c6").unwrap(),
            piece: Piece::Knight,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.halfmove_clock, start_half + 2);
    assert_eq!(board.fullmove_number, start_full + 1);

    // Undo back to start
    undo_move_basic(&mut board, u2);
    undo_move_basic(&mut board, u1);
    assert_eq!(board.halfmove_clock, start_half);
    assert_eq!(board.fullmove_number, start_full);
}

#[test]
fn quiet_moves_increment_halfmove_and_black_increments_fullmove() {
    let mut board = Board::new();
    let start_half = board.halfmove_clock;
    let start_full = board.fullmove_number;

    // White quiet move (Knight): halfmove +1, fullmove same
    let u1 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("g1").unwrap(),
            to: Square::from_str("f3").unwrap(),
            piece: Piece::Knight,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.halfmove_clock, start_half + 1);
    assert_eq!(board.fullmove_number, start_full);

    // Black quiet move: halfmove +2 total, fullmove +1
    let u2 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("g8").unwrap(),
            to: Square::from_str("f6").unwrap(),
            piece: Piece::Knight,
            promotion: None,
            flags: QUIET_MOVE,
        },
    );
    assert_eq!(board.halfmove_clock, start_half + 2);
    assert_eq!(board.fullmove_number, start_full + 1);

    // Undo
    undo_move_basic(&mut board, u2);
    undo_move_basic(&mut board, u1);
    assert_eq!(board.halfmove_clock, start_half);
    assert_eq!(board.fullmove_number, start_full);
}

#[test]
fn promotion_castling_rights_effects() {
    // Non-capture promotion should NOT affect Black castling rights.
    // Black king/rooks present, all rights; white pawn on e7 ready to promote on e8=Q.
    let fen = "r3k2r/4P3/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let mut board = Board::from_str(fen).unwrap();
    assert!(board.has_kingside_castle(Color::Black));
    assert!(board.has_queenside_castle(Color::Black));

    let u1 = make_move_basic(
        &mut board,
        Move {
            from: Square::from_str("e7").unwrap(),
            to: Square::from_str("e8").unwrap(),
            piece: Piece::Pawn,
            promotion: Some(Piece::Queen),
            flags: PROMOTION,
        },
    );
    assert!(
        board.has_kingside_castle(Color::Black),
        "Non-capture promotion must not change BK"
    );
    assert!(
        board.has_queenside_castle(Color::Black),
        "Non-capture promotion must not change BQ"
    );
    undo_move_basic(&mut board, u1);

    // Capture-promotion on a rook's start square SHOULD clear the corresponding right.
    // Put a white pawn on b7 to capture a8=Q and clear Black's queenside right.
    let fen2 = "r3k2r/1P6/8/8/8/8/8/4K3 w KQkq - 0 1";
    let mut board2 = Board::from_str(fen2).unwrap();
    assert!(board2.has_queenside_castle(Color::Black));

    let u2 = make_move_basic(
        &mut board2,
        Move {
            from: Square::from_str("b7").unwrap(),
            to: Square::from_str("a8").unwrap(),
            piece: Piece::Pawn,
            promotion: Some(Piece::Queen),
            flags: PROMOTION_CAPTURE,
        },
    );
    assert!(
        !board2.has_queenside_castle(Color::Black),
        "Capture-promotion on a8 must clear BQ"
    );
    undo_move_basic(&mut board2, u2);
    assert!(board2.has_queenside_castle(Color::Black));
}
