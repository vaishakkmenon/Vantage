use crate::board::{Board, Color, Piece};
use crate::moves::magic::MagicTables;
use crate::moves::types::Move;
use crate::search::see::SeeExt;

const PROMOTION_BASE: i32 = 20000;
const CAPTURE_BASE: i32 = 10000;
const KILLER1_SCORE: i32 = 9000;
const KILLER2_SCORE: i32 = 8000;
const BAD_CAPTURE_PENALTY: i32 = 20000;

pub fn mvv_lva_score(mv: Move, board: &Board) -> i32 {
    if !mv.is_capture() {
        return 0;
    }

    if let Some(captured) = board.piece_at(mv.to) {
        let captured_piece = captured.1;
        return captured_piece.value() * 10 - mv.piece.attacker_value();
    }

    // En Passant capture: Destination is empty, but it IS a capture.
    // Captured piece is a Pawn (value 100). Attacker is a Pawn (value 1).
    if mv.is_en_passant() {
        return 100 * 10 - 1; // 999
    }

    0
}

pub fn order_moves(
    moves: &mut [Move],
    board: &Board,
    killer_moves: &[Option<Move>; 2],
    history: &[[i32; 64]; 64],
    hash_move: Option<Move>,
    tables: &MagicTables,
) {
    // stable sort so non-captures keep their generation order
    moves.sort_by_cached_key(|&mv| {
        // Priority 0: Best move from previous iteration (Hash Move)
        if let Some(hm) = hash_move {
            if mv.from == hm.from && mv.to == hm.to && mv.promotion == hm.promotion {
                return -2_000_000_000; // Found it! Search first.
            }
        }

        // Priority 1: Promotions
        if let Some(p) = mv.promotion {
            return -(PROMOTION_BASE + p.value());
        }

        // Priority 2: Captures (MVV-LVA)
        let capture_score = mvv_lva_score(mv, board);
        if capture_score > 0 {
            if board.static_exchange_eval(mv, 0, tables) {
                return -(CAPTURE_BASE + capture_score);
            } else {
                // Bad capture (SEE < 0): Very low priority (search after quiets)
                return BAD_CAPTURE_PENALTY + capture_score;
            }
        }

        // Priority 3: Killer moves
        if let Some(k1) = killer_moves[0] {
            if mv.from == k1.from && mv.to == k1.to && mv.promotion == k1.promotion {
                return -KILLER1_SCORE;
            }
        }
        if let Some(k2) = killer_moves[1] {
            if mv.from == k2.from && mv.to == k2.to && mv.promotion == k2.promotion {
                return -KILLER2_SCORE;
            }
        }

        if mv.piece == Piece::Pawn && !mv.is_capture() {
            let to_rank = mv.to.index() / 8;
            let from_rank = mv.from.index() / 8;
            let is_advancing = if board.side_to_move == Color::White {
                to_rank > from_rank
            } else {
                to_rank < from_rank
            };

            if is_advancing && to_rank >= 3 && to_rank <= 5 {
                return -(5000 + (to_rank as i32 * 100)); // Prioritize progress over shuffles
            }
        }

        // Priority 4: History heuristic
        -history[mv.from.index() as usize][mv.to.index() as usize]
    });
}
