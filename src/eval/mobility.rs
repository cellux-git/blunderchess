use crate::board::Board;
use crate::eval::params::MobilityTables;
use crate::types::{Color, Square};

pub(crate) fn enemy_pawn_attack_mask(enemy_pawns_bb: u64, enemy: Color) -> u64 {
    let mut mask = 0u64;
    let mut pawns = enemy_pawns_bb;
    while pawns != 0 {
        let sq = Square::new(pawns.trailing_zeros() as u8).unwrap();
        mask |= crate::attack::pawn_attacks(sq, enemy);
        pawns &= pawns - 1;
    }
    mask
}

pub(crate) fn eval_mobility(board: &Board, mobility: &MobilityTables, color: Color, enemy_pawn_attacks: u64, our_knights: u64, our_bishops: u64, our_rooks: u64, our_queens: u64) -> (i32, i32) {
    let us_bb = board.colors_bb(color);
    let occ = board.occupancy();
    let mut mg = 0i32;
    let mut eg = 0i32;

    let mut knights = our_knights;
    while knights != 0 {
        let sq = knights.trailing_zeros() as u8;
        let attacks = crate::attack::knight_attacks(Square::new(sq).unwrap());
        let safe = (attacks & !us_bb & !enemy_pawn_attacks).count_ones() as usize;
        mg += mobility.knight_mobility[safe.min(8)];
        eg += mobility.knight_mobility_eg[safe.min(8)];
        knights &= knights - 1;
    }

    let mut bishops = our_bishops;
    while bishops != 0 {
        let sq = bishops.trailing_zeros() as u8;
        let attacks = crate::attack::bishop_attacks(sq, occ);
        let safe = (attacks & !us_bb & !enemy_pawn_attacks).count_ones() as usize;
        mg += mobility.bishop_mobility[safe.min(13)];
        eg += mobility.bishop_mobility_eg[safe.min(13)];
        bishops &= bishops - 1;
    }

    let mut rooks = our_rooks;
    while rooks != 0 {
        let sq = rooks.trailing_zeros() as u8;
        let attacks = crate::attack::rook_attacks(sq, occ);
        let safe = (attacks & !us_bb & !enemy_pawn_attacks).count_ones() as usize;
        mg += mobility.rook_mobility[safe.min(14)];
        eg += mobility.rook_mobility_eg[safe.min(14)];
        rooks &= rooks - 1;
    }

    let mut queens = our_queens;
    while queens != 0 {
        let sq = queens.trailing_zeros() as u8;
        let attacks = crate::attack::queen_attacks(sq, occ);
        let safe = (attacks & !us_bb & !enemy_pawn_attacks).count_ones() as usize;
        mg += mobility.queen_mobility[safe.min(27)];
        eg += mobility.queen_mobility_eg[safe.min(27)];
        queens &= queens - 1;
    }

    (mg, eg)
}
