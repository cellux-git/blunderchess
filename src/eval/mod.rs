mod params;
mod pawns;
use pawns::passed_pawns;
mod kings;
mod mobility;
mod pieces;
mod see;

pub(crate) use params::Eval;

use crate::board::Board;
use crate::types::{Color, Piece, Square};

impl Eval {
    fn material_value(&self, piece: Piece) -> i32 {
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

    fn game_phase(&self, board: &Board) -> i32 {
        let mut phase = 0i32;
        for &(_, piece, _) in board.piece_list() {
            let weight = match piece {
                Piece::Knight => 1,
                Piece::Bishop => 1,
                Piece::Rook => 2,
                Piece::Queen => 4,
                _ => 0,
            };
            phase += weight;
        }
        phase.min(24)
    }

    pub fn evaluate(&self, board: &Board) -> i32 {
        let phase = self.game_phase(board);
        let max_phase = 24;

        let (w_mg, w_eg) = self.evaluate_side(board, Color::White);
        let (b_mg, b_eg) = self.evaluate_side(board, Color::Black);

        let mg = w_mg - b_mg;
        let eg = w_eg - b_eg;

        let score = (mg * phase + eg * (max_phase - phase)) / max_phase;

        if board.side_to_move() == Color::White { score } else { -score }
    }

    fn evaluate_side(&self, board: &Board, color: Color) -> (i32, i32) {
        let mut mg_score = 0i32;
        let mut eg_score = 0i32;

        let enemy = color.flip();
        let us_bb = board.colors_bb(color);
        let enemy_bb = board.colors_bb(enemy);
        let pawns_bb = board.pieces_bb(Piece::Pawn) & us_bb;
        let enemy_pawns_bb = board.pieces_bb(Piece::Pawn) & enemy_bb;
        let occ = board.occupancy();
        let king_sq = board.king_square(color);

        // material + PST
        for &(sq, piece, pc) in board.piece_list() {
            if pc != color { continue; }
            let (mg_pst, eg_pst) = match piece {
                Piece::Pawn => self.pst_value(&self.pst.mg_pawn_table, &self.pst.eg_pawn_table, sq, color),
                Piece::Knight => self.pst_value(&self.pst.mg_knight_table, &self.pst.eg_knight_table, sq, color),
                Piece::Bishop => self.pst_value(&self.pst.mg_bishop_table, &self.pst.eg_bishop_table, sq, color),
                Piece::Rook => self.pst_value(&self.pst.mg_rook_table, &self.pst.eg_rook_table, sq, color),
                Piece::Queen => self.pst_value(&self.pst.mg_queen_table, &self.pst.eg_queen_table, sq, color),
                Piece::King => self.pst_value(&self.pst.mg_king_table, &self.pst.eg_king_table, sq, color),
            };
            mg_score += self.material_value(piece) + mg_pst;
            eg_score += self.material_value(piece) + eg_pst;
        }

        // pawn structure
        self.eval_pawns(board, pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // pawn chain / phalanx
        self.eval_pawn_chain(pawns_bb, color, &mut mg_score, &mut eg_score);

        // bishop pair
        let bishops_bb = board.pieces_bb(Piece::Bishop) & us_bb;
        if bishops_bb.count_ones() >= 2 {
            mg_score += self.piece.bishop_pair_bonus.0;
            eg_score += self.piece.bishop_pair_bonus.1;
        }

        // bad bishops (generalized)
        self.eval_bad_bishops(board, color, pawns_bb, &mut mg_score, &mut eg_score);

        // rook open/semi-open files + closed file + 7th rank
        self.eval_rooks(board, pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // rook-queen battery
        self.eval_rook_queen_battery(board, color, &mut mg_score, &mut eg_score);

        // knights: outpost + rim + trapped
        self.eval_knights(board, color, enemy_pawns_bb, &mut mg_score, &mut eg_score);

        // queen multi-attack
        self.eval_queen_multiattack(board, color, enemy, &mut mg_score, &mut eg_score);

        // passed pawn bonuses
        let my_passers = passed_pawns(pawns_bb, enemy_pawns_bb, color);
        self.eval_connected_passers(my_passers, &mut mg_score, &mut eg_score);
        self.eval_rook_behind_passer(board, color, my_passers, &mut mg_score, &mut eg_score);
        self.eval_king_passer_proximity(board, color, my_passers, &mut mg_score, &mut eg_score);

        // candidate passer
        self.eval_candidate_passers(board, pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // passer blocker
        self.eval_passer_blocker(board, pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // mobility
        let (mobility_mg, mobility_eg) = self.eval_mobility(board, color, enemy, occ);
        mg_score += mobility_mg;
        eg_score += mobility_eg;

        // king safety (MG only; outer phase blend handles taper)
        let king_safety_mg = self.eval_king_safety(board, color, king_sq, pawns_bb, enemy_bb);
        mg_score += king_safety_mg;

        // king opposition
        self.eval_king_opposition(board, color, &mut mg_score, &mut eg_score);

        // space control
        self.eval_space(board, color, pawns_bb, enemy_pawns_bb, &mut mg_score, &mut eg_score);

        // pawn majority
        self.eval_pawn_majority(pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // exchange evaluation
        self.eval_exchange(board, color, pawns_bb, &mut mg_score, &mut eg_score);

        (mg_score, eg_score)
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

    // ── 26 new positional term tests ──

    #[test]
    fn test_isolated_pawn_penalty() {
        // two isolated pawns (a3,c3) vs connected phalanx (a3,b3), same material
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/P1P5/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/PP6/8/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_backward_pawn_penalty() {
        // e4 blocked by e5, backward without support vs supported by d3 behind on adjacent
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/4p3/4P3/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/4p3/4P3/3P4/8/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_pawn_phalanx_bonus() {
        // a2+b2 phalanx vs a2+c2 spread, same count
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/PP6/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/P1P5/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_pawn_chain_bonus() {
        // d3 defended by e2 (chain) vs d3+g2 (no chain)
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/3P4/4P3/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/3P4/6P1/K7 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_rook_semi_open_file() {
        // rook on semi-open b-file (only black pawn) vs open b-file, same material
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/1p6/8/8/8/P7/8/KR6 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/p7/8/8/8/P7/8/KR6 w - -").unwrap());
        assert!(s2 > s1, "open file should score higher than semi-open");
    }

    #[test]
    fn test_rook_closed_file_penalty() {
        // rook on closed b-file (own pawn) vs open, same material
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
        // Q+R aligned on rank 1 empty between vs not aligned, same material
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
    fn test_knight_rim_penalty() {
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/N7/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/4N3/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_trapped_knight_penalty() {
        // knight on a1 (2 safe squares) vs knight on c1 (4 safe squares), same material
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
        // zero out rook-file bonuses so only behind-passer matters
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
        // knight on b1 blocks black b-pawn vs same knight not blocking
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/1p6/KN6 b - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/1p6/K1N5 b - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_king_passer_proximity() {
        // bigger distance advantage = bigger bonus
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/P7/8/8/K7 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("7k/8/8/8/P7/8/8/K7 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_king_opposition_bonus() {
        // kings in opposition, one side has knight (asymmetry so only one side gets bonus)
        let s1 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k4/8/3KN3 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k4/8/4KN2 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_king_opposition_tempo_blocked() {
        // opposition blocked when opponent has a pawn that can move
        let s1 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k4/8/3KN3 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("8/8/8/8/8/3k1p2/8/3KN3 w - -").unwrap());
        // s2 has extra pawn (+100 material) but opposition bonus blocked (~-50)
        assert!(s2 < s1 + 200);
    }

    #[test]
    fn test_space_bonus() {
        // exaggerated space bonus to overcome PST differences between ranks
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
        // light-sq bishop behind light-sq pawns vs dark-sq bishop, same material
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/2P5/1P6/K1B5 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/2P5/1P6/K5B1 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_queen_attack_count_bonus() {
        // same position, different count bonus → higher bonus = higher score
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
        // custom: big fork bonus, zero count. Fork: 2 undefended pieces attacked by Q.
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
        // bishop open diagonal vs bishop blocked by own pawn, same material
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/P7/K2B4 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/2P5/K2B4 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_rook_mobility() {
        // same material (K+R+1P), rook on b1: open file vs blocked by own pawn
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
        // s1: R b1, P a3 → b-file open, mobility ~13
        // s2: R b1, P b3 → b-file blocked, mobility ~7
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
        // RvsN, same material, open files vs closed
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
        // RvsN, inactive enemy minor vs active, same material
        let s1 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/8/8/1n6/K1R5 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k7/8/8/8/3n4/8/8/K1R5 w - -").unwrap());
        assert!(s1 > s2);
    }

    #[test]
    fn test_king_safety_open_file() {
        // open file near king with enemy rook vs shielded, same material
        let s1 = Eval::default().evaluate(&Board::from_fen("k6r/8/8/8/8/8/8/4K3 w - -").unwrap());
        let s2 = Eval::default().evaluate(&Board::from_fen("k6r/8/8/8/8/8/4P3/4K3 w - -").unwrap());
        assert!(s2 > s1);
    }

    #[test]
    fn test_king_safety_zone_attackers() {
        // zero out PST to isolate zone-attacker penalty
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
