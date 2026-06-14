use crate::board::Board;
use crate::eval::Eval;
use crate::types::{Move, MoveKind};

use crate::search::MAX_DEPTH;

pub(crate) struct MoveOrdering {
    killers: [[Option<Move>; 2]; MAX_DEPTH as usize],
    history: [[i32; 64]; 64],
}

impl MoveOrdering {
    pub(crate) fn new() -> Self {
        Self {
            killers: [[None; 2]; MAX_DEPTH as usize],
            history: [[0; 64]; 64],
        }
    }

    pub(crate) fn order_moves(
        &self, moves: &mut [Move], board: &Board, hash_move: Option<Move>, ply: u8, thread_id: u8,
    ) {
        let hash = hash_move.unwrap_or(Move::NULL);
        let k0 = self.killers[ply as usize][0].unwrap_or(Move::NULL);
        let k1 = self.killers[ply as usize][1].unwrap_or(Move::NULL);

        moves.sort_by_cached_key(|mv| {
            if *mv == hash { return i32::MAX; }
            let k = mv.kind();
            if k == MoveKind::Capture {
                let see_val = Eval::default().see(board, *mv);
                return if see_val > 0 { 10_000 + see_val }
                else { 2_000 + see_val };
            }
            if k == MoveKind::Promotion { return 30_000; }
            if *mv == k0 { return 9_000; }
            if *mv == k1 { return 8_999; }
            let hist = self.history[mv.from().index() as usize][mv.to().index() as usize];
            let perturb = if ply == 0 && thread_id > 0 {
                ((mv.from().index().wrapping_mul(thread_id)) % 16) as i32
            } else { 0 };
            hist.clamp(0, 8_000) + perturb
        });
        moves.reverse();
    }

    pub(crate) fn order_moves_q(&self, moves: &mut [Move], board: &Board) {
        let mut b = board.clone();
        moves.sort_by_cached_key(|mv| {
            if mv.kind() == MoveKind::Capture {
                let see_val = Eval::default().see(board, *mv);
                if see_val > 0 { 10_000 + see_val }
                else { 2_000 + see_val }
            } else if mv.kind() == MoveKind::Promotion {
                30_000
            } else {
                let undo = b.make_move(*mv);
                let gives_check = b.in_check() as i32;
                b.unmake_move(&undo);
                if gives_check > 0 { 5_000 } else { 0 }
            }
        });
        moves.reverse();
    }

    pub(crate) fn is_killer(&self, mv: Move, ply: u8) -> bool {
        self.killers[ply as usize][0] == Some(mv)
            || self.killers[ply as usize][1] == Some(mv)
    }

    pub(crate) fn record_beta_cutoff(&mut self, mv: Move, depth: u8, ply: u8) {
        let ki = ply as usize;
        self.killers[ki][1] = self.killers[ki][0];
        self.killers[ki][0] = Some(mv);
        let from = mv.from().index() as usize;
        let to = mv.to().index() as usize;
        self.history[from][to] += (depth as i32) * (depth as i32);
    }
}
