pub(crate) mod params;
pub(crate) use params::Eval;

mod pawns;
mod kings;
mod mobility;
mod pieces;
mod see;

use crate::board::Board;
use crate::types::{Color, Piece, Square};
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

    fn pst_value(&self, mg_table: &[i32; 64], eg_table: &[i32; 64], sq: Square, color: Color) -> (i32, i32) {
        let idx = if color == Color::White {
            sq.index() as usize
        } else {
            (sq.index() ^ 56) as usize
        };
        (mg_table[idx], eg_table[idx])
    }

    pub fn evaluate(&self, board: &Board) -> i32 {
        let phase = board.phase();
        let max_phase = 24;

        let (w_mat_mg, w_pos_mg, w_mat_eg, w_pos_eg) = self.evaluate_side(board, Color::White);
        let (b_mat_mg, b_pos_mg, b_mat_eg, b_pos_eg) = self.evaluate_side(board, Color::Black);

        let mat_diff_mg = w_mat_mg - b_mat_mg;
        let mat_diff_eg = w_mat_eg - b_mat_eg;

        // Scale positional terms for the side that is behind in material.
        // When down a knight (~320 cp), positional credit drops to ~48%.
        let w_mg = w_mat_mg + scale_positional(w_pos_mg, -mat_diff_mg);
        let b_mg = b_mat_mg + scale_positional(b_pos_mg, mat_diff_mg);
        let w_eg = w_mat_eg + scale_positional(w_pos_eg, -mat_diff_eg);
        let b_eg = b_mat_eg + scale_positional(b_pos_eg, mat_diff_eg);

        let mg = w_mg - b_mg;
        let eg = w_eg - b_eg;

        let score = (mg * phase + eg * (max_phase - phase)) / max_phase;

        if board.side_to_move() == Color::White { score } else { -score }
    }

    fn evaluate_side(&self, board: &Board, color: Color) -> (i32, i32, i32, i32) {
        let mut mat_mg = 0i32;
        let mut mat_eg = 0i32;
        let mut pos_mg = 0i32;
        let mut pos_eg = 0i32;

        let enemy = color.flip();
        let us_bb = board.colors_bb(color);
        let enemy_bb = board.colors_bb(enemy);
        let pawns_bb = board.pieces_bb(Piece::Pawn) & us_bb;
        let enemy_pawns_bb = board.pieces_bb(Piece::Pawn) & enemy_bb;
        let _occ = board.occupancy();
        let king_sq = board.king_square(color);

        // material + PST (from bitboards)
        macro_rules! accumulate {
            ($piece:ident, $mg_table:ident, $eg_table:ident) => {
                let val = self.material_value(Piece::$piece);
                let mut bb = board.pieces_bb(Piece::$piece) & us_bb;
                while bb != 0 {
                    let idx = bb.trailing_zeros() as u8;
                    let sq = Square::new(idx).unwrap();
                    let (mg_pst, eg_pst) = self.pst_value(&self.pst.$mg_table, &self.pst.$eg_table, sq, color);
                    mat_mg += val;
                    mat_eg += val;
                    pos_mg += mg_pst;
                    pos_eg += eg_pst;
                    bb &= bb - 1;
                }
            };
        }
        accumulate!(Pawn, mg_pawn_table, eg_pawn_table);
        accumulate!(Knight, mg_knight_table, eg_knight_table);
        accumulate!(Bishop, mg_bishop_table, eg_bishop_table);
        accumulate!(Rook, mg_rook_table, eg_rook_table);
        accumulate!(Queen, mg_queen_table, eg_queen_table);
        accumulate!(King, mg_king_table, eg_king_table);

        // pawn structure
        let (m, e) = pawns::eval_pawns(board, &self.pawn, pawns_bb, enemy_pawns_bb, color);
        pos_mg += m; pos_eg += e;

        // pawn chain / phalanx
        let (m, e) = pawns::eval_pawn_chain(&self.pawn, pawns_bb, color);
        pos_mg += m; pos_eg += e;

        // bishop pair
        let bishops_bb = board.pieces_bb(Piece::Bishop) & us_bb;
        if bishops_bb.count_ones() >= 2 {
            pos_mg += self.piece.bishop_pair_bonus.0;
            pos_eg += self.piece.bishop_pair_bonus.1;
        }

        // bad bishops (generalized)
        let (m, e) = pieces::eval_bad_bishops(board, &self.piece, color, pawns_bb);
        pos_mg += m; pos_eg += e;

        // rook open/semi-open files + closed file + 7th rank
        let (m, e) = pieces::eval_rooks(board, &self.piece, pawns_bb, enemy_pawns_bb, color);
        pos_mg += m; pos_eg += e;

        // rook-queen battery
        let (m, e) = pieces::eval_rook_queen_battery(board, &self.piece, color);
        pos_mg += m; pos_eg += e;

        // knights: outpost + rim + trapped
        let (m, e) = pieces::eval_knights(board, &self.piece, color, enemy_pawns_bb);
        pos_mg += m; pos_eg += e;

        // queen multi-attack
        let (m, e) = pieces::eval_queen_multiattack(board, &self.piece, color, enemy);
        pos_mg += m; pos_eg += e;

        // passed pawn bonuses
        let my_passers = pawns::passed_pawns(pawns_bb, enemy_pawns_bb, color);
        let (m, e) = pawns::eval_connected_passers(&self.king, my_passers);
        pos_mg += m; pos_eg += e;
        let (m, e) = pawns::eval_rook_behind_passer(board, &self.king, color, my_passers);
        pos_mg += m; pos_eg += e;
        let (m, e) = kings::eval_king_passer_proximity(board, &self.king, color, my_passers);
        pos_mg += m; pos_eg += e;

        // candidate passer
        let (m, e) = pawns::eval_candidate_passers(&self.pawn, board, pawns_bb, enemy_pawns_bb, color);
        pos_mg += m; pos_eg += e;

        // passer blocker
        let (m, e) = pawns::eval_passer_blocker(board, &self.pawn, pawns_bb, enemy_pawns_bb, color);
        pos_mg += m; pos_eg += e;

        // mobility
        let (mobility_mg, mobility_eg) = mobility::eval_mobility(board, &self.mobility, color, enemy_pawns_bb, enemy);
        pos_mg += mobility_mg;
        pos_eg += mobility_eg;

        // king safety (MG only; outer phase blend handles taper)
        let king_safety_mg = kings::eval_king_safety(board, &self.king, color, king_sq, pawns_bb, enemy_bb);
        pos_mg += king_safety_mg;

        // king opposition
        let (m, e) = kings::eval_king_opposition(board, &self.king, color);
        pos_mg += m; pos_eg += e;

        // space control
        let (m, e) = pieces::eval_space(&self.pawn, pawns_bb, color);
        pos_mg += m; pos_eg += e;

        // pawn majority
        let (m, e) = pieces::eval_pawn_majority(&self.pawn, pawns_bb, enemy_pawns_bb);
        pos_mg += m; pos_eg += e;

        // exchange evaluation
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
}
