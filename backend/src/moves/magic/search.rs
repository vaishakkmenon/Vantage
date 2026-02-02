use rand::RngCore;
use std::collections::HashMap;

#[inline(always)]
// Generate a sparse 64-bit number by AND-ing three random values.
// This helps ensure a low number of set bits (sparse pattern),
// which reduces the chance of collisions in magic indexing.
pub fn random_sparse_u64<R: RngCore>(rng: &mut R) -> u64 {
    rng.next_u64() & rng.next_u64() & rng.next_u64()
}

pub fn is_magic_candidate_valid(blockers: &[u64], attacks: &[u64], magic: u64, shift: u32) -> bool {
    let mut seen: HashMap<u64, u64> = HashMap::new();

    for i in 0..blockers.len() {
        let blocker = blockers[i];
        let attack = attacks[i];
        let product = blocker.wrapping_mul(magic);
        let index = product >> shift;

        if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(index) {
            e.insert(attack);
        } else {
            let existing_attack = seen[&index];
            if existing_attack != attack {
                return false;
            }
        }
    }

    true
}

pub fn find_magic_number_for_square<R: RngCore>(
    blockers: &[u64],
    attacks: &[u64],
    shift: u32,
    rng: &mut R,
) -> Result<u64, String> {
    for _attempt in 0..1_000_000 {
        let magic = random_sparse_u64(rng);
        if is_magic_candidate_valid(blockers, attacks, magic, shift) {
            return Ok(magic);
        }
    }
    Err("Failed to find a valid magic number after 1,000,000 attempts".to_string())
}
