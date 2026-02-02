use std::str::FromStr;

use vantage::board::Board;
use vantage::moves::execute::generate_legal;
use vantage::moves::magic::MagicTables;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::moves::types::Move;
use vantage::square::Square;

fn tables() -> MagicTables {
    load_magic_tables()
}

fn has_move(moves: &[Move], from: &str, to: &str) -> bool {
    let f = Square::from_str(from).unwrap();
    let t = Square::from_str(to).unwrap();
    moves.iter().any(|m| m.from == f && m.to == t)
}

#[test]
fn self_check_is_filtered_out() {
    // White: Ke1, Re2. Black: Re8. Moving Re2->f2 exposes e-file → illegal.
    let fen = "4r3/8/8/8/8/8/4R3/4K3 w - - 0 1";
    let mut b = Board::from_str(fen).unwrap();
    let t = tables();

    let mut legal = vec![];
    let mut scratch = Vec::with_capacity(256);
    generate_legal(&mut b, &t, &mut legal, &mut scratch);

    assert!(
        !has_move(&legal, "e2", "f2"),
        "Move e2f2 should be filtered (self-check)."
    );
    assert!(
        has_move(&legal, "e2", "e3"),
        "Blocking move e2e3 should remain legal."
    );
}

#[test]
fn checking_moves_are_kept() {
    // White: Re1, Kh1. Black: Ke8, Ne7. Re1xE7+ should be legal.
    let fen = "4k3/4n3/8/8/8/8/8/4R2K w - - 0 1";
    let mut b = Board::from_str(fen).unwrap();
    let t = tables();

    let mut legal = vec![];
    let mut scratch = Vec::with_capacity(256);
    generate_legal(&mut b, &t, &mut legal, &mut scratch);

    assert!(
        has_move(&legal, "e1", "e7"),
        "Checking capture e1e7 should not be filtered out."
    );
}

#[test]
fn en_passant_is_illegal_when_pawn_is_pinned_opening_file_on_own_king() {
    // Position: White king e1, White pawn e5; Black rook e8; Black pawn d5.
    // EP square = d6; White to move. If White plays e5xd6 e.p., the e-file opens and K on e1 is in check -> illegal.
    //
    // Board:
    // 8: k . . . r . . .
    // 7: . . . . . . . .
    // 6: . . . . . . . .
    // 5: . . . p P . . .
    // 4: . . . . . . . .
    // 3: . . . . . . . .
    // 2: . . . . . . . .
    // 1: . . . . K . . R
    // FEN with EP target d6 and white to move:
    let fen = "k3r3/8/8/3pP3/8/8/8/4K2R w - d6 0 1";
    let mut b = Board::from_str(fen).unwrap();
    let tables = load_magic_tables();

    let mut moves = Vec::with_capacity(64);
    let mut scratch = Vec::with_capacity(256);
    generate_legal(&mut b, &tables, &mut moves, &mut scratch);

    // Ensure no legal EP move from e5 to d6 exists
    assert!(
        !moves.iter().any(|m| m.is_en_passant()
            && m.from == Square::from_str("e5").unwrap()
            && m.to == Square::from_str("d6").unwrap()),
        "EP capture that exposes own king must be filtered out by the legality checker"
    );
}
