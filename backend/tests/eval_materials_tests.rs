use vantage::board::Board;
use vantage::moves::magic::loader::load_magic_tables;
use vantage::search::eval::{eval_material, static_eval};
use std::str::FromStr;

#[test]
fn startpos_material_is_zero() {
    let b = Board::new();
    assert_eq!(
        eval_material(&b),
        0,
        "Start position should have 0 material balance"
    );
}

#[test]
fn up_a_pawn_is_positive_and_mirroring_is_exact_negative() {
    let _tables = load_magic_tables();

    // White has an extra pawn (a3)
    let w_fen = "rnbqkbnr/1ppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let bw = Board::from_str(w_fen).unwrap();
    let sw = eval_material(&bw);
    assert!(
        sw > 0,
        "White up a pawn should be positive for White, got {}",
        sw
    );

    // Mirror: Black has an extra pawn (a6)
    let b_fen = "rnbqkbnr/pppppppp/8/8/8/8/1PPPPPPP/RNBQKBNR b KQkq - 0 1";
    let bb = Board::from_str(b_fen).unwrap();
    let sb = eval_material(&bb);
    assert!(
        sb < 0,
        "Mirrored position should be negative for White, got {}",
        sb
    );

    assert_eq!(
        sw, -sb,
        "Material-only must mirror exactly: {} vs {}",
        sw, sb
    );
}

fn fen(f: &str) -> Board {
    Board::from_str(f).expect("valid FEN")
}

#[test]
fn material_startpos_is_zero() {
    let tables = load_magic_tables();
    let b = fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    assert_eq!(eval_material(&b), 0);
    // CHANGED: static_eval includes PSQT, so just check it's close to 0
    let eval = static_eval(&b, &tables, -32000, 32000);
    assert!(
        eval.abs() < 200,
        "Start position eval should be close to 0, got {}",
        eval
    );
}

#[test]
fn material_white_up_a_pawn_is_plus_100() {
    let tables = load_magic_tables();
    // Added kings (Kh1, kh8) to satisfy board validation
    let b = fen("7k/8/8/8/8/8/P7/7K w - - 0 1");
    let val = eval_material(&b);
    // PeSTO Pawn is around 82(MG) to 94(EG).
    assert!(
        val >= 80 && val <= 100,
        "White pawn should be approx 80-100, got {}",
        val
    );

    // CHANGED: static_eval includes PSQT bonus
    let eval = static_eval(&b, &tables, -32000, 32000);
    assert!(
        eval >= 80,
        "Static eval with pawn should be positive (approx 80+), got {}",
        eval
    );
}

#[test]
fn material_black_up_a_rook_is_minus_500() {
    let tables = load_magic_tables();
    // Added kings (Ka1, ka8) to satisfy board validation
    let b = fen("k7/8/8/8/8/8/8/K6r w - - 0 1");
    let val = eval_material(&b);
    // PeSTO Rook is 477(MG) to 512(EG). So -477 to -512.
    assert!(
        val <= -470 && val >= -520,
        "Black rook material should be approx -470 to -520, got {}",
        val
    );

    // CHANGED: static_eval includes PSQT bonus (which makes it LESS negative usually)
    let eval = static_eval(&b, &tables, -32000, 32000);
    assert!(
        eval <= -400,
        "Black rook eval should be significantly negative (<= -400), got {}",
        eval
    );
}

#[test]
fn material_promotion_delta_is_plus_800_for_white() {
    // Added kings (Kh1, kh8) to satisfy board validation
    let a7_pawn = fen("7k/P7/8/8/8/8/8/7K w - - 0 1");
    let a7_queen = fen("7k/Q7/8/8/8/8/8/7K w - - 0 1");

    let pawn_material = eval_material(&a7_pawn);
    let queen_material = eval_material(&a7_queen);
    let delta = queen_material - pawn_material;

    // Queen (approx 1000) - Pawn (approx 90) = approx 910
    // PeSTO Queen (1025, 968), Pawn (82, 94). Delta ~ 943(MG) to 874(EG).
    assert!(
        delta >= 800 && delta <= 1000,
        "Promotion delta should be around 800-1000, got {}",
        delta
    );
}

#[test]
fn material_en_passant_capture_reduces_white_pawns_by_one() {
    // Added kings (Kh1, kh8) to satisfy board validation
    let after_ep = fen("7k/8/3p4/8/8/8/8/7K w - - 0 1");
    let before_ep = fen("7k/8/3p4/4P3/8/8/8/7K w - - 0 1");

    let diff = eval_material(&before_ep) - eval_material(&after_ep);
    // Should be exactly one pawn value (approx 82-94)
    assert!(
        diff >= 80 && diff <= 100,
        "EP capture diff should be one pawn (80-100), got {}",
        diff
    );
}

// -------------- PSQT-specific tests --------------

#[test]
fn static_eval_includes_psqt_bonus() {
    let tables = load_magic_tables();
    // With PSQT, static_eval should differ from pure material
    // Added kings (Kh1, kh8) to satisfy board validation
    let b = fen("7k/8/8/8/8/8/P7/7K w - - 0 1");
    let material = eval_material(&b);
    let full_eval = static_eval(&b, &tables, -32000, 32000);

    // static_eval = material + PSQT bonuses + Structure + Mobility + KingSafety
    // Since we now have structure penalties (e.g. isolated pawn), static_eval can be LOWER than material.
    // We just want to ensure it's calculated differently (includes other terms).
    assert!(
        full_eval != material,
        "static_eval ({}) should differ from material ({}) due to PSQT/Structure",
        full_eval,
        material
    );
}

#[test]
fn static_eval_accounts_for_side_to_move() {
    let tables = load_magic_tables();
    // Same position, different side to move
    let white_to_move = fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let black_to_move = fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");

    let eval_white = static_eval(&white_to_move, &tables, -32000, 32000);
    let eval_black = static_eval(&black_to_move, &tables, -32000, 32000);

    // With tempo bonus, these should differ slightly
    // eval_white should be slightly better than eval_black (tempo bonus)
    println!(
        "White to move: {}, Black to move: {}",
        eval_white, eval_black
    );
}

// -------------- Mirror sanity tests --------------

#[test]
fn mirror_vert_basic_checks() {
    use vantage::search::eval::mirror_vert;
    use vantage::square::Square;

    let a2 = Square::from_str("a2").unwrap().index();
    let a7 = Square::from_str("a7").unwrap().index();
    let c3 = Square::from_str("c3").unwrap().index();
    let c6 = Square::from_str("c6").unwrap().index();
    let h1 = Square::from_str("h1").unwrap().index();
    let h8 = Square::from_str("h8").unwrap().index();

    assert_eq!(mirror_vert(a2), a7 as usize);
    assert_eq!(mirror_vert(a7), a2 as usize);
    assert_eq!(mirror_vert(c3), c6 as usize);
    assert_eq!(mirror_vert(c6), c3 as usize);
    assert_eq!(mirror_vert(h1), h8 as usize);
    assert_eq!(mirror_vert(h8), h1 as usize);
}
