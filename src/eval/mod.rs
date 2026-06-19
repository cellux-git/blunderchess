pub(crate) mod params;
pub(crate) use params::Eval;

mod pawns;
mod kings;
mod mobility;
mod pieces;
mod see;

use crate::board::Board;
use crate::types::{Color, Piece};
#[cfg(test)]
use crate::types::Square;
use std::sync::LazyLock;

pub(crate) static EVAL: LazyLock<Eval> = LazyLock::new(Eval::default);

fn scale_positional(pos: i32, deficit: i32) -> i32 {
    // Only scale when down more than ~2 pawns (200 cp) — a single pawn
    // capture doesn't trigger scaling, avoiding false positives at
    // intermediate search nodes where the recapture hasn't happened yet.
    if deficit < 200 { return pos; }
    let s = 200.0 / (200.0 + deficit as f32);
    (pos as f32 * s) as i32
}

impl Eval {
    pub(crate) fn material_value(&self, piece: Piece) -> i32 {
        match piece {
            Piece::Pawn => self.material.pawn_value,
            Piece::Knight => self.material.knight_value,
            Piece::Bishop => self.material.bishop_value,
            Piece::Rook => self.material.rook_value,
            Piece::Queen => self.material.queen_value,
            Piece::King => self.material.king_value,
        }
    }

    #[inline]
    fn pst_value_raw(&self, mg_table: &[i32; 64], eg_table: &[i32; 64], sq_idx: u8, color: Color) -> (i32, i32) {
        let idx = if color == Color::White {
            sq_idx as usize
        } else {
            (sq_idx ^ 56) as usize
        };
        (mg_table[idx], eg_table[idx])
    }

    pub fn evaluate(&self, board: &Board) -> i32 {
        let phase = board.phase();
        let max_phase = 24;

        let w_mat = self.material_count(board, Color::White);
        let b_mat = self.material_count(board, Color::Black);
        let mat_mg_diff = w_mat - b_mat;
        let w_non_king = board.colors_bb(Color::White) & !board.pieces_bb(Piece::King);
        let b_non_king = board.colors_bb(Color::Black) & !board.pieces_bb(Piece::King);
        if mat_mg_diff.abs() > 1200 && w_non_king != 0 && b_non_king != 0 {
            let lazy_score = mat_mg_diff;
            return if board.side_to_move() == Color::White { lazy_score } else { -lazy_score };
        }

        let w_pawn_bb = board.pieces_bb(Piece::Pawn) & board.colors_bb(Color::White);
        let b_pawn_bb = board.pieces_bb(Piece::Pawn) & board.colors_bb(Color::Black);
        let w_pawn_attacks = mobility::enemy_pawn_attack_mask(w_pawn_bb, Color::White);
        let b_pawn_attacks = mobility::enemy_pawn_attack_mask(b_pawn_bb, Color::Black);

        let (w_mat_mg, w_pos_mg, w_mat_eg, w_pos_eg) = self.evaluate_side(board, Color::White, b_pawn_attacks);
        let (b_mat_mg, b_pos_mg, b_mat_eg, b_pos_eg) = self.evaluate_side(board, Color::Black, w_pawn_attacks);

        let mat_diff_mg = w_mat_mg - b_mat_mg;
        let mat_diff_eg = w_mat_eg - b_mat_eg;

        let w_mg = w_mat_mg + scale_positional(w_pos_mg, -mat_diff_mg);
        let b_mg = b_mat_mg + scale_positional(b_pos_mg, mat_diff_mg);
        let w_eg = w_mat_eg + scale_positional(w_pos_eg, -mat_diff_eg);
        let b_eg = b_mat_eg + scale_positional(b_pos_eg, mat_diff_eg);

        let mg = w_mg - b_mg;
        let eg = w_eg - b_eg;

        let score = (mg * phase + eg * (max_phase - phase)) / max_phase;

        if board.side_to_move() == Color::White { score } else { -score }
    }

    fn material_count(&self, board: &Board, color: Color) -> i32 {
        let us_bb = board.colors_bb(color);
        self.material.pawn_value * (board.pieces_bb(Piece::Pawn) & us_bb).count_ones() as i32
            + self.material.knight_value * (board.pieces_bb(Piece::Knight) & us_bb).count_ones() as i32
            + self.material.bishop_value * (board.pieces_bb(Piece::Bishop) & us_bb).count_ones() as i32
            + self.material.rook_value * (board.pieces_bb(Piece::Rook) & us_bb).count_ones() as i32
            + self.material.queen_value * (board.pieces_bb(Piece::Queen) & us_bb).count_ones() as i32
    }

    fn evaluate_side(&self, board: &Board, color: Color, enemy_pawn_attacks: u64) -> (i32, i32, i32, i32) {
        let mut mat_mg = 0i32;
        let mut mat_eg = 0i32;
        let mut pos_mg = 0i32;
        let mut pos_eg = 0i32;

        let enemy = color.flip();
        let us_bb = board.colors_bb(color);
        let enemy_bb = board.colors_bb(enemy);
        let pawns_bb = board.pieces_bb(Piece::Pawn) & us_bb;
        let enemy_pawns_bb = board.pieces_bb(Piece::Pawn) & enemy_bb;
        let king_sq = board.king_square(color);

        let our_pawns = board.pieces_bb(Piece::Pawn) & us_bb;
        let our_knights = board.pieces_bb(Piece::Knight) & us_bb;
        let our_bishops = board.pieces_bb(Piece::Bishop) & us_bb;
        let our_rooks = board.pieces_bb(Piece::Rook) & us_bb;
        let our_queens = board.pieces_bb(Piece::Queen) & us_bb;
        let our_king = board.pieces_bb(Piece::King) & us_bb;

        macro_rules! accumulate {
            ($piece:ident, $mg_table:ident, $eg_table:ident, $bb:expr) => {
                let val = self.material_value(Piece::$piece);
                let mut bb = $bb;
                let cnt = bb.count_ones() as i32;
                mat_mg += val * cnt;
                mat_eg += val * cnt;
                while bb != 0 {
                    let idx = bb.trailing_zeros() as u8;
                    let (mg_pst, eg_pst) = self.pst_value_raw(&self.pst.$mg_table, &self.pst.$eg_table, idx, color);
                    pos_mg += mg_pst;
                    pos_eg += eg_pst;
                    bb &= bb - 1;
                }
            };
        }
        accumulate!(Pawn, mg_pawn_table, eg_pawn_table, our_pawns);
        accumulate!(Knight, mg_knight_table, eg_knight_table, our_knights);
        accumulate!(Bishop, mg_bishop_table, eg_bishop_table, our_bishops);
        accumulate!(Rook, mg_rook_table, eg_rook_table, our_rooks);
        accumulate!(Queen, mg_queen_table, eg_queen_table, our_queens);
        accumulate!(King, mg_king_table, eg_king_table, our_king);

        if color == Color::White {
            if board.castling_rights().has_wk() { pos_mg += self.king.castling_rights_kingside_bonus; }
            if board.castling_rights().has_wq() { pos_mg += self.king.castling_rights_queenside_bonus; }
        } else {
            if board.castling_rights().has_bk() { pos_mg += self.king.castling_rights_kingside_bonus; }
            if board.castling_rights().has_bq() { pos_mg += self.king.castling_rights_queenside_bonus; }
        }

        let (m, e, my_passers) = pawns::eval_pawns(board, &self.pawn, pawns_bb, enemy_pawns_bb, color);
        pos_mg += m; pos_eg += e;

        if our_bishops.count_ones() >= 2 {
            pos_mg += self.piece.bishop_pair_bonus.0;
            pos_eg += self.piece.bishop_pair_bonus.1;
        }

        let (m, e) = pieces::eval_bad_bishops(board, &self.piece, color, pawns_bb, our_bishops);
        pos_mg += m; pos_eg += e;

        let (m, e) = pieces::eval_rooks(board, &self.piece, pawns_bb, enemy_pawns_bb, color, our_rooks);
        pos_mg += m; pos_eg += e;

        let (m, e) = pieces::eval_rook_queen_battery(board, &self.piece, color, our_rooks, our_queens);
        pos_mg += m; pos_eg += e;

        let (m, e) = pieces::eval_knights(board, &self.piece, color, enemy_pawns_bb, our_knights);
        pos_mg += m; pos_eg += e;

        let (m, e) = pieces::eval_queen_multiattack(board, &self.piece, color, enemy, our_queens);
        pos_mg += m; pos_eg += e;

        let (m, e) = pawns::eval_connected_passers(&self.king, my_passers);
        pos_mg += m; pos_eg += e;
        let (m, e) = pawns::eval_rook_behind_passer(board, &self.king, color, my_passers);
        pos_mg += m; pos_eg += e;
        let (m, e) = kings::eval_king_passer_proximity(board, &self.king, color, my_passers);
        pos_mg += m; pos_eg += e;

        let (m, e) = pawns::eval_candidate_passers(&self.pawn, board, pawns_bb, enemy_pawns_bb, color);
        pos_mg += m; pos_eg += e;

        let (m, e) = pawns::eval_passer_blocker(board, &self.pawn, pawns_bb, enemy_pawns_bb, color);
        pos_mg += m; pos_eg += e;

        let (mobility_mg, mobility_eg) = mobility::eval_mobility(board, &self.mobility, color, enemy_pawn_attacks, our_knights, our_bishops, our_rooks, our_queens);
        pos_mg += mobility_mg;
        pos_eg += mobility_eg;

        let king_safety_mg = kings::eval_king_safety(board, &self.king, color, king_sq, pawns_bb, enemy_bb);
        pos_mg += king_safety_mg;

        let (m, e) = kings::eval_king_opposition(board, &self.king, color);
        pos_mg += m; pos_eg += e;

        let (m, e) = pieces::eval_space(&self.pawn, pawns_bb, color);
        pos_mg += m; pos_eg += e;

        let (m, e) = pieces::eval_pawn_majority(&self.pawn, pawns_bb, enemy_pawns_bb);
        pos_mg += m; pos_eg += e;

        let (m, e) = pieces::eval_exchange(board, &self.piece, color, pawns_bb);
        pos_mg += m; pos_eg += e;

        (mat_mg, pos_mg, mat_eg, pos_eg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::types::Move;

    #[test]
    fn test_initial_position_near_zero() {
        let board = Board::from_initial();
        let score = Eval::default().evaluate(&board);
        assert!(score.abs() <= 50);
    }

    #[test]
    fn test_initial_position_symmetric() {
        let board = Board::from_initial();
        let score_white = Eval::default().evaluate(&board);
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board2 = Board::from_fen(fen).unwrap();
        let score_black = Eval::default().evaluate(&board2);
        assert!(score_white.abs() <= 50);
        assert!(score_black.abs() <= 120);
    }

    #[test]
    fn test_white_advantage_positive() {
        let board = Board::from_initial();
        let score_initial = Eval::default().evaluate(&board);
        let fen_up = "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board_up = Board::from_fen(fen_up).unwrap();
        let score_up = Eval::default().evaluate(&board_up);
        assert!(score_up > score_initial);
    }

    #[test]
    fn test_black_checkmate_scores_negative() {
        let fen = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";
        let board = Board::from_fen(fen).unwrap();
        let score = Eval::default().evaluate(&board);
        assert!(score <= 100);
    }

    #[test]
    fn test_symmetric_position_evaluates_equal() {
        let board = Board::from_initial();
        let score = Eval::default().evaluate(&board);
        assert!(score.abs() < 100);
    }

    #[test]
    fn test_bishop_pair_bonus() {
        let fen_b = "k7/8/8/8/8/8/8/2B1B2K w - - 0 1";
        let fen_n = "k7/8/8/8/8/8/8/2N1N2K w - - 0 1";
        let score_b = Eval::default().evaluate(&Board::from_fen(fen_b).unwrap());
        let score_n = Eval::default().evaluate(&Board::from_fen(fen_n).unwrap());
        assert!(score_b > score_n + 20);
    }

    #[test]
    fn test_rook_open_file() {
        let fen_open = "k7/8/8/8/8/8/8/K1R5 w - - 0 1";
        let fen_closed = "k7/8/8/8/8/2p5/8/K1R5 w - - 0 1";
        let score_open = Eval::default().evaluate(&Board::from_fen(fen_open).unwrap());
        let score_closed = Eval::default().evaluate(&Board::from_fen(fen_closed).unwrap());
        assert!(score_open > score_closed);
    }

    #[test]
    fn test_passed_pawn_bonus() {
        let fen = "k7/8/8/8/P7/8/8/K7 w - - 0 1";
        let score = Eval::default().evaluate(&Board::from_fen(fen).unwrap());
        assert!(score > 60);
    }

    #[test]
    fn test_doubled_pawns_penalty() {
        let fen_dbl = "k7/8/8/8/8/P7/P7/K7 w - - 0 1";
        let fen_spread = "k7/8/8/8/8/8/P1P5/K7 w - - 0 1";
        let score_dbl = Eval::default().evaluate(&Board::from_fen(fen_dbl).unwrap());
        let score_spread = Eval::default().evaluate(&Board::from_fen(fen_spread).unwrap());
        assert!(score_spread > score_dbl);
    }

    #[test]
    fn test_king_safety_pawn_shield() {
        let fen_shield = "k7/8/8/8/8/8/2P1K3/8 w - - 0 1";
        let fen_bare = "k7/8/8/8/8/8/4K3/8 w - - 0 1";
        let score_shield = Eval::default().evaluate(&Board::from_fen(fen_shield).unwrap());
        let score_bare = Eval::default().evaluate(&Board::from_fen(fen_bare).unwrap());
        assert!(score_shield > score_bare);
    }

    // --- Custom Eval tests ---

    #[test]
    fn test_custom_piece_values() {
        let board = Board::from_fen("k7/8/8/8/8/8/8/K1N5 w - - 0 1").unwrap();
        let mut ev = Eval::default();
        ev.material.knight_value = 200;
        let low = ev.evaluate(&board);
        ev.material.knight_value = 500;
        let high = ev.evaluate(&board);
        assert!(high > low);
    }

    // --- SEE tests ---

    #[test]
    fn test_see_pawn_takes_knight() {
        crate::attack::init_slider_tables();
        let board = Board::from_fen("8/8/8/3n4/4P3/8/8/8 w - -").unwrap();
        let e4 = Square::from_file_rank(4, 3).unwrap();
        let d5 = Square::from_file_rank(3, 4).unwrap();
        let mv = Move::capture(e4, d5);
        assert_eq!(Eval::default().see(&board, mv), 320);
    }

    #[test]
    fn test_see_losing_capture() {
        crate::attack::init_slider_tables();
        let board = Board::from_fen("8/8/3p4/4r3/8/8/4Q3/8 w - -").unwrap();
        let e2 = Square::from_file_rank(4, 1).unwrap();
        let e5 = Square::from_file_rank(4, 4).unwrap();
        let mv = Move::capture(e2, e5);
        assert!(Eval::default().see(&board, mv) < 0, "QxR with pawn recapture should be losing");
    }

    #[test]
    fn test_see_winning_capture() {
        crate::attack::init_slider_tables();
        let board = Board::from_fen("8/8/8/4n3/8/8/8/4R3 w - -").unwrap();
        let e1 = Square::E1;
        let e5 = Square::from_file_rank(4, 4).unwrap();
        let mv = Move::capture(e1, e5);
        assert!(Eval::default().see(&board, mv) > 0);
    }

    #[test]
    fn test_see_promotion_capture() {
        crate::attack::init_slider_tables();
        let board = Board::from_fen("3r4/4P3/8/8/8/8/8/8 w - -").unwrap();
        let e7 = Square::from_file_rank(4, 6).unwrap();
        let d8 = Square::from_file_rank(3, 7).unwrap();
        let mv = Move::promotion(e7, d8, Piece::Queen);
        assert!(Eval::default().see(&board, mv) > 0);
    }

    #[test]
    fn test_see_even_exchange() {
        crate::attack::init_slider_tables();
        let board = Board::from_fen("r3r3/8/8/8/8/8/8/4R3 w - -").unwrap();
        let e1 = Square::E1;
        let e8 = Square::from_file_rank(4, 7).unwrap();
        let mv = Move::capture(e1, e8);
        assert_eq!(Eval::default().see(&board, mv), 0);
    }

    #[test]
    fn test_see_en_passant_winning() {
        crate::attack::init_slider_tables();
        let fen = "8/8/8/3pP3/8/8/8/8 w - d6 0 1";
        let board = Board::from_fen(fen).unwrap();
        let e5 = Square::from_file_rank(4, 4).unwrap();
        let d6 = Square::from_file_rank(3, 5).unwrap();
        let mv = Move::ep(e5, d6);
        // Captures a pawn worth 100, base_gain should be 100
        assert_eq!(Eval::default().see(&board, mv), 100,
            "en passant should see +100 (pawn win)");
    }

    #[test]
    fn test_see_en_passant_defended() {
        crate::attack::init_slider_tables();
        // Black just played f7-f5, en passant square is f6.
        // White pawn on e5 can ep-capture, but black bishop on g7 recaptures.
        // Net: pawn for pawn = 0 (even exchange).
        let fen = "8/6b1/8/4Pp2/8/8/8/8 w - f6 0 1";
        let board = Board::from_fen(fen).unwrap();
        let e5 = Square::from_file_rank(4, 4).unwrap();
        let f6 = Square::from_file_rank(5, 5).unwrap();
        let mv = Move::ep(e5, f6);
        let see_val = Eval::default().see(&board, mv);
        assert_eq!(see_val, 0,
            "en passant with bishop defender should be even exchange, got {see_val}");
    }

    // ── 26 positional term tests ──

    #[test]
    fn test_isolated_pawn_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/P1P5/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/PP6/8/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_backward_pawn_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/4p3/4P3/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/4p3/4P3/3P4/8/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_pawn_phalanx_bonus() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/PP6/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/P1P5/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_pawn_chain_bonus() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/3P4/4P3/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/3P4/6P1/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_rook_semi_open_file() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/1p6/8/8/8/P7/8/KR6 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/p7/8/8/8/P7/8/KR6 w - -").unwrap());
        assert!(s2 > s1, "open file should score higher than semi-open");
    }

    #[test]
    fn test_rook_closed_file_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/p7/8/8/8/1P6/8/KR6 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/p7/8/8/8/P7/8/KR6 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_rook_seventh_rank_bonus() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/1R6/8/8/8/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/1R6/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_rook_queen_battery() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/8/KQ5R w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/Q7/K1R5 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_outpost_knight_bonus() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/4N3/3P4/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/3P4/4N3/8/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_outpost_requires_pawn_defense() {
        let s_undefended = Eval::default().evaluate(&Board::from_fen("k7/8/8/4N3/2P5/8/8/K7 w - -").unwrap());
        let s_defended = Eval::default().evaluate(&Board::from_fen("k7/8/8/4N3/3P4/8/8/K7 w - -").unwrap());
        assert!(s_defended > s_undefended,
            "defended outpost ({s_defended}) should score higher than undefended ({s_undefended})");
    }

    #[test]
    fn test_knight_rim_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/N7/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/4N3/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_development_preferred_over_passive_pawn_push() {
        let fen = "2rqkb1r/4pp1p/pnp3p1/8/2PP4/1Q4NP/PP3PP1/R1B1K2R b KQk - 2 14";
        let board = Board::from_fen(fen).unwrap();
        let ev = Eval::default();

        let mut b1 = board.clone();
        b1.make_move(Move::new(
            Square::from_file_rank(5, 6).unwrap(),
            Square::from_file_rank(5, 5).unwrap(),
        ));
        let s_f7f6 = ev.evaluate(&b1);

        let mut b2 = board.clone();
        b2.make_move(Move::new(
            Square::from_file_rank(5, 7).unwrap(),
            Square::from_file_rank(6, 6).unwrap(),
        ));
        let s_bg7 = ev.evaluate(&b2);

        // Score is from White perspective. Bg7 is better for Black → lower White score.
        assert!(s_bg7 < s_f7f6,
            "Bg7 should be better for Black (lower White score): f7f6={}, Bg7={}",
            s_f7f6, s_bg7);
    }

    #[test]
    fn test_trapped_knight_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/8/N1K5 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/8/2N1K3 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_connected_passers_bonus() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/PP6/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/P2P4/8/8/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_rook_behind_passer_bonus() {
        let mut ev = Eval::default();
        ev.piece.rook_open_file_bonus = (0, 0);
        ev.piece.rook_semi_open_file_bonus = (0, 0);
        ev.piece.rook_closed_file_penalty = (0, 0);
        let s1 = ev.evaluate(&Board::from_fen("k7/8/8/8/P7/8/R7/K7 w - -").unwrap());
        let s2 = ev.evaluate(&Board::from_fen("k7/8/8/8/P7/8/8/K1R5 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_passer_blocker_bonus() {
        // s1: knight on b1 blocks the b2 passer (White gets blocker bonus)
        // s2: same knight on b1, pawn on c2 (different file, no blocking)
        // Blocker helps White → Black-relative score goes DOWN → s1 < s2
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/1p6/KN6 b - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/2p5/KN6 b - -").unwrap());
        assert!(s1 < s2, "blocking passer should worsen Black's score (s1={s1} < s2={s2})");
    }

    #[test]
    fn test_king_passer_proximity() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/P7/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("7k/8/8/8/P7/8/8/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_king_opposition_bonus() {
        let s1 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k4/8/3KN3 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k4/8/4KN2 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_king_opposition_tempo_blocked() {
        let s1 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k4/8/3KN3 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k1p2/8/3KN3 w - -").unwrap());
        assert!(s2 < s1 + 200);
    }

    #[test]
    fn test_space_bonus() {
        let mut ev = Eval::default();
        ev.pawn.space_bonus = (300, 300);
        let s1 = ev.evaluate(&Board::from_fen("k7/8/8/8/2P1P3/8/8/K7 w - -").unwrap());
        let s2 = ev.evaluate(&Board::from_fen("k7/8/8/8/8/8/2P1P3/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_pawn_majority_bonus() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/PPP5/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/P1P1P3/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_bad_bishop_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/2P5/1P6/K1B5 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/2P5/1P6/K5B1 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_queen_attack_count_bonus() {
        let fen = "k7/3b4/8/8/8/8/3r4/K2Q4 w - -";
        let board = Board::from_fen(fen).unwrap();
        let mut ev = Eval::default();
        ev.piece.queen_attack_count_bonus = [0, 500, 1000, 1500, 2000, 2500, 3000, 3500];
        let s_high = ev.evaluate(&board);
        ev.piece.queen_attack_count_bonus = [0; 8];
        let s_low = ev.evaluate(&board);
        assert!(s_high > s_low);
    }

    #[test]
    fn test_queen_fork_bonus() {
        let mut ev = Eval::default();
        ev.piece.queen_fork_bonus = (300, 300);
        ev.piece.queen_attack_count_bonus = [0; 8];
        let s1 = ev.evaluate(&Board::from_fen("k7/8/5b2/8/3Q4/8/8/r6K w - -").unwrap());
        let s2 = ev.evaluate(&Board::from_fen("k7/8/6b1/8/3Q4/8/8/r6K w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_knight_mobility() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/3N4/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/8/K1N5 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_bishop_mobility() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/P7/K2B4 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/2P5/K2B4 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_rook_mobility() {
        let mut ev = Eval::default();
        ev.pst.mg_pawn_table = [0; 64]; ev.pst.eg_pawn_table = [0; 64];
        ev.pst.mg_rook_table = [0; 64]; ev.pst.eg_rook_table = [0; 64];
        ev.piece.rook_open_file_bonus = (0, 0);
        ev.piece.rook_semi_open_file_bonus = (0, 0);
        ev.piece.rook_closed_file_penalty = (0, 0);
        ev.king.rook_behind_passer_bonus = (0, 0);
        ev.pawn.passed_pawn_bonus = [0; 8];
        ev.mobility.rook_mobility = [-100, -80, -40, 0, 30, 50, 60, 65, 68, 70, 71, 71, 72, 72, 72];
        ev.mobility.rook_mobility_eg = [-100, -80, -40, 0, 30, 50, 60, 65, 68, 70, 71, 71, 72, 72, 72];
        let s1 = ev.evaluate(&Board::from_fen("k7/8/8/8/8/P7/8/KR6 w - -").unwrap());
        let s2 = ev.evaluate(&Board::from_fen("k7/8/8/8/8/1P6/8/KR6 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_queen_mobility() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/3Q4/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/8/K6Q w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_exchange_open_files() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/8/K1R1n3 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/2p2p2/8/K1R1n3 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_exchange_bishop_pair_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("1k6/8/8/8/8/8/8/K1R2b1b w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("1k6/8/8/8/8/8/8/K1R2n1n w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_exchange_minor_activity() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/1n6/K1R5 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/3n4/8/8/K1R5 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_king_safety_open_file() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k6r/8/8/8/8/8/8/4K3 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k6r/8/8/8/8/8/4P3/4K3 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_king_safety_zone_attackers() {
        let mut ev = Eval::default();
        ev.king.king_shield_missing_penalty = 0;
        ev.king.king_open_file_penalty = 0;
        ev.pst.mg_queen_table = [0; 64];
        ev.pst.eg_queen_table = [0; 64];
        let s1 = ev.evaluate(&Board::from_fen("k7/8/8/8/8/8/4q3/4K3 w - -").unwrap());
        let s2 = ev.evaluate(&Board::from_fen("k7/8/5q2/8/8/8/8/4K3 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_castling_preferred_over_king_move() {
        // O-O should evaluate better than Kd2 in a position where castling is available
        let fen = "rn1q1rk1/1b2bppp/p3pn2/1p1pN3/2pP1B2/P1N1P3/1PP1BPPP/R1Q1K2R w KQ - 5 11";
        let board = Board::from_fen(fen).unwrap();
        let ev = Eval::default();

        // O-O: e1g1
        let oo = Move::castle(Square::E1, Square::G1);
        let mut board_oo = board.clone();
        board_oo.make_move(oo);
        let eval_oo = ev.evaluate(&board_oo);

        // Kd2: e1d2
        let kd2 = Move::new(
            Square::from_file_rank(4, 0).unwrap(),
            Square::from_file_rank(3, 1).unwrap(),
        );
        let mut board_kd2 = board.clone();
        board_kd2.make_move(kd2);
        let eval_kd2 = ev.evaluate(&board_kd2);

        assert!(
            eval_oo > eval_kd2,
            "O-O eval ({eval_oo}) should be > Kd2 eval ({eval_kd2})"
        );
    }
}
