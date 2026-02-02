use crate::board::castle_bits::*;
use crate::board::{Board, Color, EMPTY_SQ, Piece};
use crate::hash::zobrist::{ep_file_to_hash, xor_castling_rights_delta, zobrist_keys};
use crate::moves::magic::MagicTables;
use crate::moves::movegen::generate_pseudo_legal;
use crate::moves::square_control::{in_check, is_legal_castling};
use crate::moves::types::{Move, MoveBuffer, NullMoveUndo, Undo};
use crate::square::Square;

/// Precomputed castling rook moves by king destination index.
#[inline(always)]
fn rook_castle_squares(king_to_idx: u8) -> Option<(Square, Square)> {
    match king_to_idx {
        6 => Some((Square::from_index(7), Square::from_index(5))), // White O-O
        2 => Some((Square::from_index(0), Square::from_index(3))), // White O-O-O
        62 => Some((Square::from_index(63), Square::from_index(61))), // Black O-O
        58 => Some((Square::from_index(56), Square::from_index(59))), // Black O-O-O
        _ => None,
    }
}

#[inline(always)]
fn rights_mask_to_clear_for_rook(color: Color, rook_sq: u8) -> u8 {
    match (color, rook_sq) {
        (Color::White, 0) => CASTLE_WQ,  // a1
        (Color::White, 7) => CASTLE_WK,  // h1
        (Color::Black, 56) => CASTLE_BQ, // a8
        (Color::Black, 63) => CASTLE_BK, // h8
        _ => 0,
    }
}

/// Helper: clear a piece bit and table entry at `idx`.
#[inline(always)]
fn remove_piece(board: &mut Board, color: Color, piece: Piece, idx: usize) {
    let new_bb = board.bb(color, piece) & !(1u64 << idx);
    board.set_bb(color, piece, new_bb);
}

/// Helper: set a piece bit and table entry at `idx`.
#[inline(always)]
fn place_piece(board: &mut Board, color: Color, piece: Piece, idx: usize) {
    let new_bb = board.bb(color, piece) | (1u64 << idx);
    board.set_bb(color, piece, new_bb);
}

pub fn make_move_basic(board: &mut Board, mv: Move) -> Undo {
    let start_zobrist = board.zobrist; // captured for history
    let color = board.side_to_move;
    let piece = mv.piece;
    let from_idx = mv.from.index() as usize;
    let to_idx = mv.to.index() as usize;

    let prev_en_passant = board.en_passant;

    // If an EP file was in the hash (relaxed rule), XOR it OUT now (pre-move, pre-flip)
    if let Some(f) = ep_file_to_hash(board) {
        board.zobrist ^= zobrist_keys().ep_file[f as usize];
    }

    board.en_passant = None;
    let prev_halfmove_clock = board.halfmove_clock;
    let prev_fullmove_number = board.fullmove_number;

    // Capture
    let mut capture = None;

    if mv.is_en_passant() {
        let cap_sq = if color == Color::White {
            to_idx - 8
        } else {
            to_idx + 8
        };
        capture = Some((
            color.opposite(),
            Piece::Pawn,
            Square::from_index(cap_sq as u8),
        ));
        remove_piece(board, color.opposite(), Piece::Pawn, cap_sq);
    } else {
        let occupant = board.piece_on_sq[to_idx];
        if occupant != EMPTY_SQ {
            let cap_color = Color::from_u8(occupant >> 3);
            let cap_piece = Piece::from_u8(occupant & 0b111);
            capture = Some((cap_color, cap_piece, mv.to));
            remove_piece(board, cap_color, cap_piece, to_idx);
        }
    }

    // Snapshot undo info
    let mut undo = Undo {
        from: mv.from,
        to: mv.to,
        piece,
        color,
        prev_side: color,
        capture,
        castling_rook: None,
        prev_castling_rights: board.castling_rights,
        promotion: None,
        prev_en_passant,
        prev_halfmove_clock,
        prev_fullmove_number,
        prev_history: None,
    };

    let old_rights = board.castling_rights;

    if mv.is_castling() {
        if let Some((rf, rt)) = rook_castle_squares(to_idx as u8) {
            undo.castling_rook = Some((rf, rt));
        }
    } else {
        undo.castling_rook = None;
    }

    if piece == Piece::Pawn {
        let from_rank = from_idx / 8;
        let to_rank = to_idx / 8;
        if (color == Color::White && from_rank == 1 && to_rank == 3)
            || (color == Color::Black && from_rank == 6 && to_rank == 4)
        {
            let ep_sq = if color == Color::White {
                from_idx + 8
            } else {
                from_idx - 8
            };
            board.en_passant = Some(Square::from_index(ep_sq as u8));

            // ── ADD THIS DEBUG INVARIANT ─────────────────────────────────────────
            let ep_rank = ep_sq / 8; // 0-based ranks: 0=rank1 … 7=rank8
            debug_assert!(
                (color == Color::White && ep_rank == 2)   // EP must be on rank 3 after white double push
            || (color == Color::Black && ep_rank == 5), // EP must be on rank 6 after black double push
                "EP square on wrong rank: {:?} (ep_rank={}, color={:?})",
                Square::from_index(ep_sq as u8),
                ep_rank,
                color
            );
            // ────────────────────────────────────────────────────────────────────
        }
    }

    // Compute all rights to clear for this move
    let mut mask_to_clear: u8 = 0;

    // (i) King moved → clear both for that color
    if piece == Piece::King {
        mask_to_clear |= match color {
            Color::White => CASTLE_WK | CASTLE_WQ,
            Color::Black => CASTLE_BK | CASTLE_BQ,
        };
    }

    // (ii) Rook moved from a corner → clear that side's right
    if piece == Piece::Rook {
        mask_to_clear |= rights_mask_to_clear_for_rook(color, mv.from.index());
    }

    // (iii) Captured a rook on its original corner → clear that side's right
    if let Some((cap_color, cap_piece, cap_sq)) = capture
        && cap_piece == Piece::Rook
    {
        mask_to_clear |= rights_mask_to_clear_for_rook(cap_color, cap_sq.index());
    }

    // Apply rights change ONCE and update hash via delta
    let new_rights = old_rights & !mask_to_clear;
    if new_rights != old_rights {
        board.castling_rights = new_rights;
        xor_castling_rights_delta(&mut board.zobrist, zobrist_keys(), old_rights, new_rights);
    }

    // Move the king
    remove_piece(board, color, piece, from_idx);

    if let Some(prom) = mv.promotion {
        debug_assert!(piece == Piece::Pawn, "Only pawns can promote");
        place_piece(board, color, prom, to_idx);
        undo.promotion = Some(prom);
    } else {
        // Normal (non-promotion) move
        place_piece(board, color, piece, to_idx);
    }

    // Move the rook if castling
    if let Some((rook_from, rook_to)) = undo.castling_rook {
        let rf = rook_from.index() as usize;
        let rt = rook_to.index() as usize;
        remove_piece(board, color, Piece::Rook, rf);
        place_piece(board, color, Piece::Rook, rt);
    }

    if capture.is_some() || piece == Piece::Pawn {
        board.halfmove_clock = 0;
    } else {
        board.halfmove_clock = prev_halfmove_clock + 1;
    }
    if color == Color::Black {
        board.fullmove_number = prev_fullmove_number + 1;
    }

    // Flip side-to-move
    board.side_to_move = color.opposite();
    board.zobrist ^= zobrist_keys().side_to_move;

    if let Some(f) = ep_file_to_hash(board) {
        board.zobrist ^= zobrist_keys().ep_file[f as usize];
    }

    #[cfg(debug_assertions)]
    debug_assert_valid_ep(board);

    // ---- Zobrist history (push PRE-move; truncate on irreversible) ----
    let irreversible = capture.is_some() || piece == Piece::Pawn || mv.promotion.is_some();

    // If irreversible, save the pre-move history so undo can restore it.
    let saved = if irreversible {
        Some(board.history.clone())
    } else {
        None
    };

    // If irreversible, we logically "reset" the history.
    if irreversible {
        board.history.clear();
    }

    // Always push the PRE-MOVE (start_zobrist) key into history
    board.history.push(start_zobrist);

    // Stash in undo so undo_move_basic can restore on irreversible
    undo.prev_history = saved;

    #[cfg(all(debug_assertions, feature = "paranoid_hash"))]
    {
        let full = board.compute_zobrist_full();
        let diff = board.zobrist ^ full;
        eprintln!("HASH DIFF: stored ^ full = 0x{:016x}", diff);

        let kz = zobrist_keys();

        // Check EP candidates
        for f in 0..8 {
            if diff == kz.ep_file[f] {
                eprintln!("Looks like EP file mismatch: file {}", f);
            }
        }
        if diff == kz.side_to_move {
            eprintln!("Side-to-move bit mismatch");
        }

        // (Optional) quickly check castling deltas
        if diff != 0 {
            for cur in 0u8..16 {
                for prev in 0u8..16 {
                    let mut z = 0u64;
                    xor_castling_rights_delta(&mut z, kz, cur, prev);
                    if z == diff {
                        eprintln!("Castling delta mismatch cur={} prev={}", cur, prev);
                    }
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    board.assert_hash();

    undo
}

pub fn undo_move_basic(board: &mut Board, undo: Undo) {
    // If the current position has an EP file included, XOR it OUT first (pre-flip)
    if let Some(f) = ep_file_to_hash(board) {
        board.zobrist ^= zobrist_keys().ep_file[f as usize];
    }

    // ---- Flip side back (hash + state) ----
    board.side_to_move = undo.prev_side;
    board.zobrist ^= zobrist_keys().side_to_move;

    // ---- Castling rights: apply HASH DELTA (cur -> prev), then assign ----
    let kz = zobrist_keys();
    let cur = board.castling_rights;
    let prev = undo.prev_castling_rights;
    if cur != prev {
        xor_castling_rights_delta(&mut board.zobrist, kz, cur, prev);
        board.castling_rights = prev;
    } else {
        // keep them equal explicitly (no hash change)
        board.castling_rights = prev;
    }

    // ---- Restore clocks ----
    board.halfmove_clock = undo.prev_halfmove_clock;
    board.fullmove_number = undo.prev_fullmove_number;

    let from_idx = undo.from.index() as usize;
    let to_idx = undo.to.index() as usize;

    // ---- Undo the moved piece (and promotion if any) ----
    if let Some(prom) = undo.promotion {
        // The piece on 'to' is the promoted piece; remove it, restore a pawn at 'from'
        remove_piece(board, undo.color, prom, to_idx);
        place_piece(board, undo.color, Piece::Pawn, from_idx);
    } else {
        // Normal move back
        remove_piece(board, undo.color, undo.piece, to_idx);
        place_piece(board, undo.color, undo.piece, from_idx);
    }

    // ---- Undo capture (including EP capture, since undo.capture holds the pawn's square) ----
    if let Some((cap_color, cap_piece, cap_sq)) = undo.capture {
        let ci = cap_sq.index() as usize;
        place_piece(board, cap_color, cap_piece, ci);
    }

    // ---- Undo castling rook, if this was a castle ----
    if let Some((rook_from, rook_to)) = undo.castling_rook {
        let rf = rook_from.index() as usize;
        let rt = rook_to.index() as usize;
        remove_piece(board, undo.color, Piece::Rook, rt);
        place_piece(board, undo.color, Piece::Rook, rf);
    }

    // ---- Restore prior EP square and, if it now contributes, XOR it IN ----
    board.en_passant = undo.prev_en_passant;
    if let Some(f) = ep_file_to_hash(board) {
        board.zobrist ^= kz.ep_file[f as usize];
    }

    #[cfg(debug_assertions)]
    debug_assert_valid_ep(board);

    // ---- Zobrist history (pop pre-move; restore pre-move slice if irreversible) ----
    // Remove the pre-move key we pushed at make()
    let _ = board.history.pop();

    // If the forward move was irreversible, restore the entire pre-move history snapshot
    if let Some(prev) = undo.prev_history {
        board.history = prev;
    }

    #[cfg(debug_assertions)]
    board.assert_hash();
}

pub fn make_null_move(board: &mut Board) -> NullMoveUndo {
    // Push current hash before null move
    board.history.push(board.zobrist);
    let undo = NullMoveUndo {
        prev_en_passant: board.en_passant,
        prev_halfmove_clock: board.halfmove_clock,
        prev_side: board.side_to_move,
    };

    // If an EP file was in the hash, XOR it OUT now
    if let Some(f) = ep_file_to_hash(board) {
        board.zobrist ^= zobrist_keys().ep_file[f as usize];
    }

    board.en_passant = None;

    // Switch side
    let color = board.side_to_move;
    board.side_to_move = color.opposite();
    board.zobrist ^= zobrist_keys().side_to_move;

    // Although it's a null move, we might theoretically increase halfmove clock?
    // Stockfish does NOT increase halfmove clock for null move in search, usually,
    // or arguably it doesn't matter for NMP reduction.
    // But let's be safe and just increment it to reflect a "move" passed.
    // board.halfmove_clock += 1;

    // Note: We do NOT push to history_since_irreversible because null move is not a real move
    // and we don't want to mess up 3-fold repetition detection in a way that persists?
    // Actually, standard engines often do hash updates.

    undo
}

pub fn undo_null_move(board: &mut Board, undo: NullMoveUndo) {
    // Restore side
    board.side_to_move = undo.prev_side;
    board.zobrist ^= zobrist_keys().side_to_move;

    // Restore EP
    board.en_passant = undo.prev_en_passant;
    if let Some(f) = ep_file_to_hash(board) {
        board.zobrist ^= zobrist_keys().ep_file[f as usize];
    }

    // Restore clock
    board.halfmove_clock = undo.prev_halfmove_clock;

    // Pop the hash we pushed
    board.history.pop();
}

pub fn generate_legal(
    board: &mut Board,
    tables: &MagicTables,
    moves: &mut impl MoveBuffer,
    scratch: &mut impl MoveBuffer,
) {
    scratch.clear();
    generate_pseudo_legal(board, tables, scratch);
    moves.clear();

    for mv in scratch.iter().copied() {
        if mv.is_castling() && !is_legal_castling(board, mv, tables) {
            continue;
        }
        let mover = board.side_to_move;
        let undo = make_move_basic(board, mv);
        let illegal = in_check(board, mover, tables);
        undo_move_basic(board, undo);
        if !illegal {
            moves.push(mv);
        }
    }
}

/// Check if a pseudo-legal move is legal (doesn't leave own king in check).
/// For castling moves, this also checks that the king doesn't pass through check.
#[inline]
pub fn is_legal_move(board: &mut Board, mv: Move, tables: &MagicTables) -> bool {
    if mv.is_castling() {
        return is_legal_castling(board, mv, tables);
    }
    let mover = board.side_to_move;
    let undo = make_move_basic(board, mv);
    let illegal = in_check(board, mover, tables);
    undo_move_basic(board, undo);
    !illegal
}

/// Generate only legal capture moves
pub fn generate_captures(
    board: &mut Board,
    tables: &MagicTables,
    moves: &mut impl MoveBuffer,
    scratch: &mut impl MoveBuffer,
) {
    // Generate all pseudo-legal moves
    scratch.clear();
    generate_pseudo_legal(board, tables, scratch);

    // Filter for captures only
    moves.clear();
    for &mv in scratch.iter() {
        // Only consider captures or promotions first
        if !mv.is_capture() && !mv.is_promotion() {
            continue;
        }

        let mover = board.side_to_move;
        let undo = make_move_basic(board, mv);
        let legal = !in_check(board, mover, tables);
        // Check if this move gives check (side_to_move has flipped after make)
        let gives_check = in_check(board, board.side_to_move, tables);

        undo_move_basic(board, undo);

        if legal && (mv.is_capture() || gives_check) {
            moves.push(mv);
        }
    }
}

#[cfg(debug_assertions)]
#[inline]
pub(crate) fn debug_assert_valid_ep(board: &Board) {
    if let Some(ep) = board.en_passant {
        let ep_idx = ep.index() as usize;
        let ep_rank = ep_idx / 8;
        // File sanity (redundant if Square guarantees 0..63, but cheap in debug):
        let ep_file = ep_idx % 8;
        debug_assert!(ep_file <= 7, "EP file out of range: {}", ep_file);

        match board.side_to_move {
            Color::White => {
                // Black just double-pushed → EP should be on rank 6 (0-based 5)
                debug_assert!(
                    ep_rank == 5,
                    "EP must be on rank 6 (r=5) when White is to move, got rank {} at {:?}",
                    ep_rank,
                    ep
                );
            }
            Color::Black => {
                // White just double-pushed → EP should be on rank 3 (0-based 2)
                debug_assert!(
                    ep_rank == 2,
                    "EP must be on rank 3 (r=2) when Black is to move, got rank {} at {:?}",
                    ep_rank,
                    ep
                );
            }
        }
    }
}
