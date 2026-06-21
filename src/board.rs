use crate::types::{Bitboard, CastlingRights, Color, Move, MoveKind, Piece, Square, CASTLE_LOSE_MASK};
use crate::zobrist;
use crate::movegen::MAX_MOVES;
use std::fmt;

pub const INITIAL_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

// ---- undo info for make/unmake ----
pub struct UndoInfo {
    mv: Move,
    captured: Option<(Piece, Color)>,
    captured_sq: Square,
    prev_castling: CastlingRights,
    prev_ep: Option<Square>,
    prev_halfmove: u8,
    prev_hash: u64,
    prev_phase: i32,
    prev_pinned: [Bitboard; 2],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attack::init_slider_tables;

    #[test]
    fn test_make_unmake_roundtrip_startpos() {
        init_slider_tables();
        let original = Board::from_initial();
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = crate::movegen::generate_legal_moves(&original, &mut moves);
        assert!(count > 0, "start position should have legal moves");

        for i in 0..count {
            let mv = moves[i];
            let saved = original.clone();

            let mut b = original.clone();
            let _undo = b.make_move(mv);

            let after_hash = b.hash;

            b.unmake_move(&_undo);

            assert_eq!(b.hash, saved.hash, "hash mismatch after unmake for move {mv}");
            assert_eq!(b.occupancy, saved.occupancy, "occupancy mismatch for move {mv}");
            assert_eq!(b.pieces_bb, saved.pieces_bb, "pieces_bb mismatch for move {mv}");
            assert_eq!(b.colors_bb, saved.colors_bb, "colors_bb mismatch for move {mv}");
            assert_eq!(b.king_square, saved.king_square, "king_square mismatch for move {mv}");
            assert_eq!(b.castling_rights, saved.castling_rights, "castling_rights mismatch for move {mv}");
            assert_eq!(b.en_passant, saved.en_passant, "en_passant mismatch for move {mv}");
            assert_eq!(b.halfmove_clock, saved.halfmove_clock, "halfmove_clock mismatch for move {mv}");
            assert_eq!(b.fullmove_number, saved.fullmove_number, "fullmove_number mismatch for move {mv}");
            assert_eq!(b.squares, saved.squares, "squares mismatch for move {mv}");
            assert_eq!(b.colors, saved.colors, "colors mismatch for move {mv}");

            assert_ne!(after_hash, saved.hash, "hash should change after move {mv}");
        }
    }

    #[test]
    fn test_fen_startpos() {
        let board = Board::from_initial();
        assert_eq!(board.piece_at(Square::A1), Some(Piece::Rook));
        assert_eq!(board.color_at(Square::A1), Some(Color::White));
        assert_eq!(board.piece_at(Square::E1), Some(Piece::King));
        assert_eq!(board.piece_at(Square::E8), Some(Piece::King));
        assert_eq!(board.piece_at(Square::D8), Some(Piece::Queen));
        assert_eq!(board.color_at(Square::D8), Some(Color::Black));
        assert_eq!(board.side_to_move(), Color::White);
        assert_eq!(board.castling_rights(), CastlingRights::ALL);
        assert_eq!(board.en_passant(), None);
    }

    #[test]
    fn test_fen_with_en_passant() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board = Board::from_fen(fen).expect("valid FEN");
        assert_eq!(board.en_passant(), Square::from_file_rank(4, 2)); // e3
        assert_eq!(board.side_to_move(), Color::Black);
    }

    #[test]
    fn test_fen_empty_board_no_castling() {
        let board = Board::from_fen("8/8/8/8/8/8/8/8 w - -").expect("valid");
        assert_eq!(board.occupancy(), 0);
        assert_eq!(board.castling_rights(), CastlingRights::NONE);
    }

    #[test]
    fn test_castling_rights_after_king_move() {
        init_slider_tables();
        let mut board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").expect("valid");
        assert_eq!(board.castling_rights(), CastlingRights::ALL);

        let e2 = Square::from_file_rank(4, 1).unwrap();
        let mv = Move::new(Square::E1, e2);
        let undo = board.make_move(mv);
        assert!(!board.castling_rights().has_wk());
        assert!(!board.castling_rights().has_wq());
        assert!(board.castling_rights().has_bk());
        assert!(board.castling_rights().has_bq());

        board.unmake_move(&undo);
        assert_eq!(board.castling_rights(), CastlingRights::ALL);
    }

    #[test]
    fn test_castling_rights_after_a1_rook_move() {
        init_slider_tables();
        let mut board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").expect("valid");

        let a2 = Square::from_file_rank(0, 1).unwrap();
        let mv = Move::new(Square::A1, a2);
        let undo = board.make_move(mv);
        assert!(!board.castling_rights().has_wq());
        assert!(board.castling_rights().has_wk());
        assert!(board.castling_rights().has_bk());
        assert!(board.castling_rights().has_bq());

        board.unmake_move(&undo);
        assert_eq!(board.castling_rights(), CastlingRights::ALL);
    }

    #[test]
    fn test_castling_rights_after_capturing_h8_rook() {
        init_slider_tables();
        let mut board = Board::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").expect("valid");

        let mv = Move::capture(Square::H1, Square::H8);
        let _undo = board.make_move(mv);
        assert!(!board.castling_rights().has_bk());
        assert!(board.castling_rights().has_bq());
    }

    #[test]
    fn test_in_check_startpos() {
        init_slider_tables();
        let board = Board::from_initial();
        assert!(!board.in_check());
        assert!(board.check_result().is_none());
    }

    #[test]
    fn test_checkmate_scholars_mate() {
        init_slider_tables();
        let fen = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";
        let board = Board::from_fen(fen).expect("valid");
        assert!(board.in_check());
        match board.check_result() {
            Some(GameResult::Checkmate(Color::White)) => {}
            other => panic!("expected Checkmate(White), got {other:?}"),
        }
    }

    #[test]
    fn test_stalemate_black_king_no_moves() {
        init_slider_tables();
        let fen = "7k/5Q2/8/8/8/8/8/K7 b - -";
        let board = Board::from_fen(fen).expect("valid");
        assert!(!board.in_check(), "king should not be in check");
        let mut moves = [Move::NULL; MAX_MOVES];
        let count = crate::movegen::generate_legal_moves(&board, &mut moves);
        assert_eq!(count, 0, "stalemate: no legal moves");
        match board.check_result() {
            Some(GameResult::Stalemate) => {}
            other => panic!("expected Stalemate, got {other:?}"),
        }
    }

    #[test]
    fn test_board_clone_independence() {
        init_slider_tables();
        let fen = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";
        let original = Board::from_fen(fen).expect("valid");

        let a7 = Square::from_file_rank(0, 6).unwrap();
        let a6 = Square::from_file_rank(0, 5).unwrap();

        let mut clone = original.clone();
        let mv = Move::new(a7, a6);
        let _undo = clone.make_move(mv);

        // original unchanged
        assert_eq!(original.piece_at(a7), Some(Piece::Pawn));
        assert_eq!(original.color_at(a7), Some(Color::Black));
        assert_eq!(original.piece_at(a6), None);
        assert_eq!(original.side_to_move(), Color::Black);

        // clone has move applied
        assert_eq!(clone.piece_at(a7), None);
        assert_eq!(clone.piece_at(a6), Some(Piece::Pawn));
        assert_eq!(clone.color_at(a6), Some(Color::Black));
        assert_eq!(clone.side_to_move(), Color::White);
    }

    #[test]
    fn test_scholars_mate_c4() {
        init_slider_tables();
        let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq -";
        let board = Board::from_fen(fen).expect("valid FEN");
        let f7 = Square::from_file_rank(5, 6).unwrap();
        let attacked = board.is_attacked_by(f7, Color::White);
        assert!(attacked, "f7 should be attacked by white (queen h5 + bishop c4)");
        // Qxf7 should be a legal move
        let mut buf = [Move::NULL; MAX_MOVES];
        let count = crate::movegen::generate_legal_moves(&board, &mut buf);
        let found_qxf7 = (0..count).any(|i| {
            let mv = buf[i];
            let s = format!("{mv}");
            if s == "h5f7" {
                let mut b = board.clone();
                let _undo = b.make_move(mv);
                let bk = b.king_square(Color::Black);
                let in_check = b.is_attacked_by(bk, Color::White);
                b.unmake_move(&_undo);
                assert!(in_check, "Black king should be in check after Qxf7#");
                true
            } else {
                false
            }
        });
        assert!(found_qxf7, "Qxf7 should be a legal move");
    }

    #[test]
    fn test_history_handles_max_depth_search() {
        init_slider_tables();
        let mut board = Board::from_initial();

        // Build up history to ~100 half-moves.
        for _ in 0..100 {
            let moves = crate::movegen::generate_legal_vec(&board);
            if moves.is_empty() { break; }
            board.make_move(moves[0]);
        }

        let base_len = board.history().len();

        // Simulate a deep search: make moves then unmake them all.
        // With the old [u64; 256], base_len(100) + search_depth(126+nulls) ~ 230,
        // which was tight. With [u64; 512] there's ample headroom.
        let mut undos = Vec::new();
        for _ in 0..150 {
            let moves = crate::movegen::generate_legal_vec(&board);
            if moves.is_empty() { break; }
            undos.push(board.make_move(moves[0]));
        }

        if !undos.is_empty() {
            let peak = board.history().len();
            assert_eq!(peak, base_len + undos.len(),
                "history should track each make_move");
        }

        for undo in undos.into_iter().rev() {
            board.unmake_move(&undo);
        }

        assert_eq!(board.history().len(), base_len,
            "history should be restored after unmake");
    }

    #[test]
    fn test_pinned_after_moving_into_pin_axis() {
        // Regression: moving a non-pinned piece into a pin axis should
        // correctly mark it as pinned for the next time that side moves.
        init_slider_tables();
        let fen = "4r3/8/8/8/8/3N4/8/4K3 w - -";
        let mut board = Board::from_fen(fen).expect("valid");
        let d3 = Square::from_file_rank(3, 2).unwrap();
        let e2 = Square::from_file_rank(4, 1).unwrap();

        // White knight on d3 is NOT pinned (rook on e8, king on e1 — knight off pin axis)
        let pinned_before = board.pinned_pieces(Color::White);
        assert_eq!(pinned_before & d3.bit(), 0, "Nd3 should not be pinned initially");

        // Move knight to e2 (now on e-file between king e1 and rook e8 → pinned)
        let mv = Move::new(d3, e2);
        let undo = board.make_move(mv);

        let pinned_after = board.pinned_pieces(Color::White);
        assert_ne!(pinned_after & e2.bit(), 0, "Ne2 should be pinned after moving into pin axis");

        board.unmake_move(&undo);
    }

    #[test]
    fn test_pinned_after_first_blocker_moves_away() {
        // Regression: when the first friendly piece on a pin ray moves away,
        // the piece behind it becomes pinned.
        init_slider_tables();
        let fen = "4r3/8/8/8/8/4R3/4N3/4K3 w - -";
        let mut board = Board::from_fen(fen).expect("valid");
        let e2 = Square::from_file_rank(4, 1).unwrap();
        let e3 = Square::from_file_rank(4, 2).unwrap();
        let d3 = Square::from_file_rank(3, 2).unwrap();

        // Ne2 is the first blocker on the e-file from king e1. Re3 is behind it.
        // Neither is pinned (two friendly pieces between king and enemy rook).
        let pinned_before = board.pinned_pieces(Color::White);
        assert_eq!(pinned_before & e2.bit(), 0, "Ne2 should not be pinned initially");
        assert_eq!(pinned_before & e3.bit(), 0, "Re3 should not be pinned initially");

        // Move knight off the e-file. Re3 is now the sole blocker → pinned.
        let mv = Move::new(e2, d3);
        let undo = board.make_move(mv);

        let pinned_after = board.pinned_pieces(Color::White);
        assert_eq!(pinned_after & d3.bit(), 0, "Nd3 should not be pinned (off pin axis)");
        assert_ne!(pinned_after & e3.bit(), 0, "Re3 should be pinned after first blocker moved away");

        board.unmake_move(&undo);
    }
}

// ---- Board struct ----
#[derive(Clone)]
pub struct Board {
    // mailbox (kept for piece-at queries and FEN)
    squares: [Option<Piece>; 64],
    colors: [Option<Color>; 64],

    // bitboards
    pieces_bb: [Bitboard; 6],    // per piece type (both colors)
    colors_bb: [Bitboard; 2],    // per color
    occupancy: Bitboard,

    side_to_move: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u8,
    fullmove_number: u16,
    hash: u64,
    king_square: [Square; 2],
    history: [u64; 512],
    history_len: u16,

    // cached derived state (updated incrementally in make_move)
    pinned: [Bitboard; 2],       // per-color bitboard of pinned pieces
    phase: i32,                   // game phase (0-24, non-pawn non-king weighted sum)
}

impl Board {
    #[cfg(test)]
    pub(crate) fn squares(&self) -> &[Option<Piece>; 64] { &self.squares }
    #[cfg(test)]
    pub(crate) fn colors(&self) -> &[Option<Color>; 64] { &self.colors }

    pub fn pieces_bb(&self, piece: Piece) -> Bitboard { self.pieces_bb[piece as usize] }
    pub fn colors_bb(&self, color: Color) -> Bitboard { self.colors_bb[color.index()] }
    pub fn occupancy(&self) -> Bitboard { self.occupancy }
    pub fn side_to_move(&self) -> Color { self.side_to_move }
    pub fn castling_rights(&self) -> CastlingRights { self.castling_rights }
    pub fn en_passant(&self) -> Option<Square> { self.en_passant }
    pub fn halfmove_clock(&self) -> u8 { self.halfmove_clock }
    pub fn fullmove_number(&self) -> u16 { self.fullmove_number }
    pub fn hash(&self) -> u64 { self.hash }
    pub fn king_square(&self, color: Color) -> Square { self.king_square[color.index()] }
    pub fn history(&self) -> &[u64] { &self.history[..(self.history_len as usize).min(self.history.len())] }

    pub fn phase(&self) -> i32 { self.phase }

    fn piece_phase_weight(piece: Piece) -> i32 {
        match piece {
            Piece::Knight => 1, Piece::Bishop => 1, Piece::Rook => 2, Piece::Queen => 4, _ => 0,
        }
    }

    fn new() -> Board {
        Board {
            squares: [None; 64], colors: [None; 64],
            pieces_bb: [0; 6], colors_bb: [0; 2], occupancy: 0,
            side_to_move: Color::White,
            castling_rights: CastlingRights::ALL,
            en_passant: None, halfmove_clock: 0, fullmove_number: 1,
            hash: 0,
            king_square: [Square::E1, Square::E8],
            history: [0; 512],
            history_len: 0,
            pinned: [0; 2],
            phase: 0,
        }
    }

    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 { return Err("Invalid FEN: need at least 4 fields".to_string()); }

        let mut board = Board::new();
        board.history_len = 0;

        let rank_strings: Vec<&str> = parts[0].split('/').collect();
        if rank_strings.len() != 8 { return Err("Invalid FEN: wrong number of ranks".to_string()); }

        for (rank_idx, rank_str) in rank_strings.iter().enumerate() {
            let rank = 7 - rank_idx as u8;
            let mut file: u8 = 0;
            for ch in rank_str.chars() {
                if file >= 8 { return Err("Invalid FEN: too many squares in rank".to_string()); }
                if ch.is_ascii_digit() {
                    file += ch.to_digit(10).unwrap() as u8;
                } else {
                    let color = if ch.is_ascii_uppercase() { Color::White } else { Color::Black };
                    let piece = match ch.to_ascii_lowercase() {
                        'p' => Piece::Pawn, 'n' => Piece::Knight, 'b' => Piece::Bishop,
                        'r' => Piece::Rook, 'q' => Piece::Queen, 'k' => Piece::King,
                        _ => return Err(format!("Invalid FEN: unknown piece '{ch}'")),
                    };
                    let sq = Square::from_file_rank(file, rank).unwrap();
                    board.place_piece(sq, piece, color);
                    if piece == Piece::King { board.king_square[color.index()] = sq; }
                    file += 1;
                }
            }
        }

        board.side_to_move = match parts[1] {
            "w" => Color::White, "b" => Color::Black,
            _ => return Err("Invalid FEN: side to move must be 'w' or 'b'".to_string()),
        };

        board.castling_rights = CastlingRights::NONE;
        for ch in parts[2].chars() {
            match ch {
                'K' => board.castling_rights.set_wk(true),
                'Q' => board.castling_rights.set_wq(true),
                'k' => board.castling_rights.set_bk(true),
                'q' => board.castling_rights.set_bq(true),
                '-' => {}
                _ => return Err(format!("Invalid FEN: unknown castling char '{ch}'")),
            }
        }

        board.en_passant = match parts[3] {
            "-" => None,
            sq_str if sq_str.len() == 2 => {
                let file = sq_str.as_bytes()[0].wrapping_sub(b'a');
                let rank = sq_str.as_bytes()[1].wrapping_sub(b'1');
                if file < 8 && rank < 8 { Square::from_file_rank(file, rank) }
                else { return Err(format!("Invalid FEN: bad en passant square '{sq_str}'")); }
            }
            _ => return Err("Invalid FEN: bad en passant field".to_string()),
        };

        if parts.len() >= 5 { board.halfmove_clock = parts[4].parse::<u8>().map_err(|_| "Invalid halfmove clock".to_string())?; }
        if parts.len() >= 6 { board.fullmove_number = parts[5].parse::<u16>().map_err(|_| "Invalid fullmove number".to_string())?; }

        board.hash = zobrist::compute_initial_hash(&board.squares, &board.colors, board.side_to_move, board.castling_rights, board.en_passant);
        board.phase = board.compute_phase();
        board.pinned = [board.compute_pinned(Color::White), board.compute_pinned(Color::Black)];
        Ok(board)
    }

    pub fn from_initial() -> Board { Board::from_fen(INITIAL_FEN).expect("initial FEN should be valid") }

    fn place_piece(&mut self, sq: Square, piece: Piece, color: Color) {
        let idx = sq.index() as usize;
        self.squares[idx] = Some(piece);
        self.colors[idx] = Some(color);
        self.pieces_bb[piece as usize] |= sq.bit();
        self.colors_bb[color.index()] |= sq.bit();
        self.occupancy |= sq.bit();
    }

    fn remove_piece(&mut self, sq: Square, piece: Piece, color: Color) {
        let idx = sq.index() as usize;
        self.squares[idx] = None;
        self.colors[idx] = None;
        self.pieces_bb[piece as usize] &= !sq.bit();
        self.colors_bb[color.index()] &= !sq.bit();
        self.occupancy &= !sq.bit();
    }

    fn move_piece(&mut self, from: Square, to: Square, piece: Piece, color: Color) {
        let fi = from.index() as usize;
        let ti = to.index() as usize;
        self.squares[fi] = None;
        self.colors[fi] = None;
        self.squares[ti] = Some(piece);
        self.colors[ti] = Some(color);
        let from_bit = from.bit();
        let to_bit = to.bit();
        self.pieces_bb[piece as usize] = (self.pieces_bb[piece as usize] & !from_bit) | to_bit;
        self.colors_bb[color.index()] = (self.colors_bb[color.index()] & !from_bit) | to_bit;
        self.occupancy = (self.occupancy & !from_bit) | to_bit;
    }

    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> { self.squares[sq.index() as usize] }

    #[inline]
    pub fn color_at(&self, sq: Square) -> Option<Color> { self.colors[sq.index() as usize] }

    #[inline]
    pub fn empty_square(&self, sq: Square) -> bool { self.squares[sq.index() as usize].is_none() }

    pub fn is_attacked_by(&self, sq: Square, by_color: Color) -> bool {
        let occ = self.occupancy;
        let s = sq.index();
        let enemy = self.colors_bb[by_color.index()];
        if enemy == 0 { return false; }

        // pawn attacks
        let pawn_bb = self.pieces_bb[Piece::Pawn as usize] & enemy;
        if pawn_bb & crate::attack::pawn_attacks(sq, by_color.flip()) != 0 { return true; }

        // knight attacks
        let knight_bb = self.pieces_bb[Piece::Knight as usize] & enemy;
        if knight_bb & crate::attack::knight_attacks(sq) != 0 { return true; }

        // king attacks
        let king_bb = self.pieces_bb[Piece::King as usize] & enemy;
        if king_bb & crate::attack::king_attacks(sq) != 0 { return true; }

        // sliding pieces — compute each direction only if relevant pieces exist
        let rooks = self.pieces_bb[Piece::Rook as usize] & enemy;
        let bishops = self.pieces_bb[Piece::Bishop as usize] & enemy;
        let queens = self.pieces_bb[Piece::Queen as usize] & enemy;

        let rook_sliders = rooks | queens;
        let bishop_sliders = bishops | queens;
        if rook_sliders != 0 {
            let r_atk = crate::attack::rook_attacks(s, occ);
            if rook_sliders & r_atk != 0 { return true; }
        }
        if bishop_sliders != 0 {
            let b_atk = crate::attack::bishop_attacks(s, occ);
            if bishop_sliders & b_atk != 0 { return true; }
        }

        false
    }

    pub fn in_check(&self) -> bool {
        self.is_attacked_by(self.king_square[self.side_to_move.index()], self.side_to_move.flip())
    }

    pub fn pinned_pieces(&self, king_color: Color) -> Bitboard {
        self.pinned[king_color.index()]
    }

    fn compute_phase(&self) -> i32 {
        let mut phase = 0i32;
        phase += self.pieces_bb[Piece::Knight as usize].count_ones() as i32;
        phase += self.pieces_bb[Piece::Bishop as usize].count_ones() as i32;
        phase += self.pieces_bb[Piece::Rook as usize].count_ones() as i32 * 2;
        phase += self.pieces_bb[Piece::Queen as usize].count_ones() as i32 * 4;
        phase.min(24)
    }

    fn compute_pinned(&self, king_color: Color) -> Bitboard {
        self.compute_pinned_impl(king_color)
    }

    /// Bitboard of pieces pinned to the given king.
    /// A piece is pinned if it stands between its king and an enemy slider (rook/bishop/queen).
    fn compute_pinned_impl(&self, king_color: Color) -> Bitboard {
        let king_sq = self.king_square[king_color.index()];
        let friend = self.colors_bb[king_color.index()];
        let enemy = self.colors_bb[king_color.flip().index()];
        let enemy_rooks = self.pieces_bb[Piece::Rook as usize] & enemy;
        let enemy_bishops = self.pieces_bb[Piece::Bishop as usize] & enemy;
        let enemy_queens = self.pieces_bb[Piece::Queen as usize] & enemy;
        let has_ortho_sliders = (enemy_rooks | enemy_queens) != 0;
        let has_diag_sliders = (enemy_bishops | enemy_queens) != 0;

        // Early exit: no enemy sliders → no pins
        if !has_ortho_sliders && !has_diag_sliders {
            return 0;
        }

        let king_file = king_sq.file() as i32;
        let king_rank = king_sq.rank() as i32;

        let mut pinned = 0u64;
        let dirs: [(i32, i32); 8] = [
            (0, 1), (1, 1), (1, 0), (1, -1),
            (0, -1), (-1, -1), (-1, 0), (-1, 1),
        ];

        for &(df, dr) in dirs.iter() {
            let is_ortho = df == 0 || dr == 0;
            // Skip scan if no relevant sliders for this direction
            if is_ortho && !has_ortho_sliders { continue; }
            if !is_ortho && !has_diag_sliders { continue; }

            let mut maybe_pinned: Option<u64> = None;
            let mut f = king_file + df;
            let mut r = king_rank + dr;

            while f >= 0 && f < 8 && r >= 0 && r < 8 {
                let sq_bit = 1u64 << (r * 8 + f);
                if sq_bit & friend != 0 {
                    if maybe_pinned.is_some() {
                        // Two friendly pieces on this ray — no pin
                        break;
                    }
                    maybe_pinned = Some(sq_bit);
                } else if sq_bit & enemy != 0 {
                    if let Some(pb) = maybe_pinned {
                        let is_slider = if is_ortho {
                            sq_bit & (enemy_rooks | enemy_queens) != 0
                        } else {
                            sq_bit & (enemy_bishops | enemy_queens) != 0
                        };
                        if is_slider {
                            pinned |= pb;
                        }
                    }
                    break;
                }
                f += df;
                r += dr;
            }
        }
        pinned
    }

    // ---- make/unmake ----
    pub fn make_move(&mut self, mv: Move) -> UndoInfo {
        let from = mv.from();
        let to = mv.to();
        let piece = self.piece_at(from).unwrap();
        let color = self.side_to_move;

        let mut undo = UndoInfo {
            mv,
            captured: None,
            captured_sq: to,
            prev_castling: self.castling_rights,
            prev_ep: self.en_passant,
            prev_halfmove: self.halfmove_clock,
            prev_hash: self.hash,
            prev_phase: self.phase,
            prev_pinned: self.pinned,
        };

        self.history[self.history_len as usize] = self.hash;
        self.history_len += 1;
        self.hash ^= zobrist::zobrist_piece_square(color, piece, from);

        let captured = self.piece_at(to);
        let mut phase_delta: i32 = 0;

        match mv.kind() {
            MoveKind::Normal | MoveKind::Promotion => {
                if let Some(cap_piece) = captured {
                    let cap_color = color.flip();
                    undo.captured = Some((cap_piece, cap_color));
                    self.hash ^= zobrist::zobrist_piece_square(cap_color, cap_piece, to);
                    self.remove_piece(to, cap_piece, cap_color);
                    phase_delta -= Self::piece_phase_weight(cap_piece);
                }
            }
            MoveKind::Capture => {
                if let Some(cap_piece) = captured {
                    let cap_color = color.flip();
                    undo.captured = Some((cap_piece, cap_color));
                    undo.captured_sq = to;
                    self.hash ^= zobrist::zobrist_piece_square(cap_color, cap_piece, to);
                    self.remove_piece(to, cap_piece, cap_color);
                    phase_delta -= Self::piece_phase_weight(cap_piece);
                } else if let Some(ep) = self.en_passant {
                    if to == ep && piece == Piece::Pawn {
                        let cap_rank = from.rank();
                        let cap_sq = Square::from_file_rank(to.file(), cap_rank).unwrap();
                        let cap_color = color.flip();
                        undo.captured = Some((Piece::Pawn, cap_color));
                        undo.captured_sq = cap_sq;
                        self.hash ^= zobrist::zobrist_piece_square(cap_color, Piece::Pawn, cap_sq);
                        self.remove_piece(cap_sq, Piece::Pawn, cap_color);
                        phase_delta -= Self::piece_phase_weight(Piece::Pawn);
                    }
                }
            }
            MoveKind::Castle => {
                let (rook_from, rook_to) = if to.file() > from.file() {
                    (Square::from_file_rank(7, from.rank()).unwrap(),
                     Square::from_file_rank(5, from.rank()).unwrap())
                } else {
                    (Square::from_file_rank(0, from.rank()).unwrap(),
                     Square::from_file_rank(3, from.rank()).unwrap())
                };
                self.hash ^= zobrist::zobrist_piece_square(color, Piece::Rook, rook_from);
                self.hash ^= zobrist::zobrist_piece_square(color, Piece::Rook, rook_to);
                self.move_piece(rook_from, rook_to, Piece::Rook, color);
            }
        }

        if piece == Piece::Pawn || captured.is_some() || mv.kind() == MoveKind::Capture {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if color == Color::Black { self.fullmove_number += 1; }

        if mv.kind() == MoveKind::Promotion {
            let promo = mv.promotion_piece().unwrap_or(Piece::Queen);
            self.remove_piece(from, piece, color);
            self.place_piece(to, promo, color);
            self.hash ^= zobrist::zobrist_piece_square(color, promo, to);
            phase_delta += Self::piece_phase_weight(promo) - Self::piece_phase_weight(Piece::Pawn);
        } else {
            self.move_piece(from, to, piece, color);
            self.hash ^= zobrist::zobrist_piece_square(color, piece, to);
        }

        if let Some(ep) = self.en_passant {
            self.hash ^= zobrist::zobrist_en_passant(Some(ep.file()));
        }

        self.en_passant = if mv.kind() == MoveKind::Normal && piece == Piece::Pawn
            && (from.rank() as i8 - to.rank() as i8).abs() == 2
        {
            Some(Square::from_file_rank(from.file(), (from.rank() + to.rank()) / 2).unwrap())
        } else { None };

        if let Some(ep) = self.en_passant {
            self.hash ^= zobrist::zobrist_en_passant(Some(ep.file()));
        }

        let old_castling = self.castling_rights;
        let mut lose_mask = 0u8;
        if piece == Piece::King {
            lose_mask |= CASTLE_LOSE_MASK[from.index() as usize];
            self.king_square[color.index()] = to;
        } else if piece == Piece::Rook {
            lose_mask |= CASTLE_LOSE_MASK[from.index() as usize];
        }
        if captured.is_some() {
            lose_mask |= CASTLE_LOSE_MASK[to.index() as usize];
        }
        self.castling_rights.remove_by_mask(lose_mask);

        self.hash ^= zobrist::zobrist_castling(old_castling);
        self.hash ^= zobrist::zobrist_castling(self.castling_rights);
        self.hash ^= zobrist::zobrist_side_to_move();
        self.side_to_move = color.flip();

        self.phase = (self.phase + phase_delta).clamp(0, 24);
        if piece == Piece::King || (from.bit() & self.pinned[color.index()]) != 0 {
            self.pinned[color.index()] = self.compute_pinned_impl(color);
        } else {
            let ks = self.king_square[color.index()];
            let kf = ks.file() as i32;
            let kr = ks.rank() as i32;
            let on_axis = |sq: Square| -> bool {
                let f = sq.file() as i32;
                let r = sq.rank() as i32;
                f == kf || r == kr || (f - kf).abs() == (r - kr).abs()
            };
            if on_axis(from) || on_axis(to) {
                self.pinned[color.index()] = self.compute_pinned_impl(color);
            }
        }
        let is_slider = piece == Piece::Bishop || piece == Piece::Rook || piece == Piece::Queen;
        let need_opponent_pin_update = is_slider
            || mv.kind() == MoveKind::Promotion
            || captured.is_some()
            || mv.kind() == MoveKind::Capture
            || mv.kind() == MoveKind::Castle;
        if need_opponent_pin_update {
            self.pinned[color.flip().index()] = self.compute_pinned_impl(color.flip());
        }

        undo
    }

    pub fn unmake_move(&mut self, undo: &UndoInfo) {
        let mv = undo.mv;
        let from = mv.from();
        let to = mv.to();
        let color = self.side_to_move.flip();
        let piece = self.piece_at(to).unwrap();

        if mv.kind() == MoveKind::Promotion {
            let pawn_color = color;
            self.remove_piece(to, piece, color);
            self.place_piece(from, Piece::Pawn, pawn_color);
        } else {
            self.move_piece(to, from, piece, color);
        }

        if let Some((cap_piece, cap_color)) = undo.captured {
            self.place_piece(undo.captured_sq, cap_piece, cap_color);
        }

        if mv.kind() == MoveKind::Castle {
            let rook_to = if to.file() > from.file() {
                Square::from_file_rank(5, from.rank()).unwrap()
            } else {
                Square::from_file_rank(3, from.rank()).unwrap()
            };
            let rook_from = if to.file() > from.file() {
                Square::from_file_rank(7, from.rank()).unwrap()
            } else {
                Square::from_file_rank(0, from.rank()).unwrap()
            };
            self.move_piece(rook_to, rook_from, Piece::Rook, color);
        }

        if piece == Piece::King {
            self.king_square[color.index()] = from;
        }

        self.castling_rights = undo.prev_castling;
        self.en_passant = undo.prev_ep;
        self.halfmove_clock = undo.prev_halfmove;
        self.hash = undo.prev_hash;
        self.phase = undo.prev_phase;
        self.pinned = undo.prev_pinned;
        self.history_len -= 1;
        self.side_to_move = color;
        self.fullmove_number = if color == Color::Black { self.fullmove_number - 1 } else { self.fullmove_number };
    }

    // null move (for null-move pruning in search)
    pub fn make_null_move(&mut self) -> UndoInfo {
        let undo = UndoInfo {
            mv: Move::NULL,
            captured: None,
            captured_sq: Square::A1, // dummy
            prev_castling: self.castling_rights,
            prev_ep: self.en_passant,
            prev_halfmove: self.halfmove_clock,
            prev_hash: self.hash,
            prev_phase: self.phase,
            prev_pinned: self.pinned,
        };
        self.history[self.history_len as usize] = self.hash;
        self.history_len += 1;
        if let Some(ep) = self.en_passant {
            self.hash ^= crate::zobrist::zobrist_en_passant(Some(ep.file()));
        }
        self.en_passant = None;
        self.side_to_move = self.side_to_move.flip();
        self.hash ^= crate::zobrist::zobrist_side_to_move();
        undo
    }

    pub fn unmake_null_move(&mut self, undo: &UndoInfo) {
        self.castling_rights = undo.prev_castling;
        self.en_passant = undo.prev_ep;
        self.halfmove_clock = undo.prev_halfmove;
        self.hash = undo.prev_hash;
        self.phase = undo.prev_phase;
        self.pinned = undo.prev_pinned;
        self.history_len -= 1;
        self.side_to_move = self.side_to_move.flip();
    }

    pub fn check_result(&self) -> Option<GameResult> {
        let mut moves_buf = [Move::NULL; MAX_MOVES];
        let count = crate::movegen::generate_legal_moves(self, &mut moves_buf);
        if count == 0 {
            if self.in_check() {
                Some(GameResult::Checkmate(self.side_to_move.flip()))
            } else {
                Some(GameResult::Stalemate)
            }
        } else if crate::draw::is_draw_by_rule(self) {
            Some(GameResult::Draw)
        } else {
            None
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{} |", rank + 1)?;
            for file in 0..8 {
                let sq = Square::from_file_rank(file, rank).unwrap();
                match self.piece_at(sq) {
                    None => write!(f, " .")?,
                    Some(piece) => {
                        let c = match piece {
                            Piece::Pawn => 'p', Piece::Knight => 'n', Piece::Bishop => 'b',
                            Piece::Rook => 'r', Piece::Queen => 'q', Piece::King => 'k',
                        };
                        let ch = if self.color_at(sq) == Some(Color::White) { c.to_ascii_uppercase() } else { c };
                        write!(f, " {ch}")?;
                    }
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "   ----------------")?;
        writeln!(f, "    a b c d e f g h")?;
        write!(f, "    {} to move", if self.side_to_move() == Color::White { "White" } else { "Black" })?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    Checkmate(Color),
    Stalemate,
    Draw,
}

impl fmt::Display for GameResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameResult::Checkmate(winner) => write!(f, "Checkmate! {winner:?} wins"),
            GameResult::Stalemate => write!(f, "Stalemate! Draw"),
            GameResult::Draw => write!(f, "Draw"),
        }
    }
}
