use crate::board::{Board, MAX_MOVES};
use crate::eval::EVAL;
use crate::types::{Move, MoveKind, MAX_DEPTH};

const HISTORY_MAX: i32 = 16384;

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

    pub(crate) fn history_score(&self, mv: Move) -> i32 {
        self.history[mv.from().index() as usize][mv.to().index() as usize]
    }

    fn decay_history(&mut self) {
        for row in self.history.iter_mut() {
            for v in row.iter_mut() {
                *v /= 2;
            }
        }
    }

    pub(crate) fn order_moves(
        &self, moves: &mut [Move], board: &Board, hash_move: Option<Move>, ply: u8, thread_id: u8,
    ) {
        let hash = hash_move.unwrap_or(Move::NULL);
        let k0 = self.killers[ply as usize][0].unwrap_or(Move::NULL);
        let k1 = self.killers[ply as usize][1].unwrap_or(Move::NULL);

        let mut scores: [i32; MAX_MOVES] = [0; MAX_MOVES];
        for i in 0..moves.len() {
            let mv = moves[i];
            scores[i] = if mv == hash {
                i32::MAX
            } else {
                let k = mv.kind();
                if k == MoveKind::Capture {
                    let see_val = EVAL.see(board, mv);
                    if see_val > 0 { 10_000 + see_val } else { 2_000 + see_val }
                } else if k == MoveKind::Promotion {
                    30_000
                } else if mv == k0 {
                    9_000
                } else if mv == k1 {
                    8_999
                } else {
                    let hist = self.history[mv.from().index() as usize][mv.to().index() as usize];
                    let perturb = if ply == 0 && thread_id > 0 {
                        ((mv.from().index().wrapping_mul(thread_id)) % 16) as i32
                    } else { 0 };
                    hist + perturb
                }
            };
        }
        sort_by_score_desc(moves, &mut scores);
    }

    pub(crate) fn order_moves_q(&self, moves: &mut [Move], board: &mut Board) {
        let mut scores: [i32; MAX_MOVES] = [0; MAX_MOVES];
        for i in 0..moves.len() {
            let mv = moves[i];
            scores[i] = if mv.kind() == MoveKind::Capture {
                let see_val = EVAL.see(board, mv);
                if see_val > 0 { 10_000 + see_val } else { 2_000 + see_val }
            } else if mv.kind() == MoveKind::Promotion {
                30_000
            } else {
                let undo = board.make_move(mv);
                let gives_check = board.in_check() as i32;
                board.unmake_move(&undo);
                if gives_check > 0 { 5_000 } else { 0 }
            };
        }
        sort_by_score_desc(moves, &mut scores);
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
        let bonus = (depth as i32) * (depth as i32);
        self.history[from][to] += bonus;
        if self.history[from][to] > HISTORY_MAX {
            self.decay_history();
        }
    }
}

fn sort_by_score_desc(moves: &mut [Move], scores: &mut [i32]) {
    let n = moves.len();
    if n <= 1 { return; }
    for i in 1..n {
        let key_score = scores[i];
        let key_move = moves[i];
        let mut j = i;
        while j > 0 && scores[j - 1] < key_score {
            scores[j] = scores[j - 1];
            moves[j] = moves[j - 1];
            j -= 1;
        }
        scores[j] = key_score;
        moves[j] = key_move;
    }
}
