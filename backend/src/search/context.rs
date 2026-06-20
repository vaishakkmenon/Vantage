use crate::moves::types::Move;

/// Represents the context for a search algorithm.
///
/// This struct holds various data structures and arrays that are used to keep track of
/// important information during the search process, such as killer moves, history heuristic,
/// principal variation (PV) table, and more.
pub const MAX_PLY: usize = 64;

pub struct SearchContext {
    /// Array to store killer moves for each ply.
    pub killer_moves: Vec<[Option<Move>; 2]>,
    
    /// History heuristic array to encourage moves that have been successful in the past.
    pub history: [[i32; 64]; 64],
    
    /// Principal Variation (PV) table to store the best move sequence found so far.
    pub pv_table: [[Option<Move>; MAX_PLY]; MAX_PLY],
    
    /// Array to store the length of the principal variation for each ply.
    pub pv_length: [usize; MAX_PLY],
}

impl Default for SearchContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchContext {
    /// Initializes a new `SearchContext` with default values.
    pub fn new() -> Self {
        Self {
            killer_moves: vec![[None; 2]; 64],
            history: [[0; 64]; 64],
            pv_table: [[None; MAX_PLY]; MAX_PLY],
            pv_length: [0; MAX_PLY],
        }
    }

    /// Updates the killer moves for a given ply.
    pub fn update_killer(&mut self, ply: usize, mv: Move) {
        if self.killer_moves[ply][0] != Some(mv) {
            self.killer_moves[ply][1] = self.killer_moves[ply][0];
            self.killer_moves[ply][0] = Some(mv);
        }
    }

    /// Updates the history heuristic for a given move.
    pub fn update_history(&mut self, mv: Move, depth: i32) {
        let bonus = (depth * depth).min(400);
        self.history[mv.from.index() as usize][mv.to.index() as usize] += bonus;
    }
}
