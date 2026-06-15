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
    let mut pieces: Vec<(Piece, Color)> = Vec::new();
    for i in 0..64 {
        if let Some(p) = board.piece_at(Square::new(i as u8).unwrap()) {
            if p != Piece::King {
                pieces.push((p, board.color_at(Square::new(i as u8).unwrap()).unwrap()));
            }
        }
    }
    match pieces.len() {
        0 => true,
        1 => { let (p, _) = pieces[0]; p == Piece::Bishop || p == Piece::Knight }
        2 => {
            let (p1, c1) = pieces[0]; let (p2, c2) = pieces[1];
            if c1 != c2 { return false; }
            p1 == Piece::Bishop && p2 == Piece::Bishop
                && bishop_square_color(board, p1, c1) == bishop_square_color(board, p2, c2)
        }
        _ => false,
    }
}

fn bishop_square_color(board: &Board, piece: Piece, color: Color) -> usize {
    for i in 0..64 {
        let sq = Square::new(i as u8).unwrap();
        if board.piece_at(sq) == Some(piece) && board.color_at(sq) == Some(color) {
            return ((sq.file() + sq.rank()) % 2) as usize;
        }
    }
    0
}
