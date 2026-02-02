#[cfg(not(feature = "load_magic"))]
use rand::{SeedableRng, rngs::StdRng};

use vantage::moves::magic::attacks::{
    bishop_attacks_per_square, get_bishop_attack_bitboards, get_rook_attack_bitboards,
    rook_attacks_per_square,
};
use vantage::moves::magic::masks::{
    bishop_vision_mask, generate_bishop_blockers, generate_rook_blockers, rook_vision_mask,
};
// These are used unconditionally
use vantage::moves::magic::precompute::{precompute_bishop_attacks, precompute_rook_attacks};

// These are only used when we generate magic tables on the fly
#[cfg(not(feature = "load_magic"))]
use vantage::moves::magic::precompute::{generate_bishop_magic_tables, generate_rook_magic_tables};

// These search functions are only used in non-load_magic tests
#[cfg(not(feature = "load_magic"))]
use vantage::moves::magic::search::find_magic_number_for_square;

// Used unconditionally
use vantage::moves::magic::search::is_magic_candidate_valid;

#[cfg(not(feature = "load_magic"))]
use vantage::moves::magic::structs::MagicTables;

// 0x45 == 69₁₀; keeps the literal “69” out of the source
#[cfg(not(feature = "load_magic"))]
const TEST_SEED: u64 = 0x45;

#[cfg(not(feature = "load_magic"))]
fn seeded_rng() -> StdRng {
    StdRng::seed_from_u64(TEST_SEED)
}

/// Helper: Pretty-print a bitboard
fn print_bitboard(mask: u64) {
    for rank in (0..8).rev() {
        for file in 0..8 {
            let sq = rank * 8 + file;
            let is_set = (mask >> sq) & 1;
            if is_set == 1 {
                let file_char = (b'a' + file as u8) as char;
                let rank_char = (b'1' + rank as u8) as char;
                print!("{}{} ", file_char, rank_char);
            }
        }
    }
    println!();
}

#[test]
fn test_rook_vision_d4() {
    let d4 = 3 + 3 * 8;
    let mask = rook_vision_mask(d4);
    println!("Rook vision mask d4:");
    print_bitboard(mask);
}

#[test]
fn test_rook_vision_a1() {
    let a1 = 0;
    let mask = rook_vision_mask(a1);
    println!("Rook vision mask a1:");
    print_bitboard(mask);
}

#[test]
fn test_bishop_vision_d4() {
    let d4 = 3 + 3 * 8;
    let mask = bishop_vision_mask(d4);
    println!("Bishop vision mask d4:");
    print_bitboard(mask);
}

#[test]
fn test_bishop_vision_c1() {
    let c1 = 2;
    let mask = bishop_vision_mask(c1);
    println!("Bishop vision mask c1:");
    print_bitboard(mask);
}

#[test]
fn test_rook_blocker_count_d4() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_rook_blockers(d4);
    assert_eq!(blockers.len(), 1024);
}

#[test]
fn test_bishop_blocker_count_d4() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_bishop_blockers(d4);
    assert_eq!(blockers.len(), 512);
}

#[test]
fn test_rook_attacks_no_blockers_d4() {
    let d4 = 3 + 3 * 8;
    let attacks = rook_attacks_per_square(d4, 0);
    println!("Rook attacks no blockers d4:");
    print_bitboard(attacks);
}

#[test]
fn test_rook_attacks_blockers_d4() {
    let d4 = 3 + 3 * 8;
    let blocker_d6 = (5 * 8) + 3;
    let blocker_f4 = (3 * 8) + 5;
    let blockers = (1u64 << blocker_d6) | (1u64 << blocker_f4);

    let attacks = rook_attacks_per_square(d4, blockers);
    println!("Rook attacks with blockers on d6 and f4:");
    print_bitboard(attacks);
}

#[test]
fn test_bishop_attacks_no_blockers_d4() {
    let d4 = 3 + 3 * 8;
    let attacks = bishop_attacks_per_square(d4, 0);
    println!("Bishop attacks no blockers d4:");
    print_bitboard(attacks);
}

#[test]
fn test_bishop_attacks_blockers_d4() {
    let d4 = 3 + 3 * 8;
    let blocker_f6 = (5 * 8) + 5;
    let blockers = 1u64 << blocker_f6;

    let attacks = bishop_attacks_per_square(d4, blockers);
    println!("Bishop attacks with blocker on f6:");
    print_bitboard(attacks);
}

#[test]
fn test_rook_attacks_no_blockers_a1() {
    let a1 = 0;
    let attacks = rook_attacks_per_square(a1, 0);
    println!("Rook attacks no blockers a1:");
    print_bitboard(attacks);
}

#[test]
fn test_bishop_attacks_no_blockers_c1() {
    let c1 = 2;
    let attacks = bishop_attacks_per_square(c1, 0);
    println!("Bishop attacks no blockers c1:");
    print_bitboard(attacks);
}

#[test]
fn test_rook_blocker_count_a1() {
    let a1 = 0;
    let blockers = generate_rook_blockers(a1);
    assert_eq!(blockers.len(), 4096);
}

#[test]
fn test_bishop_blocker_count_c1() {
    let c1 = 2;
    let blockers = generate_bishop_blockers(c1);
    println!("Bishop blocker count c1: {}", blockers.len());
}

#[test]
fn test_rook_attacks_blockers_first_square() {
    let d4 = 3 + 3 * 8;

    let blocker_north = (4 * 8) + 3; // d5
    let blocker_south = (2 * 8) + 3; // d3
    let blocker_east = (3 * 8) + 4; // e4
    let blocker_west = (3 * 8) + 2; // c4

    let blockers = (1u64 << blocker_north)
        | (1u64 << blocker_south)
        | (1u64 << blocker_east)
        | (1u64 << blocker_west);

    let attacks = rook_attacks_per_square(d4, blockers);
    println!("Rook attacks with blockers in first square each direction:");
    print_bitboard(attacks);
}

#[test]
fn test_bishop_attacks_blockers_first_square() {
    let d4 = 3 + 3 * 8;

    let blocker_ne = (4 * 8) + 4; // e5
    let blocker_nw = (4 * 8) + 2; // c5
    let blocker_se = (2 * 8) + 4; // e3
    let blocker_sw = (2 * 8) + 2; // c3

    let blockers =
        (1u64 << blocker_ne) | (1u64 << blocker_nw) | (1u64 << blocker_se) | (1u64 << blocker_sw);

    let attacks = bishop_attacks_per_square(d4, blockers);
    println!("Bishop attacks with blockers in first square each diagonal:");
    print_bitboard(attacks);
}

#[test]
fn test_rook_attack_table_counts() {
    let table = precompute_rook_attacks();

    for (square, entry) in table.iter().enumerate() {
        let mask = rook_vision_mask(square);
        let bit_count = mask.count_ones();
        let expected_len = 1 << bit_count;

        assert_eq!(
            entry.len(),
            expected_len as usize,
            "Square {}: expected {} entries, got {}",
            square,
            expected_len,
            entry.len()
        );
    }
}

#[test]
fn test_bishop_attack_table_counts() {
    let table = precompute_bishop_attacks();

    for (square, entry) in table.iter().enumerate() {
        let mask = bishop_vision_mask(square);
        let bit_count = mask.count_ones(); // bits in the vision mask
        let expected_len = 1 << bit_count; // 2^n blocker patterns

        assert_eq!(
            entry.len(),
            expected_len as usize,
            "Square {}: expected {} entries, got {}",
            square,
            expected_len,
            entry.len()
        );
    }
}

#[test]
fn test_invalid_magic_detects_collisions() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_rook_blockers(d4);
    let attacks = get_rook_attack_bitboards(d4, &blockers);

    // This magic number is almost certainly invalid
    let bad_magic = 1u64;

    let shift = 64 - rook_vision_mask(d4).count_ones();

    let valid = is_magic_candidate_valid(&blockers, &attacks, bad_magic, shift);
    assert!(!valid, "Expected invalid magic to produce collisions");
}

#[test]
fn test_trivial_magic_zero_fails() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_rook_blockers(d4);
    let attacks = get_rook_attack_bitboards(d4, &blockers);

    // Trivial magic number zero is invalid
    let bad_magic = 0u64;

    let shift = 64 - rook_vision_mask(d4).count_ones();

    let valid = is_magic_candidate_valid(&blockers, &attacks, bad_magic, shift);
    assert!(!valid, "Magic 0 must be invalid");
}

#[test]
fn test_all_same_attack_always_valid() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_rook_blockers(d4);

    // If all attacks are identical, any magic is "valid"
    let attacks = vec![0xDEADBEEF; blockers.len()];

    let shift = 64 - rook_vision_mask(d4).count_ones();

    let magic = 1u64;
    let valid = is_magic_candidate_valid(&blockers, &attacks, magic, shift);
    assert!(
        valid,
        "If all attacks are identical, any magic must be valid"
    );
}

#[test]
fn test_invalid_bishop_magic_detects_collisions() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_bishop_blockers(d4);
    let attacks = get_bishop_attack_bitboards(d4, &blockers);

    // Almost certainly invalid
    let bad_magic = 1u64;

    let shift = 64 - bishop_vision_mask(d4).count_ones();

    let valid = is_magic_candidate_valid(&blockers, &attacks, bad_magic, shift);
    assert!(
        !valid,
        "Expected invalid bishop magic to produce collisions"
    );
}

#[test]
fn test_trivial_bishop_magic_zero_fails() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_bishop_blockers(d4);
    let attacks = get_bishop_attack_bitboards(d4, &blockers);

    let bad_magic = 0u64;
    let shift = 64 - bishop_vision_mask(d4).count_ones();

    let valid = is_magic_candidate_valid(&blockers, &attacks, bad_magic, shift);
    assert!(!valid, "Magic 0 must be invalid");
}

#[test]
fn test_bishop_all_same_attack_always_valid() {
    let d4 = 3 + 3 * 8;
    let blockers = generate_bishop_blockers(d4);

    // All identical attacks: any magic passes
    let attacks = vec![0xDEADBEEF; blockers.len()];

    let shift = 64 - bishop_vision_mask(d4).count_ones();
    let magic = 1u64;

    let valid = is_magic_candidate_valid(&blockers, &attacks, magic, shift);
    assert!(
        valid,
        "If all attacks are identical, any magic must be valid"
    );
}

// This test generates magic tables, skip when using pre-loaded tables
#[test]
#[cfg(not(feature = "load_magic"))]
fn test_find_magic_for_rook_d4_real_search() -> Result<(), String> {
    let d4 = 3 + 3 * 8;
    let blockers = generate_rook_blockers(d4);
    let attacks = get_rook_attack_bitboards(d4, &blockers);
    let shift = 64 - rook_vision_mask(d4).count_ones();

    let mut rng = seeded_rng();
    let magic = find_magic_number_for_square(&blockers, &attacks, shift, &mut rng)?;

    let valid = is_magic_candidate_valid(&blockers, &attacks, magic, shift);
    assert!(valid, "The found rook magic must be valid");

    Ok(())
}

// This test generates magic tables, skip when using pre-loaded tables
#[test]
#[cfg(not(feature = "load_magic"))]
fn test_find_magic_for_bishop_d4_real_search() -> Result<(), String> {
    let d4 = 3 + 3 * 8;
    let blockers = generate_bishop_blockers(d4);
    let attacks = get_bishop_attack_bitboards(d4, &blockers);
    let shift = 64 - bishop_vision_mask(d4).count_ones();

    let mut rng = seeded_rng();
    let magic = find_magic_number_for_square(&blockers, &attacks, shift, &mut rng)?;

    let valid = is_magic_candidate_valid(&blockers, &attacks, magic, shift);
    assert!(valid, "The found bishop magic must be valid");

    Ok(())
}

// This test generates magic tables, skip when using pre-loaded tables
#[test]
#[cfg(not(feature = "load_magic"))]
fn test_magic_lookup_matches_scan_rook() {
    let square = 27; // D4
    let blockers = 0x0000_0800_0000_0000; // D6, example blocker
    let expected = rook_attacks_per_square(square, blockers);

    let table =
        generate_rook_magic_tables(&mut seeded_rng()).expect("Failed to generate rook magic table");

    let result = table.get_attacks(square, blockers);

    assert_eq!(
        result, expected,
        "Rook magic lookup failed at square {}",
        square
    );
}

// This test generates magic tables, skip when using pre-loaded tables
#[test]
#[cfg(not(feature = "load_magic"))]
fn test_magic_lookup_matches_scan_bishop() {
    let square = 27; // D4
    let blockers = 0x0000_0010_0000_0000; // B6, example blocker
    let expected = bishop_attacks_per_square(square, blockers);

    let table = generate_bishop_magic_tables(&mut seeded_rng())
        .expect("Failed to generate bishop magic table");

    let result = table.get_attacks(square, blockers);

    assert_eq!(
        result, expected,
        "Bishop magic lookup failed at square {}",
        square
    );
}

// This test generates magic tables, skip when using pre-loaded tables
#[test]
#[cfg(not(feature = "load_magic"))]
fn test_magic_lookup_matches_scan_queen() {
    let square = 27; // D4
    let blockers = 0x0000_0810_0000_0000; // D6 + B6

    let expected_rook = rook_attacks_per_square(square, blockers);
    let expected_bishop = bishop_attacks_per_square(square, blockers);
    let expected = expected_rook | expected_bishop;

    let mut rng = seeded_rng();
    let rook = generate_rook_magic_tables(&mut rng).unwrap();
    let bishop = generate_bishop_magic_tables(&mut rng).unwrap();
    let tables = MagicTables { rook, bishop };

    let result = tables.queen_attacks(square, blockers);

    assert_eq!(
        result, expected,
        "Queen magic lookup failed at square {}",
        square
    );
}
