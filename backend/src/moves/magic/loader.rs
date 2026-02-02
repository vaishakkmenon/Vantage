//! src/moves/magic/loader.rs

// ── Always needed ───────────────────────────────────────────────────────────
use crate::moves::magic::MagicTables;

// ── Only needed when we generate at run time (i.e. *not* load_magic) ───────
#[cfg(not(feature = "load_magic"))]
use crate::moves::magic::precompute::{MagicTableSeed, generate_magic_tables};

// Only compile these when *deterministic_magic* is on
#[cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]
use std::sync::Once;

#[cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]
/// 0x45 == 69₁₀; the seed every deterministic build must use
const TEST_SEED: u64 = 0x45;

#[cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]
/// Where to drop the bincode dump during tests
const DEFAULT_TEST_PATH: &str = "../data/test_magic_tables.bin";

//////////////////////////////////////////////////////////////////////////////
// 1)  Embedded-at-build-time (feature = "load_magic")
//////////////////////////////////////////////////////////////////////////////
#[cfg(feature = "load_magic")]
pub fn load_magic_tables() -> MagicTables {
    const MAGIC_DATA: &[u8] = include_bytes!("../../../../data/magic_tables.bin");

    bincode::deserialize(MAGIC_DATA)
        .expect("Failed to deserialize magic_tables.bin (corrupt or outdated?)")
}

//////////////////////////////////////////////////////////////////////////////
// 2)  Run-time deterministic generation *and* emit bincode to disk
//     (feature = "deterministic_magic")
//////////////////////////////////////////////////////////////////////////////
#[cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]
pub fn load_magic_tables() -> MagicTables {
    use std::path::Path; // only compiled in this branch

    // Where the test file should live
    let path = std::env::var("MAGIC_TEST_BIN").unwrap_or_else(|_| DEFAULT_TEST_PATH.into());

    // 2-a  Try to read an existing file first
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(tables) = bincode::deserialize::<MagicTables>(&bytes) {
            return tables; // ✓ file exists & is valid: we're done
        } else {
            eprintln!("⚠️  Could not deserialize {path} – regenerating …");
        }
    }

    // 2-b  File missing or corrupt → regenerate with the fixed seed
    let tables = generate_magic_tables(MagicTableSeed::Fixed(TEST_SEED))
        .expect("Failed to generate deterministic magic tables");

    // 2-c  Write the file once (skip on parallel writes)
    static WRITE_ONCE: Once = Once::new();
    WRITE_ONCE.call_once(|| {
        // Ensure the parent directory exists
        if let Some(dir) = Path::new(&path).parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!("⚠️  Could not create directory {dir:?}: {e}");
            }
        }
        match bincode::serialize(&tables) {
            Ok(blob) => {
                if let Err(e) = std::fs::write(&path, blob) {
                    eprintln!("⚠️  Could not write {path}: {e}");
                } else {
                    println!("📦  Magic tables dumped to {path}");
                }
            }
            Err(e) => eprintln!("⚠️  Failed to bincode-serialise MagicTables: {e}"),
        }
    });

    tables
}

//////////////////////////////////////////////////////////////////////////////
// 3)  Default: run-time *randomised* generation (no special I/O)
//////////////////////////////////////////////////////////////////////////////
#[cfg(all(not(feature = "load_magic"), not(feature = "deterministic_magic")))]
pub fn load_magic_tables() -> MagicTables {
    generate_magic_tables(MagicTableSeed::Randomized)
        .expect("Failed to generate randomized magic tables")
}
