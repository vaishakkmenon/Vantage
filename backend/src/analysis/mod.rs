use crate::board::{Board, Piece};
use crate::moves::execute::{generate_legal, make_move_basic};
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

fn still_winning_mate(score: &Score) -> bool {
    matches!(score, Score::Mate(m) if *m > 0)
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

fn classify(eval_before: &Score, eval_after: &Score) -> (String, Option<i32>) {
    match (eval_before, eval_after) {
        (Score::Mate(n), _) if *n > 0 && !still_winning_mate(eval_after) => {
            ("blunder".to_string(), None)
        }
        (_, Score::Mate(m)) if *m < 0 => ("blunder".to_string(), None),
        (Score::Mate(n), Score::Mate(m)) if *n < 0 && *m < 0 => {
            ("theoretical_loss".to_string(), None)
        }
        (Score::Mate(_), Score::Mate(_)) => ("best".to_string(), None),
        (Score::Cp(a), Score::Cp(b)) => {
            let cp_loss = a - b;
            (
                match cp_loss {
                    c if c >= 1000 => "blunder".to_string(),
                    c if c >= 300 => "mistake".to_string(),
                    c if c >= 100 => "inaccuracy".to_string(),
                    _ => "best".to_string(),
                },
                Some(cp_loss),
            )
        }
        _ => ("best".to_string(), None),
    }
}

pub fn analyze_move(
    fen: &str,
    mv: &str,
    nodes: u64,
    tables: &MagicTables,
    tt: &mut TranspositionTable,
) -> Result<AnalyzeMoveResult, String> {
    let mut board = Board::default();
    board.set_fen(fen)?;

    let Some(mv_obj) = find_legal_move(&mut board, tables, mv) else {
        return Ok(AnalyzeMoveResult {
            legal: false,
            classification: None,
            cp_loss: None,
            eval_before: None,
            eval_after: None,
            best_move: None,
            better_line: vec![],
            refutation: vec![],
        });
    };

    tt.clear();
    let limits = SearchLimits {
        max_depth: 64,
        node_limit: Some(nodes),
        time_limit: None,
    };

    let result_before = search(&mut board, tables, tt, limits);
    let eval_before = Score::from_raw(result_before.score);

    let best_move = result_before.best_move.map(|m| m.to_uci());
    let better_line: Vec<String> = result_before.pv.iter().map(|m| m.to_uci()).collect();

    let _undo = make_move_basic(&mut board, mv_obj);

    tt.clear();
    let result_after = search(&mut board, tables, tt, limits);
    let eval_after = Score::from_raw(-result_after.score);
    let refutation: Vec<String> = result_after.pv.iter().map(|m| m.to_uci()).collect();

    let (classification, cp_loss) = classify(&eval_before, &eval_after);

    Ok(AnalyzeMoveResult {
        legal: true,
        classification: Some(classification),
        cp_loss,
        eval_before: Some(eval_before),
        eval_after: Some(eval_after),
        best_move,
        better_line,
        refutation,
    })
}
