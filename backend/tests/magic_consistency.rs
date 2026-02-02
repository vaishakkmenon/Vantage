#![cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]
//! tests/magic_table_consistency.rs
//! Verify that the embedded and regenerated magic tables are identical.

use vantage::moves::magic::{
    loader::load_magic_tables,
    precompute::{MagicTableSeed, generate_magic_tables},
};

use indicatif::{ProgressBar, ProgressStyle};

/// Seed that was used to embed the tables at build-time (0x45 == 69)
const EMBEDDED_SEED: u64 = 0x45;

#[test]
#[cfg(all(not(feature = "load_magic"), feature = "deterministic_magic"))]
fn test_magic_table_consistency() {
    // 1) Embedded tables (via build-script or include_bytes!)
    let embedded = load_magic_tables();
    println!("Magic Tables Loaded In!");
    // 2) Deterministic regeneration (same seed)
    let generated = generate_magic_tables(MagicTableSeed::Fixed(EMBEDDED_SEED))
        .expect("Failed to regenerate magic tables");

    // ── Progress bar setup ────────────────────────────────────────────────
    // 64 rook entries + 64 bishop entries
    let pb = ProgressBar::new(128);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>3}/{len:3} squares",
        )
        .unwrap()
        .progress_chars("##-"),
    );

    // ── Rooks ─────────────────────────────────────────────────────────────
    for (i, (expected, actual)) in generated
        .rook
        .entries
        .iter()
        .zip(&embedded.rook.entries)
        .enumerate()
    {
        assert_eq!(
            expected.magic, actual.magic,
            "Rook magic mismatch at square {i}"
        );
        assert_eq!(
            expected.shift, actual.shift,
            "Rook shift mismatch at square {i}"
        );
        assert_eq!(
            &*expected.table, &*actual.table,
            "Rook table mismatch at square {i}"
        );
        pb.inc(1);
    }

    // ── Bishops ───────────────────────────────────────────────────────────
    for (i, (expected, actual)) in generated
        .bishop
        .entries
        .iter()
        .zip(&embedded.bishop.entries)
        .enumerate()
    {
        assert_eq!(
            expected.magic, actual.magic,
            "Bishop magic mismatch at square {i}"
        );
        assert_eq!(
            expected.shift, actual.shift,
            "Bishop shift mismatch at square {i}"
        );
        assert_eq!(
            &*expected.table, &*actual.table,
            "Bishop table mismatch at square {i}"
        );
        pb.inc(1);
    }

    pb.finish_with_message("✓ magic tables identical");
}
