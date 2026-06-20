use crate::moves::types::Move;

pub const MAX_PLY: usize = 64;

pub struct SearchContext {
    pub killer_moves: Vec<[Option<Move>; 2]>,
    pub history: [[i32; 64]; 64],
    pub pv_table: [[Option<Move>; MAX_PLY]; MAX_PLY],
    pub pv_length: [usize; MAX_PLY],
}

impl Default for SearchContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchContext {
    pub fn new() -> Self {
        Self {
            killer_moves: vec![[None; 2]; 64],
            history: [[0; 64]; 64],
            pv_table: [[None; MAX_PLY]; MAX_PLY],
            pv_length: [0; MAX_PLY],
        }
    }

    pub fn update_killer(&mut self, ply: usize, mv: Move) {
        if self.killer_moves[ply][0] != Some(mv) {
            self.killer_moves[ply][1] = self.killer_moves[ply][0];
            self.killer_moves[ply][0] = Some(mv);
        }
    }

    pub fn update_history(&mut self, mv: Move, depth: i32) {
        let bonus = (depth * depth).min(400);
        self.history[mv.from.index() as usize][mv.to.index() as usize] += bonus;
    }
}
