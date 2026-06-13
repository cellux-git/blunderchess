use crate::board::Board;
use crate::types::{Color, Move, Piece, Square};

fn attackers_to(board: &Board, sq: Square, occ: u64) -> u64 {
    let si = sq.index();
    let knights = board.pieces_bb(Piece::Knight) & crate::attack::knight_attacks(sq);
    let kings = board.pieces_bb(Piece::King) & crate::attack::king_attacks(sq);
    let pawns_w = board.pieces_bb(Piece::Pawn)
        & board.colors_bb(Color::White)
        & crate::attack::pawn_attacks(sq, Color::Black);
    let pawns_b = board.pieces_bb(Piece::Pawn)
        & board.colors_bb(Color::Black)
        & crate::attack::pawn_attacks(sq, Color::White);
    let rooks = (board.pieces_bb(Piece::Rook) | board.pieces_bb(Piece::Queen))
        & crate::attack::rook_attacks(si, occ);
    let bishops = (board.pieces_bb(Piece::Bishop) | board.pieces_bb(Piece::Queen))
        & crate::attack::bishop_attacks(si, occ);
    (knights | kings | pawns_w | pawns_b | rooks | bishops) & occ
}

fn smallest_attacker(board: &Board, sq: Square, side: Color, occ: u64) -> Option<(Square, Piece)> {
    let attackers = attackers_to(board, sq, occ) & board.colors_bb(side);
    if attackers == 0 { return None; }
    let p = attackers & board.pieces_bb(Piece::Pawn);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Pawn)); }
    let p = attackers & board.pieces_bb(Piece::Knight);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Knight)); }
    let p = attackers & board.pieces_bb(Piece::Bishop);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Bishop)); }
    let p = attackers & board.pieces_bb(Piece::Rook);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Rook)); }
    let p = attackers & board.pieces_bb(Piece::Queen);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Queen)); }
    let p = attackers & board.pieces_bb(Piece::King);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::King)); }
    None
}

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

        let occ = board.occupancy() ^ from.bit() ^ to.bit();
        let opp_gain = self.see_rec(board, to, board.side_to_move().flip(), occ, moving);
        base_gain - opp_gain
    }
}
