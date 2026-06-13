use super::Eval;
use crate::board::Board;
use crate::types::{Color, Piece, Square};

impl Eval {
    pub(crate) fn eval_mobility(&self, board: &Board, color: Color, enemy: Color, occ: u64) -> (i32, i32) {
        let us_bb = board.colors_bb(color);
        let _enemy_bb = board.colors_bb(enemy);
        let mut mg = 0i32;
        let mut eg = 0i32;

        let mut knights = board.pieces_bb(Piece::Knight) & us_bb;
        while knights != 0 {
            let sq = knights.trailing_zeros() as u8;
            let attacks = crate::attack::knight_attacks(Square::new(sq).unwrap());
            let safe = (attacks & !us_bb).count_ones() as usize;
            mg += self.knight_mobility[safe.min(8)];
            eg += self.knight_mobility_eg[safe.min(8)];
            knights &= knights - 1;
        }

        let mut bishops = board.pieces_bb(Piece::Bishop) & us_bb;
        while bishops != 0 {
            let sq = bishops.trailing_zeros() as u8;
            let attacks = crate::attack::bishop_attacks(sq, occ);
            let safe = (attacks & !us_bb).count_ones() as usize;
            mg += self.bishop_mobility[safe.min(13)];
            eg += self.bishop_mobility_eg[safe.min(13)];
            bishops &= bishops - 1;
        }

        let mut rooks = board.pieces_bb(Piece::Rook) & us_bb;
        while rooks != 0 {
            let sq = rooks.trailing_zeros() as u8;
            let attacks = crate::attack::rook_attacks(sq, occ);
            let safe = (attacks & !us_bb).count_ones() as usize;
            mg += self.rook_mobility[safe.min(14)];
            eg += self.rook_mobility_eg[safe.min(14)];
            rooks &= rooks - 1;
        }

        let mut queens = board.pieces_bb(Piece::Queen) & us_bb;
        while queens != 0 {
            let sq = queens.trailing_zeros() as u8;
            let attacks = crate::attack::queen_attacks(sq, occ);
            let safe = (attacks & !us_bb).count_ones() as usize;
            mg += self.queen_mobility[safe.min(27)];
            eg += self.queen_mobility_eg[safe.min(27)];
            queens &= queens - 1;
        }

        (mg, eg)
    }
}
