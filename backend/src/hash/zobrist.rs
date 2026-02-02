// src/hash/zobrist.rs

use crate::board::castle_bits::*;
use crate::board::{Board, Color, Piece};
use once_cell::sync::OnceCell;
use rand::{RngCore, SeedableRng, rngs::StdRng};

const FILE_A: u64 = 0x0101_0101_0101_0101;
const FILE_H: u64 = 0x8080_8080_8080_8080;

#[cfg(feature = "deterministic_zobrist")]
const ZOBRIST_SEED: u64 = 0x9E37_79B9_AAAC_5C87;

fn make_zobrist_rng() -> StdRng {
    #[cfg(feature = "deterministic_zobrist")]
    {
        StdRng::seed_from_u64(ZOBRIST_SEED)
    }
    #[cfg(not(feature = "deterministic_zobrist"))]
    {
        // Version-agnostic: fill a 32-byte seed from thread_rng
        let mut seed = [0u8; 32];
        rand::rng().fill_bytes(&mut seed);
        StdRng::from_seed(seed)
    }
}

#[allow(dead_code)]
pub struct ZobristKeys {
    /// [color][piece][square] with {White=0, Black=1} and {P,N,B,R,Q,K}={0..5}
    pub piece: [[[u64; 64]; 6]; 2],
    pub side_to_move: u64,
    /// [0]=K, [1]=Q, [2]=k, [3]=q  (bit order K,Q,k,q)
    pub castling: [u64; 4],
    /// a..h => 0..7
    pub ep_file: [u64; 8],
}

#[inline]
pub fn xor_castling_rights_delta(hash: &mut u64, keys: &ZobristKeys, old: u8, new_: u8) {
    let d = old ^ new_;
    if d & CASTLE_WK != 0 {
        *hash ^= keys.castling[0];
    } // K
    if d & CASTLE_WQ != 0 {
        *hash ^= keys.castling[1];
    } // Q
    if d & CASTLE_BK != 0 {
        *hash ^= keys.castling[2];
    } // k
    if d & CASTLE_BQ != 0 {
        *hash ^= keys.castling[3];
    } // q
}

/// Returns Some(file 0..7) if EP should contribute to the hash *this ply*; else None.
/// Rule: include EP only if side-to-move has at least one pawn that could capture onto ep_square.
/// Pseudo-legal only (ignore pins/king safety).
pub fn ep_file_to_hash(board: &Board) -> Option<u8> {
    let ep = board.en_passant?;
    let s = ep.index();

    let r = s / 8;
    if !(r == 2 || r == 5) {
        // only rank 3 or 6 ever counts
        return None;
    }

    let bb_s: u64 = 1u64 << s;

    let has_capturing_pawn = match board.side_to_move {
        Color::White => {
            let src_ne = (bb_s >> 9) & !FILE_H;
            let src_nw = (bb_s >> 7) & !FILE_A;
            ((src_ne | src_nw) & board.bb(Color::White, Piece::Pawn)) != 0
        }
        Color::Black => {
            let src_se = (bb_s << 7) & !FILE_A;
            let src_sw = (bb_s << 9) & !FILE_H;
            ((src_se | src_sw) & board.bb(Color::Black, Piece::Pawn)) != 0
        }
    };

    if has_capturing_pawn {
        Some(s % 8)
    } else {
        None
    }
}

// Global keys, initialized on first use.
pub fn zobrist_keys() -> &'static ZobristKeys {
    static KEYS: OnceCell<ZobristKeys> = OnceCell::new();
    KEYS.get_or_init(|| generate_zobrist_keys_with_rng(make_zobrist_rng()))
}

// Stub for the next step (we’ll fill the arrays soon).
fn generate_zobrist_keys_with_rng(mut rng: StdRng) -> ZobristKeys {
    #[inline]
    fn non_zero(r: &mut StdRng) -> u64 {
        // avoid zero keys to reduce degenerate collisions
        let mut v = r.next_u64();
        while v == 0 {
            v = r.next_u64();
        }
        v
    }

    let mut keys = ZobristKeys {
        piece: [[[0u64; 64]; 6]; 2],
        side_to_move: 0,
        castling: [0u64; 4], // [K,Q,k,q]
        ep_file: [0u64; 8],  // a..h => 0..7
    };

    // piece[color][piece][square]
    for c in 0..2 {
        for p in 0..6 {
            for sq in 0..64 {
                keys.piece[c][p][sq] = non_zero(&mut rng);
            }
        }
    }

    // castling: [0]=K, [1]=Q, [2]=k, [3]=q
    for i in 0..4 {
        keys.castling[i] = non_zero(&mut rng);
    }

    // en passant file keys a..h => 0..7
    for f in 0..8 {
        keys.ep_file[f] = non_zero(&mut rng);
    }

    // side to move (XOR when Black to move)
    keys.side_to_move = non_zero(&mut rng);

    keys
}
