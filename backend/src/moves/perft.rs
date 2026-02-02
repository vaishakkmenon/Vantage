use crate::board::Board;
use crate::moves::{
    execute::{generate_legal, make_move_basic, undo_move_basic},
    magic::MagicTables,
    square_control::in_check,
    types::Move,
};
use tracing::{debug, instrument};

const MAX_LOG_DEPTH: u32 = 3; // only trace details for shallow nodes
const MAX_PERFT_DEPTH: usize = 20; // support perft up to depth 20

pub struct PerftCounters {
    pub nodes: u64,
    pub captures: u64,
    pub ep_captures: u64,
    pub castles: u64,
    pub promotions: u64,
    pub checks: u64,
    pub checkmates: u64,
}

impl PerftCounters {
    pub fn zero() -> Self {
        Self {
            nodes: 0,
            captures: 0,
            ep_captures: 0,
            castles: 0,
            promotions: 0,
            checks: 0,
            checkmates: 0,
        }
    }
    pub fn add(&mut self, o: &PerftCounters) {
        self.nodes += o.nodes;
        self.captures += o.captures;
        self.ep_captures += o.ep_captures;
        self.castles += o.castles;
        self.promotions += o.promotions;
        self.checks += o.checks;
        self.checkmates += o.checkmates;
    }
}

// Helper functions for creating buffer arrays
fn create_move_buffer_array() -> [Vec<Move>; MAX_PERFT_DEPTH] {
    std::array::from_fn(|_| Vec::with_capacity(64))
}

fn create_pseudo_buffer_array() -> [Vec<Move>; MAX_PERFT_DEPTH] {
    std::array::from_fn(|_| Vec::with_capacity(256))
}

#[inline]
fn sq_as_a1_zero(idx: u8) -> String {
    // Assumes 0 = a1, 63 = h8 (file = idx % 8, rank = idx / 8)
    let file = idx % 8;
    let rank = idx / 8;
    let f = (b'a' + file) as char;
    let r = (b'1' + rank) as char;
    format!("{f}{r}")
}

#[inline]
fn sq_as_a8_zero(idx: u8) -> String {
    // Assumes 0 = a8, 63 = h1 (file = idx % 8, rank = 7 - idx / 8)
    let file = idx % 8;
    let rank = 7 - (idx / 8);
    let f = (b'a' + file) as char;
    let r = (b'1' + rank) as char;
    format!("{f}{r}")
}

// Recursive perft implementation with per-ply buffers
fn perft_recursive(
    board: &mut Board,
    tables: &MagicTables,
    depth: u32,
    ply: usize,
    move_buffers: &mut [Vec<Move>],
    pseudo_buffers: &mut [Vec<Move>],
) -> u64 {
    if depth == 0 {
        return 1;
    }

    // Generate moves (reusing buffers)
    {
        let moves = &mut move_buffers[ply];
        let pseudo = &mut pseudo_buffers[ply];
        moves.clear();
        generate_legal(board, tables, moves, pseudo);
    }

    // Copy moves out to avoid borrow conflict
    let move_count = move_buffers[ply].len();
    let mut node_count = 0;

    for i in 0..move_count {
        let mv = move_buffers[ply][i];
        let undo = make_move_basic(board, mv);

        // Child uses ply+1 buffers
        node_count += perft_recursive(
            board,
            tables,
            depth - 1,
            ply + 1,
            move_buffers,
            pseudo_buffers,
        );

        undo_move_basic(board, undo);
    }

    node_count
}

#[instrument(skip(board, tables), fields(depth))]
pub fn perft(board: &mut Board, tables: &MagicTables, depth: u32) -> u64 {
    if depth as usize > MAX_PERFT_DEPTH {
        panic!(
            "Depth {} exceeds MAX_PERFT_DEPTH {}",
            depth, MAX_PERFT_DEPTH
        );
    }

    // Allocate buffers once
    let mut move_buffers = create_move_buffer_array();
    let mut pseudo_buffers = create_pseudo_buffer_array();

    // Start recursive search
    perft_recursive(
        board,
        tables,
        depth,
        0,
        &mut move_buffers,
        &mut pseudo_buffers,
    )
}

#[instrument(skip(board, tables), fields(depth))]
pub fn perft_divide(board: &mut Board, tables: &MagicTables, depth: u32) -> u64 {
    if depth as usize > MAX_PERFT_DEPTH {
        panic!(
            "Depth {} exceeds MAX_PERFT_DEPTH {}",
            depth, MAX_PERFT_DEPTH
        );
    }

    // Allocate buffers once
    let mut move_buffers = create_move_buffer_array();
    let mut pseudo_buffers = create_pseudo_buffer_array();

    // Generate root moves
    {
        let moves = &mut move_buffers[0];
        let pseudo = &mut pseudo_buffers[0];
        moves.clear();
        generate_legal(board, tables, moves, pseudo);

        if depth <= MAX_LOG_DEPTH {
            debug!(depth, moves = moves.len(), "divide: root legal moves");
        }
    }

    let mut total = 0;
    let move_count = move_buffers[0].len();

    for i in 0..move_count {
        let mv = move_buffers[0][i];

        if depth <= MAX_LOG_DEPTH {
            let from = mv.from.index();
            let to = mv.to.index();

            debug!(
                %mv, depth,
                from_idx = from, to_idx = to,
                from_a1 = %sq_as_a1_zero(from), to_a1 = %sq_as_a1_zero(to),
                from_a8 = %sq_as_a8_zero(from), to_a8 = %sq_as_a8_zero(to),
                "divide: exploring root move (decode check)"
            );
        }

        let undo = make_move_basic(board, mv);

        let count = if depth == 1 {
            1
        } else {
            perft_recursive(
                board,
                tables,
                depth - 1,
                1,
                &mut move_buffers,
                &mut pseudo_buffers,
            )
        };

        undo_move_basic(board, undo);

        if depth <= MAX_LOG_DEPTH {
            debug!(%mv, nodes = count, "divide: root child total");
        }

        println!("{}: {}", mv, count);
        total += count;
    }

    debug!(depth, total, "divide: total");
    println!("Total: {}", total);
    total
}

// Recursive implementation with per-ply buffers for breakdown
fn perft_count_recursive(
    board: &mut Board,
    tables: &MagicTables,
    depth: u32,
    ply: usize,
    out: &mut PerftCounters,
    move_buffers: &mut [Vec<Move>],
    pseudo_buffers: &mut [Vec<Move>],
) {
    if depth == 0 {
        out.nodes += 1;

        // Leaf: check/mate status
        let side_in_check = in_check(board, board.side_to_move, tables);
        if side_in_check {
            out.checks += 1;
        }

        {
            let tmp = &mut move_buffers[ply];
            let scratch = &mut pseudo_buffers[ply];
            tmp.clear();
            generate_legal(board, tables, tmp, scratch);
        }

        if move_buffers[ply].is_empty() && side_in_check {
            out.checkmates += 1;
        }
        return;
    }

    // Generate moves
    {
        let moves = &mut move_buffers[ply];
        let pseudo = &mut pseudo_buffers[ply];
        moves.clear();
        generate_legal(board, tables, moves, pseudo);
    }

    let move_count = move_buffers[ply].len();
    for i in 0..move_count {
        let mv = move_buffers[ply][i];

        // --- breakdown tags at this ply (edge-based) ---
        if mv.is_capture() {
            out.captures += 1;
            if mv.is_en_passant() {
                out.ep_captures += 1;
            }
        }
        if mv.is_castling() {
            out.castles += 1;
        }
        if mv.promotion.is_some() {
            out.promotions += 1;
        }

        #[cfg(debug_assertions)]
        let z0 = board.zobrist;

        let undo = make_move_basic(board, mv);
        perft_count_recursive(
            board,
            tables,
            depth - 1,
            ply + 1,
            out,
            move_buffers,
            pseudo_buffers,
        );
        undo_move_basic(board, undo);

        #[cfg(debug_assertions)]
        {
            debug_assert_eq!(board.zobrist, z0, "zobrist changed across make/undo");
            debug_assert_eq!(
                board.compute_zobrist_full(),
                board.zobrist,
                "full recompute mismatch"
            );
        }
    }
}

pub fn perft_count_with_breakdown(
    board: &mut Board,
    tables: &MagicTables,
    depth: u32,
    out: &mut PerftCounters,
) {
    if depth as usize > MAX_PERFT_DEPTH {
        panic!(
            "Depth {} exceeds MAX_PERFT_DEPTH {}",
            depth, MAX_PERFT_DEPTH
        );
    }

    let mut move_buffers = create_move_buffer_array();
    let mut pseudo_buffers = create_pseudo_buffer_array();

    perft_count_recursive(
        board,
        tables,
        depth,
        0,
        out,
        &mut move_buffers,
        &mut pseudo_buffers,
    );
}

pub fn perft_divide_with_breakdown(
    board: &mut Board,
    tables: &MagicTables,
    depth: u32,
) -> Vec<(Move, PerftCounters)> {
    if depth as usize > MAX_PERFT_DEPTH {
        panic!(
            "Depth {} exceeds MAX_PERFT_DEPTH {}",
            depth, MAX_PERFT_DEPTH
        );
    }

    // Allocate buffers once
    let mut move_buffers = create_move_buffer_array();
    let mut pseudo_buffers = create_pseudo_buffer_array();

    // Generate root moves
    {
        let moves = &mut move_buffers[0];
        let pseudo = &mut pseudo_buffers[0];
        moves.clear();
        generate_legal(board, tables, moves, pseudo);
    }

    let move_count = move_buffers[0].len();
    let mut out = Vec::with_capacity(move_count);

    for i in 0..move_count {
        let mv = move_buffers[0][i];
        let undo = make_move_basic(board, mv);
        let mut pc = PerftCounters::zero();

        if depth > 1 {
            perft_count_recursive(
                board,
                tables,
                depth - 1,
                1,
                &mut pc,
                &mut move_buffers,
                &mut pseudo_buffers,
            );
        } else {
            pc.nodes = 1;
        }

        undo_move_basic(board, undo);
        out.push((mv, pc));
    }
    out
}
