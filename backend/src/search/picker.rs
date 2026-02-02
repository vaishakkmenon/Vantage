//! Staged Move Picker for lazy move generation and ordering.
//!
//! This module implements a Just-In-Time staged move picker that generates
//! moves on demand rather than generating all moves upfront. The stages are:
//!
//! HashMove -> GoodCaptures -> Killer1 -> Killer2 -> Quiets -> BadCaptures
//!
//! If an early move causes a beta cutoff, later moves are never generated.

use crate::board::Board;
use crate::moves::execute::is_legal_move;
use crate::moves::magic::MagicTables;
use crate::moves::movegen::{generate_pseudo_legal_captures, generate_pseudo_legal_quiets};
use crate::moves::types::Move;
use crate::search::ordering::mvv_lva_score;
use crate::search::see::SeeExt;
use arrayvec::ArrayVec;

/// The current stage of move generation/picking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerStage {
    HashMove,
    GenerateCaptures,
    GoodCaptures,
    Killer1,
    Killer2,
    GenerateQuiets,
    Quiets,
    BadCaptures,
    Done,
}

/// Staged move picker that generates moves lazily on demand.
pub struct MovePicker {
    stage: PickerStage,

    // Move buffers (stack-allocated)
    good_captures: ArrayVec<Move, 64>,
    bad_captures: ArrayVec<Move, 64>,
    quiets: ArrayVec<Move, 256>,

    // Scores for pick-best selection
    good_capture_scores: ArrayVec<i32, 64>,
    quiet_scores: ArrayVec<i32, 256>,

    // Buffer indices for pick-best iteration
    good_cap_idx: usize,
    quiet_idx: usize,
    bad_cap_idx: usize,

    // Special moves
    hash_move: Option<Move>,
    killers: [Option<Move>; 2],

    // Mode
    captures_only: bool, // For quiescence search
}

impl MovePicker {
    /// Create a new MovePicker.
    ///
    /// # Arguments
    /// * `hash_move` - The hash move from the transposition table (if any)
    /// * `killers` - Killer moves for this ply
    /// * `captures_only` - If true, skip killers and quiets (for quiescence search)
    pub fn new(hash_move: Option<Move>, killers: [Option<Move>; 2], captures_only: bool) -> Self {
        Self {
            stage: PickerStage::HashMove,
            good_captures: ArrayVec::new(),
            bad_captures: ArrayVec::new(),
            quiets: ArrayVec::new(),
            good_capture_scores: ArrayVec::new(),
            quiet_scores: ArrayVec::new(),
            good_cap_idx: 0,
            quiet_idx: 0,
            bad_cap_idx: 0,
            hash_move,
            killers,
            captures_only,
        }
    }

    /// Check if a move is the hash move.
    #[inline]
    fn is_hash_move(&self, mv: Move) -> bool {
        if let Some(hm) = self.hash_move {
            mv.from == hm.from && mv.to == hm.to && mv.promotion == hm.promotion
        } else {
            false
        }
    }

    /// Check if a move is a killer move.
    #[inline]
    fn is_killer(&self, mv: Move) -> bool {
        for killer in &self.killers {
            if let Some(k) = killer
                && mv.from == k.from && mv.to == k.to && mv.promotion == k.promotion {
                    return true;
                }
        }
        false
    }

    /// Check if a move is a duplicate (hash move or killer).
    #[inline]
    fn is_duplicate(&self, mv: Move) -> bool {
        self.is_hash_move(mv) || self.is_killer(mv)
    }

    /// Generate captures and classify them into good (SEE >= 0) and bad (SEE < 0).
    fn generate_and_classify_captures(&mut self, board: &Board, tables: &MagicTables) {
        let mut captures: ArrayVec<Move, 128> = ArrayVec::new();
        generate_pseudo_legal_captures(board, tables, &mut captures);

        for mv in captures {
            // Skip hash move - it was already tried
            if self.is_hash_move(mv) {
                continue;
            }

            // Calculate MVV-LVA score
            let mvv_lva = mvv_lva_score(mv, board);

            // Use SEE to classify captures
            if board.static_exchange_eval(mv, 0, tables) {
                // Good capture (SEE >= 0)
                self.good_captures.push(mv);
                self.good_capture_scores.push(mvv_lva);
            } else {
                // Bad capture (SEE < 0) - save for later
                self.bad_captures.push(mv);
            }
        }
    }

    /// Generate quiet moves and score them with history heuristic and pawn advancement bonus.
    fn generate_quiets(&mut self, board: &Board, tables: &MagicTables, history: &[[i32; 64]; 64]) {
        use crate::board::{Color, Piece};

        generate_pseudo_legal_quiets(board, tables, &mut self.quiets);

        // Score each quiet move with history + pawn advancement bonus
        for mv in &self.quiets {
            let mut score = history[mv.from.index() as usize][mv.to.index() as usize];

            // Pawn advancement bonus: encourage pushing pawns toward promotion
            if mv.piece == Piece::Pawn {
                let to_rank = mv.to.index() / 8;
                let from_rank = mv.from.index() / 8;

                // Check if pawn is advancing (direction depends on color)
                let is_advancing = match board.side_to_move {
                    Color::White => to_rank > from_rank,
                    Color::Black => to_rank < from_rank,
                };

                if is_advancing {
                    // Bonus for reaching ranks 4/5 (0-indexed: 3, 4)
                    if to_rank == 3 || to_rank == 4 {
                        score += 1000;
                    }
                    // Higher bonus for reaching ranks 6/7 (0-indexed: 5, 6)
                    if to_rank == 5 || to_rank == 6 {
                        score += 2000;
                    }
                }
            }

            self.quiet_scores.push(score);
        }
    }

    /// Pick the best capture from the remaining good captures using selection sort.
    /// Returns None if no captures remain.
    fn pick_best_capture(&mut self) -> Option<Move> {
        if self.good_cap_idx >= self.good_captures.len() {
            return None;
        }

        // Find the best remaining capture
        let mut best_idx = self.good_cap_idx;
        let mut best_score = self.good_capture_scores[best_idx];

        for i in (self.good_cap_idx + 1)..self.good_captures.len() {
            if self.good_capture_scores[i] > best_score {
                best_score = self.good_capture_scores[i];
                best_idx = i;
            }
        }

        // Swap best to current position
        self.good_captures.swap(self.good_cap_idx, best_idx);
        self.good_capture_scores.swap(self.good_cap_idx, best_idx);

        let mv = self.good_captures[self.good_cap_idx];
        self.good_cap_idx += 1;
        Some(mv)
    }

    /// Pick the best quiet from the remaining quiets using selection sort.
    /// Returns None if no quiets remain.
    fn pick_best_quiet(&mut self) -> Option<Move> {
        if self.quiet_idx >= self.quiets.len() {
            return None;
        }

        // Find the best remaining quiet
        let mut best_idx = self.quiet_idx;
        let mut best_score = self.quiet_scores[best_idx];

        for i in (self.quiet_idx + 1)..self.quiets.len() {
            if self.quiet_scores[i] > best_score {
                best_score = self.quiet_scores[i];
                best_idx = i;
            }
        }

        // Swap best to current position
        self.quiets.swap(self.quiet_idx, best_idx);
        self.quiet_scores.swap(self.quiet_idx, best_idx);

        let mv = self.quiets[self.quiet_idx];
        self.quiet_idx += 1;
        Some(mv)
    }

    /// Returns the next legal move, or None when exhausted.
    ///
    /// CRITICAL: This is loop-based, NOT recursive, to prevent stack overflow.
    pub fn next(
        &mut self,
        board: &mut Board,
        tables: &MagicTables,
        history: &[[i32; 64]; 64],
    ) -> Option<Move> {
        loop {
            match self.stage {
                PickerStage::HashMove => {
                    self.stage = PickerStage::GenerateCaptures;
                    if let Some(hm) = self.hash_move {
                        // Validate hash move is pseudo-legal and legal
                        if is_pseudo_legal(board, hm, tables) && is_legal_move(board, hm, tables) {
                            return Some(hm);
                        }
                    }
                    // continue loop to next stage
                }

                PickerStage::GenerateCaptures => {
                    self.generate_and_classify_captures(board, tables);
                    self.stage = PickerStage::GoodCaptures;
                }

                PickerStage::GoodCaptures => {
                    while let Some(mv) = self.pick_best_capture() {
                        // Skip duplicates (hash move already handled above)
                        if self.is_hash_move(mv) {
                            continue;
                        }
                        if is_legal_move(board, mv, tables) {
                            return Some(mv);
                        }
                    }
                    // All good captures exhausted
                    self.stage = if self.captures_only {
                        PickerStage::BadCaptures // Skip killers/quiets in qsearch
                    } else {
                        PickerStage::Killer1
                    };
                }

                PickerStage::Killer1 => {
                    self.stage = PickerStage::Killer2;
                    if let Some(k1) = self.killers[0] {
                        // Killers are quiet moves - skip if it's a capture or the hash move
                        if !k1.is_capture()
                            && !self.is_hash_move(k1)
                            && is_pseudo_legal(board, k1, tables)
                            && is_legal_move(board, k1, tables)
                        {
                            return Some(k1);
                        }
                    }
                }

                PickerStage::Killer2 => {
                    self.stage = PickerStage::GenerateQuiets;
                    if let Some(k2) = self.killers[1] {
                        // Skip if capture, hash move, or same as killer 1
                        if !k2.is_capture() && !self.is_hash_move(k2) {
                            // Also check not same as killer 1
                            let is_k1 = if let Some(k1) = self.killers[0] {
                                k2.from == k1.from && k2.to == k1.to && k2.promotion == k1.promotion
                            } else {
                                false
                            };
                            if !is_k1
                                && is_pseudo_legal(board, k2, tables)
                                && is_legal_move(board, k2, tables)
                            {
                                return Some(k2);
                            }
                        }
                    }
                }

                PickerStage::GenerateQuiets => {
                    self.generate_quiets(board, tables, history);
                    self.stage = PickerStage::Quiets;
                }

                PickerStage::Quiets => {
                    while let Some(mv) = self.pick_best_quiet() {
                        // Skip hash move and killers (already tried)
                        if self.is_duplicate(mv) {
                            continue;
                        }
                        if is_legal_move(board, mv, tables) {
                            return Some(mv);
                        }
                    }
                    self.stage = PickerStage::BadCaptures;
                }

                PickerStage::BadCaptures => {
                    while self.bad_cap_idx < self.bad_captures.len() {
                        let mv = self.bad_captures[self.bad_cap_idx];
                        self.bad_cap_idx += 1;
                        // Skip hash move
                        if self.is_hash_move(mv) {
                            continue;
                        }
                        if is_legal_move(board, mv, tables) {
                            return Some(mv);
                        }
                    }
                    self.stage = PickerStage::Done;
                }

                PickerStage::Done => return None,
            }
        }
    }
}

/// Check if a move is pseudo-legal (valid move for the current position).
/// This validates that the move could have been generated by the move generator.
fn is_pseudo_legal(board: &Board, mv: Move, tables: &MagicTables) -> bool {
    use crate::board::{Color, Piece};
    use crate::moves::king::KING_ATTACKS;
    use crate::moves::knight::KNIGHT_ATTACKS;
    use crate::moves::pawn::{BLACK_PAWN_ATTACKS, WHITE_PAWN_ATTACKS};

    let color = board.side_to_move;
    let from_idx = mv.from.index() as usize;
    let to_idx = mv.to.index() as usize;
    let from_bb = 1u64 << from_idx;
    let to_bb = 1u64 << to_idx;

    // Check that the piece exists on the from square
    if board.pieces(mv.piece, color) & from_bb == 0 {
        return false;
    }

    // Check that the destination isn't occupied by a friendly piece
    let friendly = board.occupancy(color);
    if friendly & to_bb != 0 {
        return false;
    }

    // For captures, check there's an enemy piece (or it's en passant)
    let enemy = board.opponent_occupancy(color);
    if mv.is_capture() && !mv.is_en_passant() && enemy & to_bb == 0 {
        return false;
    }

    // Don't allow capturing the king
    let enemy_king = board.pieces(Piece::King, color.opposite());
    if to_bb & enemy_king != 0 {
        return false;
    }

    // Piece-specific validation
    match mv.piece {
        Piece::Pawn => {
            let pawn_attacks = match color {
                Color::White => WHITE_PAWN_ATTACKS[from_idx],
                Color::Black => BLACK_PAWN_ATTACKS[from_idx],
            };

            if mv.is_en_passant() {
                // En passant must target the EP square and be a pawn attack
                if let Some(ep_sq) = board.en_passant {
                    if to_idx != ep_sq.index() as usize {
                        return false;
                    }
                    if pawn_attacks & to_bb == 0 {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if mv.is_capture() {
                // Normal capture - must be a pawn attack
                if pawn_attacks & to_bb == 0 {
                    return false;
                }
            } else {
                // Pawn push - check direction and blockers
                let empty = !board.occupied();
                let (push_delta, double_rank, double_delta): (i32, u64, i32) = match color {
                    Color::White => (8, 0x0000_0000_0000_FF00, 16),
                    Color::Black => (-8, 0x00FF_0000_0000_0000, -16),
                };

                if mv.is_double_pawn_push() {
                    // Double push from starting rank
                    if from_bb & double_rank == 0 {
                        return false;
                    }
                    let expected_to = (from_idx as i32 + double_delta) as usize;
                    if to_idx != expected_to {
                        return false;
                    }
                    // Check both squares are empty
                    let middle = (from_idx as i32 + push_delta) as usize;
                    if empty & (1u64 << middle) == 0 || empty & to_bb == 0 {
                        return false;
                    }
                } else {
                    // Single push
                    let expected_to = (from_idx as i32 + push_delta) as usize;
                    if to_idx != expected_to {
                        return false;
                    }
                    if empty & to_bb == 0 {
                        return false;
                    }
                }
            }

            // Promotion validation
            if mv.is_promotion() {
                let promo_rank = match color {
                    Color::White => 7,
                    Color::Black => 0,
                };
                if to_idx / 8 != promo_rank {
                    return false;
                }
            }
        }
        Piece::Knight => {
            if KNIGHT_ATTACKS[from_idx] & to_bb == 0 {
                return false;
            }
        }
        Piece::Bishop => {
            let attacks = tables.bishop.get_attacks(from_idx, board.occupied());
            if attacks & to_bb == 0 {
                return false;
            }
        }
        Piece::Rook => {
            let attacks = tables.rook.get_attacks(from_idx, board.occupied());
            if attacks & to_bb == 0 {
                return false;
            }
        }
        Piece::Queen => {
            let attacks = tables.queen_attacks(from_idx, board.occupied());
            if attacks & to_bb == 0 {
                return false;
            }
        }
        Piece::King => {
            if mv.is_castling() {
                // Castling validation - check rights and path
                let occ = board.occupied();
                if mv.is_kingside_castle() {
                    if !board.has_kingside_castle(color) {
                        return false;
                    }
                    let between = match color {
                        Color::White => 0x0000_0000_0000_0060,
                        Color::Black => 0x6000_0000_0000_0000,
                    };
                    if occ & between != 0 {
                        return false;
                    }
                } else {
                    if !board.has_queenside_castle(color) {
                        return false;
                    }
                    let between = match color {
                        Color::White => 0x0000_0000_0000_000E,
                        Color::Black => 0x0E00_0000_0000_0000,
                    };
                    if occ & between != 0 {
                        return false;
                    }
                }
            } else if KING_ATTACKS[from_idx] & to_bb == 0 {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::moves::execute::generate_legal;
    use crate::moves::magic::loader::load_magic_tables;
    use std::str::FromStr;

    fn tables() -> MagicTables {
        load_magic_tables()
    }

    #[test]
    fn test_picker_generates_all_legal_moves() {
        let tables = tables();
        let mut board = Board::new();
        let history = [[0i32; 64]; 64];

        // Get all legal moves the traditional way
        let mut legal_moves: ArrayVec<Move, 256> = ArrayVec::new();
        let mut scratch: ArrayVec<Move, 256> = ArrayVec::new();
        generate_legal(&mut board, &tables, &mut legal_moves, &mut scratch);

        // Get all moves from the picker
        let mut picker = MovePicker::new(None, [None, None], false);
        let mut picker_moves: Vec<Move> = Vec::new();
        while let Some(mv) = picker.next(&mut board, &tables, &history) {
            picker_moves.push(mv);
        }

        // Both should have the same number of moves
        assert_eq!(
            legal_moves.len(),
            picker_moves.len(),
            "Picker generated {} moves, expected {}",
            picker_moves.len(),
            legal_moves.len()
        );

        // All picker moves should be in the legal moves list
        for mv in &picker_moves {
            let found = legal_moves
                .iter()
                .any(|lm| lm.from == mv.from && lm.to == mv.to && lm.promotion == mv.promotion);
            assert!(found, "Picker generated illegal move: {}", mv.to_uci());
        }
    }

    #[test]
    fn test_picker_captures_only_mode() {
        let tables = tables();
        // Position with captures available
        let mut board =
            Board::from_str("r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4")
                .unwrap();
        let history = [[0i32; 64]; 64];

        let mut picker = MovePicker::new(None, [None, None], true);
        let mut moves: Vec<Move> = Vec::new();
        while let Some(mv) = picker.next(&mut board, &tables, &history) {
            moves.push(mv);
        }

        // In captures_only mode, all returned moves should be captures or promotions
        for mv in &moves {
            assert!(
                mv.is_capture() || mv.is_promotion(),
                "Captures-only mode returned non-capture: {}",
                mv.to_uci()
            );
        }
    }

    #[test]
    fn test_picker_hash_move_first() {
        let tables = tables();
        let mut board = Board::new();
        let history = [[0i32; 64]; 64];

        // Create a hash move (e2e4)
        let hash_move = Move {
            from: crate::square::Square::from_index(12), // e2
            to: crate::square::Square::from_index(28),   // e4
            piece: crate::board::Piece::Pawn,
            promotion: None,
            flags: crate::moves::types::DOUBLE_PAWN_PUSH,
        };

        let mut picker = MovePicker::new(Some(hash_move), [None, None], false);
        let first_move = picker.next(&mut board, &tables, &history);

        assert!(first_move.is_some());
        let first = first_move.unwrap();
        assert_eq!(first.from, hash_move.from);
        assert_eq!(first.to, hash_move.to);
    }

    #[test]
    fn test_picker_no_duplicate_moves() {
        let tables = tables();
        let mut board =
            Board::from_str("r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2")
                .unwrap();
        let history = [[0i32; 64]; 64];

        let mut picker = MovePicker::new(None, [None, None], false);
        let mut moves: Vec<Move> = Vec::new();
        while let Some(mv) = picker.next(&mut board, &tables, &history) {
            // Check for duplicates
            let is_dup = moves
                .iter()
                .any(|m| m.from == mv.from && m.to == mv.to && m.promotion == mv.promotion);
            assert!(!is_dup, "Duplicate move found: {}", mv.to_uci());
            moves.push(mv);
        }
    }
}
