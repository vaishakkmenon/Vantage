use super::*;

#[test]
fn test_new_empty_board() {
    let b = Board::new_empty();

    // All piece bitboards should be zero:
    assert_empty_board(&b);

    // Move‐clock fields:
    assert_eq!(b.halfmove_clock, 0);
    assert_eq!(b.fullmove_number, 1);

    // Side to move:
    assert!(
        matches!(b.side_to_move, Color::White),
        "Expected White to move on a new empty board"
    );

    // Castling rights & en passant:
    assert_empty_castling(&b);
    assert!(b.en_passant.is_none());
}

// Helper in tests:
fn assert_empty_board(b: &Board) {
    for &(piece, color) in &[
        (Piece::Pawn, Color::White),
        (Piece::Knight, Color::White),
        (Piece::Bishop, Color::White),
        (Piece::Rook, Color::White),
        (Piece::Queen, Color::White),
        (Piece::King, Color::White),
        (Piece::Pawn, Color::Black),
        (Piece::Knight, Color::Black),
        (Piece::Bishop, Color::Black),
        (Piece::Rook, Color::Black),
        (Piece::Queen, Color::Black),
        (Piece::King, Color::Black),
    ] {
        assert_eq!(
            b.pieces(piece, color),
            0,
            "Expected no {:?} of color {:?}",
            piece,
            color
        );
    }
}

fn assert_empty_castling(b: &Board) {
    for &bb in &[CASTLE_WK, CASTLE_WQ, CASTLE_BK, CASTLE_BQ] {
        assert!(!b.has_castling(bb));
    }
}

#[test]
fn test_starting_position_pawns() {
    let b = Board::new();
    assert_eq!(b.pieces(Piece::Pawn, Color::White), WHITE_PAWN_MASK);
    assert_eq!(b.pieces(Piece::Pawn, Color::Black), BLACK_PAWN_MASK);
}

#[test]
fn test_starting_position_white_backrank() {
    let b = Board::new();
    // Verify individual back-rank pieces
    assert_eq!(b.pieces(Piece::Knight, Color::White), WHITE_KNIGHT_MASK);
    assert_eq!(b.pieces(Piece::Bishop, Color::White), WHITE_BISHOP_MASK);
    assert_eq!(b.pieces(Piece::Rook, Color::White), WHITE_ROOK_MASK);
    assert_eq!(b.pieces(Piece::Queen, Color::White), WHITE_QUEEN_MASK);
    assert_eq!(b.pieces(Piece::King, Color::White), WHITE_KING_MASK);
}

#[test]
fn test_starting_position_black_backrank() {
    let b = Board::new();
    // Verify individual back-rank pieces
    assert_eq!(b.pieces(Piece::Knight, Color::Black), BLACK_KNIGHT_MASK);
    assert_eq!(b.pieces(Piece::Bishop, Color::Black), BLACK_BISHOP_MASK);
    assert_eq!(b.pieces(Piece::Rook, Color::Black), BLACK_ROOK_MASK);
    assert_eq!(b.pieces(Piece::Queen, Color::Black), BLACK_QUEEN_MASK);
    assert_eq!(b.pieces(Piece::King, Color::Black), BLACK_KING_MASK);
}

#[test]
fn test_full_starting_position() {
    let b = Board::new();
    // White back-rank + pawns
    let white_expected = WHITE_PAWN_MASK
        | WHITE_ROOK_MASK
        | WHITE_KNIGHT_MASK
        | WHITE_BISHOP_MASK
        | WHITE_QUEEN_MASK
        | WHITE_KING_MASK;
    // Black back-rank + pawns
    let black_expected = BLACK_PAWN_MASK
        | BLACK_ROOK_MASK
        | BLACK_KNIGHT_MASK
        | BLACK_BISHOP_MASK
        | BLACK_QUEEN_MASK
        | BLACK_KING_MASK;

    // Check each side’s pieces

    assert_eq!(b.pieces(Piece::Pawn, Color::White), WHITE_PAWN_MASK);
    assert_eq!(b.pieces(Piece::Knight, Color::White), WHITE_KNIGHT_MASK);
    assert_eq!(b.pieces(Piece::Bishop, Color::White), WHITE_BISHOP_MASK);
    assert_eq!(b.pieces(Piece::Rook, Color::White), WHITE_ROOK_MASK);
    assert_eq!(b.pieces(Piece::Queen, Color::White), WHITE_QUEEN_MASK);
    assert_eq!(b.pieces(Piece::King, Color::White), WHITE_KING_MASK);

    assert_eq!(b.pieces(Piece::Pawn, Color::Black), BLACK_PAWN_MASK);
    assert_eq!(b.pieces(Piece::Knight, Color::Black), BLACK_KNIGHT_MASK);
    assert_eq!(b.pieces(Piece::Bishop, Color::Black), BLACK_BISHOP_MASK);
    assert_eq!(b.pieces(Piece::Rook, Color::Black), BLACK_ROOK_MASK);
    assert_eq!(b.pieces(Piece::Queen, Color::Black), BLACK_QUEEN_MASK);
    assert_eq!(b.pieces(Piece::King, Color::Black), BLACK_KING_MASK);

    // And ensure the overall occupied mask covers both sides
    assert_eq!(b.occupied(), white_expected | black_expected);
}

#[test]
fn test_new_board_castling() {
    let b = Board::new();
    assert!(b.has_castling(CASTLE_WK));
    assert!(b.has_castling(CASTLE_WQ));
    assert!(b.has_castling(CASTLE_BK));
    assert!(b.has_castling(CASTLE_BQ));
}

#[test]
fn test_new_board_defaults() {
    let b = Board::new();

    // Move‐clock fields:
    assert_eq!(b.halfmove_clock, 0, "Starting halfmove clock should be 0");
    assert_eq!(b.fullmove_number, 1, "Starting fullmove number should be 1");

    // Side to move & en passant:
    assert!(
        matches!(b.side_to_move, Color::White),
        "Expected White to move on the standard starting board"
    );
    assert!(
        b.en_passant.is_none(),
        "En passant square should be None at start"
    );

    // Occupied mask covers all pieces:
    let expected = WHITE_PAWN_MASK
        | WHITE_ROOK_MASK
        | WHITE_KNIGHT_MASK
        | WHITE_BISHOP_MASK
        | WHITE_QUEEN_MASK
        | WHITE_KING_MASK
        | BLACK_PAWN_MASK
        | BLACK_ROOK_MASK
        | BLACK_KNIGHT_MASK
        | BLACK_BISHOP_MASK
        | BLACK_QUEEN_MASK
        | BLACK_KING_MASK;
    assert_eq!(
        b.occupied(),
        expected,
        "occupied() should match all starting pieces"
    );
}

#[test]
fn test_starting_placement() {
    assert_eq!(
        Board::new().placement_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"
    );
}

#[test]
fn test_castling_fen() {
    let mut b = Board::new_empty();
    // no rights ⇒ "-"
    assert_eq!(b.castling_fen(), "-");

    // grant each right in isolation
    b.castling_rights = CASTLE_WK;
    assert_eq!(b.castling_fen(), "K");
    b.castling_rights = CASTLE_WQ;
    assert_eq!(b.castling_fen(), "Q");
    b.castling_rights = CASTLE_BK;
    assert_eq!(b.castling_fen(), "k");
    b.castling_rights = CASTLE_BQ;
    assert_eq!(b.castling_fen(), "q");

    // starting full rights
    b = Board::new();
    assert_eq!(b.castling_fen(), "KQkq");
}

#[test]
fn test_en_passant_fen_helper() {
    let mut b = Board::new_empty();
    // default None → "-"
    assert_eq!(b.en_passant_fen(), "-");

    // set en_passant to e3
    let sq_e3 = "e3".parse::<Square>().unwrap();
    b.en_passant = Some(sq_e3);
    assert_eq!(b.en_passant_fen(), "e3");

    // set en_passant to h6
    let sq_h6 = "h6".parse::<Square>().unwrap();
    b.en_passant = Some(sq_h6);
    assert_eq!(b.en_passant_fen(), "h6");
}

#[test]
fn test_to_fen_starting_position() {
    // The full FEN for a fresh new board:
    let expected = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    assert_eq!(Board::new().to_fen(), expected);
}

#[test]
fn test_to_fen_empty_board() {
    // An empty board (all 8s), White to move, no castling, no en-passant:
    let expected = "8/8/8/8/8/8/8/8 w - - 0 1";
    assert_eq!(Board::new_empty().to_fen(), expected);
}

#[test]
fn test_split_fen_valid() {
    // Exactly six fields
    let s = "a b c d e f";
    let parts = Board::split_fen(s).expect("should split");
    assert_eq!(parts, ("a", "b", "c", "d", "e", "f"));
}

#[test]
fn test_split_fen_invalid() {
    // Too few fields (need at least 4)
    let err = Board::split_fen("only three fields").unwrap_err();
    assert!(err.contains("Expected at least 4 FEN fields"));
}

#[test]
fn test_parse_placement_empty() {
    let mut b = Board::new_empty();
    // eight empty ranks
    let placement = "8/8/8/8/8/8/8/8";
    b.parse_placement(placement).unwrap();
    // No pieces ⇒ occupied() == 0, and placement_fen round-trips
    assert_eq!(b.occupied(), 0);
    assert_eq!(b.placement_fen(), placement);
}

#[test]
fn test_parse_placement_starting() {
    let mut b = Board::new_empty();
    let placement = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
    b.parse_placement(placement).unwrap();
    // Check a couple of bitboards:
    assert_eq!(b.pieces(Piece::Pawn, Color::White), WHITE_PAWN_MASK);
    assert_eq!(b.pieces(Piece::Pawn, Color::Black), BLACK_PAWN_MASK);
    // And ensure placement_fen reproduces it
    assert_eq!(b.placement_fen(), placement);
}

#[test]
fn test_parse_active_color() {
    let mut b = Board::new_empty();
    b.parse_active_color("w").unwrap();
    assert_eq!(b.side_to_move, Color::White);

    b.parse_active_color("b").unwrap();
    assert_eq!(b.side_to_move, Color::Black);

    let err = b.parse_active_color("x").unwrap_err();
    assert!(err.contains("Invalid active-color"));
}

#[test]
fn test_parse_castling_rights_none() {
    let mut b = Board::new_empty();
    b.parse_castling_rights("-").unwrap();
    assert_eq!(b.castling_rights, 0);
}

#[test]
fn test_parse_castling_rights_individual() {
    let mut b = Board::new_empty();

    b.parse_castling_rights("K").unwrap();
    assert_eq!(b.castling_rights, CASTLE_WK);

    b.parse_castling_rights("Q").unwrap();
    assert_eq!(b.castling_rights, CASTLE_WQ);

    b.parse_castling_rights("k").unwrap();
    assert_eq!(b.castling_rights, CASTLE_BK);

    b.parse_castling_rights("q").unwrap();
    assert_eq!(b.castling_rights, CASTLE_BQ);
}

#[test]
fn test_parse_castling_rights_all() {
    let mut b = Board::new_empty();
    b.parse_castling_rights("KQkq").unwrap();
    let expected = CASTLE_WK | CASTLE_WQ | CASTLE_BK | CASTLE_BQ;
    assert_eq!(b.castling_rights, expected);
}

#[test]
fn test_parse_castling_rights_invalid() {
    let mut b = Board::new_empty();
    let err = b.parse_castling_rights("KX").unwrap_err();
    assert!(err.contains("Invalid castling-rights character"));
}

#[test]
fn test_parse_en_passant_none() {
    let mut b = Board::new_empty();
    b.parse_en_passant("-").unwrap();
    assert!(b.en_passant.is_none());
}

#[test]
fn test_parse_en_passant_valid() {
    let mut b = Board::new_empty();

    b.parse_en_passant("a6").unwrap();
    assert_eq!(b.en_passant, Some("a6".parse::<Square>().unwrap()));

    b.parse_en_passant("h3").unwrap();
    assert_eq!(b.en_passant, Some("h3".parse::<Square>().unwrap()));
}

#[test]
fn test_parse_en_passant_invalid() {
    let mut b = Board::new_empty();
    let err = b.parse_en_passant("z9").unwrap_err();
    assert!(err.contains("Invalid en-passant square"));
}

#[test]
fn test_parse_clocks_valid() {
    let mut b = Board::new_empty();
    b.parse_clocks("0", "1").unwrap();
    assert_eq!(b.halfmove_clock, 0);
    assert_eq!(b.fullmove_number, 1);

    b.parse_clocks("15", "42").unwrap();
    assert_eq!(b.halfmove_clock, 15);
    assert_eq!(b.fullmove_number, 42);
}

#[test]
fn test_parse_clocks_invalid_halfmove() {
    let mut b = Board::new_empty();
    let err = b.parse_clocks("foo", "1").unwrap_err();
    assert!(err.contains("Invalid halfmove clock"));
}

#[test]
fn test_parse_clocks_invalid_fullmove() {
    let mut b = Board::new_empty();
    let err = b.parse_clocks("0", "bar").unwrap_err();
    assert!(err.contains("Invalid fullmove number"));
}

#[test]
fn test_set_fen_round_trip_starting() {
    let mut b = Board::new_empty();
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    b.set_fen(fen).unwrap();
    assert_eq!(b.to_fen(), fen);
}

#[test]
fn test_set_fen_round_trip_custom() {
    let mut b = Board::new_empty();
    let fen = "8/8/8/3n4/8/2B5/PPP1PPPP/R1BQK1NR b q - 15 42";
    b.set_fen(fen).unwrap();
    assert_eq!(b.to_fen(), fen);
}

#[test]
fn test_set_fen_invalid_field_count() {
    let mut b = Board::new_empty();
    let err = b.set_fen("only three fields").unwrap_err();
    assert!(err.contains("Expected at least 4 FEN fields"));
}

#[test]
fn test_set_fen_invalid_placement() {
    let mut b = Board::new_empty();
    // only 7 ranks => error
    let err = b.set_fen("8/8/8/8/8/8/8 w KQ - 0 1").unwrap_err();
    assert!(err.contains("Expected 8 ranks"));
}

#[test]
fn test_set_fen_invalid_active_color() {
    let mut b = Board::new_empty();
    let err = b.set_fen("8/8/8/8/8/8/8/8 x KQ - 0 1").unwrap_err();
    assert!(err.contains("Invalid active-color"));
}

#[test]
fn test_set_fen_invalid_castling() {
    let mut b = Board::new_empty();
    let err = b.set_fen("8/8/8/8/8/8/8/8 w KX - 0 1").unwrap_err();
    assert!(err.contains("Invalid castling-rights character"));
}

#[test]
fn test_set_fen_invalid_en_passant() {
    let mut b = Board::new_empty();
    let err = b.set_fen("8/8/8/8/8/8/8/8 w - z9 0 1").unwrap_err();
    assert!(err.contains("Invalid en-passant square"));
}

#[test]
fn test_set_fen_invalid_halfmove() {
    let mut b = Board::new_empty();
    let err = b.set_fen("8/8/8/8/8/8/8/8 w - - foo 1").unwrap_err();
    assert!(err.contains("Invalid halfmove clock"));
}

#[test]
fn test_set_fen_invalid_fullmove() {
    let mut b = Board::new_empty();
    let err = b.set_fen("8/8/8/8/8/8/8/8 w - - 0 bar").unwrap_err();
    assert!(err.contains("Invalid fullmove number"));
}

#[test]
fn test_validate_no_overlap() {
    let b = Board::new();
    assert!(b.validate().is_ok());
}

#[test]
fn test_validate_with_overlap() {
    let mut b = Board::new();
    // Force an overlap on a1 (bit 0) by OR’ing into the pawn bitboard:
    let mut pawns = b.pieces(Piece::Pawn, Color::White);
    pawns |= 1 << 0;
    b.set_bb(Color::White, Piece::Pawn, pawns);
    assert!(b.validate().is_err());
}

#[test]
fn test_valid_clocks() {
    let mut b = Board::new_empty();
    assert!(b.parse_clocks("5", "10").is_ok());
    assert_eq!(b.halfmove_clock, 5);
    assert_eq!(b.fullmove_number, 10);
}

#[test]
fn test_invalid_fullmove_zero() {
    let mut b = Board::new_empty();
    let res = b.parse_clocks("0", "0");
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("must be >= 1"));
}

#[test]
fn test_parse_fen_from_str() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board: Board = fen.parse().expect("Failed to parse FEN");
    assert_eq!(board.fullmove_number, 1);
    assert_eq!(board.side_to_move, Color::White);
}

#[test]
fn test_parse_fen_invalid() {
    let fen = "invalid fen string";
    let res: Result<Board, _> = fen.parse();
    assert!(res.is_err());
}

#[test]
fn test_display_fen() {
    let board = Board::new();
    let fen = format!("{}", board);
    assert_eq!(
        fen,
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    );
}
