use rand::Rng;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use super::polyglot_entry::PolyglotEntry;
use super::polyglot_hash::compute_polyglot_hash;
use crate::board::Board;
use crate::moves::types::Move;

pub struct PolyglotBook {
    entries: Vec<PolyglotEntry>,
}

impl PolyglotBook {
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        let count = buffer.len() / 16;
        let mut entries = Vec::with_capacity(count);

        for i in 0..count {
            let start = i * 16;
            entries.push(PolyglotEntry::from_bytes(&buffer[start..start + 16]));
        }

        Ok(Self { entries })
    }

    pub fn probe(&self, board: &Board) -> Option<Move> {
        let hash = compute_polyglot_hash(board);

        let idx = self.entries.partition_point(|e| e.key < hash);

        if idx >= self.entries.len() || self.entries[idx].key != hash {
            return None;
        }

        let mut candidates = Vec::new();
        let mut i = idx;
        while i < self.entries.len() && self.entries[i].key == hash {
            candidates.push(&self.entries[i]);
            i += 1;
        }

        let total_weight: u32 = candidates.iter().map(|e| e.weight as u32).sum();

        if total_weight == 0 {
            // All weights are zero, just pick the first valid one
            for entry in &candidates {
                if let Some(mv) = entry.decode_move(board) {
                    return Some(mv);
                }
            }
            return None;
        }

        let mut rng = rand::rng();
        let mut pick = rng.random_range(0..total_weight);

        for entry in candidates {
            let w = entry.weight as u32;
            if pick < w {
                return entry.decode_move(board);
            }
            pick -= w;
        }

        None
    }

    /// Returns the number of entries in the book
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the book is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
