use crate::moves::types::Move;

// Make sure MATE_THRESHOLD matches what we define in search.rs (30000)
pub const MATE_THRESHOLD: i32 = 30000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeType {
    Exact = 0,
    LowerBound = 1, // Beta cutoff (failed high)
    UpperBound = 2, // Alpha cutoff (failed low)
}

#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,
    pub best_move: Option<Move>,
    pub score: i16,
    pub depth: u8,
    pub bound: u8, // 0=Exact, 1=Lower, 2=Upper
    pub generation: u8,
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    pub generation: u8,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        // Allocate TT based on size in MB.
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;

        // Round down to power of 2
        let mut capacity = 1;
        while capacity * 2 <= num_entries {
            capacity *= 2;
        }

        Self {
            entries: vec![
                TTEntry {
                    key: 0,
                    best_move: None,
                    score: 0,
                    depth: 0,
                    bound: 0,
                    generation: 0,
                };
                capacity
            ],

            generation: 0,
        }
    }

    pub fn new_search(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.key = 0;
            entry.best_move = None;
            entry.score = 0;
            entry.depth = 0;
            entry.bound = 0;
            entry.generation = 0;
        }
        self.generation = 0;
    }

    pub fn save(
        &mut self,
        key: u64,
        mv: Option<Move>,
        score: i32,
        depth: u8,
        bound: u8,
        _ply: i32,
    ) {
        // Safety clamp
        let score_i16 = score.clamp(-32000, 32000) as i16;

        let index = (key as usize) & (self.entries.len() - 1);
        let entry = &mut self.entries[index];

        if entry.key == 0 || depth >= entry.depth || entry.generation != self.generation {
            // Preserve existing best_move if the new entry doesn't provide one.
            let best_move = if mv.is_some() { mv } else { entry.best_move };

            entry.key = key;
            entry.best_move = best_move;
            entry.score = score_i16;
            entry.depth = depth;
            entry.bound = bound;
            entry.generation = self.generation;
        }
    }

    pub fn probe(
        &self,
        key: u64,
        _depth: u8,
        _alpha: i32,
        _beta: i32,
        _ply: i32,
    ) -> Option<(Option<Move>, i32, u8, u8)> {
        let index = (key as usize) & (self.entries.len() - 1);
        let entry = &self.entries[index];

        if entry.key == key {
            let score = entry.score as i32;
            return Some((entry.best_move, score, entry.depth, entry.bound));
        }
        None
    }
}
