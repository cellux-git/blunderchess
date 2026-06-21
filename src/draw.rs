use crate::board::Board;
use crate::types::{Color, Piece, Square};

pub fn is_draw_by_rule(board: &Board) -> bool {
    is_insufficient_material(board) || is_fifty_move(board) || is_threefold(board)
}

pub fn is_fifty_move(board: &Board) -> bool {
    board.halfmove_clock() >= 100
}

pub fn is_threefold(board: &Board) -> bool {
    let hash = board.hash();
    let mut count = 0u8;
    for &h in board.history() {
        if h == hash { count += 1; if count >= 2 { return true; } }
    }
    false
}

pub fn is_insufficient_material(board: &Board) -> bool {
    let w_king_bb = board.pieces_bb(Piece::King) & board.colors_bb(Color::White);
    let b_king_bb = board.pieces_bb(Piece::King) & board.colors_bb(Color::Black);
    let non_kings = board.occupancy() & !(w_king_bb | b_king_bb);
    let n = non_kings.count_ones();

    if n == 0 { return true; }
    if n > 2 { return false; }

    let bishops = board.pieces_bb(Piece::Bishop);
    let knights = board.pieces_bb(Piece::Knight);
    let w_pieces = board.colors_bb(Color::White) & non_kings;
    let b_pieces = board.colors_bb(Color::Black) & non_kings;

    if (w_pieces & !(bishops | knights)) != 0 || (b_pieces & !(bishops | knights)) != 0 {
        return false;
    }

    if n == 1 { return true; }

    let w_count = w_pieces.count_ones();
    let b_count = b_pieces.count_ones();
    if w_count != 2 && b_count != 2 { return false; }

    let side_bb = if w_count == 2 { w_pieces } else { b_pieces };
    if (side_bb & bishops).count_ones() != 2 { return false; }

    let mut bb = side_bb;
    let s1 = Square::new(bb.trailing_zeros() as u8).unwrap();
    bb &= bb - 1;
    let s2 = Square::new(bb.trailing_zeros() as u8).unwrap();
    ((s1.file() + s1.rank()) & 1) == ((s2.file() + s2.rank()) & 1)
}
