use vantage::board::{Board, Color, Piece};
use vantage::moves::execute::generate_legal;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::movegen::generate_bishop_moves;
use vantage::moves::movegen::generate_king_moves;
use vantage::moves::movegen::generate_knight_moves;
use vantage::moves::movegen::generate_pawn_moves;
use vantage::moves::movegen::generate_queen_moves;
use vantage::moves::movegen::generate_rook_moves;
use vantage::moves::types::Move;
use vantage::square::Square;
use std::str::FromStr;

/// Set the bitboard for (color, piece) to exactly `mask`, then recompute occupancies.
fn set_piece_mask(board: &mut Board, color: Color, piece: Piece, mask: u64) {
    // 1) Overwrite that piece’s bitboard with the given mask:
    board.piece_bb[color as usize][piece as usize] = mask;

    // 2) Recompute White’s occupancy by OR‐ing together all White piece bitboards:
    let mut white_occ = 0u64;
    for &wbb in &board.piece_bb[Color::White as usize] {
        white_occ |= wbb;
    }
    board.occ_white = white_occ;

    // 3) Recompute Black’s occupancy in the same way:
    let mut black_occ = 0u64;
    for &bbb in &board.piece_bb[Color::Black as usize] {
        black_occ |= bbb;
    }
    board.occ_black = black_occ;

    // 4) Finally, global occupancy is the union of both:
    board.occ_all = board.occ_white | board.occ_black;
}

#[test]
fn knight_moves_from_d4() {
    let mut board = Board::new_empty();
    let d4 = 27; // square index 27

    // Place a knight at d4 for White
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_knight_moves(&board, &mut moves);

    let expected_dests = [
        10, 12, 17, 21, 33, 37, 42, 44, // valid knight jumps from d4
    ];

    assert_eq!(moves.len(), expected_dests.len());

    for dest in expected_dests {
        let found = moves.iter().any(|m| m.to.index() == dest);
        assert!(found, "Expected move to square {} not found", dest);
    }
}

#[test]
fn knight_moves_from_corner_a1() {
    let mut board = Board::new_empty();
    let a1 = 0;
    let piece_mask = 1u64 << a1;
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_knight_moves(&board, &mut moves);

    let expected_dests = [10, 17];
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        assert!(moves.iter().any(|m| m.to.index() == to));
    }
}

#[test]
fn knight_blocked_by_friendly_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let mut piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);

    piece_mask = 1 << 44;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_knight_moves(&board, &mut moves);

    // Knight has 8 targets, 1 is blocked → expect 7 moves
    assert_eq!(moves.len(), 7);
    assert!(!moves.iter().any(|m| m.to.index() == 44));
}

#[test]
fn knight_captures_enemy_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);

    let piece_mask = 1 << 33;
    set_piece_mask(&mut board, Color::Black, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_knight_moves(&board, &mut moves);

    let mv = moves
        .iter()
        .find(|m| m.to.index() == 33)
        .expect("Missing capture");
    assert!(mv.is_capture());
}

#[test]
fn multiple_knights_generate_moves() {
    let mut board = Board::new_empty();
    let piece_mask = (1 << 1) | (1 << 18);
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_knight_moves(&board, &mut moves);
    // b1 has 2 moves, c3 has 8 → total 10
    assert_eq!(moves.len(), 9);
    assert!(moves.iter().any(|m| m.from.index() == 1));
    assert!(moves.iter().any(|m| m.from.index() == 18));
}

#[test]
fn no_knights_yields_no_moves() {
    let mut board = Board::new_empty();
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_knight_moves(&board, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn bishop_moves_from_d4_empty_board() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Bishop, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_bishop_moves(&board, &magic_tables.bishop, &mut moves);

    let expected_dests = [18, 20, 9, 0, 36, 45, 54, 63, 34, 41, 48, 6, 13];
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        assert!(
            moves.iter().any(|m| m.to.index() == to),
            "Missing move to square {}",
            to
        );
    }
}

#[test]
fn bishop_moves_from_corner_a1() {
    let mut board = Board::new_empty();
    let a1 = 0;
    let piece_mask = 1u64 << a1;
    set_piece_mask(&mut board, Color::White, Piece::Bishop, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_bishop_moves(&board, &magic_tables.bishop, &mut moves);

    let expected_dests = [9, 18, 27, 36, 45, 54, 63];
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        assert!(moves.iter().any(|m| m.to.index() == to));
    }
}

#[test]
fn bishop_blocked_by_friendly_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Bishop, piece_mask);

    let piece_mask = 1 << 36;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_bishop_moves(&board, &magic_tables.bishop, &mut moves);

    // d4 has 12 moves on empty board, blocked at 36 removes one (plus all behind it)
    let expected_removed = 36;
    assert!(!moves.iter().any(|m| m.to.index() == expected_removed));
    assert!(moves.len() < 12);
}

#[test]
fn bishop_captures_enemy_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Bishop, piece_mask);

    let piece_mask = 1 << 36;
    set_piece_mask(&mut board, Color::Black, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_bishop_moves(&board, &magic_tables.bishop, &mut moves);

    let mv = moves
        .iter()
        .find(|m| m.to.index() == 36)
        .expect("Expected capture move to 36");
    assert!(mv.is_capture());
}

#[test]
fn multiple_bishops_generate_moves() {
    let mut board = Board::new_empty();
    let piece_mask = (1 << 2) | (1 << 20);
    set_piece_mask(&mut board, Color::White, Piece::Bishop, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_bishop_moves(&board, &magic_tables.bishop, &mut moves);

    assert!(moves.iter().any(|m| m.from.index() == 2));
    assert!(moves.iter().any(|m| m.from.index() == 20));
    assert!(!moves.is_empty());
}

#[test]
fn no_bishops_yields_no_moves() {
    let mut board = Board::new_empty();
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_bishop_moves(&board, &magic_tables.bishop, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn rook_moves_from_d4_empty_board() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_rook_moves(&board, &magic_tables.rook, &mut moves);

    // Horizontal (rank 4): c4 (26), b4 (25), a4 (24), e4 (28), f4 (29), g4 (30), h4 (31)
    // Vertical (file d): d5 (35), d6 (43), d7 (51), d8 (59), d3 (19), d2 (11), d1 (3)
    let expected_dests = [
        26, 25, 24, 28, 29, 30, 31, // rank
        35, 43, 51, 59, 19, 11, 3, // file
    ];

    assert_eq!(moves.len(), expected_dests.len());
    for &to in &expected_dests {
        assert!(
            moves.iter().any(|m| m.to.index() == to),
            "Missing move to square {}",
            to
        );
    }
}

#[test]
fn rook_moves_from_corner_a1() {
    let mut board = Board::new_empty();
    let a1 = 0;
    let piece_mask = 1u64 << a1;
    set_piece_mask(&mut board, Color::White, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_rook_moves(&board, &magic_tables.rook, &mut moves);

    // Vertical: a2 (8) to a8 (56) = +8 steps
    // Horizontal: b1 (1) to h1 (7) = +1 steps
    let expected_dests: Vec<u8> = (1..8)
        .chain((1..8).map(|i| i * 8))
        .map(|i| i as u8)
        .collect();
    assert_eq!(moves.len(), expected_dests.len());
    for to in expected_dests {
        assert!(
            moves.iter().any(|m| m.to.index() == to),
            "Missing move to square {}",
            to
        );
    }
}

#[test]
fn rook_blocked_by_friendly_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Rook, piece_mask);

    let piece_mask = 1 << 28;
    set_piece_mask(&mut board, Color::White, Piece::Bishop, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_rook_moves(&board, &magic_tables.rook, &mut moves);

    // Normally D4 has 14 targets; 28 blocked => expect fewer
    assert!(!moves.iter().any(|m| m.to.index() == 28));
    assert!(moves.len() < 14);
}

#[test]
fn rook_captures_enemy_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Rook, piece_mask);

    let piece_mask = 1 << 28;
    set_piece_mask(&mut board, Color::Black, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_rook_moves(&board, &magic_tables.rook, &mut moves);

    let capture = moves.iter().find(|m| m.to.index() == 28);
    assert!(capture.is_some(), "Expected capture move to e4 (28)");
    assert!(capture.unwrap().is_capture());
}

#[test]
fn multiple_rooks_generate_moves() {
    let mut board = Board::new_empty();
    let piece_mask = (1 << 0) | (1 << 56);
    set_piece_mask(&mut board, Color::White, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_rook_moves(&board, &magic_tables.rook, &mut moves);

    assert!(moves.iter().any(|m| m.from.index() == 0));
    assert!(moves.iter().any(|m| m.from.index() == 56));
    assert!(!moves.is_empty());
}

#[test]
fn no_rooks_yields_no_moves() {
    let mut board = Board::new_empty();
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_rook_moves(&board, &magic_tables.rook, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn queen_moves_from_d4_empty_board() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Queen, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_queen_moves(&board, &magic_tables, &mut moves);

    // Rook moves: 26,25,24,28,29,30,31,35,43,51,59,19,11,3 (14)
    // Bishop moves: 18,20,9,0,36,45,54,63,34,41,48,6,13 (13)
    // Total = 27 unique squares
    let expected_dests = [
        26, 25, 24, 28, 29, 30, 31, 35, 43, 51, 59, 19, 11, 3, // rook
        18, 20, 9, 0, 36, 45, 54, 63, 34, 41, 48, 6, 13, // bishop
    ];

    assert_eq!(moves.len(), expected_dests.len());
    for &to in &expected_dests {
        assert!(
            moves.iter().any(|m| m.to.index() == to),
            "Missing move to square {}",
            to
        );
    }
}

#[test]
fn queen_moves_from_corner_a1() {
    let mut board = Board::new_empty();
    let a1 = 0;
    let piece_mask = 1u64 << a1;
    set_piece_mask(&mut board, Color::White, Piece::Queen, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_queen_moves(&board, &magic_tables, &mut moves);

    // Rook: a2–a8 (8–56 by 8), b1–h1 (1–7)
    // Bishop: b2 (9), c3 (18), d4 (27), e5 (36), f6 (45), g7 (54), h8 (63)
    let expected_dests = [
        1, 2, 3, 4, 5, 6, 7, // horizontal
        8, 16, 24, 32, 40, 48, 56, // vertical
        9, 18, 27, 36, 45, 54, 63, // diagonal
    ];

    assert_eq!(moves.len(), expected_dests.len());
    for &to in &expected_dests {
        assert!(
            moves.iter().any(|m| m.to.index() == to),
            "Missing move to square {}",
            to
        );
    }
}

#[test]
fn queen_blocked_by_friendly_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Queen, piece_mask);

    let piece_mask = 1 << 28;
    set_piece_mask(&mut board, Color::White, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_queen_moves(&board, &magic_tables, &mut moves);

    assert!(!moves.iter().any(|m| m.to.index() == 28));
    assert!(moves.len() < 27);
}

#[test]
fn queen_captures_enemy_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Queen, piece_mask);

    let piece_mask = 1 << 28;
    set_piece_mask(&mut board, Color::Black, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_queen_moves(&board, &magic_tables, &mut moves);

    let capture = moves.iter().find(|m| m.to.index() == 28);
    assert!(capture.is_some(), "Expected capture move to e4 (28)");
    assert!(capture.unwrap().is_capture());
}

#[test]
fn multiple_queens_generate_moves() {
    let mut board = Board::new_empty();
    let piece_mask = (1 << 0) | (1 << 63); // a1 and h8
    set_piece_mask(&mut board, Color::White, Piece::Queen, piece_mask);

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_queen_moves(&board, &magic_tables, &mut moves);

    assert!(moves.iter().any(|m| m.from.index() == 0));
    assert!(moves.iter().any(|m| m.from.index() == 63));
    assert!(!moves.is_empty());
}

#[test]
fn no_queens_yields_no_moves() {
    let mut board = Board::new_empty();
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_queen_moves(&board, &magic_tables, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn king_moves_from_center_e4() {
    let mut board = Board::new_empty();
    let e4 = 4 + 8 * 3; // index 28
    let piece_mask = 1u64 << e4;
    set_piece_mask(&mut board, Color::White, Piece::King, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&board, &magic_tables, &mut moves);

    let expected_dests = [19, 20, 21, 27, 29, 35, 36, 37];

    assert_eq!(moves.len(), expected_dests.len());
    for &dest in &expected_dests {
        assert!(
            moves.iter().any(|m| m.to.index() == dest),
            "Expected move to {} not found",
            dest
        );
    }
}

#[test]
fn king_moves_from_corner_a1() {
    let mut board = Board::new_empty();
    let a1 = 0;
    let piece_mask = 1u64 << a1;
    set_piece_mask(&mut board, Color::White, Piece::King, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&board, &magic_tables, &mut moves);

    let expected_dests = [1, 8, 9];
    assert_eq!(moves.len(), expected_dests.len());
    for &to in &expected_dests {
        assert!(moves.iter().any(|m| m.to.index() == to));
    }
}

#[test]
fn king_blocked_by_friendly_piece() {
    let mut board = Board::new_empty();
    let e4 = 28;
    let piece_mask = 1u64 << e4;
    set_piece_mask(&mut board, Color::White, Piece::King, piece_mask);

    let piece_mask = 1 << 29;
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&board, &magic_tables, &mut moves);

    assert!(!moves.iter().any(|m| m.to.index() == 29));
    assert_eq!(moves.len(), 7);
}

#[test]
fn king_captures_enemy_piece() {
    let mut board = Board::new_empty();
    let e4 = 28;
    let piece_mask = 1u64 << e4;
    set_piece_mask(&mut board, Color::White, Piece::King, piece_mask);

    let piece_mask = 1 << 36;
    set_piece_mask(&mut board, Color::Black, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&board, &magic_tables, &mut moves);

    let mv = moves
        .iter()
        .find(|m| m.to.index() == 36)
        .expect("Missing capture move to e5");
    assert!(mv.is_capture());
}

#[test]
fn no_king_yields_no_moves() {
    let mut board = Board::new_empty();
    board.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&board, &magic_tables, &mut moves);
    assert!(moves.is_empty());
}

#[test]
fn pawn_push_from_d2() {
    let mut board = Board::new_empty();
    let d2 = 11;
    let piece_mask = 1u64 << d2;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    let expected_dests = [19, 27]; // d3
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        assert!(moves.iter().any(|m| m.to.index() == to));
    }
}

#[test]
fn pawn_blocked_by_piece() {
    let mut board = Board::new_empty();
    let d2 = 11;
    let d3 = 19;
    let piece_mask = 1u64 << d2;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);

    let piece_mask = 1u64 << d3;
    set_piece_mask(&mut board, Color::White, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn pawn_captures_diagonally() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);

    let piece_mask = (1 << 34) | (1 << 36) | (1 << 35);
    set_piece_mask(&mut board, Color::Black, Piece::Rook, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);
    println!("{:#?}", moves);

    let expected_dests = [34, 36];
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        let mv = moves.iter().find(|m| m.to.index() == to);
        assert!(mv.unwrap().is_capture());
        assert!(mv.is_some(), "Missing capture to {}", to);
    }
}

#[test]
fn multiple_pawns_generate_moves() {
    let mut board = Board::new_empty();
    let piece_mask = (1 << 11) | (1 << 27); // d2 and d4
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    let expected_dests = [19, 35]; // d3 and d5
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        assert!(moves.iter().any(|m| m.to.index() == to));
    }
}

#[test]
fn no_pawns_yields_no_moves() {
    let mut board = Board::new_empty();
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn black_pawn_push_from_d7() {
    let mut board = Board::new_empty();
    let d7 = 51;
    let piece_mask = 1u64 << d7;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    let expected_dests = [43, 35]; // d6
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        assert!(moves.iter().any(|m| m.to.index() == to));
    }
}

#[test]
fn black_pawn_blocked_by_piece() {
    let mut board = Board::new_empty();
    let d7 = 51;
    let d6 = 43;
    let piece_mask = 1u64 << d7;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);

    let piece_mask = 1u64 << d6;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn black_pawn_captures_diagonally() {
    let mut board = Board::new_empty();
    let d5 = 35;
    let piece_mask = 1u64 << d5;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);
    let piece_mask = (1 << 26) | (1 << 27) | (1 << 28);
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    let expected_dests = [26, 28];
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        let mv = moves.iter().find(|m| m.to.index() == to);
        assert!(mv.is_some(), "Missing capture to {}", to);
        assert!(mv.unwrap().is_capture());
    }
}

#[test]
fn multiple_black_pawns_generate_moves() {
    let mut board = Board::new_empty();
    let piece_mask = (1 << 51) | (1 << 35); // d7 and d5
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    let expected_dests = [43, 27]; // d6 and d4
    assert_eq!(moves.len(), expected_dests.len());

    for &to in &expected_dests {
        assert!(moves.iter().any(|m| m.to.index() == to));
    }
}

#[test]
fn no_black_pawns_yields_no_moves() {
    let mut board = Board::new_empty();
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    assert!(moves.is_empty());
}

#[test]
fn white_pawn_does_not_capture_friendly_piece() {
    let mut board = Board::new_empty();
    let d4 = 27;
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    let piece_mask = (1 << 34) | (1 << 36);
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    // Should not generate any diagonal captures
    assert!(
        moves.iter().all(|m| !m.is_capture()),
        "Should not capture friendly pieces"
    );
}

#[test]
fn black_pawn_does_not_capture_friendly_piece() {
    let mut board = Board::new_empty();
    let d5 = 35;
    let piece_mask = 1u64 << d5;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);
    let piece_mask = (1 << 26) | (1 << 28);
    set_piece_mask(&mut board, Color::Black, Piece::Knight, piece_mask);
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    // Should not generate any diagonal captures
    assert!(
        moves.iter().all(|m| !m.is_capture()),
        "Should not capture friendly pieces"
    );
}

#[test]
fn white_promotion_push() {
    let mut board = Board::new_empty();
    let a7 = 48;
    let piece_mask = 1u64 << a7;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    let expected_to = 56;
    assert_eq!(moves.len(), 4); // Q, R, B, N
    for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
        assert!(moves.iter().any(|m| m.from.index() == a7
            && m.to.index() == expected_to
            && m.promotion == Some(piece)
            && !m.is_capture()));
    }
}

#[test]
fn white_promotion_captures() {
    let mut board = Board::new_empty();
    let d7 = 51;
    let piece_mask = 1u64 << d7;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    let piece_mask = (1 << 58) | (1 << 60);
    set_piece_mask(&mut board, Color::Black, Piece::Knight, piece_mask);
    board.side_to_move = Color::White;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    assert_eq!(moves.len(), 12); // 2 targets × 4 promos
    for &to in [58, 60].iter() {
        for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
            assert!(moves.iter().any(|m| m.from.index() == d7
                && m.to.index() == to
                && m.promotion == Some(piece)
                && m.is_capture()));
        }
    }
}

#[test]
fn black_promotion_push() {
    let mut board = Board::new_empty();
    let a2 = 8;
    let piece_mask = 1u64 << a2;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    let expected_to = 0;
    assert_eq!(moves.len(), 4); // Q, R, B, N
    for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
        assert!(moves.iter().any(|m| m.from.index() == a2
            && m.to.index() == expected_to
            && m.promotion == Some(piece)
            && !m.is_capture()));
    }
}

#[test]
fn black_promotion_captures() {
    let mut board = Board::new_empty();
    let d2 = 11;
    let piece_mask = 1u64 << d2;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);
    let piece_mask = (1 << 2) | (1 << 4); // c1 and e1
    set_piece_mask(&mut board, Color::White, Piece::Knight, piece_mask);
    board.side_to_move = Color::Black;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);
    println!("{:#?}", moves);

    assert_eq!(moves.len(), 12); // 2 targets × 4 promos
    for &to in [2, 4].iter() {
        for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
            assert!(moves.iter().any(|m| m.from.index() == d2
                && m.to.index() == to
                && m.promotion == Some(piece)
                && m.is_capture()));
        }
    }
}

#[test]
fn white_en_passant_capture() {
    let mut board = Board::new_empty();

    let e5 = 36; // White pawn on e5
    let piece_mask = 1u64 << e5;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);

    let d5 = 35; // Black pawn that double-pushed to d5
    let piece_mask = 1u64 << d5;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);

    board.side_to_move = Color::White;
    board.en_passant = Some(Square::from_index(43)); // d6 is the square behind the pawn that double-pushed

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);
    println!("{:#?}", moves);

    let ep_move = moves.iter().find(|m| m.is_en_passant());
    assert!(ep_move.is_some(), "Expected en passant move");

    let m = ep_move.unwrap();
    assert_eq!(m.from.index(), e5);
    assert_eq!(m.to.index(), 43); // d6
    assert!(m.is_capture());
}

#[test]
fn black_en_passant_capture() {
    let mut board = Board::new_empty();

    let d4 = 27; // Black pawn on d4
    let piece_mask = 1u64 << d4;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);

    let e2 = 28; // White pawn that double-pushed to e4
    let piece_mask = 1u64 << e2;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);

    board.side_to_move = Color::Black;
    board.en_passant = Some(Square::from_index(20)); // e3

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);
    println!("{:#?}", moves);

    let ep_move = moves.iter().find(|m| m.is_en_passant());
    assert!(ep_move.is_some(), "Expected en passant move");

    let m = ep_move.unwrap();
    assert_eq!(m.from.index(), d4);
    assert_eq!(m.to.index(), 20); // e3
    assert!(m.is_capture());
}

#[test]
fn no_en_passant_when_not_set() {
    let mut board = Board::new_empty();

    let piece_mask = 1 << 28;
    set_piece_mask(&mut board, Color::White, Piece::Pawn, piece_mask);
    let piece_mask = 1 << 27;
    set_piece_mask(&mut board, Color::Black, Piece::Pawn, piece_mask);
    board.side_to_move = Color::White;
    board.en_passant = None;

    let mut moves = Vec::new();
    generate_pawn_moves(&board, &mut moves);

    assert!(
        moves.iter().all(|m| !m.is_en_passant()),
        "No en passant move expected"
    );
}

/// Helper to look for a castling move that lands on `to`
fn has_castle(moves: &[Move], to: u8) -> bool {
    moves.iter().any(|m| m.is_castling() && m.to.index() == to)
}

#[test]
fn white_kingside_castle_generated() {
    let mut b = Board::new_empty();
    let piece_mask = 1u64 << 4;
    set_piece_mask(&mut b, Color::White, Piece::King, piece_mask);
    let piece_mask = 1u64 << 7;
    set_piece_mask(&mut b, Color::White, Piece::Rook, piece_mask);
    b.castling_rights = 0b0001; // CASTLE_WK
    b.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&b, &magic_tables, &mut moves);

    assert!(
        has_castle(&moves, 6),
        "White KS castle (to g1) not generated"
    );
}

#[test]
fn white_queenside_castle_generated() {
    let mut b = Board::new_empty();
    let piece_mask = 1u64 << 4;
    set_piece_mask(&mut b, Color::White, Piece::King, piece_mask);
    let piece_mask = 1u64 << 0;
    set_piece_mask(&mut b, Color::White, Piece::Rook, piece_mask);
    b.castling_rights = 0b0010; // CASTLE_WQ
    b.side_to_move = Color::White;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&b, &magic_tables, &mut moves);

    assert!(
        has_castle(&moves, 2),
        "White QS castle (to c1) not generated"
    );
}

#[test]
fn black_kingside_castle_generated() {
    let mut b = Board::new_empty();
    let piece_mask = 1u64 << 60;
    set_piece_mask(&mut b, Color::Black, Piece::King, piece_mask);
    let piece_mask = 1u64 << 63;
    set_piece_mask(&mut b, Color::Black, Piece::Rook, piece_mask);
    b.castling_rights = 0b0100; // CASTLE_BK
    b.side_to_move = Color::Black;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&b, &magic_tables, &mut moves);

    assert!(
        has_castle(&moves, 62),
        "Black KS castle (to g8) not generated"
    );
}

#[test]
fn black_queenside_castle_generated() {
    let mut b = Board::new_empty();
    let piece_mask = 1u64 << 60;
    set_piece_mask(&mut b, Color::Black, Piece::King, piece_mask);
    let piece_mask = 1u64 << 56;
    set_piece_mask(&mut b, Color::Black, Piece::Rook, piece_mask);
    b.castling_rights = 0b1000; // CASTLE_BQ
    b.side_to_move = Color::Black;

    let magic_tables = load_magic_tables();
    let mut moves = Vec::new();
    generate_king_moves(&b, &magic_tables, &mut moves);

    assert!(
        has_castle(&moves, 58),
        "Black QS castle (to c8) not generated"
    );
}

#[test]
fn white_kingside_castle_forbidden_if_f1_or_g1_attacked() {
    // White: K e1, R h1; Black bishop g2 attacks f1 (path square)
    // Only K-side right present, path empty, but attacked square forbids castling.
    let fen = "k7/8/8/8/8/8/6b1/4K2R w K - 0 1";
    let mut b = Board::from_str(fen).unwrap();
    let tables = load_magic_tables();

    let mut moves = Vec::new();
    let mut scratch = Vec::with_capacity(256);
    generate_legal(&mut b, &tables, &mut moves, &mut scratch);

    assert!(
        !moves.iter().any(|m| m.is_castling()
            && m.from == Square::from_str("e1").unwrap()
            && m.to == Square::from_str("g1").unwrap()),
        "White cannot castle through check when f1 or g1 is attacked"
    );
}

#[test]
fn black_queenside_castle_forbidden_if_d8_or_c8_attacked() {
    // Black: K e8, R a8 with 'q' right; White bishop c7 attacks d8 (transit square)
    let fen = "r3k3/2B5/8/8/8/8/8/8 b q - 0 1";
    let mut b = Board::from_str(fen).unwrap();
    let tables = load_magic_tables();

    let mut moves = Vec::new();
    let mut scratch = Vec::with_capacity(256);
    generate_legal(&mut b, &tables, &mut moves, &mut scratch);

    assert!(
        !moves.iter().any(|m| m.is_castling()
            && m.from == Square::from_str("e8").unwrap()
            && m.to == Square::from_str("c8").unwrap()),
        "Black cannot castle through check when d8 or c8 is attacked"
    );
}
