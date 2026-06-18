use crate::attack::{bishop_attacks, rook_attacks};
use crate::board::{Board, MAX_MOVES};
use crate::types::{Color, Move, Piece, Square};

pub fn generate_legal_moves(board: &Board, moves: &mut [Move; MAX_MOVES]) -> usize {
    let side = board.side_to_move();
    let mut count = 0;
    generate_pseudo_legal(board, moves, &mut count);
    let mut legal = 0;
    let mut b = board.clone();
    for i in 0..count {
        let mv = moves[i];
        let undo = b.make_move(mv);
        let king = b.king_square(side);
        if !b.is_attacked_by(king, side.flip()) {
            moves[legal] = mv;
            legal += 1;
        }
        b.unmake_move(&undo);
    }
    legal
}

pub fn generate_pseudo_legal(board: &Board, moves: &mut [Move; MAX_MOVES], count: &mut usize) {
    let side = board.side_to_move();
    let us_bb = board.colors_bb(side);
    let them_bb = board.colors_bb(side.flip());
    let occ = board.occupancy();
    let ep = board.en_passant();

    generate_pawn_moves(board, side, us_bb, them_bb, occ, ep, moves, count);
    generate_knight_moves(board, side, us_bb, them_bb, moves, count);
    generate_sliding_moves(board, side, us_bb, them_bb, occ, moves, count);
    generate_king_moves(board, side, us_bb, them_bb, moves, count);
}

fn generate_pawn_moves(
    board: &Board, color: Color,
    us_bb: u64, them_bb: u64, occ: u64, ep: Option<Square>,
    moves: &mut [Move; MAX_MOVES], count: &mut usize,
) {
    let pawns = board.pieces_bb(Piece::Pawn) & us_bb;
    if pawns == 0 { return; }

    let dir: i32 = if color == Color::White { 8 } else { -8 };
    let start_rank_shift: i32 = if color == Color::White { 8 } else { 48 };
    let promo_rank_shift: i32 = if color == Color::White { 48 } else { 8 };
    let promo_pieces = [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];

    let mut bb = pawns;
    while bb != 0 {
        let sq_idx = bb.trailing_zeros() as i32;
        let sq = Square::new(sq_idx as u8).unwrap();
        bb &= bb - 1;

        let rank = sq_idx >> 3;

        // single push
        let fwd_idx = sq_idx + dir;
        if fwd_idx >= 0 && fwd_idx < 64 && ((1u64 << fwd_idx) & occ) == 0 {
            let fwd = Square::new(fwd_idx as u8).unwrap();
            if rank == (promo_rank_shift >> 3) {
                for &p in &promo_pieces {
                    moves[*count] = Move::promotion(sq, fwd, p);
                    *count += 1;
                }
            } else {
                moves[*count] = Move::new(sq, fwd);
                *count += 1;
                // double push
                if rank == (start_rank_shift >> 3) {
                    let dbl_idx = sq_idx + dir * 2;
                    if dbl_idx >= 0 && dbl_idx < 64 && ((1u64 << dbl_idx) & occ) == 0 {
                        moves[*count] = Move::new(sq, Square::new(dbl_idx as u8).unwrap());
                        *count += 1;
                    }
                }
            }
        }

        // attacks
        let attacks = crate::attack::pawn_attacks(sq, color);
        let captures = attacks & them_bb;
        let mut caps = captures;
        while caps != 0 {
            let cap_idx = caps.trailing_zeros() as u8;
            caps &= caps - 1;
            let to = Square::new(cap_idx).unwrap();
            if rank == (promo_rank_shift >> 3) {
                for &p in &promo_pieces {
                    moves[*count] = Move::promotion(sq, to, p);
                    *count += 1;
                }
            } else {
                moves[*count] = Move::capture(sq, to);
                *count += 1;
            }
        }

        // en passant
        if let Some(ep_sq) = ep {
            if (attacks & (1u64 << ep_sq.index())) != 0 {
                moves[*count] = Move::ep(sq, ep_sq);
                *count += 1;
            }
        }
    }
}

fn generate_knight_moves(
    _board: &Board, _color: Color,
    us_bb: u64, them_bb: u64,
    moves: &mut [Move; MAX_MOVES], count: &mut usize,
) {
    let knights = _board.pieces_bb(Piece::Knight) & us_bb;
    let mut bb = knights;
    while bb != 0 {
        let sq_idx = bb.trailing_zeros() as u8;
        let sq = Square::new(sq_idx).unwrap();
        bb &= bb - 1;
        let attacks = crate::attack::knight_attacks(sq);
        let quiet = attacks & !(us_bb | them_bb);
        let captures = attacks & them_bb;
        let mut q = quiet;
        while q != 0 {
            let to_idx = q.trailing_zeros() as u8;
            q &= q - 1;
            moves[*count] = Move::new(sq, Square::new(to_idx).unwrap());
            *count += 1;
        }
        let mut c = captures;
        while c != 0 {
            let to_idx = c.trailing_zeros() as u8;
            c &= c - 1;
            moves[*count] = Move::capture(sq, Square::new(to_idx).unwrap());
            *count += 1;
        }
    }
}

fn generate_sliding_moves(
    _board: &Board, _color: Color,
    us_bb: u64, them_bb: u64, occ: u64,
    moves: &mut [Move; MAX_MOVES], count: &mut usize,
) {
    let bishops = _board.pieces_bb(Piece::Bishop) & us_bb;
    let rooks = _board.pieces_bb(Piece::Rook) & us_bb;
    let queens = _board.pieces_bb(Piece::Queen) & us_bb;

    // bishops + queens (diagonal)
    let mut diag = bishops | queens;
    while diag != 0 {
        let sq_idx = diag.trailing_zeros() as u8;
        let sq = Square::new(sq_idx).unwrap();
        diag &= diag - 1;
        let attacks = bishop_attacks(sq_idx, occ) & !us_bb;
        push_slider_moves(sq, attacks, them_bb, moves, count);
    }

    // rooks + queens (orthogonal)
    let mut ortho = rooks | queens;
    while ortho != 0 {
        let sq_idx = ortho.trailing_zeros() as u8;
        let sq = Square::new(sq_idx).unwrap();
        ortho &= ortho - 1;
        let attacks = rook_attacks(sq_idx, occ) & !us_bb;
        push_slider_moves(sq, attacks, them_bb, moves, count);
    }
}

#[inline]
fn push_slider_moves(from: Square, attacks: u64, them_bb: u64, moves: &mut [Move; MAX_MOVES], count: &mut usize) {
    let captures = attacks & them_bb;
    let quiets = attacks & !them_bb;
    let mut bb = captures;
    while bb != 0 {
        let lsb = bb.trailing_zeros() as u8;
        bb &= bb - 1;
        moves[*count] = Move::capture(from, Square::new(lsb).unwrap());
        *count += 1;
    }
    let mut bb = quiets;
    while bb != 0 {
        let lsb = bb.trailing_zeros() as u8;
        bb &= bb - 1;
        moves[*count] = Move::new(from, Square::new(lsb).unwrap());
        *count += 1;
    }
}

fn generate_king_moves(
    board: &Board, color: Color,
    us_bb: u64, them_bb: u64,
    moves: &mut [Move; MAX_MOVES], count: &mut usize,
) {
    if board.pieces_bb(Piece::King) & us_bb == 0 { return; }
    let king_sq = board.king_square(color);
    let attacks = crate::attack::king_attacks(king_sq);
    let quiet = attacks & !(us_bb | them_bb);
    let captures = attacks & them_bb;
    let mut q = quiet;
    while q != 0 {
        let to_idx = q.trailing_zeros() as u8;
        q &= q - 1;
        moves[*count] = Move::new(king_sq, Square::new(to_idx).unwrap());
        *count += 1;
    }
    let mut c = captures;
    while c != 0 {
        let to_idx = c.trailing_zeros() as u8;
        c &= c - 1;
        moves[*count] = Move::capture(king_sq, Square::new(to_idx).unwrap());
        *count += 1;
    }

    // castling
    let rank = king_sq.rank();
    if color == Color::White && rank == 0 {
        if board.castling_rights().white_kingside
            && board.empty_square(Square::F1)
            && board.empty_square(Square::G1)
        {
            let e1_ok = !board.is_attacked_by(Square::E1, Color::Black);
            let f1_ok = !board.is_attacked_by(Square::F1, Color::Black);
            let g1_ok = !board.is_attacked_by(Square::G1, Color::Black);
            if e1_ok && f1_ok && g1_ok {
                moves[*count] = Move::castle(Square::E1, Square::G1);
                *count += 1;
            }
        }
        if board.castling_rights().white_queenside
            && board.empty_square(Square::D1)
            && board.empty_square(Square::C1)
            && board.empty_square(Square::B1)
            && !board.is_attacked_by(Square::E1, Color::Black)
            && !board.is_attacked_by(Square::D1, Color::Black)
            && !board.is_attacked_by(Square::C1, Color::Black)
        {
            moves[*count] = Move::castle(Square::E1, Square::C1);
            *count += 1;
        }
    } else if color == Color::Black && rank == 7 {
        if board.castling_rights().black_kingside
            && board.empty_square(Square::F8)
            && board.empty_square(Square::G8)
            && !board.is_attacked_by(Square::E8, Color::White)
            && !board.is_attacked_by(Square::F8, Color::White)
            && !board.is_attacked_by(Square::G8, Color::White)
        {
            moves[*count] = Move::castle(Square::E8, Square::G8);
            *count += 1;
        }
        if board.castling_rights().black_queenside
            && board.empty_square(Square::D8)
            && board.empty_square(Square::C8)
            && board.empty_square(Square::B8)
            && !board.is_attacked_by(Square::E8, Color::White)
            && !board.is_attacked_by(Square::D8, Color::White)
            && !board.is_attacked_by(Square::C8, Color::White)
        {
            moves[*count] = Move::castle(Square::E8, Square::C8);
            *count += 1;
        }
    }
}

pub fn generate_legal_vec(board: &Board) -> Vec<Move> {
    let mut buf = [Move::NULL; MAX_MOVES];
    let count = generate_legal_moves(board, &mut buf);
    buf[..count].to_vec()
}

pub fn perft(board: &Board, depth: u8) -> u64 {
    if depth == 0 { return 1; }
    let mut buf = [Move::NULL; MAX_MOVES];
    if depth == 1 { return generate_legal_moves(board, &mut buf) as u64; }
    let mut nodes = 0;
    let mut b = board.clone();
    let count = generate_legal_moves(board, &mut buf);
    for i in 0..count {
        let undo = b.make_move(buf[i]);
        nodes += perft(&b, depth - 1);
        b.unmake_move(&undo);
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_perft_initial_pos() {
        let board = Board::from_initial();
        assert_eq!(perft(&board, 1), 20);
        assert_eq!(perft(&board, 2), 400);
        assert_eq!(perft(&board, 3), 8902);
    }

    #[test]
    fn test_perft_kiwipete() {
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let board = Board::from_fen(fen).expect("valid FEN");
        assert_eq!(perft(&board, 1), 48);
        assert_eq!(perft(&board, 2), 2039);
    }
}

#[cfg(test)]
mod perft_extra {
    use super::*;
    use crate::board::Board;
    use crate::attack::init_slider_tables;

    #[test]
    fn test_perft_position_3() {
        init_slider_tables();
        let fen = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
        let board = Board::from_fen(fen).expect("valid FEN");
        assert_eq!(perft(&board, 1), 14);
        assert_eq!(perft(&board, 2), 191);
        assert_eq!(perft(&board, 3), 2812);
    }

    #[test]
    fn test_perft_position_4() {
        init_slider_tables();
        let fen = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
        let board = Board::from_fen(fen).expect("valid FEN");
        assert_eq!(perft(&board, 1), 6);
        assert_eq!(perft(&board, 2), 264);
    }

    #[test]
    fn test_perft_position_5() {
        init_slider_tables();
        let fen = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
        let board = Board::from_fen(fen).expect("valid FEN");
        assert_eq!(perft(&board, 1), 44);
    }

    #[test]
    fn test_perft_position_6() {
        init_slider_tables();
        let fen = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";
        let board = Board::from_fen(fen).expect("valid FEN");
        assert_eq!(perft(&board, 1), 46);
        assert_eq!(perft(&board, 2), 2079);
    }
}

#[cfg(test)]
mod tests_edge_cases {
    use super::*;
    use crate::board::Board;
    use crate::attack::init_slider_tables;
    use crate::types::{Piece, Square};

    #[test]
    fn test_pinned_knight_no_moves() {
        init_slider_tables();
        let fen = "4r3/8/8/8/8/4N3/8/4K3 w - -";
        let board = Board::from_fen(fen).expect("valid");
        let e3 = Square::from_file_rank(4, 2).unwrap();
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);
        for i in 0..count {
            let mv = moves[i];
            assert!(
                mv.from() != e3,
                "pinned knight on e3 should have no legal moves, but found {mv}"
            );
        }
    }

    #[test]
    fn test_pinned_rook_moves_along_pin_axis() {
        init_slider_tables();
        let fen = "4r3/8/8/8/8/4R3/8/4K3 w - -";
        let board = Board::from_fen(fen).expect("valid");
        let e3 = Square::from_file_rank(4, 2).unwrap();
        let along_pin = [
            Square::from_file_rank(4, 1).unwrap(),
            Square::from_file_rank(4, 3).unwrap(),
            Square::from_file_rank(4, 4).unwrap(),
            Square::from_file_rank(4, 5).unwrap(),
            Square::from_file_rank(4, 6).unwrap(),
            Square::E8,
        ];
        let off_pin = [
            Square::from_file_rank(3, 2).unwrap(),
            Square::from_file_rank(5, 2).unwrap(),
            Square::from_file_rank(6, 2).unwrap(),
            Square::from_file_rank(7, 2).unwrap(),
            Square::from_file_rank(2, 2).unwrap(),
            Square::from_file_rank(1, 2).unwrap(),
            Square::from_file_rank(0, 2).unwrap(),
        ];
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);

        for i in 0..count {
            let mv = moves[i];
            if mv.from() == e3 {
                assert!(
                    along_pin.contains(&mv.to()),
                    "rook move {mv} should be along pin axis (e-file)"
                );
                assert!(
                    !off_pin.contains(&mv.to()),
                    "rook move {mv} should be illegal (off pin axis)"
                );
            }
        }
    }

    #[test]
    fn test_en_passant_discovery_check() {
        init_slider_tables();
        let fen = "4r3/8/8/3pP3/8/8/8/4K3 w - d6";
        let board = Board::from_fen(fen).expect("valid");
        let e5 = Square::from_file_rank(4, 4).unwrap();
        let d6 = Square::from_file_rank(3, 5).unwrap();
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);
        for i in 0..count {
            let mv = moves[i];
            if mv.from() == e5 && mv.to() == d6 {
                panic!("e5xd6 en passant should be illegal due to discovery check from rook on e8");
            }
        }
    }

    #[test]
    fn test_castling_through_check_kingside() {
        init_slider_tables();
        let fen = "5r2/8/8/8/8/8/8/4K2R w K -";
        let board = Board::from_fen(fen).expect("valid");
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);
        for i in 0..count {
            let mv = moves[i];
            if mv.from() == Square::E1 && mv.to() == Square::G1 {
                panic!("O-O should be illegal: f1 is attacked by rook on f8");
            }
        }
    }

    #[test]
    fn test_castling_through_check_queenside() {
        init_slider_tables();
        let fen = "3r4/8/8/8/8/8/8/R3K3 w Q -";
        let board = Board::from_fen(fen).expect("valid");
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);
        for i in 0..count {
            let mv = moves[i];
            if mv.from() == Square::E1 && mv.to() == Square::C1 {
                panic!("O-O-O should be illegal: d1 is attacked by rook on d8");
            }
        }
    }

    #[test]
    fn test_double_check_only_king_moves() {
        init_slider_tables();
        let fen = "4r1k1/8/8/b7/8/8/8/4K3 w - -";
        let board = Board::from_fen(fen).expect("valid");
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);
        assert!(count > 0, "king should have legal escape squares");
        for i in 0..count {
            let mv = moves[i];
            assert!(
                mv.from() == Square::E1,
                "only king moves legal under double check, but found {mv}"
            );
        }
    }

    #[test]
    fn test_stalemate_no_legal_moves() {
        init_slider_tables();
        let fen = "7k/5Q2/8/8/8/8/8/K7 b - -";
        let board = Board::from_fen(fen).expect("valid");
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_promotion_underpromotion() {
        init_slider_tables();
        let fen = "8/4P3/8/8/8/8/8/8 w - -";
        let board = Board::from_fen(fen).expect("valid");
        let e7 = Square::from_file_rank(4, 6).unwrap();
        let e8 = Square::E8;
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = generate_legal_moves(&board, &mut moves);
        let mut found_q = false;
        let mut found_r = false;
        let mut found_b = false;
        let mut found_n = false;
        for i in 0..count {
            let mv = moves[i];
            if mv.from() == e7 && mv.to() == e8 {
                match mv.promotion_piece() {
                    Some(Piece::Queen) => found_q = true,
                    Some(Piece::Rook) => found_r = true,
                    Some(Piece::Bishop) => found_b = true,
                    Some(Piece::Knight) => found_n = true,
                    _ => {}
                }
            }
        }
        assert!(found_q, "missing promotion e8=Q");
        assert!(found_r, "missing promotion e8=R");
        assert!(found_b, "missing promotion e8=B");
        assert!(found_n, "missing promotion e8=N");
    }
}
