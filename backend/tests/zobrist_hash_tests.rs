// tests/zobrist_tests.rs
use vantage::board::{Board, Color, Piece};
use vantage::hash::zobrist::zobrist_keys;
use vantage::moves::types::{Move, PROMOTION, PROMOTION_CAPTURE, QUIET_MOVE};
use vantage::{
    // move executor
    moves::execute::{make_move_basic, undo_move_basic},
    // if you keep the helper public, otherwise copy the check inline
    // hash::zobrist::zobrist_keys,
    square::Square,
};
use std::str::FromStr;

// Castling White Kingside
const CASTLE_WK: u8 = 0b0001;
// Castling White Queenside
const CASTLE_WQ: u8 = 0b0010;
// Castling Black Kingside
const CASTLE_BK: u8 = 0b0100;
// Castling Black Queenside
const CASTLE_BQ: u8 = 0b1000;

//Helpers

fn sq(i: u8) -> Square {
    Square::from_index(i)
}

fn mv_king(from: u8, to: u8) -> Move {
    Move {
        from: sq(from),
        to: sq(to),
        piece: Piece::King,
        promotion: None,
        flags: QUIET_MOVE,
    }
}

fn mv_pawn(from: u8, to: u8) -> Move {
    Move {
        from: sq(from),
        to: sq(to),
        piece: Piece::Pawn,
        promotion: None,
        flags: QUIET_MOVE,
    }
}

fn mv_promo(from: u8, to: u8, p: Piece) -> Move {
    Move {
        from: sq(from),
        to: sq(to),
        piece: Piece::Pawn,
        promotion: Some(p),
        flags: PROMOTION,
    }
}

fn mv_promo_capture(from: u8, to: u8, p: Piece) -> Move {
    Move {
        from: sq(from),
        to: sq(to),
        piece: Piece::Pawn,
        promotion: Some(p),
        flags: PROMOTION_CAPTURE,
    }
}

// Actual tests

#[test]
fn zobrist_start_hash_stable() {
    let b = Board::new();
    assert_eq!(b.zobrist, b.compute_zobrist_full());
}

#[test]
fn zobrist_fen_recompute_stable() {
    let fens = &[
        // Start position
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        // After 1.e4: Black to move, EP square e3
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        // Kings + rooks only with all rights
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
    ];

    for fen in fens {
        let mut b = Board::new_empty();
        b.set_fen(fen).expect("valid FEN");
        assert_eq!(b.zobrist, b.compute_zobrist_full(), "FEN: {fen}");
    }
}

#[test]
fn zobrist_castling_rights_toggle_changes_hash() {
    let mut b = Board::new();
    b.castling_rights = 0;
    b.refresh_zobrist();
    let h_none = b.zobrist;

    for (bit, name) in &[(0b0001u8, "K"), (0b0010, "Q"), (0b0100, "k"), (0b1000, "q")] {
        b.castling_rights = *bit;
        b.refresh_zobrist();
        assert_ne!(
            b.zobrist, h_none,
            "Enabling right {name} should change hash"
        );

        b.castling_rights = 0;
        b.refresh_zobrist();
        assert_eq!(
            b.zobrist, h_none,
            "Clearing right {name} should restore hash"
        );
    }
}

#[test]
fn zobrist_ep_capturable_differs_fen() {
    // Kings e1/e8, white pawn e4, black pawn d4. Black to move.
    // EP square e3 is capturable by the pawn on d4.
    let fen_with_ep = "4k3/8/8/8/3pP3/8/8/4K3 b KQkq e3 0 1";
    let fen_no_ep = "4k3/8/8/8/3pP3/8/8/4K3 b KQkq - 0 1";

    let mut b_ep = Board::new_empty();
    b_ep.set_fen(fen_with_ep).expect("FEN with EP");
    let h_ep = b_ep.zobrist;

    let mut b_no = Board::new_empty();
    b_no.set_fen(fen_no_ep).expect("FEN without EP");
    let h_no = b_no.zobrist;

    assert_ne!(h_ep, h_no, "Capturable EP should change the Zobrist hash");
}

#[test]
fn zobrist_side_to_move_xor_matches_recompute() {
    // 1) Start from any legal board (startpos here)
    let mut b = Board::new();
    let h0 = b.zobrist;

    // 2) Flip the side-to-move *without* touching anything else
    b.side_to_move = match b.side_to_move {
        Color::White => Color::Black,
        Color::Black => Color::White,
    };

    // 3) Full recompute is our ground truth
    let expected = b.compute_zobrist_full();

    // 4) Incremental rule: toggle exactly once with the side key
    let got = h0 ^ zobrist_keys().side_to_move;

    // 5) They must match
    assert_eq!(
        got, expected,
        "Side-to-move XOR must equal a full recompute after flipping the mover"
    );
}

#[allow(dead_code)]
fn mv(from: u8, to: u8, piece: Piece) -> vantage::moves::types::Move {
    vantage::moves::types::Move {
        from: Square::from_index(from),
        to: Square::from_index(to),
        piece,
        promotion: None,
        flags: QUIET_MOVE,
    }
}

#[test]
fn fen_parity_castling_variants() {
    let fens = [
        // pieces-only startpos with different castling rights
        "rn1qkbnr/pppbpppp/8/3p4/3P4/5N2/PPP1PPPP/RNBQKB1R w KQkq - 2 3",
        "rn1qkbnr/pppbpppp/8/3p4/3P4/5N2/PPP1PPPP/RNBQKB1R w K - 2 3",
        "rn1qkbnr/pppbpppp/8/3p4/3P4/5N2/PPP1PPPP/RNBQKB1R w Q - 2 3",
        "rn1qkbnr/pppbpppp/8/3p4/3P4/5N2/PPP1PPPP/RNBQKB1R w k - 2 3",
        "rn1qkbnr/pppbpppp/8/3p4/3P4/5N2/PPP1PPPP/RNBQKB1R w q - 2 3",
        "rn1qkbnr/pppbpppp/8/3p4/3P4/5N2/PPP1PPPP/RNBQKB1R w - - 2 3",
    ];
    for fen in fens {
        let b = Board::from_str(fen).expect("FEN parse");
        assert_eq!(
            b.zobrist,
            b.compute_zobrist_full(),
            "FEN parity failed: {fen}"
        );
    }
}

#[test]
fn castling_rights_clear_on_white_rook_moves() {
    // Startpos with full rights
    let mut b = Board::new();
    assert_eq!(
        b.castling_rights & (CASTLE_WK | CASTLE_WQ),
        CASTLE_WK | CASTLE_WQ
    );

    // a1 -> a2 clears WQ
    let u1 = make_move_basic(&mut b, mv(0, 8, Piece::Rook));
    assert_eq!(
        b.castling_rights & CASTLE_WQ,
        0,
        "WQ should be cleared after a1 rook moves"
    );
    assert_eq!(b.zobrist, b.compute_zobrist_full(), "parity after a1->a2");
    undo_move_basic(&mut b, u1);
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after undo a1->a2"
    );

    // h1 -> h2 clears WK
    let u2 = make_move_basic(&mut b, mv(7, 15, Piece::Rook));
    assert_eq!(
        b.castling_rights & CASTLE_WK,
        0,
        "WK should be cleared after h1 rook moves"
    );
    assert_eq!(b.zobrist, b.compute_zobrist_full(), "parity after h1->h2");
    undo_move_basic(&mut b, u2);
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after undo h1->h2"
    );
}

#[test]
fn castling_rights_clear_on_black_rook_moves() {
    let mut b = Board::new();
    // play a dummy full move so it's Black to move and we can move a8/h8 directly
    let u = make_move_basic(&mut b, mv(12, 20, Piece::Pawn)); // white: a2->a3
    undo_move_basic(&mut b, u); // optional; alternatively, push a white move then a black move properly

    // Move a8->a7 clears BQ
    // To get black move legally, do a minimal sequence:
    let _u_w = make_move_basic(&mut b, mv(12, 20, Piece::Pawn)); // a2->a3 (white)
    let u_b1 = make_move_basic(&mut b, mv(56, 48, Piece::Rook)); // a8->a7 (black)
    assert_eq!(
        b.castling_rights & CASTLE_BQ,
        0,
        "BQ should be cleared after a8 rook moves"
    );
    assert_eq!(b.zobrist, b.compute_zobrist_full(), "parity after a8->a7");
    undo_move_basic(&mut b, u_b1);
    undo_move_basic(&mut b, _u_w);
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after undo a8->a7"
    );

    // h8->h7 clears BK
    let _u_w2 = make_move_basic(&mut b, mv(13, 21, Piece::Pawn)); // b2->b3
    let u_b2 = make_move_basic(&mut b, mv(63, 55, Piece::Rook)); // h8->h7
    assert_eq!(
        b.castling_rights & CASTLE_BK,
        0,
        "BK should be cleared after h8 rook moves"
    );
    assert_eq!(b.zobrist, b.compute_zobrist_full(), "parity after h8->h7");
    undo_move_basic(&mut b, u_b2);
    undo_move_basic(&mut b, _u_w2);
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after undo h8->h7"
    );
}

#[test]
fn castling_rights_clear_on_king_move() {
    let mut b = Board::new();
    // e1 -> e2 clears both WK|WQ
    let u = make_move_basic(&mut b, mv(4, 12, Piece::King));
    assert_eq!(
        b.castling_rights & (CASTLE_WK | CASTLE_WQ),
        0,
        "white king moved → clear WK|WQ"
    );
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after king move"
    );
    undo_move_basic(&mut b, u);
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after undo king move"
    );
}

#[test]
fn castling_rights_clear_on_corner_rook_capture() {
    // Place a black piece that can capture the white rook on a1
    // let b = Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/1PPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    // Black bishop from c8 (58) to a6 (40) is not relevant; instead, we’ll just do a simple white move then black captures a1.
    // For simplicity: white plays a2->a3, black plays b4xa3? That's messy.
    // Easier: put a black rook on a2 in FEN and capture a1.
    let mut b =
        Board::from_str("rnbqkbnr/pppppppp/8/8/8/r7/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    // White passes with a light move
    let u_w = make_move_basic(&mut b, mv(12, 20, Piece::Pawn)); // a2->a3
    // Black: rook a3->a1 capturing rook on a1
    let u_b = make_move_basic(&mut b, mv(40, 0, Piece::Rook));
    assert_eq!(
        b.castling_rights & CASTLE_WQ,
        0,
        "captured rook on a1 → clear WQ"
    );
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after capture a1"
    );
    undo_move_basic(&mut b, u_b);
    undo_move_basic(&mut b, u_w);
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after undo capture a1"
    );
}

#[test]
fn castling_rights_do_not_return_when_rook_moves_back() {
    let mut b = Board::new();

    // a1->a2 clears WQ
    let u1 = make_move_basic(&mut b, mv(0, 8, Piece::Rook));
    assert_eq!(b.castling_rights & CASTLE_WQ, 0);
    assert_eq!(b.zobrist, b.compute_zobrist_full());

    // ... some moves to allow white to move again ...
    let u2 = make_move_basic(&mut b, mv(12, 20, Piece::Pawn)); // Black reply later, but keep parity checks simple:
    undo_move_basic(&mut b, u2); // just to balance if needed

    // a2->a1 (rights must stay cleared)
    let u3 = make_move_basic(&mut b, mv(8, 0, Piece::Rook));
    assert_eq!(b.castling_rights & CASTLE_WQ, 0, "rights must not return");
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after rook returns"
    );
    undo_move_basic(&mut b, u3);
    undo_move_basic(&mut b, u1);
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after full undo"
    );
}

#[test]
fn relaxed_ep_hashing_edges_white_double_push() {
    // White to move, black pawn on b4 -> after a2->a4, EP=a3 is capturable by Black
    let fen = "8/8/8/8/1p6/8/P6P/8 w - - 0 1"; // white pawns a2,h2; black pawn b4
    let mut b = Board::from_str(fen).unwrap();

    // a2->a4 (12->28)
    let u = make_move_basic(&mut b, mv(8, 24, Piece::Pawn));
    assert_eq!(b.en_passant.unwrap().index(), 16, "EP square should be a3");
    // EP should contribute to hash (b pawn on b4 can capture a3)
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after EP set (capturable)"
    );
    undo_move_basic(&mut b, u);
    assert_eq!(b.zobrist, b.compute_zobrist_full(), "parity after undo");
}

#[test]
fn relaxed_ep_hashing_edges_white_double_push_not_capturable() {
    // Same but without black pawn on b4 → EP should not contribute
    let fen = "8/8/8/8/8/8/P6P/8 w - - 0 1";
    let mut b = Board::from_str(fen).unwrap();

    let u = make_move_basic(&mut b, mv(8, 24, Piece::Pawn));
    assert_eq!(b.en_passant.unwrap().index(), 16, "EP square should be a3");
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity even when EP not capturable"
    );
    undo_move_basic(&mut b, u);
    assert_eq!(b.zobrist, b.compute_zobrist_full());
}

#[test]
fn relaxed_ep_hashing_edges_black_double_push() {
    // Black to move with white pawn on g4: after h7->h5, EP=h6 should be capturable by White
    let fen = "8/7p/8/6P1/8/8/8/8 b - - 0 1"; // black pawn h7, white pawn g5
    let mut b = Board::from_str(fen).unwrap();

    // h7->h5 (55->39) indices: h7=55, h5=39
    let u = make_move_basic(&mut b, mv(55, 39, Piece::Pawn));
    // EP should be h6 (index 47)
    assert_eq!(b.en_passant.unwrap().index(), 47, "EP square should be h6");
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity after EP set (capturable)"
    );
    undo_move_basic(&mut b, u);
    assert_eq!(b.zobrist, b.compute_zobrist_full());
}

#[test]
fn relaxed_ep_hashing_edges_black_double_push_not_capturable() {
    // Black double-push with no white pawn that can capture EP → EP not hashed
    let fen = "8/7p/8/8/8/8/8/8 b - - 0 1";
    let mut b = Board::from_str(fen).unwrap();

    let u = make_move_basic(&mut b, mv(55, 39, Piece::Pawn)); // h7->h5
    assert_eq!(b.en_passant.unwrap().index(), 47, "EP square should be h6");
    assert_eq!(
        b.zobrist,
        b.compute_zobrist_full(),
        "parity with non-capturable EP"
    );
    undo_move_basic(&mut b, u);
    assert_eq!(b.zobrist, b.compute_zobrist_full());
}

// White quiet promotion: a7 -> a8=Q
#[test]
fn zobrist_promo_white_quiet_q() {
    // 8/P7/8/8/8/8/8/4k2K w - - 0 1
    let fen = "8/P7/8/8/8/8/8/4k2K w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).expect("valid FEN");

    // a7 = rank 6, file 0 => 6*8+0 = 48
    let from = Square::from_index(48);
    // a8 = rank 7, file 0 => 56
    let to = Square::from_index(56);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION,
    };

    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(
        board.halfmove_clock, 0,
        "promotion must reset halfmove clock"
    );
    assert!(board.en_passant.is_none(), "promotion must not create EP");

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black quiet promotion: a2 -> a1=Q
#[test]
fn zobrist_promo_black_quiet_q() {
    // 4k3/8/8/8/8/8/p7/7K b - - 0 1
    let fen = "4k3/8/8/8/8/8/p7/7K b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).expect("valid FEN");

    // a2 = rank 1, file 0 => 8
    let from = Square::from_index(8);
    // a1 = rank 0, file 0 => 0
    let to = Square::from_index(0);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION,
    };

    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// White capture promotion: g7xh8=Q (captures a black rook on h8)
#[test]
fn zobrist_promo_white_capture_h8_q() {
    // k6r/6P1/8/8/8/8/8/4K3 w - - 0 1
    let fen = "k6r/6P1/8/8/8/8/8/4K3 w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).expect("valid FEN");

    // g7 = rank 6, file 6 => 54
    let from = Square::from_index(54);
    // h8 = rank 7, file 7 => 63
    let to = Square::from_index(63);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION_CAPTURE,
    };

    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black capture promotion: g2xh1=Q (captures a white rook on h1)
#[test]
fn zobrist_promo_black_capture_h1_q() {
    // 4k3/8/8/8/8/8/6p1/K6R b - - 0 1
    let fen = "4k3/8/8/8/8/8/6p1/K6R b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).expect("valid FEN");

    // g2 = rank 1, file 6 => 14
    let from = Square::from_index(14);
    // h1 = rank 0, file 7 => 7
    let to = Square::from_index(7);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION_CAPTURE,
    };

    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// White captures h8 rook with g7h8=Q; black initially has 'k' → should clear to '-'
#[test]
fn zobrist_promo_white_capture_h8_clears_k_rights() {
    // 4k2r/6P1/8/8/8/8/8/4K3 w k - 0 1
    let fen = "4k2r/6P1/8/8/8/8/8/4K3 w k - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).expect("valid FEN");

    // g7 -> h8
    let from = Square::from_index(54); // g7
    let to = Square::from_index(63); // h8

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    let old_rights = board.castling_rights;
    assert_ne!(
        old_rights & CASTLE_BK,
        0,
        "precondition: black has 'k' right"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION_CAPTURE,
    };

    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());
    assert_eq!(
        board.castling_rights & CASTLE_BK,
        0,
        "k right must be cleared"
    );

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black captures a1 rook with b2a1=Q; white initially has 'Q' → should clear to '-'
#[test]
fn zobrist_promo_black_capture_a1_clears_q_rights() {
    // Position: White rook on a1, white king on e1 (so 'Q' is plausibly set),
    // Black pawn on b2 ready to capture a1 and promote, Black to move, castling rights = 'Q'
    // FEN ranks (8→1):
    // 8: 4k3
    // 7: 8
    // 6: 8
    // 5: 8
    // 4: 8
    // 3: 8
    // 2: 1p6   (b2 black pawn)
    // 1: R3K3  (a1 white rook, e1 white king)
    let fen = "4k3/8/8/8/8/8/1p6/R3K3 b Q - 0 1";

    let mut board = Board::new();
    board.set_fen(fen).expect("valid FEN");

    // b2 -> a1 = capture + promotion
    // b2 = file 1, rank 1 => 1 + 1*8 = 9
    // a1 = file 0, rank 0 => 0 + 0*8 = 0
    let from = Square::from_index(9);
    let to = Square::from_index(0);

    // Pre-move: parity + precondition
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    let old_rights = board.castling_rights;
    assert_ne!(
        old_rights & CASTLE_WQ,
        0,
        "precondition: white has 'Q' right"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION_CAPTURE,
    };

    // Make
    let undo = make_move_basic(&mut board, mv);

    // Post-move: parity + invariants + rights cleared
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(
        board.halfmove_clock, 0,
        "promotion must reset halfmove clock"
    );
    assert!(board.en_passant.is_none(), "promotion must not create EP");
    assert_eq!(
        board.castling_rights & CASTLE_WQ,
        0,
        "Q right must be cleared"
    );

    // Undo
    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// White quiet promotion to Rook: a7 -> a8=R
#[test]
fn zobrist_promo_white_quiet_r() {
    let fen = "8/P7/8/8/8/8/8/4k2K w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(48); // a7
    let to = Square::from_index(56); // a8
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let old_rights = board.castling_rights;

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Rook),
        flags: PROMOTION,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());
    assert_eq!(
        board.castling_rights, old_rights,
        "quiet promo must not change rights"
    );

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// White quiet promotion to Bishop: a7 -> a8=B
#[test]
fn zobrist_promo_white_quiet_b() {
    let fen = "8/P7/8/8/8/8/8/4k2K w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(48);
    let to = Square::from_index(56);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    let old_rights = board.castling_rights;

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Bishop),
        flags: PROMOTION,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());
    assert_eq!(board.castling_rights, old_rights);

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// White quiet promotion to Knight: a7 -> a8=N
#[test]
fn zobrist_promo_white_quiet_n() {
    let fen = "8/P7/8/8/8/8/8/4k2K w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(48);
    let to = Square::from_index(56);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    let old_rights = board.castling_rights;

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Knight),
        flags: PROMOTION,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());
    assert_eq!(board.castling_rights, old_rights);

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// -----------------------
// BLACK QUIET PROMOTIONS
// -----------------------

// Black quiet promotion to Rook: a2 -> a1=R
#[test]
fn zobrist_promo_black_quiet_r() {
    let fen = "4k3/8/8/8/8/8/p7/7K b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(8); // a2
    let to = Square::from_index(0); // a1
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    let old_rights = board.castling_rights;

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Rook),
        flags: PROMOTION,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());
    assert_eq!(board.castling_rights, old_rights);

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black quiet promotion to Bishop: a2 -> a1=B
#[test]
fn zobrist_promo_black_quiet_b() {
    let fen = "4k3/8/8/8/8/8/p7/7K b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(8);
    let to = Square::from_index(0);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    let old_rights = board.castling_rights;

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Bishop),
        flags: PROMOTION,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());
    assert_eq!(board.castling_rights, old_rights);

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black quiet promotion to Knight: a2 -> a1=N
#[test]
fn zobrist_promo_black_quiet_n() {
    let fen = "4k3/8/8/8/8/8/p7/7K b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(8);
    let to = Square::from_index(0);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    let old_rights = board.castling_rights;

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Knight),
        flags: PROMOTION,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());
    assert_eq!(board.castling_rights, old_rights);

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// -----------------------------------
// WHITE CAPTURE PROMOTIONS (corner h8)
// -----------------------------------

// White capture promotion to Rook: g7xh8=R
#[test]
fn zobrist_promo_white_capture_h8_r() {
    let fen = "k6r/6P1/8/8/8/8/8/4K3 w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(54); // g7
    let to = Square::from_index(63); // h8
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Rook),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// White capture promotion to Bishop: g7xh8=B
#[test]
fn zobrist_promo_white_capture_h8_b() {
    let fen = "k6r/6P1/8/8/8/8/8/4K3 w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(54);
    let to = Square::from_index(63);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Bishop),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// White capture promotion to Knight: g7xh8=N
#[test]
fn zobrist_promo_white_capture_h8_n() {
    let fen = "k6r/6P1/8/8/8/8/8/4K3 w - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(54);
    let to = Square::from_index(63);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Knight),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// -----------------------------------
// BLACK CAPTURE PROMOTIONS (corner h1)
// -----------------------------------

// Black capture promotion to Rook: g2xh1=R
#[test]
fn zobrist_promo_black_capture_h1_r() {
    let fen = "4k3/8/8/8/8/8/6p1/K6R b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(14); // g2
    let to = Square::from_index(7); // h1
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Rook),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black capture promotion to Bishop: g2xh1=B
#[test]
fn zobrist_promo_black_capture_h1_b() {
    let fen = "4k3/8/8/8/8/8/6p1/K6R b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(14);
    let to = Square::from_index(7);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Bishop),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black capture promotion to Knight: g2xh1=N
#[test]
fn zobrist_promo_black_capture_h1_n() {
    let fen = "4k3/8/8/8/8/8/6p1/K6R b - - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(14);
    let to = Square::from_index(7);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Knight),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// ------------------------------------------------------
// COMPLETE THE CORNER RIGHTS-CLEARING SYMMETRY (2 tests)
// ------------------------------------------------------

// White captures a8 rook with b7a8=Q; black initially has 'q' → should clear to '-'
#[test]
fn zobrist_promo_white_capture_a8_clears_q_rights() {
    // Black: rook a8, king e8; White: pawn b7 to capture a8; rights = 'q'
    let fen = "r3k3/1P6/8/8/8/8/8/4K3 w q - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(57); // b7
    let to = Square::from_index(56); // a8
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    assert_ne!(board.castling_rights & CASTLE_BQ, 0, "pre: black has 'q'");

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.castling_rights & CASTLE_BQ, 0, "'q' must be cleared");
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

// Black captures h1 rook with g2h1=Q; white initially has 'K' → should clear to '-'
#[test]
fn zobrist_promo_black_capture_h1_clears_k_rights() {
    // White: rook h1, king e1; Black: pawn g2 to capture h1; rights = 'K'
    let fen = "4k3/8/8/8/8/8/6p1/4K2R b K - 0 1";
    let mut board = Board::new();
    board.set_fen(fen).unwrap();

    let from = Square::from_index(14); // g2
    let to = Square::from_index(7); // h1
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "pre-move parity"
    );
    assert_ne!(board.castling_rights & CASTLE_WK, 0, "pre: white has 'K'");

    let mv = Move {
        from,
        to,
        piece: Piece::Pawn,
        promotion: Some(Piece::Queen),
        flags: PROMOTION_CAPTURE,
    };
    let undo = make_move_basic(&mut board, mv);

    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-move parity"
    );
    assert_eq!(board.castling_rights & CASTLE_WK, 0, "'K' must be cleared");
    assert_eq!(board.halfmove_clock, 0);
    assert!(board.en_passant.is_none());

    undo_move_basic(&mut board, undo);
    assert_eq!(
        board.zobrist,
        board.compute_zobrist_full(),
        "post-undo parity"
    );
}

#[test]
fn repetition_two_cycle_bare_kings_is_not_threefold() {
    let mut b = Board::new();
    b.set_fen("8/8/8/8/8/8/4k3/4K3 w - - 0 1").unwrap();
    assert_eq!(b.repetition_count(), 1);
    assert!(!b.is_threefold());

    // e1=4, d1=3, e2=12, d2=11
    let _u1 = make_move_basic(&mut b, mv_king(4, 3)); // W: Ke1-d1
    let _u2 = make_move_basic(&mut b, mv_king(12, 11)); // B: Ke2-d2
    let _u3 = make_move_basic(&mut b, mv_king(3, 4)); // W: Kd1-e1
    let _u4 = make_move_basic(&mut b, mv_king(11, 12)); // B: Kd2-e2

    assert_eq!(b.repetition_count(), 2, "two-cycle should yield count=2");
    assert!(!b.is_threefold(), "two-cycle is not threefold");
}

#[test]
fn repetition_threefold_bare_kings() {
    let mut b = Board::new();
    b.set_fen("8/8/8/8/8/8/4k3/4K3 w - - 0 1").unwrap();

    // 1st cycle
    let _ = make_move_basic(&mut b, mv_king(4, 3));
    let _ = make_move_basic(&mut b, mv_king(12, 11));
    let _ = make_move_basic(&mut b, mv_king(3, 4));
    let _ = make_move_basic(&mut b, mv_king(11, 12));
    assert_eq!(b.repetition_count(), 2);

    // 2nd cycle (brings count to 3+)
    let _ = make_move_basic(&mut b, mv_king(4, 3));
    let _ = make_move_basic(&mut b, mv_king(12, 11));
    let _ = make_move_basic(&mut b, mv_king(3, 4));
    let _ = make_move_basic(&mut b, mv_king(11, 12));

    assert!(b.repetition_count() >= 3);
    assert!(b.is_threefold());
}

#[test]
fn repetition_resets_after_pawn_push() {
    let mut b = Board::new();
    b.set_fen("8/8/8/8/8/8/3Pk3/4K3 w - - 0 1").unwrap();
    // White king e1=4, Black king e2=12, white pawn d2=11

    // reversible two-cycle
    let _ = make_move_basic(&mut b, mv_king(4, 3)); // W: Ke1-d1
    let _ = make_move_basic(&mut b, mv_king(12, 13)); // B: Ke2-d2
    let _ = make_move_basic(&mut b, mv_king(3, 4)); // W: Kd1-e1
    let _ = make_move_basic(&mut b, mv_king(13, 12)); // B: Kd2-e2
    assert_eq!(b.repetition_count(), 2);

    // irreversible: d2 (11) -> d3 (19)
    let _ = make_move_basic(&mut b, mv_pawn(11, 19));
    assert_eq!(
        b.history.len(),
        1,
        "history should truncate on irreversible"
    );
    assert_eq!(b.repetition_count(), 1);
    assert!(!b.is_threefold());
}

#[test]
fn repetition_ep_relaxed_policy_affects_equality() {
    let mut b = Board::new();
    b.set_fen("8/8/8/3pP3/8/8/4k3/4K3 w - d6 0 1").unwrap();
    let start_hash = b.zobrist;
    assert_eq!(b.repetition_count(), 1);

    // Do a reversible king pair: Ke1-d1, Ke2-d2
    let _ = make_move_basic(&mut b, mv_king(4, 3));
    let _ = make_move_basic(&mut b, mv_king(12, 11));
    // And back: Kd1-e1, Kd2-e2
    let _ = make_move_basic(&mut b, mv_king(3, 4));
    let _ = make_move_basic(&mut b, mv_king(11, 12));

    // We returned to same pieces/side, but EP got cleared on the first make.
    assert_ne!(
        b.zobrist, start_hash,
        "EP clearing should change the key under relaxed policy"
    );
    assert_eq!(
        b.repetition_count(),
        1,
        "no additional repetition since EP difference prevents equality"
    );
}

#[test]
fn repetition_promotion_truncates_and_restores_on_undo() {
    let mut b = Board::new();
    b.set_fen("8/P7/8/8/8/8/8/4k2K w - - 0 1").unwrap();
    let before_len = b.history.len();

    let u = make_move_basic(&mut b, mv_promo(48, 56, Piece::Queen)); // a7->a8=Q
    assert_eq!(b.history.len(), 1, "promotion should truncate history");
    assert_eq!(b.repetition_count(), 1);

    undo_move_basic(&mut b, u);
    assert_eq!(
        b.history.len(),
        before_len,
        "undo should restore prior history"
    );
}

#[test]
fn repetition_capture_truncates_and_restores_on_undo() {
    let mut b = Board::new();
    b.set_fen("k6r/6P1/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    let before_len = b.history.len();

    let u = make_move_basic(&mut b, mv_promo_capture(54, 63, Piece::Queen)); // g7xh8=Q
    assert_eq!(
        b.history.len(),
        1,
        "capture+promotion should truncate history"
    );
    assert_eq!(b.repetition_count(), 1);

    undo_move_basic(&mut b, u);
    assert_eq!(
        b.history.len(),
        before_len,
        "undo should restore prior history"
    );
}

#[test]
fn repetition_side_to_move_matters() {
    let mut b = Board::new();
    b.set_fen("8/8/8/8/8/8/4k3/4K3 w - - 0 1").unwrap();
    let start_hash = b.zobrist;

    let _ = make_move_basic(&mut b, mv_king(4, 3)); // W moves only
    assert_ne!(
        b.zobrist, start_hash,
        "side-to-move toggled; key must differ"
    );
    assert_eq!(
        b.repetition_count(),
        1,
        "different STM => not the same repetition"
    );
}

use vantage::moves::execute::generate_legal;
use vantage::moves::magic::loader::load_magic_tables;

#[test]
fn test_zobrist_consistency_after_make_undo() {
    // Test that make_move + undo_move preserves zobrist hash
    let mut board =
        Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let tables = load_magic_tables();

    let original_hash = board.zobrist;
    println!("Original hash: {:016x}", original_hash);

    // Generate legal moves
    let mut moves = Vec::new();
    let mut pseudo_scratch = Vec::new();
    generate_legal(&mut board, &tables, &mut moves, &mut pseudo_scratch);

    // Test each move
    for (i, &mv) in moves.iter().enumerate() {
        let hash_before = board.zobrist;

        let undo = make_move_basic(&mut board, mv);
        let hash_after_make = board.zobrist;

        undo_move_basic(&mut board, undo);
        let hash_after_undo = board.zobrist;

        assert_eq!(
            hash_before, hash_after_undo,
            "Move {}: Zobrist mismatch after make/undo! Before: {:016x}, After: {:016x}, Move: {:?}",
            i, hash_before, hash_after_undo, mv
        );

        assert_ne!(
            hash_before, hash_after_make,
            "Move {}: Zobrist didn't change after make_move! Move: {:?}",
            i, mv
        );
    }

    println!(
        "✓ All {} moves preserved zobrist hash after make/undo",
        moves.len()
    );
}

// #[test]
// fn test_zobrist_consistency_during_search() {
//     // Test zobrist during actual search
//     let mut board =
//         Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
//     let tables = load_magic_tables();

//     let original_hash = board.zobrist;
//     let original_full_hash = board.compute_zobrist_full();

//     println!("Before search:");
//     println!("  Incremental: {:016x}", original_hash);
//     println!("  Full:        {:016x}", original_full_hash);

//     assert_eq!(
//         original_hash, original_full_hash,
//         "Hash mismatch before search!"
//     );

//     // Do a shallow search
//     use vantage::search::search::search_iterative_deepening;
//     let (_score, _mv) = search_iterative_deepening(&mut board, &tables, 3, None);

//     let after_hash = board.zobrist;
//     let after_full_hash = board.compute_zobrist_full();

//     println!("\nAfter search:");
//     println!("  Incremental: {:016x}", after_hash);
//     println!("  Full:        {:016x}", after_full_hash);

//     assert_eq!(
//         original_hash, after_hash,
//         "Zobrist changed during search! Before: {:016x}, After: {:016x}",
//         original_hash, after_hash
//     );

//     assert_eq!(
//         after_hash, after_full_hash,
//         "Hash mismatch after search! Incremental: {:016x}, Full: {:016x}",
//         after_hash, after_full_hash
//     );

//     println!("✓ Zobrist consistent before and after search");
// }

// #[test]
// fn test_zobrist_null_move_consistency() {
//     // Test that null move pruning preserves zobrist
//     let mut board =
//         Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

//     let original_hash = board.zobrist;
//     let original_side = board.side_to_move;

//     println!(
//         "Original hash: {:016x}, side: {:?}",
//         original_hash, original_side
//     );

//     // Simulate null move
//     use vantage::hash::zobrist::zobrist_keys;
//     let keys = zobrist_keys();

//     board.side_to_move = original_side.opposite();
//     board.zobrist ^= keys.side_to_move;

//     let after_null = board.zobrist;
//     println!(
//         "After null:    {:016x}, side: {:?}",
//         after_null, board.side_to_move
//     );

//     // Undo null move
//     board.side_to_move = original_side;
//     board.zobrist ^= keys.side_to_move;

//     let after_undo = board.zobrist;
//     println!(
//         "After undo:    {:016x}, side: {:?}",
//         after_undo, board.side_to_move
//     );

//     assert_eq!(
//         original_hash, after_undo,
//         "Null move didn't restore zobrist! Before: {:016x}, After: {:016x}",
//         original_hash, after_undo
//     );

//     println!("✓ Null move zobrist handling is correct");
// }

// #[test]
// fn test_zobrist_deep_search() {
//     // Test with deeper search on a tactical position
//     let mut board =
//         Board::from_str("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
//             .unwrap();
//     let tables = load_magic_tables();

//     let hash_before = board.zobrist;
//     let full_before = board.compute_zobrist_full();

//     println!("Before deep search (depth 4):");
//     println!("  Incremental: {:016x}", hash_before);
//     println!("  Full:        {:016x}", full_before);

//     assert_eq!(hash_before, full_before, "Hash mismatch before search!");

//     use vantage::search::search::search_iterative_deepening;
//     let (_score, _mv) = search_iterative_deepening(&mut board, &tables, 4, None);

//     let hash_after = board.zobrist;
//     let full_after = board.compute_zobrist_full();

//     println!("\nAfter deep search:");
//     println!("  Incremental: {:016x}", hash_after);
//     println!("  Full:        {:016x}", full_after);

//     assert_eq!(
//         hash_before, hash_after,
//         "Board state changed during search!"
//     );

//     assert_eq!(
//         hash_after, full_after,
//         "Hash consistency broken! Inc: {:016x}, Full: {:016x}",
//         hash_after, full_after
//     );
// }

// #[test]
// fn test_zobrist_with_captures() {
//     // Test zobrist with capture moves specifically
//     let mut board =
//         Board::from_str("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2").unwrap();
//     let tables = load_magic_tables();

//     let original_hash = board.zobrist;

//     // Generate moves and find a capture
//     let mut moves = Vec::new();
//     let mut pseudo_scratch = Vec::new();
//     generate_legal(&mut board, &tables, &mut moves, &mut pseudo_scratch);

//     let capture = moves.iter().find(|mv| mv.is_capture());

//     if let Some(&capture_move) = capture {
//         println!("Testing capture move: {:?}", capture_move);

//         let undo = make_move_basic(&mut board, capture_move);

//         undo_move_basic(&mut board, undo);
//         let hash_restored = board.zobrist;

//         assert_eq!(
//             original_hash, hash_restored,
//             "Capture move didn't restore zobrist!"
//         );

//         println!("✓ Capture move zobrist handling correct");
//     } else {
//         println!("No capture moves available in this position");
//     }
// }

// #[test]
// fn test_zobrist_with_castling() {
//     // Test zobrist with castling
//     let mut board = Board::from_str("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
//     let tables = load_magic_tables();

//     // Generate moves and find castling
//     let mut moves = Vec::new();
//     let mut pseudo_scratch = Vec::new();
//     generate_legal(&mut board, &tables, &mut moves, &mut pseudo_scratch);

//     // Castling moves should have special flags
//     for &mv in moves.iter() {
//         let hash_before = board.zobrist;

//         let undo = make_move_basic(&mut board, mv);
//         undo_move_basic(&mut board, undo);

//         let hash_after = board.zobrist;

//         assert_eq!(
//             hash_before, hash_after,
//             "Move {:?} didn't restore zobrist! Before: {:016x}, After: {:016x}",
//             mv, hash_before, hash_after
//         );
//     }

//     println!("✓ All moves (including castling) preserve zobrist");
// }

// #[test]
// fn test_search_iterative_deepening_zobrist() {
//     // The actual failing case - test iterative deepening specifically
//     let mut board =
//         Board::from_str("r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1")
//             .unwrap();
//     let tables = load_magic_tables();

//     println!("\nTesting iterative deepening on scholar's mate position");

//     for depth in 1..=5 {
//         let hash_before = board.zobrist;
//         let full_before = board.compute_zobrist_full();

//         assert_eq!(
//             hash_before, full_before,
//             "Hash mismatch before depth {} search",
//             depth
//         );

//         use vantage::search::search::search_iterative_deepening;
//         let (_score, _mv) = search_iterative_deepening(&mut board, &tables, depth, None);

//         let hash_after = board.zobrist;
//         let full_after = board.compute_zobrist_full();

//         assert_eq!(
//             hash_before, hash_after,
//             "Depth {}: Board changed during search! Before: {:016x}, After: {:016x}",
//             depth, hash_before, hash_after
//         );

//         assert_eq!(
//             hash_after, full_after,
//             "Depth {}: Hash mismatch after search! Inc: {:016x}, Full: {:016x}",
//             depth, hash_after, full_after
//         );

//         println!("  Depth {}: ✓ zobrist consistent", depth);
//     }
// }
