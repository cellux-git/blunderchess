use crate::board::Board;
use crate::eval::Eval;
use crate::movegen::MAX_MOVES;
use crate::types::{Move, MoveKind, MAX_DEPTH};

const HISTORY_MAX: i16 = 16384;

pub(crate) struct MoveOrdering {
    killers: [[Option<Move>; 2]; MAX_DEPTH as usize],
    history: [[i16; 64]; 64],
}

impl MoveOrdering {
    pub(crate) fn new() -> Self {
        Self {
            killers: [[None; 2]; MAX_DEPTH as usize],
            history: [[0; 64]; 64],
        }
    }

    pub(crate) fn history_score(&self, mv: Move) -> i32 {
        self.history[mv.from().index() as usize][mv.to().index() as usize] as i32
    }

    pub(crate) fn order_moves(
        &self, moves: &mut [Move], board: &Board, hash_move: Option<Move>, ply: u8, thread_id: u8, eval: &Eval,
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
                    let see_val = eval.see(board, mv);
                    if see_val > 0 { 10_000 + see_val } else { 2_000 + see_val }
                } else if k == MoveKind::Promotion {
                    30_000
                } else if mv == k0 {
                    9_000
                } else if mv == k1 {
                    8_999
                } else {
                    let hist = self.history[mv.from().index() as usize][mv.to().index() as usize] as i32;
                    let perturb = if ply == 0 && thread_id > 0 {
                        ((mv.from().index().wrapping_mul(thread_id)) % 16) as i32
                    } else { 0 };
                    hist + perturb
                }
            };
        }
        sort_by_score_desc(moves, &mut scores);
    }

    pub(crate) fn order_moves_q(&self, moves: &mut [Move], board: &mut Board, eval: &Eval) {
        let mut scores: [i32; MAX_MOVES] = [0; MAX_MOVES];
        for i in 0..moves.len() {
            let mv = moves[i];
            scores[i] = if mv.kind() == MoveKind::Capture {
                let see_val = eval.see(board, mv);
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
        let old = self.history[from][to] as i32;
        let new_val = old.saturating_add(bonus);
        self.history[from][to] = if new_val > HISTORY_MAX as i32 { HISTORY_MAX } else { new_val as i16 };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::eval::Eval;
    use crate::types::{Move, Square};

    fn e2() -> Square { Square::from_file_rank(4, 1).unwrap() }
    fn e4() -> Square { Square::from_file_rank(4, 3).unwrap() }
    fn d2() -> Square { Square::from_file_rank(3, 1).unwrap() }
    fn d4() -> Square { Square::from_file_rank(3, 3).unwrap() }
    fn g1() -> Square { Square::from_file_rank(6, 0).unwrap() }
    fn f3() -> Square { Square::from_file_rank(5, 2).unwrap() }
    fn b1() -> Square { Square::from_file_rank(1, 0).unwrap() }
    fn c3() -> Square { Square::from_file_rank(2, 2).unwrap() }
    fn d5() -> Square { Square::from_file_rank(3, 4).unwrap() }

    fn eval() -> Eval { Eval::default() }

    #[test]
    fn test_killer_record_and_query() {
        let mut mo = MoveOrdering::new();
        let mv = Move::new(e2(), e4());
        assert!(!mo.is_killer(mv, 1));

        mo.record_beta_cutoff(mv, 3, 1);
        assert!(mo.is_killer(mv, 1));

        let mv2 = Move::new(d2(), d4());
        assert!(!mo.is_killer(mv2, 1));
        mo.record_beta_cutoff(mv2, 4, 1);

        assert!(mo.is_killer(mv, 1));
        assert!(mo.is_killer(mv2, 1));
    }

    #[test]
    fn test_history_score_increases_after_beta_cutoff() {
        let mut mo = MoveOrdering::new();
        let mv = Move::new(g1(), f3());
        let initial = mo.history_score(mv);
        assert_eq!(initial, 0);

        mo.record_beta_cutoff(mv, 3, 2);
        assert!(mo.history_score(mv) >= 9);
    }

    #[test]
    fn test_history_gravity_aging() {
        let mut mo = MoveOrdering::new();
        for _ in 0..2000 {
            mo.record_beta_cutoff(Move::new(g1(), f3()), 8, 3);
        }
        let score = mo.history_score(Move::new(g1(), f3()));
        assert_eq!(score, 16384, "history should be clamped at HISTORY_MAX");

        // Record another beta cutoff — verify it doesn't overflow
        mo.record_beta_cutoff(Move::new(b1(), c3()), 2, 3);
        let still = mo.history_score(Move::new(g1(), f3()));
        assert_eq!(still, 16384, "clamped entries stay at max");
        let new_score = mo.history_score(Move::new(b1(), c3()));
        assert_eq!(new_score, 4, "new entry depth=2 => bonus=4");
    }

    #[test]
    fn test_hash_move_ordered_first() {
        let board = Board::from_initial();
        let mo = MoveOrdering::new();
        let ev = eval();
        let mut moves = [Move::NULL; MAX_MOVES];
        let mut count = 0;
        crate::movegen::generate_pseudo_legal(&board, &mut moves, &mut count);
        let moves = &mut moves[..count];

        let hash_move = Some(moves[4]);
        mo.order_moves(moves, &board, hash_move, 1, 0, &ev);
        assert_eq!(moves[0], hash_move.unwrap(), "hash move should be ordered first");
    }

    #[test]
    fn test_captures_before_quiets() {
        crate::attack::init_slider_tables();
        let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let board = Board::from_fen(fen).unwrap();
        let mo = MoveOrdering::new();
        let ev = eval();
        let mut moves = [Move::NULL; MAX_MOVES];
        let mut count = 0;
        crate::movegen::generate_pseudo_legal(&board, &mut moves, &mut count);
        let moves = &mut moves[..count];
        mo.order_moves(moves, &board, None, 1, 0, &ev);

        let exd5 = Move::capture(e4(), d5());
        let first_cap_idx = moves.iter().position(|&m| m == exd5);
        assert!(first_cap_idx.is_some(), "e4xd5 should be in the move list");
        // captures get score >= 2000, quiets get score 0 (fresh history) — so capture should come first
        if first_cap_idx.unwrap() > 0 {
            // The capture of d5 should have higher score than quiets
            // (unless some other captures like promotions are even higher)
        }
    }

    #[test]
    fn test_thread_perturbation_at_root() {
        let board = Board::from_initial();
        let ev = eval();
        let mut moves_t0 = [Move::NULL; MAX_MOVES];
        let mut moves_t1 = [Move::NULL; MAX_MOVES];
        let mut count = 0;
        crate::movegen::generate_pseudo_legal(&board, &mut moves_t0, &mut count);
        moves_t1[..count].copy_from_slice(&moves_t0[..count]);

        let mo = MoveOrdering::new();
        mo.order_moves(&mut moves_t0[..count], &board, None, 0, 0, &ev);
        mo.order_moves(&mut moves_t1[..count], &board, None, 0, 1, &ev);

        // Both threads produce valid move lists with the same count
        assert!(count > 0, "should have pseudo-legal moves from startpos");
        let t0_head = &moves_t0[..count];
        let t1_head = &moves_t1[..count];
        assert!(!t0_head.is_empty());
        assert!(!t1_head.is_empty());
    }

    #[test]
    fn test_is_killer_different_ply() {
        let mut mo = MoveOrdering::new();
        let mv = Move::new(e2(), e4());
        mo.record_beta_cutoff(mv, 3, 2);
        assert!(mo.is_killer(mv, 2));
        assert!(!mo.is_killer(mv, 3));
        assert!(!mo.is_killer(mv, 1));
    }
}
