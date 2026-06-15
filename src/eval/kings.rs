use crate::board::Board;
use crate::eval::params::KingEval;
use crate::types::{Color, Piece};
use crate::types::Square;
use crate::attack::{file_mask, king_distance};

pub(crate) fn eval_king_safety(board: &Board, king: &KingEval, color: Color, king_sq: Square, pawns_bb: u64, enemy_bb: u64) -> i32 {
    let mut penalty = 0i32;
    let kf = king_sq.file();
    let kr = king_sq.rank();

    let shield_offsets: [(i32, i32); 6] = if color == Color::White {
        [(0, 1), (1, 1), (-1, 1), (0, 2), (1, 2), (-1, 2)]
    } else {
        [(0, -1), (1, -1), (-1, -1), (0, -2), (1, -2), (-1, -2)]
    };

    for &(df, dr) in &shield_offsets {
        let ff = kf as i32 + df;
        let rr = kr as i32 + dr;
        if ff >= 0 && ff < 8 && rr >= 0 && rr < 8 {
            let sq_mask = 1u64 << (rr * 8 + ff) as u64;
            if sq_mask & pawns_bb == 0 {
                penalty += king.king_shield_missing_penalty;
            }
        }
    }

    let start_file = kf.saturating_sub(1);
    let end_file = (kf + 1).min(7);
    for f in start_file..=end_file {
        let fm = file_mask(f);
        if fm & pawns_bb == 0 && fm & enemy_bb != 0 {
            penalty += king.king_open_file_penalty;
        }
    }

    let king_zone = crate::attack::king_attacks(king_sq) | (1u64 << king_sq.index());
    let enemy_queens = board.pieces_bb(Piece::Queen) & enemy_bb;
    let enemy_rooks = board.pieces_bb(Piece::Rook) & enemy_bb;
    let zone_attackers = (enemy_queens | enemy_rooks) & king_zone;
    penalty += (zone_attackers.count_ones() as i32) * -15;

    penalty
}

pub(crate) fn eval_king_opposition(board: &Board, king: &KingEval, color: Color) -> (i32, i32) {
    let my_king = board.king_square(color);
    let enemy_king = board.king_square(color.flip());
    let kf = my_king.file() as i32;
    let kr = my_king.rank() as i32;
    let ekf = enemy_king.file() as i32;
    let ekr = enemy_king.rank() as i32;

    let enemy_bb = board.colors_bb(color.flip());
    let enemy_majors = (board.pieces_bb(Piece::Queen)
        | board.pieces_bb(Piece::Rook)
        | board.pieces_bb(Piece::Bishop)
        | board.pieces_bb(Piece::Knight)) & enemy_bb;
    if enemy_majors != 0 { return (0, 0); }

    let df = (kf - ekf).abs();
    let dr = (kr - ekr).abs();
    if !((df == 2 && dr == 0) || (df == 0 && dr == 2)) { return (0, 0); }

    let between = if df == 2 {
        let min_f = kf.min(ekf);
        1u64 << ((kr as u32) * 8 + (min_f + 1) as u32)
    } else {
        let min_r = kr.min(ekr);
        1u64 << ((min_r + 1) as u32 * 8 + kf as u32)
    };
    if between & board.occupancy() != 0 { return (0, 0); }

    let to_move = board.side_to_move();
    if to_move == color.flip() {
        let enemy_pawns = board.pieces_bb(Piece::Pawn) & enemy_bb;
        let mut pawns = enemy_pawns;
        let mut can_tempo = false;
        while pawns != 0 {
            let pi = pawns.trailing_zeros() as u8;
            pawns &= pawns - 1;
            let fwd = if to_move == Color::White { pi + 8 } else { pi - 8 };
            if fwd < 64 && ((1u64 << fwd) & board.occupancy()) == 0 {
                let fwd_sq = Square::new(fwd).unwrap();
                let attacked = crate::attack::pawn_attacks(fwd_sq, to_move.flip()) & board.pieces_bb(Piece::Pawn) & board.colors_bb(color) != 0;
                if !attacked {
                    can_tempo = true;
                    break;
                }
            }
        }
        if can_tempo { return (0, 0); }
    }

    (king.king_opposition_bonus, king.king_opposition_bonus)
}

pub(crate) fn eval_king_passer_proximity(board: &Board, king: &KingEval, color: Color, passers: u64) -> (i32, i32) {
    if passers == 0 { return (0, 0); }
    let my_king = board.king_square(color);
    let enemy_king = board.king_square(color.flip());

    let mut mg = 0i32;
    let mut eg = 0i32;
    let mut bb = passers;
    while bb != 0 {
        let sq_idx = bb.trailing_zeros() as u8;
        bb &= bb - 1;
        let passer_sq = Square::new(sq_idx).unwrap();
        let dist_own = king_distance(my_king, passer_sq) as i32;
        let dist_enemy = king_distance(enemy_king, passer_sq) as i32;

        if dist_own < dist_enemy {
            let diff = dist_enemy as i32 - dist_own as i32;
            mg += king.king_passer_proximity_bonus_mg * diff;
            eg += king.king_passer_proximity_bonus * diff;
        }
    }
    (mg, eg)
}
