use super::attacks::*;
use super::masks::*;
use super::search::*;
use super::structs::*;
use crate::utils::enumerate_subsets;
use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::StdRng;

pub fn precompute_rook_attacks() -> Vec<Vec<u64>> {
    let mut table = Vec::with_capacity(64);

    for square in 0..64 {
        let mask = rook_vision_mask(square);

        let mut subsets = Vec::new();
        enumerate_subsets(mask, |subset| {
            subsets.push(subset);
        });

        let mut attacks_for_square = Vec::with_capacity(subsets.len());

        for blockers in subsets {
            let attacks = rook_attacks_per_square(square, blockers);
            attacks_for_square.push(attacks);
        }

        table.push(attacks_for_square);
    }

    table
}

pub fn precompute_bishop_attacks() -> Vec<Vec<u64>> {
    let mut table = Vec::with_capacity(64);

    for square in 0..64 {
        let mask = bishop_vision_mask(square);

        let mut subsets = Vec::new();
        enumerate_subsets(mask, |subset| {
            subsets.push(subset);
        });

        let mut attacks_for_square = Vec::with_capacity(subsets.len());

        for blockers in subsets {
            let attacks = bishop_attacks_per_square(square, blockers);
            attacks_for_square.push(attacks);
        }

        table.push(attacks_for_square);
    }

    table
}

/// Generates a full set of 64 `MagicEntry`s for a sliding piece (e.g. rook or bishop)
/// by building magic bitboard lookup tables for each square.
///
/// # Parameters
/// - `piece_name`: A string label used only for logging.
/// - `gen_blockers`: Function that returns all possible blocker permutations for a given square.
/// - `get_attacks`: Function that returns the expected attack bitboards for all blocker permutations.
/// - `get_mask`: Function that returns the vision mask for a given square.
/// - `attacks_per_square`: Function that returns the actual attack bitboard for a given square and blocker configuration.
///
/// # Returns
/// An array of 64 `MagicEntry`s, one for each square on the board. Each entry contains:
/// - a magic number (used for hashing blocker configurations),
/// - a shift value (how many bits to shift the hash down),
/// - and a lookup table mapping hashed blockers to precomputed attacks.
///
/// # How It Works
/// For each square:
/// 1. All blocker permutations within the piece's vision mask are generated.
/// 2. The corresponding attack bitboards are computed.
/// 3. A "magic" number is found that produces a unique index for each blocker configuration.
/// 4. A lookup table is populated using these magic indices.
///
/// # Panics
/// Panics if the final `Vec<MagicEntry>` does not contain exactly 64 entries. This
/// would indicate a logic error in how blockers were generated or processed.
fn generate_magic_entries<FBlockers, FAttacks, FMask, FPerSquare, R>(
    gen_blockers: FBlockers,
    get_attacks: FAttacks,
    get_mask: FMask,
    attacks_per_square: FPerSquare,
    rng: &mut R,
) -> Result<Vec<MagicEntry>, String>
where
    FBlockers: Fn(usize) -> Vec<u64>,
    FAttacks: Fn(usize, &[u64]) -> Vec<u64>,
    FMask: Fn(usize) -> u64,
    FPerSquare: Fn(usize, u64) -> u64,
    R: RngCore,
{
    let mut entries_vec = Vec::with_capacity(64);

    for square in 0..64 {
        let blockers = gen_blockers(square);
        let attacks = get_attacks(square, &blockers);
        let mask = get_mask(square);
        let shift = 64 - mask.count_ones();

        let magic = match find_magic_number_for_square(&blockers, &attacks, shift, rng) {
            Ok(magic) => magic,
            Err(e) => {
                return Err(format!("Square {}: {}", square, e));
            }
        };

        let table_size = 1 << mask.count_ones();
        let mut table = vec![0u64; table_size].into_boxed_slice();

        for &b in &blockers {
            let idx = ((b & mask).wrapping_mul(magic)) >> shift;
            table[idx as usize] = attacks_per_square(square, b);
        }

        entries_vec.push(MagicEntry {
            magic,
            shift,
            mask,
            table,
        });
    }

    Ok(entries_vec)
}

pub fn generate_rook_magic_tables<R: RngCore>(rng: &mut R) -> Result<RookMagicTables, String> {
    let entries: Vec<MagicEntry> = generate_magic_entries(
        generate_rook_blockers,
        get_rook_attack_bitboards,
        rook_vision_mask,
        rook_attacks_per_square,
        rng,
    )?;

    Ok(RookMagicTables { entries })
}

pub fn generate_bishop_magic_tables<R: RngCore>(rng: &mut R) -> Result<BishopMagicTables, String> {
    let entries = generate_magic_entries(
        generate_bishop_blockers,
        get_bishop_attack_bitboards,
        bishop_vision_mask,
        bishop_attacks_per_square,
        rng,
    )?;

    Ok(BishopMagicTables { entries })
}

pub enum MagicTableSeed {
    Randomized,
    Fixed(u64),
}

pub fn generate_magic_tables(seed_mode: MagicTableSeed) -> Result<MagicTables, String> {
    // 1) Initialize the StdRng according to the seed mode
    let mut rng = match seed_mode {
        MagicTableSeed::Fixed(seed) => StdRng::seed_from_u64(seed),

        MagicTableSeed::Randomized => {
            let mut entropy = rand::rng();
            let seed = entropy.next_u64();
            StdRng::seed_from_u64(seed)
        }
    };

    // 2) Pass the same seeded RNG to both generators
    let rook = generate_rook_magic_tables(&mut rng)?;
    let bishop = generate_bishop_magic_tables(&mut rng)?;

    Ok(MagicTables { rook, bishop })
}

// These tests generate magic tables from scratch, so skip when using pre-loaded tables
#[cfg(all(test, not(feature = "load_magic")))]
mod tests {
    use super::*;
    use crate::moves::magic::masks::{bishop_vision_mask, rook_vision_mask};
    use crate::utils::{bitboard_to_string, square_index};

    /// Same seed for every run so CI is repeatable
    const TEST_SEED: u64 = 0b100_0101;

    /// Helper: convert "e4" → 27, etc.
    fn square_from_str(sq: &str) -> usize {
        let file = (sq.as_bytes()[0] - b'a') as usize;
        let rank = (sq.as_bytes()[1] - b'1') as usize;
        square_index(rank, file)
    }

    /// Helper: build all magic tables once per test
    fn build_tables() -> Result<MagicTables, String> {
        generate_magic_tables(MagicTableSeed::Fixed(TEST_SEED))
    }

    #[test]
    fn test_rook_attacks_e4_no_blockers() -> Result<(), String> {
        let tables = build_tables()?; // ← deterministic
        let square = square_from_str("e4");
        let blockers = 0u64;

        let entry = &tables.rook.entries[square];
        let idx = ((blockers & rook_vision_mask(square)).wrapping_mul(entry.magic)) >> entry.shift;
        let actual = entry.table[idx as usize];

        // Expected: file e + rank 4, minus the square itself
        let file_mask = 0x0101_0101_0101_0101u64 << (square % 8);
        let rank_mask = 0xFFu64 << (8 * (square / 8));
        let expected = (file_mask | rank_mask) & !(1u64 << square);

        assert_eq!(
            actual,
            expected,
            "Incorrect rook attack for e4 (no blockers)\nExpected:\n{}\nGot:\n{}",
            bitboard_to_string(expected),
            bitboard_to_string(actual)
        );
        Ok(())
    }

    #[test]
    fn test_bishop_attacks_c1_with_e3_blocker() -> Result<(), String> {
        let tables = build_tables()?; // ← same seed
        let square = square_from_str("c1");
        let blocker_square = square_index(2, 4); // e3
        let blockers = 1u64 << blocker_square;

        let entry = &tables.bishop.entries[square];
        let idx =
            ((blockers & bishop_vision_mask(square)).wrapping_mul(entry.magic)) >> entry.shift;
        let actual = entry.table[idx as usize];

        // Expected attack squares from c1 with a blocker on e3
        let mut expected = 0u64;
        expected |= 1u64 << square_index(1, 3); // d2
        expected |= 1u64 << square_index(2, 4); // e3
        expected |= 1u64 << square_index(1, 1); // b2
        expected |= 1u64 << square_index(2, 0); // a3

        assert_eq!(
            actual,
            expected,
            "Incorrect bishop attack for c1 with blocker on e3\nExpected:\n{}\nGot:\n{}",
            bitboard_to_string(expected),
            bitboard_to_string(actual)
        );
        Ok(())
    }
}
