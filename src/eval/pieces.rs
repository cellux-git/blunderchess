use crate::board::Board;
use crate::eval::params::{PieceEval, PawnEval};
use crate::types::{Color, Piece, Square};
use crate::attack::file_mask;

pub(crate) fn eval_rooks(_board: &Board, piece: &PieceEval, pawns_bb: u64, enemy_pawns_bb: u64, color: Color, our_rooks: u64) -> (i32, i32) {
    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut rooks = our_rooks;
    while rooks != 0 {
        let sq_idx = rooks.trailing_zeros() as u8;
        let file = sq_idx & 7;
        let rank = sq_idx >> 3;
        let fm = file_mask(file);
        let our_pawns = pawns_bb & fm;
        let enemy_pawns = enemy_pawns_bb & fm;
        if our_pawns == 0 {
            if enemy_pawns == 0 {
                mg += piece.rook_open_file_bonus.0;
                eg += piece.rook_open_file_bonus.1;
            } else {
                mg += piece.rook_semi_open_file_bonus.0;
                eg += piece.rook_semi_open_file_bonus.1;
            }
        } else {
            mg += piece.rook_closed_file_penalty.0;
            eg += piece.rook_closed_file_penalty.1;
        }
        let seventh = if color == Color::White { rank == 6 } else { rank == 1 };
        if seventh {
            mg += piece.rook_seventh_rank_bonus.0;
            eg += piece.rook_seventh_rank_bonus.1;
        }
        rooks &= rooks - 1;
    }
    (mg, eg)
}

pub(crate) fn eval_bad_bishops(board: &Board, piece: &PieceEval, color: Color, pawns_bb: u64, our_bishops: u64) -> (i32, i32) {
    let my_bishops = our_bishops;
    if my_bishops == 0 { return (0, 0); }

    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut bishops = my_bishops;
    while bishops != 0 {
        let sq_idx = bishops.trailing_zeros() as u8;
        let _sq = Square::new(sq_idx).unwrap();
        bishops &= bishops - 1;

        let sq_color = ((sq_idx & 7) + (sq_idx >> 3)) & 1;
        let attacks = crate::attack::bishop_attacks(sq_idx, board.occupancy());
        let safe = (attacks & !board.colors_bb(color)).count_ones() as usize;

        if safe <= 2 {
            let home_rank: u8 = if color == Color::White { 1 } else { 6 };
            let same_color_pawns = {
                let mut scp = 0u32;
                let mut p = pawns_bb;
                while p != 0 {
                    let pi = p.trailing_zeros() as u8;
                    let pr = pi >> 3;
                    if pr == home_rank { p &= p - 1; continue; }
                    let pc = ((pi & 7) + pr) & 1;
                    if pc == sq_color {
                        let pawn_rank = pi >> 3;
                        let bishop_rank = sq_idx >> 3;
                        let pawn_file = pi & 7;
                        let bishop_file = sq_idx & 7;
                        let df = (pawn_file as i32 - bishop_file as i32).unsigned_abs();
                        let dr = if color == Color::White {
                            (pawn_rank as i32 - bishop_rank as i32).max(0)
                        } else {
                            (bishop_rank as i32 - pawn_rank as i32).max(0)
                        };
                        if df == dr as u32 && dr > 0 {
                            scp += 1;
                        }
                    }
                    p &= p - 1;
                }
                scp
            };

            if same_color_pawns > 0 {
                let penalty = piece.bad_bishop_penalty;
                let multiplier = piece.bad_bishop_fixed_multiplier;
                let fixed = {
                    let mut fixed_count = 0u32;
                    let mut p = pawns_bb;
                    let enemy_pawns = board.pieces_bb(Piece::Pawn) & board.colors_bb(color.flip());
                    while p != 0 {
                        let pi = p.trailing_zeros() as u8;
                        p &= p - 1;
                        let pc = ((pi & 7) + (pi >> 3)) & 1;
                        if pc != sq_color { continue; }
                        let pawn_rank = pi >> 3;
                        let bishop_file = sq_idx & 7;
                        let df = ((pi & 7) as i32 - bishop_file as i32).unsigned_abs();
                        let dr = if color == Color::White {
                            (pawn_rank as i32 - (sq_idx >> 3) as i32).max(0)
                        } else {
                            ((sq_idx >> 3) as i32 - pawn_rank as i32).max(0)
                        };
                        if df != dr as u32 || dr == 0 { continue; }
                        let fwd = if color == Color::White { pi + 8 } else { pi - 8 };
                        let fwd_bb = 1u64 << fwd;
                        if fwd < 64 && (fwd_bb & board.occupancy()) != 0 {
                            fixed_count += 1;
                        } else if fwd < 64 {
                            let attacked = crate::attack::pawn_attacks(Square::new(fwd).unwrap(), color) & enemy_pawns != 0;
                            if attacked { fixed_count += 1; }
                        }
                    }
                    fixed_count
                };
                let total_penalty_mg = penalty.0 * same_color_pawns as i32 + penalty.0 * fixed as i32 * (multiplier - 1);
                let total_penalty_eg = penalty.1 * same_color_pawns as i32 + penalty.1 * fixed as i32 * (multiplier - 1);
                mg += total_penalty_mg;
                eg += total_penalty_eg;
            }
        }
    }
    (mg, eg)
}

pub(crate) fn eval_knights(board: &Board, piece: &PieceEval, color: Color, enemy_pawns_bb: u64, our_knights: u64) -> (i32, i32) {
    let my_knights = our_knights;
    let my_pawns = board.pieces_bb(Piece::Pawn) & board.colors_bb(color);
    let us_bb = board.colors_bb(color);
    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut knights = my_knights;
    while knights != 0 {
        let sq_idx = knights.trailing_zeros() as u8;
        let sq = Square::new(sq_idx).unwrap();
        knights &= knights - 1;

        let file = sq.file();
        let rank = sq.rank();
        let in_enemy_half = if color == Color::White { rank >= 4 } else { rank <= 3 };

        let enemy_attacks = crate::attack::pawn_attacks(sq, color);
        if in_enemy_half && (enemy_attacks & enemy_pawns_bb == 0) {
            let friendly_defenders = crate::attack::pawn_attacks(sq, color.flip());
            if friendly_defenders & my_pawns != 0 {
                mg += piece.outpost_knight_bonus.0;
                eg += piece.outpost_knight_bonus.1;
            }
        }

        if file == 0 || file == 7 {
            mg += piece.knight_rim_penalty.0;
            eg += piece.knight_rim_penalty.1;
        }

        let attacks = crate::attack::knight_attacks(sq);
        let safe = (attacks & !us_bb).count_ones() as usize;
        if safe == 0 {
            mg += piece.knight_trapped_penalty.0;
            eg += piece.knight_trapped_penalty.1;
        }
    }
    (mg, eg)
}

pub(crate) fn eval_rook_queen_battery(board: &Board, piece: &PieceEval, color: Color, our_rooks: u64, our_queens: u64) -> (i32, i32) {
    let us_bb = board.colors_bb(color);
    let queens = our_queens;
    let rooks = our_rooks;

    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut qs = queens;
    while qs != 0 {
        let q_sq = qs.trailing_zeros() as u8;
        qs &= qs - 1;
        let q_file = q_sq & 7;
        let q_rank = q_sq >> 3;

        let mut rs = rooks;
        while rs != 0 {
            let r_sq = rs.trailing_zeros() as u8;
            rs &= rs - 1;
            let r_file = r_sq & 7;
            let r_rank = r_sq >> 3;

            let on_same_file = q_file == r_file;
            let on_same_rank = q_rank == r_rank;
            if !on_same_file && !on_same_rank { continue; }

            let between = if on_same_file {
                let min_r = q_rank.min(r_rank);
                let max_r = q_rank.max(r_rank);
                ((1u64 << (max_r * 8)) - (1u64 << ((min_r + 1) * 8))) & (0x0101010101010101u64 << q_file)
            } else {
                let min_f = q_file.min(r_file);
                let max_f = q_file.max(r_file);
                ((1u64 << max_f) - (1u64 << (min_f + 1))) << (q_rank * 8)
            };

            let our_pawns = board.pieces_bb(Piece::Pawn) & us_bb & between;
            let enemy_pawns = board.pieces_bb(Piece::Pawn) & board.colors_bb(color.flip()) & between;
            let blocked = between & board.occupancy();

            let multiplier = if blocked == 0 { 2 } else if our_pawns == 0 && enemy_pawns == 0 { 1 } else { 0 };
            if multiplier > 0 {
                mg += piece.rook_queen_battery_bonus.0 * multiplier;
                eg += piece.rook_queen_battery_bonus.1 * multiplier;
            }
        }
    }
    (mg, eg)
}

pub(crate) fn eval_queen_multiattack(board: &Board, piece: &PieceEval, _color: Color, enemy: Color, our_queens: u64) -> (i32, i32) {
    let enemy_bb = board.colors_bb(enemy);
    let queens = our_queens;
    let occ = board.occupancy();
    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut qs = queens;
    while qs != 0 {
        let sq_idx = qs.trailing_zeros() as u8;
        qs &= qs - 1;
        let attacks = crate::attack::queen_attacks(sq_idx, occ);
        let attacked_pieces = attacks & enemy_bb;
        let count = attacked_pieces.count_ones() as usize;
        if count > 0 && count <= 7 {
            mg += piece.queen_attack_count_bonus[count];
            eg += piece.queen_attack_count_bonus[count] / 2;
        }

        if count >= 2 {
            let undefended = {
                let mut ud = 0u64;
                let mut pieces = attacked_pieces;
                while pieces != 0 {
                    let pi = pieces.trailing_zeros() as u8;
                    pieces &= pieces - 1;
                    let defenders = crate::attack::attackers_to(board, Square::new(pi).unwrap(), occ) & enemy_bb;
                    if defenders == 0 {
                        ud |= 1u64 << pi;
                    }
                }
                ud
            };
            if undefended.count_ones() >= 2 {
                mg += piece.queen_fork_bonus.0;
                eg += piece.queen_fork_bonus.1;
            }
        }
    }
    (mg, eg)
}

pub(crate) fn eval_exchange(board: &Board, piece: &PieceEval, color: Color, pawns_bb: u64) -> (i32, i32) {
    let us_bb = board.colors_bb(color);
    let enemy_bb = board.colors_bb(color.flip());
    let our_rooks = (board.pieces_bb(Piece::Rook) & us_bb).count_ones();
    let enemy_rooks = (board.pieces_bb(Piece::Rook) & enemy_bb).count_ones();
    let our_minors = (board.pieces_bb(Piece::Bishop) & us_bb).count_ones()
        + (board.pieces_bb(Piece::Knight) & us_bb).count_ones();
    let enemy_minors = (board.pieces_bb(Piece::Bishop) & enemy_bb).count_ones()
        + (board.pieces_bb(Piece::Knight) & enemy_bb).count_ones();

    let rook_diff = our_rooks as i32 - enemy_rooks as i32;
    let minor_diff = our_minors as i32 - enemy_minors as i32;
    if rook_diff != 1 || minor_diff != -1 { return (0, 0); }

    let mut mg = 0i32;
    let mut eg = 0i32;
    let enemy_pawns = board.pieces_bb(Piece::Pawn) & enemy_bb;
    let open_files = {
        let mut count = 0i32;
        for f in 0..8u8 {
            let fm = file_mask(f);
            if fm & pawns_bb == 0 && fm & enemy_pawns == 0 { count += 1; }
            else if fm & pawns_bb == 0 && fm & enemy_pawns != 0 { count += 1; }
        }
        count
    };
    mg += piece.exchange_open_file_bonus.0 * open_files;
    eg += piece.exchange_open_file_bonus.1 * open_files;

    let enemy_bishops = (board.pieces_bb(Piece::Bishop) & enemy_bb).count_ones();
    if enemy_bishops >= 2 {
        mg += piece.exchange_bishop_pair_penalty.0;
        eg += piece.exchange_bishop_pair_penalty.1;
    }

    let occ = board.occupancy();
    let mut minors = (board.pieces_bb(Piece::Bishop) | board.pieces_bb(Piece::Knight)) & enemy_bb;
    while minors != 0 {
        let sq_idx = minors.trailing_zeros() as u8;
        minors &= minors - 1;
        let sq = Square::new(sq_idx).unwrap();
        let is_bishop = (1u64 << sq_idx) & board.pieces_bb(Piece::Bishop) != 0;
        let safe = if is_bishop {
            let attacks = crate::attack::bishop_attacks(sq_idx, occ);
            (attacks & !enemy_bb).count_ones() as usize
        } else {
            let attacks = crate::attack::knight_attacks(sq);
            (attacks & !enemy_bb).count_ones() as usize
        };
        let max_mobility = if is_bishop { 13 } else { 8 };
        if max_mobility > 0 && safe <= max_mobility / 4 {
            mg += piece.exchange_minor_activity_bonus.0;
            eg += piece.exchange_minor_activity_bonus.1;
        }
    }
    (mg, eg)
}

pub(crate) fn eval_space(pawn: &PawnEval, pawns_bb: u64, color: Color) -> (i32, i32) {
    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut pawns = pawns_bb;
    while pawns != 0 {
        let sq_idx = pawns.trailing_zeros() as u8;
        pawns &= pawns - 1;
        let rank = sq_idx >> 3;
        let advanced = if color == Color::White { rank >= 3 && rank <= 5 } else { rank >= 2 && rank <= 4 };
        if advanced {
            mg += pawn.space_bonus.0;
            eg += pawn.space_bonus.1;
        }
    }
    (mg, eg)
}

pub(crate) fn eval_pawn_majority(pawn: &PawnEval, pawns_bb: u64, enemy_pawns_bb: u64) -> (i32, i32) {
    let queenside_mask: u64 = file_mask(0) | file_mask(1) | file_mask(2) | file_mask(3);
    let kingside_mask: u64 = file_mask(4) | file_mask(5) | file_mask(6) | file_mask(7);

    let own_qs = (pawns_bb & queenside_mask).count_ones() as i32;
    let own_ks = (pawns_bb & kingside_mask).count_ones() as i32;
    let enemy_qs = (enemy_pawns_bb & queenside_mask).count_ones() as i32;
    let enemy_ks = (enemy_pawns_bb & kingside_mask).count_ones() as i32;

    let mut mg = 0i32;
    let mut eg = 0i32;
    let qs_diff = own_qs - enemy_qs;
    let ks_diff = own_ks - enemy_ks;

    if qs_diff > 0 {
        mg += pawn.pawn_majority_bonus.0 * qs_diff;
        eg += pawn.pawn_majority_bonus.1 * qs_diff;
    }
    if ks_diff > 0 {
        mg += pawn.pawn_majority_bonus.0 * ks_diff;
        eg += pawn.pawn_majority_bonus.1 * ks_diff;
    }
    (mg, eg)
}
