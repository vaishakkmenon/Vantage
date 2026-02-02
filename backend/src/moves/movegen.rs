use crate::board::{Board, Color, Piece};
use crate::moves::king::KING_ATTACKS;
use crate::moves::knight::KNIGHT_ATTACKS;
use crate::moves::magic::MagicTables;
use crate::moves::magic::structs::{BishopMagicTables, RookMagicTables};
use crate::moves::pawn::{BLACK_PAWN_ATTACKS, WHITE_PAWN_ATTACKS};
use crate::moves::square_control::is_legal_castling;
use crate::moves::types::{
    CAPTURE, DOUBLE_PAWN_PUSH, EN_PASSANT, KINGSIDE_CASTLE, Move, MoveBuffer, PROMOTION,
    PROMOTION_CAPTURE, QUEENSIDE_CASTLE, QUIET_MOVE,
};
use crate::square::Square;
use crate::utils::pop_lsb;

// Predefined Rank Constants
const RANK1: u64 = 0x0000_0000_0000_00FF;
const RANK2: u64 = 0x0000_0000_0000_FF00;
const RANK7: u64 = 0x00FF_0000_0000_0000;
const RANK8: u64 = 0xFF00_0000_0000_0000;

// Castling Constants
const WHITE_KINGSIDE_BETWEEN: u64 = 0x0000_0000_0000_0060;
const WHITE_QUEENSIDE_BETWEEN: u64 = 0x0000_0000_0000_000E;
const BLACK_KINGSIDE_BETWEEN: u64 = 0x6000_0000_0000_0000;
const BLACK_QUEENSIDE_BETWEEN: u64 = 0x0E00_0000_0000_0000;

// Promotion Array
const PROMOS: [Piece; 4] = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];

/// Helper functionality to push latest found move
#[inline(always)]
fn push_piece_moves(
    from: u8,
    mut targets: u64,
    enemy: u64,
    move_piece: Piece,
    move_list: &mut impl MoveBuffer,
) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let is_cap = (enemy >> to) & 1 != 0;
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(to),
            piece: move_piece,
            promotion: None,
            flags: if is_cap { CAPTURE } else { QUIET_MOVE },
        });
    }
}

/// Function to determine which squares to check for kingside castling
#[inline(always)]
fn kingside_between(color: Color) -> u64 {
    match color {
        Color::White => WHITE_KINGSIDE_BETWEEN,
        Color::Black => BLACK_KINGSIDE_BETWEEN,
    }
}

/// Function to determine which squares to check for queenside castling
#[inline(always)]
fn queenside_between(color: Color) -> u64 {
    match color {
        Color::White => WHITE_QUEENSIDE_BETWEEN,
        Color::Black => BLACK_QUEENSIDE_BETWEEN,
    }
}

pub fn generate_knight_moves(board: &Board, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let knights = board.pieces(Piece::Knight, color);
    let friendly = board.occupancy(color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;

    let mut bb = knights;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let targets = KNIGHT_ATTACKS[from as usize] & !friendly & !enemy_king;
        push_piece_moves(from, targets, enemy_without_king, Piece::Knight, move_list);
    }
}

pub fn generate_bishop_moves(
    board: &Board,
    tables: &BishopMagicTables,
    move_list: &mut impl MoveBuffer,
) {
    let color = board.side_to_move;
    let bishops = board.pieces(Piece::Bishop, color);
    let friendly = board.occupancy(color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;
    let blockers = board.occupied();

    let mut bb = bishops;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.get_attacks(from as usize, blockers);
        let targets = attacks & !friendly & !enemy_king;
        push_piece_moves(from, targets, enemy_without_king, Piece::Bishop, move_list);
    }
}

pub fn generate_rook_moves(
    board: &Board,
    tables: &RookMagicTables,
    move_list: &mut impl MoveBuffer,
) {
    let color = board.side_to_move;
    let rooks: u64 = board.pieces(Piece::Rook, color);
    let friendly = board.occupancy(color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;
    let blockers = board.occupied();

    let mut bb = rooks;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.get_attacks(from as usize, blockers);
        let targets = attacks & !friendly & !enemy_king;
        push_piece_moves(from, targets, enemy_without_king, Piece::Rook, move_list);
    }
}

pub fn generate_queen_moves(board: &Board, tables: &MagicTables, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let queens: u64 = board.pieces(Piece::Queen, color);
    let friendly = board.occupancy(color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;
    let blockers = board.occupied();

    let mut bb = queens;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.queen_attacks(from as usize, blockers);
        let targets = attacks & !friendly & !enemy_king;
        push_piece_moves(from, targets, enemy_without_king, Piece::Queen, move_list);
    }
}

pub fn generate_king_moves(board: &Board, tables: &MagicTables, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let king_bb = board.pieces(Piece::King, color);

    if king_bb == 0 {
        return;
    } // illegal position safeguard

    let from = king_bb.trailing_zeros() as u8; // only one king
    let friendly = board.occupancy(color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy = board.opponent_occupancy(color);

    let targets = KING_ATTACKS[from as usize] & !friendly & !enemy_king;
    push_piece_moves(from, targets, enemy, Piece::King, move_list);

    let occ = board.occupied();

    // King-side castle
    if board.has_kingside_castle(color) && (occ & kingside_between(color)) == 0 {
        let mv = Move {
            from: Square::from_index(from),
            to: Square::from_index(from + 2), // g-file
            piece: Piece::King,
            promotion: None,
            flags: KINGSIDE_CASTLE,
        };

        if is_legal_castling(board, mv, tables) {
            move_list.push(mv);
        }
    }

    // Queen-side castle
    if board.has_queenside_castle(color) && (occ & queenside_between(color)) == 0 {
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(from - 2), // c-file
            piece: Piece::King,
            promotion: None,
            flags: QUEENSIDE_CASTLE,
        });
    }
}

pub fn generate_pawn_moves(board: &Board, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let pawns = board.pieces(Piece::Pawn, color);
    let enemy_without_king =
        board.opponent_occupancy(color) & !board.pieces(Piece::King, color.opposite());
    let empty = !board.occupied();

    // Rank masks
    let (start_rank, promo_rank, push_up) = match color {
        Color::White => (RANK7, RANK8, 8i8),
        Color::Black => (RANK2, RANK1, -8i8),
    };

    // Single accessor that captures `color` (one closure type)
    let pawn_attacks = |sq: usize| -> u64 {
        match color {
            Color::White => WHITE_PAWN_ATTACKS[sq],
            Color::Black => BLACK_PAWN_ATTACKS[sq],
        }
    };

    // ===== 1) Quiet single pushes (exclude promotion rank) =====
    let single_pushes = match color {
        Color::White => ((pawns << 8) & empty) & !promo_rank,
        Color::Black => ((pawns >> 8) & empty) & !promo_rank,
    };
    let mut bb = single_pushes;
    while bb != 0 {
        let to = pop_lsb(&mut bb);
        let from = match color {
            Color::White => to - 8,
            Color::Black => to + 8,
        };
        debug_assert!(from < 64 && to < 64);
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(to),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        });
    }

    // ===== 2) Quiet double pushes =====
    let double_pushes = match color {
        Color::White => (((pawns & RANK2) << 8) & empty) << 8 & empty,
        Color::Black => (((pawns & RANK7) >> 8) & empty) >> 8 & empty,
    };
    let mut bb = double_pushes;
    while bb != 0 {
        let to = pop_lsb(&mut bb);
        let from = match color {
            Color::White => to - 16,
            Color::Black => to + 16,
        };
        debug_assert!(from < 64 && to < 64);
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(to),
            piece: Piece::Pawn,
            promotion: None,
            flags: DOUBLE_PAWN_PUSH,
        });
    }

    // ===== 3) Normal captures (exclude promotion targets) =====
    // Capture targets must exclude King (already handled by enemy_without_king mask)
    // But we doubly ensure here if we used raw attacks.
    // Actually, `enemy_without_king` is correct for capture targets.
    // So logic below is fine as long as `enemy_without_king` is used.

    let mut attackers = pawns;
    while attackers != 0 {
        let from = pop_lsb(&mut attackers);
        let targets = pawn_attacks(from as usize) & enemy_without_king & !promo_rank;
        let mut t = targets;
        while t != 0 {
            let to = pop_lsb(&mut t);
            move_list.push(Move {
                from: Square::from_index(from),
                to: Square::from_index(to),
                piece: Piece::Pawn,
                promotion: None,
                flags: CAPTURE,
            });
        }
    }

    // ===== 4) Promotion pushes =====
    let shift = push_up.unsigned_abs();
    let promo_pushes = if push_up > 0 {
        (pawns & start_rank) << shift & empty
    } else {
        (pawns & start_rank) >> shift & empty
    };
    let mut bb = promo_pushes;
    while bb != 0 {
        let to = pop_lsb(&mut bb);
        let from = if push_up > 0 { to - shift } else { to + shift };
        for &promo in PROMOS.iter() {
            move_list.push(Move {
                from: Square::from_index(from),
                to: Square::from_index(to),
                piece: Piece::Pawn,
                promotion: Some(promo),
                flags: PROMOTION,
            });
        }
    }

    // ===== 5) Promotion captures =====
    let mut promo_attackers = pawns & start_rank;
    while promo_attackers != 0 {
        let from = pop_lsb(&mut promo_attackers);
        let targets = pawn_attacks(from as usize) & enemy_without_king & promo_rank;
        let mut t = targets;
        while t != 0 {
            let to = pop_lsb(&mut t);
            for &promo in PROMOS.iter() {
                move_list.push(Move {
                    from: Square::from_index(from),
                    to: Square::from_index(to),
                    piece: Piece::Pawn,
                    promotion: Some(promo),
                    flags: PROMOTION_CAPTURE,
                });
            }
        }
    }

    // ===== 6) En passant (sanity-checked pseudo-legal) =====
    if let Some(ep_sq) = board.en_passant {
        let ep = ep_sq.index();
        if (empty & (1u64 << ep)) != 0 {
            let cap_sq = match color {
                Color::White => ep - 8,
                Color::Black => ep + 8,
            };
            let enemy_pawns = board.pieces(Piece::Pawn, color.opposite());
            if (enemy_pawns & (1u64 << cap_sq)) != 0 {
                let mut atk = pawns;
                while atk != 0 {
                    let from = pop_lsb(&mut atk);
                    if (pawn_attacks(from as usize) & (1u64 << ep)) != 0 {
                        move_list.push(Move {
                            from: Square::from_index(from),
                            to: Square::from_index(ep),
                            piece: Piece::Pawn,
                            promotion: None,
                            flags: EN_PASSANT,
                        });
                    }
                }
            }
        }
    }
}

pub fn generate_pseudo_legal(board: &Board, tables: &MagicTables, moves: &mut impl MoveBuffer) {
    moves.clear();
    generate_pawn_moves(board, moves);
    generate_knight_moves(board, moves);
    generate_bishop_moves(board, &tables.bishop, moves);
    generate_rook_moves(board, &tables.rook, moves);
    generate_queen_moves(board, tables, moves);
    generate_king_moves(board, tables, moves);
}

// ============================================================================
// SPLIT MOVE GENERATION FOR STAGED MOVE PICKER
// ============================================================================

/// Helper: push only captures for a non-pawn piece
#[inline(always)]
fn push_captures_only(
    from: u8,
    mut targets: u64,
    enemy: u64,
    move_piece: Piece,
    move_list: &mut impl MoveBuffer,
) {
    // Only consider squares that are enemy-occupied
    targets &= enemy;
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(to),
            piece: move_piece,
            promotion: None,
            flags: CAPTURE,
        });
    }
}

/// Helper: push only quiet moves for a non-pawn piece
#[inline(always)]
fn push_quiets_only(
    from: u8,
    mut targets: u64,
    empty: u64,
    move_piece: Piece,
    move_list: &mut impl MoveBuffer,
) {
    // Only consider empty squares
    targets &= empty;
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(to),
            piece: move_piece,
            promotion: None,
            flags: QUIET_MOVE,
        });
    }
}

// --- Knight ---

fn generate_knight_captures(board: &Board, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let knights = board.pieces(Piece::Knight, color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;

    let mut bb = knights;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let targets = KNIGHT_ATTACKS[from as usize] & !board.occupancy(color) & !enemy_king;
        push_captures_only(from, targets, enemy_without_king, Piece::Knight, move_list);
    }
}

fn generate_knight_quiets(board: &Board, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let knights = board.pieces(Piece::Knight, color);
    let empty = !board.occupied();

    let mut bb = knights;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let targets = KNIGHT_ATTACKS[from as usize];
        push_quiets_only(from, targets, empty, Piece::Knight, move_list);
    }
}

// --- Bishop ---

fn generate_bishop_captures(
    board: &Board,
    tables: &BishopMagicTables,
    move_list: &mut impl MoveBuffer,
) {
    let color = board.side_to_move;
    let bishops = board.pieces(Piece::Bishop, color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;
    let blockers = board.occupied();

    let mut bb = bishops;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.get_attacks(from as usize, blockers);
        let targets = attacks & !board.occupancy(color) & !enemy_king;
        push_captures_only(from, targets, enemy_without_king, Piece::Bishop, move_list);
    }
}

fn generate_bishop_quiets(
    board: &Board,
    tables: &BishopMagicTables,
    move_list: &mut impl MoveBuffer,
) {
    let color = board.side_to_move;
    let bishops = board.pieces(Piece::Bishop, color);
    let blockers = board.occupied();
    let empty = !blockers;

    let mut bb = bishops;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.get_attacks(from as usize, blockers);
        push_quiets_only(from, attacks, empty, Piece::Bishop, move_list);
    }
}

// --- Rook ---

fn generate_rook_captures(
    board: &Board,
    tables: &RookMagicTables,
    move_list: &mut impl MoveBuffer,
) {
    let color = board.side_to_move;
    let rooks = board.pieces(Piece::Rook, color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;
    let blockers = board.occupied();

    let mut bb = rooks;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.get_attacks(from as usize, blockers);
        let targets = attacks & !board.occupancy(color) & !enemy_king;
        push_captures_only(from, targets, enemy_without_king, Piece::Rook, move_list);
    }
}

fn generate_rook_quiets(board: &Board, tables: &RookMagicTables, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let rooks = board.pieces(Piece::Rook, color);
    let blockers = board.occupied();
    let empty = !blockers;

    let mut bb = rooks;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.get_attacks(from as usize, blockers);
        push_quiets_only(from, attacks, empty, Piece::Rook, move_list);
    }
}

// --- Queen ---

fn generate_queen_captures(board: &Board, tables: &MagicTables, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let queens = board.pieces(Piece::Queen, color);
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy_without_king = board.opponent_occupancy(color) & !enemy_king;
    let blockers = board.occupied();

    let mut bb = queens;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.queen_attacks(from as usize, blockers);
        let targets = attacks & !board.occupancy(color) & !enemy_king;
        push_captures_only(from, targets, enemy_without_king, Piece::Queen, move_list);
    }
}

fn generate_queen_quiets(board: &Board, tables: &MagicTables, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let queens = board.pieces(Piece::Queen, color);
    let blockers = board.occupied();
    let empty = !blockers;

    let mut bb = queens;
    while bb != 0 {
        let from = pop_lsb(&mut bb);
        let attacks = tables.queen_attacks(from as usize, blockers);
        push_quiets_only(from, attacks, empty, Piece::Queen, move_list);
    }
}

// --- King ---

fn generate_king_captures(board: &Board, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let king_bb = board.pieces(Piece::King, color);

    if king_bb == 0 {
        return;
    }

    let from = king_bb.trailing_zeros() as u8;
    let enemy_king = board.pieces(Piece::King, color.opposite());
    let enemy = board.opponent_occupancy(color) & !enemy_king;

    let targets = KING_ATTACKS[from as usize] & !board.occupancy(color) & !enemy_king;
    push_captures_only(from, targets, enemy, Piece::King, move_list);
}

fn generate_king_quiets(board: &Board, tables: &MagicTables, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let king_bb = board.pieces(Piece::King, color);

    if king_bb == 0 {
        return;
    }

    let from = king_bb.trailing_zeros() as u8;
    let empty = !board.occupied();

    // Normal king moves to empty squares
    let targets = KING_ATTACKS[from as usize];
    push_quiets_only(from, targets, empty, Piece::King, move_list);

    // Castling (quiet moves)
    let occ = board.occupied();

    // King-side castle
    if board.has_kingside_castle(color) && (occ & kingside_between(color)) == 0 {
        let mv = Move {
            from: Square::from_index(from),
            to: Square::from_index(from + 2),
            piece: Piece::King,
            promotion: None,
            flags: KINGSIDE_CASTLE,
        };
        if is_legal_castling(board, mv, tables) {
            move_list.push(mv);
        }
    }

    // Queen-side castle
    if board.has_queenside_castle(color) && (occ & queenside_between(color)) == 0 {
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(from - 2),
            piece: Piece::King,
            promotion: None,
            flags: QUEENSIDE_CASTLE,
        });
    }
}

// --- Pawn (captures) ---

fn generate_pawn_captures(board: &Board, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let pawns = board.pieces(Piece::Pawn, color);
    let enemy_without_king =
        board.opponent_occupancy(color) & !board.pieces(Piece::King, color.opposite());

    let (start_rank, promo_rank) = match color {
        Color::White => (RANK7, RANK8),
        Color::Black => (RANK2, RANK1),
    };

    let pawn_attacks = |sq: usize| -> u64 {
        match color {
            Color::White => WHITE_PAWN_ATTACKS[sq],
            Color::Black => BLACK_PAWN_ATTACKS[sq],
        }
    };

    // Normal captures (non-promotion)
    let mut attackers = pawns & !start_rank; // Exclude promotion rank pawns
    while attackers != 0 {
        let from = pop_lsb(&mut attackers);
        let targets = pawn_attacks(from as usize) & enemy_without_king & !promo_rank;
        let mut t = targets;
        while t != 0 {
            let to = pop_lsb(&mut t);
            move_list.push(Move {
                from: Square::from_index(from),
                to: Square::from_index(to),
                piece: Piece::Pawn,
                promotion: None,
                flags: CAPTURE,
            });
        }
    }

    // Promotion pushes (captures and non-captures)
    let empty = !board.occupied();
    let shift: u8 = 8;

    // Promotion pushes (non-capture)
    let promo_pushes = if color == Color::White {
        (pawns & start_rank) << shift & empty
    } else {
        (pawns & start_rank) >> shift & empty
    };
    let mut bb = promo_pushes;
    while bb != 0 {
        let to = pop_lsb(&mut bb);
        let from = if color == Color::White {
            to - shift
        } else {
            to + shift
        };
        for &promo in PROMOS.iter() {
            move_list.push(Move {
                from: Square::from_index(from),
                to: Square::from_index(to),
                piece: Piece::Pawn,
                promotion: Some(promo),
                flags: PROMOTION,
            });
        }
    }

    // Promotion captures
    let mut promo_attackers = pawns & start_rank;
    while promo_attackers != 0 {
        let from = pop_lsb(&mut promo_attackers);
        let targets = pawn_attacks(from as usize) & enemy_without_king & promo_rank;
        let mut t = targets;
        while t != 0 {
            let to = pop_lsb(&mut t);
            for &promo in PROMOS.iter() {
                move_list.push(Move {
                    from: Square::from_index(from),
                    to: Square::from_index(to),
                    piece: Piece::Pawn,
                    promotion: Some(promo),
                    flags: PROMOTION_CAPTURE,
                });
            }
        }
    }

    // En passant
    if let Some(ep_sq) = board.en_passant {
        let ep = ep_sq.index();
        let empty = !board.occupied();
        if (empty & (1u64 << ep)) != 0 {
            let cap_sq = if color == Color::White {
                ep - 8
            } else {
                ep + 8
            };
            let enemy_pawns = board.pieces(Piece::Pawn, color.opposite());
            if (enemy_pawns & (1u64 << cap_sq)) != 0 {
                let mut atk = pawns;
                while atk != 0 {
                    let from = pop_lsb(&mut atk);
                    if (pawn_attacks(from as usize) & (1u64 << ep)) != 0 {
                        move_list.push(Move {
                            from: Square::from_index(from),
                            to: Square::from_index(ep),
                            piece: Piece::Pawn,
                            promotion: None,
                            flags: EN_PASSANT,
                        });
                    }
                }
            }
        }
    }
}

// --- Pawn (quiets) ---

fn generate_pawn_quiets(board: &Board, move_list: &mut impl MoveBuffer) {
    let color = board.side_to_move;
    let pawns = board.pieces(Piece::Pawn, color);
    let empty = !board.occupied();

    let promo_rank = match color {
        Color::White => RANK8,
        Color::Black => RANK1,
    };

    // Single pushes (non-promoting)
    let single_pushes = match color {
        Color::White => ((pawns << 8) & empty) & !promo_rank,
        Color::Black => ((pawns >> 8) & empty) & !promo_rank,
    };
    let mut bb = single_pushes;
    while bb != 0 {
        let to = pop_lsb(&mut bb);
        let from = match color {
            Color::White => to - 8,
            Color::Black => to + 8,
        };
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(to),
            piece: Piece::Pawn,
            promotion: None,
            flags: QUIET_MOVE,
        });
    }

    // Double pushes
    let double_pushes = match color {
        Color::White => (((pawns & RANK2) << 8) & empty) << 8 & empty,
        Color::Black => (((pawns & RANK7) >> 8) & empty) >> 8 & empty,
    };
    let mut bb = double_pushes;
    while bb != 0 {
        let to = pop_lsb(&mut bb);
        let from = match color {
            Color::White => to - 16,
            Color::Black => to + 16,
        };
        move_list.push(Move {
            from: Square::from_index(from),
            to: Square::from_index(to),
            piece: Piece::Pawn,
            promotion: None,
            flags: DOUBLE_PAWN_PUSH,
        });
    }
}

// ============================================================================
// PUBLIC SPLIT GENERATORS
// ============================================================================

/// Generate all pseudo-legal captures and promotions.
/// Includes: captures, promotion pushes, promotion captures, en passant.
pub fn generate_pseudo_legal_captures(
    board: &Board,
    tables: &MagicTables,
    moves: &mut impl MoveBuffer,
) {
    generate_pawn_captures(board, moves);
    generate_knight_captures(board, moves);
    generate_bishop_captures(board, &tables.bishop, moves);
    generate_rook_captures(board, &tables.rook, moves);
    generate_queen_captures(board, tables, moves);
    generate_king_captures(board, moves);
}

/// Generate all pseudo-legal quiet moves (non-captures).
/// Includes: single/double pawn pushes, piece moves to empty squares, castling.
pub fn generate_pseudo_legal_quiets(
    board: &Board,
    tables: &MagicTables,
    moves: &mut impl MoveBuffer,
) {
    generate_pawn_quiets(board, moves);
    generate_knight_quiets(board, moves);
    generate_bishop_quiets(board, &tables.bishop, moves);
    generate_rook_quiets(board, &tables.rook, moves);
    generate_queen_quiets(board, tables, moves);
    generate_king_quiets(board, tables, moves);
}
