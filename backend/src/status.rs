use crate::board::{Board, Color, Piece};
use crate::moves::execute::generate_legal;
use crate::moves::magic::MagicTables;
use crate::moves::square_control::in_check;

// Public enum you can use anywhere without pulling movegen into board
#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum GameStatus {
    InPlay,
    DrawFivefold,
    DrawSeventyFiveMove,
    DrawThreefold,
    DrawFiftyMove,
    DrawDeadPosition,
    Stalemate,
    Checkmate,
}

// Free helpers that do not live on Board (prevents board → status imports)
pub fn is_draw_by_threefold(board: &Board) -> bool {
    board.is_threefold()
}
pub fn is_draw_by_fifty_move(board: &Board) -> bool {
    board.halfmove_clock >= 100
}

pub fn is_fivefold(board: &Board) -> bool {
    board.repetition_count() >= 5
}
pub fn is_seventyfive_move(board: &Board) -> bool {
    board.halfmove_clock >= 150
}

pub fn is_insufficient_material(board: &Board) -> bool {
    // Quick reject: any pawn/rook/queen on the board => mating material exists.
    let wp = board.bb(Color::White, Piece::Pawn);
    let bp = board.bb(Color::Black, Piece::Pawn);
    let wr = board.bb(Color::White, Piece::Rook);
    let br = board.bb(Color::Black, Piece::Rook);
    let wq = board.bb(Color::White, Piece::Queen);
    let bq = board.bb(Color::Black, Piece::Queen);
    if (wp | bp | wr | br | wq | bq) != 0 {
        return false;
    }

    // Count minor pieces
    let wb = board.bb(Color::White, Piece::Bishop).count_ones();
    let wn = board.bb(Color::White, Piece::Knight).count_ones();
    let bb = board.bb(Color::Black, Piece::Bishop).count_ones();
    let bn = board.bb(Color::Black, Piece::Knight).count_ones();

    let w_minors = wb + wn;
    let b_minors = bb + bn;
    let total_minors = w_minors + b_minors;

    // K vs K
    if total_minors == 0 {
        return true;
    }

    // K vs KB or KN (exactly one minor on the board)
    if total_minors == 1 {
        return true;
    }

    // Exactly two minors total
    if total_minors == 2 {
        // Two knights on one side (KNN vs K) cannot mate
        if wn == 2 || bn == 2 {
            return true;
        }
        // One minor each side (KN vs kn, KB vs kb, KB vs kn, etc.) cannot mate
        if w_minors == 1 && b_minors == 1 {
            return true;
        }
        // Remaining possibilities are two minors on one side:
        // - KBB vs K  (mate possible)  -> NOT insufficient
        // - KBN vs K  (mate possible)  -> NOT insufficient
        return false;
    }

    // 3+ minors total: conservatively say "not dead".
    // (These sets can allow mate, e.g., KBB vs KN or KBN vs K.)
    false
}

/// Determine the game status for the current position.
///
/// Priority (highest → lowest):
///  1) `DrawFivefold`          (automatic)
///  2) `DrawSeventyFiveMove`   (automatic; halfmove_clock ≥ 150)
///  3) `DrawDeadPosition`      (insufficient material)
///  4) `DrawThreefold`         (claimable)
///  5) `DrawFiftyMove`         (claimable; halfmove_clock ≥ 100)
///  6) `Checkmate` / `Stalemate` / `InPlay`
pub fn position_status(board: &mut Board, tables: &MagicTables) -> GameStatus {
    // Snapshot the clock once; decisions below use this immutable view.
    let hmc = board.halfmove_clock;

    // FIDE automatic first
    if board.repetition_count() >= 5 {
        return GameStatus::DrawFivefold;
    }
    if hmc >= 150 {
        return GameStatus::DrawSeventyFiveMove;
    }

    // Dead position (insufficient material)
    if is_insufficient_material(board) {
        return GameStatus::DrawDeadPosition;
    }

    // Claim-based
    if board.is_threefold() {
        return GameStatus::DrawThreefold;
    }
    if hmc >= 100 {
        return GameStatus::DrawFiftyMove;
    }

    // Move-based outcomes
    let mut legal = Vec::with_capacity(64);
    let mut scratch = Vec::with_capacity(256);
    generate_legal(board, tables, &mut legal, &mut scratch);
    if legal.is_empty() {
        if in_check(board, board.side_to_move, tables) {
            GameStatus::Checkmate
        } else {
            GameStatus::Stalemate
        }
    } else {
        GameStatus::InPlay
    }
}
