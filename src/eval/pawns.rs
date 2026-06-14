use super::Eval;
use crate::board::Board;
use crate::types::{Color, Piece, Square};
use crate::attack::{file_mask, adjacent_files_mask, rank_mask_forward};

impl Eval {
    pub(crate) fn eval_pawns(&self, board: &Board, pawns_bb: u64, enemy_pawns_bb: u64, color: Color, mg: &mut i32, eg: &mut i32) {
        let white_rank = |r: u8| if color == Color::White { r } else { 7 - r };
        let mut pawns = pawns_bb;
        while pawns != 0 {
            let sq_idx = pawns.trailing_zeros() as u8;
            let sq = Square::new(sq_idx).unwrap();
            let file = sq.file();
            let rank = sq.rank();
            let fwd_rank = white_rank(rank);

            let ahead_on_file = file_mask(file) & rank_mask_forward(sq, color) & pawns_bb;
            if ahead_on_file != 0 {
                *mg += self.pawn.doubled_pawn_penalty.0;
                *eg += self.pawn.doubled_pawn_penalty.1;
            }

            if adjacent_files_mask(file) & pawns_bb == (1u64 << sq_idx) & adjacent_files_mask(file) {
                *mg += self.pawn.isolated_pawn_penalty.0;
                *eg += self.pawn.isolated_pawn_penalty.1;
            }

            let ahead = rank_mask_forward(sq, color);
            if ahead & enemy_pawns_bb & adjacent_files_mask(file) == 0 {
                *mg += self.pawn.passed_pawn_bonus[fwd_rank as usize];
                *eg += self.pawn.passed_pawn_bonus[fwd_rank as usize] * 2;
            }

            if fwd_rank > 0 && fwd_rank < 6 {
                let fwd_sq = ((sq_idx as i32) + if color == Color::White { 8 } else { -8 }) as u8;
                let fwd_attacked = (crate::attack::pawn_attacks(Square::new(fwd_sq).unwrap(), color) & enemy_pawns_bb) != 0;
                let fwd_blocked = (1u64 << fwd_sq) & board.occupancy() != 0;
                if fwd_attacked || fwd_blocked {
                    if (adjacent_files_mask(file) & pawns_bb & rank_mask_forward(sq, color.flip())) == 0 {
                        *mg += self.pawn.backward_pawn_penalty.0;
                        *eg += self.pawn.backward_pawn_penalty.1;
                    }
                }
            }

            pawns &= pawns - 1;
        }
    }

}

pub(crate) fn passed_pawns(pawns_bb: u64, enemy_pawns_bb: u64, color: Color) -> u64 {
    let mut passed = 0u64;
    let mut bb = pawns_bb;
    while bb != 0 {
        let sq_idx = bb.trailing_zeros() as u8;
        let sq = Square::new(sq_idx).unwrap();
        let ahead = rank_mask_forward(sq, color);
        let file = sq.file();
        if ahead & enemy_pawns_bb & adjacent_files_mask(file) == 0 {
            passed |= 1u64 << sq_idx;
        }
        bb &= bb - 1;
    }
    passed
}

impl Eval {

    pub(crate) fn eval_connected_passers(&self, passers: u64, mg: &mut i32, eg: &mut i32) {
        let mut bb = passers;
        while bb != 0 {
            let sq_idx = bb.trailing_zeros() as u8;
            bb &= bb - 1;
            let file = (sq_idx & 7) as u8;
            let adj = if file == 0 {
                file_mask(1)
            } else if file == 7 {
                file_mask(6)
            } else {
                file_mask(file - 1) | file_mask(file + 1)
            };
            if passers & adj != 0 {
                *mg += self.king.connected_passer_bonus;
                *eg += self.king.connected_passer_bonus * 2;
            }
        }
    }

    pub(crate) fn eval_rook_behind_passer(&self, board: &Board, color: Color, passers: u64, mg: &mut i32, eg: &mut i32) {
        if passers == 0 { return; }
        let my_rooks = board.pieces_bb(Piece::Rook) & board.colors_bb(color);
        let enemy_rooks = board.pieces_bb(Piece::Rook) & board.colors_bb(color.flip());
        let mut rooks = my_rooks | enemy_rooks;

        while rooks != 0 {
            let sq_idx = rooks.trailing_zeros() as u8;
            rooks &= rooks - 1;
            let rook_file = (sq_idx & 7) as u8;
            let rook_rank = (sq_idx >> 3) as u8;

            let file_passers = passers & file_mask(rook_file);
            if file_passers == 0 { continue; }

            let passer_sq = Square::new(file_passers.trailing_zeros() as u8).unwrap();
            let passer_rank = passer_sq.rank();

            let is_mine = (my_rooks >> sq_idx) & 1 != 0;
            let behind = if is_mine {
                if color == Color::White { rook_rank < passer_rank } else { rook_rank > passer_rank }
            } else {
                if color == Color::White { rook_rank < passer_rank } else { rook_rank > passer_rank }
            };

            if behind {
                if is_mine {
                    *mg += self.king.rook_behind_passer_bonus.0;
                    *eg += self.king.rook_behind_passer_bonus.1;
                } else {
                    *mg -= self.king.rook_behind_passer_bonus.0;
                    *eg -= self.king.rook_behind_passer_bonus.1;
                }
            }
        }
    }

    pub(crate) fn eval_pawn_chain(&self, pawns_bb: u64, color: Color, mg: &mut i32, eg: &mut i32) {
        let enemy = color.flip();
        let mut pawns = pawns_bb;
        while pawns != 0 {
            let sq_idx = pawns.trailing_zeros() as u8;
            pawns &= pawns - 1;
            let file = sq_idx & 7;
            let rank = sq_idx >> 3;

            let adj = adjacent_files_mask(file);
            let same_rank = if color == Color::White { 0xFFu64 << (rank * 8) } else { 0xFFu64 << (rank * 8) };
            if (adj & pawns_bb & same_rank & !(1u64 << sq_idx)) != 0 {
                *mg += self.pawn.pawn_phalanx_bonus.0;
                *eg += self.pawn.pawn_phalanx_bonus.1;
            }

            let behind_sqs = crate::attack::pawn_attacks(Square::new(sq_idx).unwrap(), enemy);
            if behind_sqs & pawns_bb != 0 {
                *mg += self.pawn.pawn_chain_bonus.0;
                *eg += self.pawn.pawn_chain_bonus.1;
            }
        }
    }

    pub(crate) fn eval_candidate_passers(&self, _board: &Board, pawns_bb: u64, enemy_pawns_bb: u64, color: Color, mg: &mut i32, eg: &mut i32) {
        let white_rank = |r: u8| if color == Color::White { r } else { 7 - r };
        let mut pawns = pawns_bb;
        while pawns != 0 {
            let sq_idx = pawns.trailing_zeros() as u8;
            pawns &= pawns - 1;
            let file = sq_idx & 7;
            let fwd_rank = white_rank(sq_idx >> 3);
            if fwd_rank >= 6 { continue; }

            let ahead = rank_mask_forward(Square::new(sq_idx).unwrap(), color);
            let adj_ahead = ahead & adjacent_files_mask(file);
            let enemy_ahead = adj_ahead & enemy_pawns_bb;
            if enemy_ahead.count_ones() == 1 {
                let cap_file = (enemy_ahead.trailing_zeros() & 7) as u8;
                let cap_rank = enemy_ahead.trailing_zeros() >> 3;
                let cap_sq_idx = (cap_rank * 8 + cap_file as u32) as u8;
                let cap_sq = Square::new(cap_sq_idx).unwrap();
                let ahead_after = rank_mask_forward(cap_sq, color);
                let remaining = ahead_after & enemy_pawns_bb & adjacent_files_mask(file);
                if remaining == 0 {
                    *mg += self.pawn.candidate_passer_bonus[fwd_rank as usize];
                    *eg += self.pawn.candidate_passer_bonus[fwd_rank as usize] * 2;
                }
            }
        }
    }

    pub(crate) fn eval_passer_blocker(&self, board: &Board, _pawns_bb: u64, enemy_pawns_bb: u64, color: Color, mg: &mut i32, eg: &mut i32) {
        let enemy = color.flip();
        let enemy_passers = passed_pawns(enemy_pawns_bb, board.pieces_bb(Piece::Pawn) & board.colors_bb(color), enemy);
        let us_bb = board.colors_bb(color);
        let mut passers = enemy_passers;
        while passers != 0 {
            let sq_idx = passers.trailing_zeros() as u8;
            passers &= passers - 1;
            let block_sq = if enemy == Color::White { sq_idx + 8 } else { sq_idx - 8 };
            if block_sq >= 64 { continue; }
            if (1u64 << block_sq) & us_bb != 0 {
                *mg += self.pawn.passer_blocker_bonus.0;
                *eg += self.pawn.passer_blocker_bonus.1;
            }
        }
    }
}
