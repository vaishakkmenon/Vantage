use crate::moves::types::Move;

pub struct SearchContext {
    pub killer_moves: Vec<[Option<Move>; 2]>,
    pub history: [[i32; 64]; 64],
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
