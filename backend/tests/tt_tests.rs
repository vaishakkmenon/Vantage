use vantage::search::tt::TranspositionTable;

#[test]
fn test_tt_mate_score_normalization() {
    let mut tt = TranspositionTable::new(1); // 1MB table
    let key = 123456789;

    // Scenario: We found a mate at ply 15.
    // We are currently at ply 10.
    // So distance to mate is 5 moves.
    // Engine uses MATE_SCORE - ply.
    // MATE_SCORE = 31000.
    // Score = 31000 - 15 = 30985.
    let search_score = 30985;
    let search_ply = 10;

    // Store it
    tt.save(key, None, search_score, 5, 0, search_ply);

    // 1. Probe at same ply (10)
    // Should return 30985
    let probe_ply_10 = 10;
    if let Some((_, score, _, _)) = tt.probe(key, 0, -50000, 50000, probe_ply_10) {
        assert_eq!(
            score, 30985,
            "Probing at original ply should return original score"
        );
    } else {
        panic!("Entry not found at ply 10");
    }

    // 2. Transposition: Probe at ply 20
    // Same position encountered later in search.
    // Since TT is now passive (normalization happens in search.rs),
    // we expect the EXACT SAME score we stored, regardless of ply.
    let probe_ply_20 = 20;
    if let Some((_, score, _, _)) = tt.probe(key, 0, -50000, 50000, probe_ply_20) {
        assert_eq!(
            score, 30985,
            "Probing at ply 20 should return raw stored score (normalization is now in search)"
        );
    } else {
        panic!("Entry not found at ply 20");
    }
}
