use crate::board::Board;
use crate::types::{Color, Move, Piece, Square};
use crate::attack::smallest_attacker;

impl super::Eval {
    /// Recursive SEE: evaluate whether `side` can profitably capture the piece on `sq`.
    /// Returns the net material gain from `side`'s perspective (>= 0).
    fn see_rec(&self, board: &Board, sq: Square, side: Color, occ: u64, piece_on_sq: Option<Piece>) -> i32 {
        let att = smallest_attacker(board, sq, side, occ);
        if att.is_none() { return 0; }
        let (att_sq, att_piece) = att.unwrap();
        let captured_val = match piece_on_sq {
            Some(p) => self.material_value(p),
            None => 0,
        };
        let new_occ = occ ^ att_sq.bit();
        let opp_gain = self.see_rec(board, sq, side.flip(), new_occ, Some(att_piece));
        0i32.max(captured_val - opp_gain)
    }

    /// Static Exchange Evaluation for the given capture.
    /// Returns net material gain from the perspective of the side that made the capture
    /// (positive = winning exchange, negative = losing).
    pub fn see(&self, board: &Board, mv: Move) -> i32 {
        let from = mv.from();
        let to = mv.to();
        let moving = board.piece_at(from);
        let victim = board.piece_at(to);

        let mut base_gain = match victim {
            Some(p) => self.material_value(p),
            None => 0,
        };
        if let Some(pp) = mv.promotion_piece() {
            base_gain += self.material_value(pp) - self.material_value(Piece::Pawn);
        }

        let is_ep = victim.is_none() && board.en_passant() == Some(to)
            && moving == Some(Piece::Pawn);
        let mut occ = board.occupancy() ^ from.bit() ^ to.bit();
        if is_ep {
            base_gain += self.material_value(Piece::Pawn);
            let cap_sq = Square::from_file_rank(to.file(), from.rank()).unwrap();
            occ ^= cap_sq.bit();
        }

        let opp_gain = self.see_rec(board, to, board.side_to_move().flip(), occ, moving);
        base_gain - opp_gain
    }
}
