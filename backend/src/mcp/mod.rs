use crate::board::{Board, Piece};
use crate::moves::execute::generate_legal;
use crate::moves::magic::MagicTables;
use crate::moves::types::Move;
use crate::search::search::{SearchLimits, search};
use crate::search::tt::TranspositionTable;
use serde::{Deserialize, Serialize};

const MATE_THRESHOLD: i32 = 30000;
const MATE_SCORE: i32 = 31000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Score {
    Cp(i32),
    Mate(i32),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluateResult {
    pub score: Score,
    pub best_move: Option<String>,
    pub pv: Vec<String>,
    pub depth: i32,
    pub nodes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeMoveResult {
    pub legal: bool,
    pub classification: Option<String>,
    pub cp_loss: Option<i32>,
    pub eval_before: Option<Score>,
    pub eval_after: Option<Score>,
    pub best_move: Option<String>,
    pub better_line: Vec<String>,
    pub refutation: Vec<String>,
}

impl Score {
    pub fn from_raw(score: i32) -> Score {
        if score.abs() >= MATE_THRESHOLD {
            let moves = (MATE_SCORE - score.abs() + 1) / 2;
            if score > 0 {
                Score::Mate(moves)
            } else {
                Score::Mate(-moves)
            }
        } else {
            Score::Cp(score)
        }
    }
}

pub fn evaluate(
    fen: &str,
    nodes: u64,
    tables: &MagicTables,
    tt: &mut TranspositionTable,
) -> Result<EvaluateResult, String> {
    let mut board = Board::default();
    board.set_fen(fen)?;
    tt.clear();
    let limits = SearchLimits {
        max_depth: 64,
        node_limit: Some(nodes),
        time_limit: None,
    };
    let result = search(&mut board, tables, tt, limits);
    Ok(EvaluateResult {
        score: Score::from_raw(result.score),
        best_move: result.best_move.map(|m| m.to_uci()),
        pv: result.pv.iter().map(|m| m.to_uci()).collect(),
        depth: result.depth,
        nodes: result.nodes,
    })
}

#[allow(dead_code)]
fn find_legal_move(board: &mut Board, tables: &MagicTables, mv_str: &str) -> Option<Move> {
    if mv_str.len() < 4 {
        return None;
    }

    let chars: Vec<char> = mv_str.chars().collect();
    let from_file = (chars[0] as u8).wrapping_sub(b'a');
    let from_rank = (chars[1] as u8).wrapping_sub(b'1');
    let to_file = (chars[2] as u8).wrapping_sub(b'a');
    let to_rank = (chars[3] as u8).wrapping_sub(b'1');

    if from_file > 7 || from_rank > 7 || to_file > 7 || to_rank > 7 {
        return None;
    }

    let from_square = (from_rank * 8 + from_file) as usize;
    let to_square = (to_rank * 8 + to_file) as usize;

    let promo_piece = if mv_str.len() >= 5 {
        match chars[4] {
            'q' => Some(Piece::Queen),
            'r' => Some(Piece::Rook),
            'b' => Some(Piece::Bishop),
            'n' => Some(Piece::Knight),
            _ => None,
        }
    } else {
        None
    };

    let mut moves: Vec<Move> = Vec::with_capacity(256);
    let mut scratch: Vec<Move> = Vec::with_capacity(256);
    generate_legal(board, tables, &mut moves, &mut scratch);

    moves.into_iter().find(|&mv| {
        mv.from.index() as usize == from_square
            && mv.to.index() as usize == to_square
            && mv.promotion == promo_piece
    })
}

#[allow(dead_code)]
fn classify(_eval_before: &Score, _eval_after: &Score, _cp_loss: Option<i32>) -> String {
    // TODO: classify move quality (blunder/mistake/inaccuracy/good/best) from cp_loss
    "unknown".to_string()
}
