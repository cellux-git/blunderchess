mod params;
mod pawns;
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
            Piece::Pawn => self.pawn_value,
            Piece::Knight => self.knight_value,
            Piece::Bishop => self.bishop_value,
            Piece::Rook => self.rook_value,
            Piece::Queen => self.queen_value,
            Piece::King => self.king_value,
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
                Piece::Pawn => self.pst_value(&self.mg_pawn_table, &self.eg_pawn_table, sq, color),
                Piece::Knight => self.pst_value(&self.mg_knight_table, &self.eg_knight_table, sq, color),
                Piece::Bishop => self.pst_value(&self.mg_bishop_table, &self.eg_bishop_table, sq, color),
                Piece::Rook => self.pst_value(&self.mg_rook_table, &self.eg_rook_table, sq, color),
                Piece::Queen => self.pst_value(&self.mg_queen_table, &self.eg_queen_table, sq, color),
                Piece::King => self.pst_value(&self.mg_king_table, &self.eg_king_table, sq, color),
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
            mg_score += self.bishop_pair_bonus.0;
            eg_score += self.bishop_pair_bonus.1;
        }

        // bad bishops (generalized)
        self.eval_bad_bishops(board, color, pawns_bb, &mut mg_score, &mut eg_score);

        // rook open/semi-open files + closed file + 7th rank
        self.eval_rooks(board, pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // rook-queen battery
        self.eval_rook_queen_battery(board, color, &mut mg_score, &mut eg_score);

        // knights: outpost + rim + trapped + redundancy
        self.eval_knights(board, color, enemy_pawns_bb, &mut mg_score, &mut eg_score);

        // queen multi-attack
        self.eval_queen_multiattack(board, color, enemy, &mut mg_score, &mut eg_score);

        // passed pawn bonuses
        let my_passers = self.passed_pawns(pawns_bb, enemy_pawns_bb, color);
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
        assert!(score_black.abs() <= 50);
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
        assert!(score <= 0);
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
        ev.knight_value = 200;
        let low = ev.evaluate(&board);
        ev.knight_value = 500;
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
}
