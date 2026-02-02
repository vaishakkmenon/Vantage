#[cfg(test)]
mod tests {
    use vantage::board::Board;
    use vantage::logger::init_logging;
    use vantage::moves::perft::{perft, perft_divide, perft_divide_with_breakdown};
    use vantage::moves::{
        execute::{generate_legal, make_move_basic, undo_move_basic},
        magic::loader::load_magic_tables,
        square_control::in_check,
    };

    const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const KIWI_FEN: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

    const FENS: &[&str] = &[
        // startpos
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        // Kiwipete
        "rnbq1k1r/pppp1ppp/5n2/4p3/1b1P4/5N2/PPPNPPPP/R1BQKB1R w KQkq - 0 1",
        // EP immediately available for White: e5xd6ep
        "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
        // Promotion-ready for White: a7-a8=Q
        "4k3/P7/8/8/8/8/8/4K3 w - - 0 1",
    ];

    fn splitmix64(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    // use std::time::Instant;

    // Replace your existing `perft_startpos_depths` with this block (paste anywhere in tests/perft_tests.rs)

    fn run_startpos_depth(depth: u32, expected_nodes: u64) -> (u64, std::time::Duration) {
        use std::time::Instant;
        let tables = load_magic_tables();

        let mut board = Board::new();
        board.set_fen(START_FEN).expect("valid startpos");

        let start = Instant::now();
        let nodes = perft(&mut board, &tables, depth);
        let elapsed = start.elapsed();

        let secs = elapsed.as_secs_f64().max(1e-9); // avoid div-by-zero on tiny depths
        let nps = (nodes as f64 / secs) as u64;
        println!("d{depth}: nodes={nodes} time={:.3}s nps={}", secs, nps);

        assert_eq!(
            nodes, expected_nodes,
            "Perft mismatch at depth {depth}: got {nodes}, expected {expected_nodes}"
        );
        (nodes, elapsed)
    }

    // Parallelizable per-depth tests (fast on CI)
    #[test]
    fn perft_startpos_d1() {
        let _ = run_startpos_depth(1, 20);
    }
    #[test]
    fn perft_startpos_d2() {
        let _ = run_startpos_depth(2, 400);
    }
    #[test]
    fn perft_startpos_d3() {
        let _ = run_startpos_depth(3, 8_902);
    }
    #[test]
    fn perft_startpos_d4() {
        let _ = run_startpos_depth(4, 197_281);
    }
    #[test]
    fn perft_startpos_d5() {
        let _ = run_startpos_depth(5, 4_865_609);
    }

    // Deep nodes — opt-in on CI
    #[test]
    #[ignore]
    fn perft_startpos_d6() {
        let _ = run_startpos_depth(6, 119_060_324);
    }
    #[test]
    #[ignore]
    fn perft_startpos_d7() {
        let _ = run_startpos_depth(7, 3_195_901_860);
    }

    // Aggregate run that reproduces the TOTAL summary (opt-in)
    #[test]
    #[ignore]
    fn perft_startpos_aggregate() {
        let depths: [(u32, u64); 7] = [
            (1u32, 20u64),
            (2, 400),
            (3, 8_902),
            (4, 197_281),
            (5, 4_865_609),
            (6, 119_060_324),
            (7, 3_195_901_860),
        ];
        let mut total_nodes: u128 = 0;
        let mut total_elapsed = std::time::Duration::ZERO;
        for (d, exp) in depths {
            let (nodes, dt) = run_startpos_depth(d, exp);
            total_nodes += nodes as u128;
            total_elapsed += dt;
        }
        let total_secs = total_elapsed.as_secs_f64().max(1e-9);
        let total_nps = (total_nodes as f64 / total_secs) as u64;
        println!(
            "TOTAL: nodes={} time={:.3}s nps={}",
            total_nodes, total_secs, total_nps
        );
    }

    #[test]
    #[ignore] // Writes to log file, run manually with --ignored
    fn perft_debug_divide() {
        use tracing::info;
        init_logging(
            "logs/perft.log",
            "vantage::moves::perft=trace,vantage::moves::execute=info,info",
        );
        info!("perft_divide started");
        let tables = load_magic_tables();
        let mut board = Board::new();
        board.set_fen(START_FEN).unwrap();
        perft_divide(&mut board, &tables, 3);
    }

    /// Kiwipete known perft counts:
    /// d1 = 48
    /// d2 = 2,039
    /// d3 = 97,862
    /// d4 = 4,085,603
    /// d5 = 193,690,690  (heavy; usually skipped unless highly optimized)
    #[test]
    fn perft_kiwipete_complete() {
        let tables = load_magic_tables();

        let expected = [
            (1, 48u64),
            (2, 2_039),
            (3, 97_862),
            (4, 4_085_603),
            // (5, 193_690_690), // uncomment when fast enough
        ];

        for (depth, expected_nodes) in expected {
            let mut board = Board::new();
            board.set_fen(KIWI_FEN).expect("valid kiwipete");
            let nodes = perft(&mut board, &tables, depth);
            println!("[Kiwipete] Depth: {depth}, Nodes: {nodes}");
            assert_eq!(
                nodes, expected_nodes,
                "[Kiwipete] Perft mismatch at depth {depth}: got {nodes}, expected {expected_nodes}"
            );
        }
    }

    #[test]
    fn perft_kiwipete_divide() {
        let tables = load_magic_tables();
        let mut board = Board::new();
        board.set_fen(KIWI_FEN).unwrap();
        perft_divide(&mut board, &tables, 2);
    }

    #[test]
    fn kiwipete_d2_tally() {
        use std::str::FromStr;
        use vantage::board::Board;
        use vantage::moves::execute::generate_legal;

        let mut b = Board::from_str(KIWI_FEN).unwrap();
        let tables = load_magic_tables();

        let mut roots = vec![];
        let mut scratch = Vec::with_capacity(256);
        generate_legal(&mut b, &tables, &mut roots, &mut scratch);

        let mut nodes = 0u64;
        let mut captures = 0u64;
        let mut ep = 0u64;
        let mut castles = 0u64;
        let mut checks = 0u64;

        for mv in roots {
            let u = vantage::moves::execute::make_move_basic(&mut b, mv);
            // depth-2: enumerate Black replies
            let mut replies = vec![];
            generate_legal(&mut b, &tables, &mut replies, &mut scratch);

            nodes += replies.len() as u64;
            for r in &replies {
                if r.is_capture() {
                    captures += 1;
                }
                if r.is_en_passant() {
                    ep += 1;
                }
                if r.is_castling() {
                    castles += 1;
                }
                // quick check detector
                let uu = vantage::moves::execute::make_move_basic(&mut b, *r);
                let in_chk = vantage::moves::square_control::in_check(&b, b.side_to_move, &tables);
                if in_chk {
                    checks += 1;
                }
                vantage::moves::execute::undo_move_basic(&mut b, uu);
            }

            vantage::moves::execute::undo_move_basic(&mut b, u);
        }

        println!("d2 nodes={nodes} captures={captures} ep={ep} castles={castles} checks={checks}");
        assert_eq!(nodes, 2039);
        assert_eq!(captures, 351);
        assert_eq!(ep, 1);
        assert_eq!(castles, 91);
        assert_eq!(checks, 3);
    }

    #[test]
    fn perft_fuzz() {
        let tables = load_magic_tables();
        let seeds = [1_u64, 2, 3, 42, 99];
        for &seed0 in &seeds {
            for &fen in FENS {
                let mut board = Board::new();
                board.set_fen(fen).expect("fen");
                let mut seed = seed0;
                for _ply in 0..200 {
                    // parity before
                    assert_eq!(board.zobrist, board.compute_zobrist_full());

                    // generate legal
                    let mut moves = Vec::new();
                    let mut scratch = Vec::with_capacity(256);
                    generate_legal(&mut board, &tables, &mut moves, &mut scratch);
                    if moves.is_empty() {
                        // optional smoke on terminal nodes
                        let _ = in_check(&board, board.side_to_move, &tables);
                        break;
                    }

                    // pick a move via tiny RNG
                    seed = splitmix64(seed);
                    let mv = moves[(seed as usize) % moves.len()];

                    let u = make_move_basic(&mut board, mv);
                    undo_move_basic(&mut board, u);

                    // parity after
                    assert_eq!(board.zobrist, board.compute_zobrist_full());
                }
            }
        }
    }

    #[test]
    fn divide_startpos_d2_matches_total() {
        let tables = load_magic_tables();
        let mut b = Board::new();
        b.set_fen(START_FEN).unwrap();
        let rows = perft_divide_with_breakdown(&mut b, &tables, 2);
        let total: u64 = rows.iter().map(|(_, pc)| pc.nodes).sum();
        assert_eq!(total, 400);
    }
}

#[cfg(debug_assertions)]
#[test]
fn make_undo_fuzz_sanity() {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use vantage::board::Board;
    use vantage::moves::execute::{generate_legal, make_move_basic, undo_move_basic};
    use vantage::moves::magic::loader::load_magic_tables;

    let tables = load_magic_tables();
    let mut b = Board::new();
    let mut rng = StdRng::seed_from_u64(42);
    let plies = 1000usize;

    for _ in 0..plies {
        let mut ms = Vec::with_capacity(64);
        let mut scratch = Vec::with_capacity(256);
        generate_legal(&mut b, &tables, &mut ms, &mut scratch);
        if ms.is_empty() {
            break;
        }

        let idx = rng.random_range(0..ms.len());
        let u = make_move_basic(&mut b, ms[idx]);

        // Hash should be coherent after make
        #[cfg(debug_assertions)]
        {
            b.assert_hash();
        }

        undo_move_basic(&mut b, u);

        // Hash should be coherent after undo
        #[cfg(debug_assertions)]
        {
            b.assert_hash();
        }
    }
}
