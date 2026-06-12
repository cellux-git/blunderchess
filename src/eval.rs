use crate::board::{Board};
use crate::types::{Bitboard, Color, Move, Piece, Square};

// ---- Eval struct: all tunable parameters ----

#[derive(Debug, Clone)]
pub struct Eval {
    pub pawn_value: i32,
    pub knight_value: i32,
    pub bishop_value: i32,
    pub rook_value: i32,
    pub queen_value: i32,
    pub king_value: i32,

    pub mg_pawn_table: [i32; 64],
    pub eg_pawn_table: [i32; 64],
    pub mg_knight_table: [i32; 64],
    pub eg_knight_table: [i32; 64],
    pub mg_bishop_table: [i32; 64],
    pub eg_bishop_table: [i32; 64],
    pub mg_rook_table: [i32; 64],
    pub eg_rook_table: [i32; 64],
    pub mg_queen_table: [i32; 64],
    pub eg_queen_table: [i32; 64],
    pub mg_king_table: [i32; 64],
    pub eg_king_table: [i32; 64],

    pub doubled_pawn_penalty: (i32, i32),
    pub isolated_pawn_penalty: (i32, i32),
    pub passed_pawn_bonus: [i32; 8],
    pub backward_pawn_penalty: (i32, i32),
    pub bishop_pair_bonus: (i32, i32),
    pub rook_open_file_bonus: (i32, i32),
    pub rook_semi_open_file_bonus: (i32, i32),
    pub knight_mobility: [i32; 9],
    pub bishop_mobility: [i32; 14],
    pub rook_mobility: [i32; 15],
    pub queen_mobility: [i32; 28],
    pub king_shield_missing_penalty: i32,
    pub king_open_file_penalty: i32,
    pub outpost_knight_bonus: (i32, i32),
    pub connected_passer_bonus: i32,
    pub rook_behind_passer_bonus: (i32, i32),
    pub king_passer_proximity_bonus: i32,
    pub trapped_bishop_penalty: (i32, i32),
}

impl Default for Eval {
    fn default() -> Self {
        Self {
            pawn_value: 100,
            knight_value: 320,
            bishop_value: 330,
            rook_value: 500,
            queen_value: 900,
            king_value: 20000,

            mg_pawn_table: [
                0,   0,   0,   0,   0,   0,  0,   0,
               98, 134,  61,  95,  68, 126, 34, -11,
               -6,   7,  26,  31,  65,  56, 25, -20,
              -14,  13,   6,  21,  23,  12, 17, -23,
              -27,  -2,  -5,  12,  17,   6, 10, -25,
              -26,  -4,  -4, -10,   3,   3, 33, -12,
              -35,  -1, -20, -23, -15,  24, 38, -22,
                0,   0,   0,   0,   0,   0,  0,   0,
            ],
            eg_pawn_table: [
                0,   0,   0,   0,   0,   0,  0,   0,
              178, 173, 158, 134, 147, 132, 165, 187,
               94, 100,  85,  67,  56,  53,  82,  84,
               32,  24,  13,   5,  -2,   4,  17,  17,
               13,   9,  -3,  -7,  -7,  -8,   3,  -1,
                4,   7,  -6,   1,   0,  -5,  -1,  -8,
               13,   8,   8,  10,  13,   0,   2,  -7,
                0,   0,   0,   0,   0,   0,  0,   0,
            ],
            mg_knight_table: [
                -167, -89, -34, -49,  61, -97, -15, -107,
                 -73, -41,  72,  36,  23,  62,   7,  -17,
                 -47,  60,  37,  65,  84, 129,  73,   44,
                  -9,  17,  19,  53,  37,  69,  18,   22,
                 -13,   4,  16,  13,  28,  19,  21,   -8,
                 -23,  -9,  12,  10,  19,  17,  25,  -16,
                 -29, -53, -12,  -3,  -1,  18, -14,  -19,
                -105, -21, -58, -33, -17, -28, -19,  -23,
            ],
            eg_knight_table: [
                -58, -38, -13, -28, -31, -27, -63, -99,
                -25,  -8, -25,  -2,  -9, -25, -24, -52,
                -24, -20,  10,   9,  -1,  -9, -19, -41,
                -17,   3,  22,  22,  22,  11,   8, -18,
                -18,  -6,  16,  25,  16,  17,   4, -18,
                -23,  -3,  -1,  15,  10,  -3, -20, -22,
                -42, -20, -10,  -5,  -2, -20, -23, -44,
                -29, -51, -23, -15, -22, -18, -50, -64,
            ],
            mg_bishop_table: [
                -29,   4, -82, -37, -25, -42,   7,  -8,
                -26,  16, -18, -13,  30,  59,  18, -47,
                -16,  37,  43,  40,  35,  50,  37,  -2,
                 -4,   5,  19,  50,  37,  37,   7,  -2,
                 -6,  13,  13,  26,  34,  12,  10,   4,
                  0,  15,  15,  15,  14,  27,  18,  10,
                  4,  15,  16,   0,   7,  21,  33,   1,
                -33,  -3, -14, -21, -13, -12, -39, -21,
            ],
            eg_bishop_table: [
                -14, -21, -11,  -8, -7,  -9, -17, -24,
                 -8,  -4,   7, -12, -3, -13,  -4, -14,
                  2,  -8,   0,  -1, -2,   6,   0,   4,
                 -3,   9,  12,   9, 14,  10,   3,   2,
                 -6,   3,  13,  19,  7,  10,  -3,  -9,
                -12,  -3,   8,  10, 13,   3,  -7, -15,
                -14, -18,  -7,  -1,  4,  -9, -15, -27,
                -23,  -9, -23,  -5, -9, -16,  -5, -17,
            ],
            mg_rook_table: [
                 32,  42,  32,  51, 63,  9,  31,  43,
                 27,  32,  58,  62, 80, 67,  26,  44,
                 -5,  19,  26,  36, 17, 45,  61,  16,
                -24, -11,   7,  26, 24, 35,  -8, -20,
                -36, -26, -12,  -1,  9, -7,   6, -23,
                -45, -25, -16, -17,  3,  0,  -5, -33,
                -44, -16, -20,  -9, -1, 11,  -6, -71,
                -19, -13,   1,  17, 16,  7, -37, -26,
            ],
            eg_rook_table: [
                13, 10, 18, 15, 12,  12,   8,   5,
                11, 13, 13, 11, -3,   3,   8,   3,
                 7,  7,  7,  5, 16,   8,   6,  -4,
                -2,  4,  3,  2,  6,   3,  -1,   0,
                -3,  5,  2,  3,  4,   1,   5,  -2,
                -5, -1, -2,  0,  1,   1,   3,  -5,
                -9, -7, -2,  1, -4,   2,  -2,  -9,
                -9, -1, -4,  2,  4,  -8,  -1,  -2,
            ],
            mg_queen_table: [
                -28,   0,  29,  12,  59,  44,  43,  45,
                -24, -39,  -5,   1, -16,  57,  28,  54,
                -13, -17,   7,   8,  29,  56,  47,  57,
                -27, -27, -16, -16,  -1,  17,  -2,   1,
                 -9, -26,  -9, -10,  -2,  -4,   3,  -3,
                -14,   2, -11,  -2,  -5,   2,  14,   5,
                -35,  -8,  11,   2,   8,  15,  -3,   1,
                 -1, -18,  -9,  10, -15, -25, -31, -50,
            ],
            eg_queen_table: [
                 -9,  22,  22,  27,  27,  19,  10,  20,
                -17,  20,  32,  41,  58,  25,  30,   0,
                -20,   6,   9,  49,  47,  35,  19,   9,
                  3,  22,  24,  45,  57,  40,  57,  36,
                -18,  28,  19,  47,  31,  34,  39,  23,
                -16, -27,  15,   6,   9,  17,  10,   5,
                -22, -23, -30, -16, -16, -23, -36, -32,
                -33, -28, -22, -43,  -5, -32, -20, -41,
            ],
            mg_king_table: [
                -65,  23,  16, -15, -56, -34,   2,  13,
                 29,  -1, -20,  -7,  -8,  -4, -38, -29,
                 -9,  24,   2, -16, -20,   6,  22, -22,
                -17, -20, -12, -27, -30, -25, -14, -36,
                -49,  -1, -27, -39, -46, -44, -33, -51,
                -14, -14, -22, -46, -44, -30, -15, -27,
                  1,   7,  -8, -64, -43, -16,   9,   8,
                -15,  36,  12, -54,   8, -28,  24,  14,
            ],
            eg_king_table: [
                -74, -35, -18, -18, -11,  15,   4, -17,
                -12,  17,  14,  17,  17,  38,  23,  11,
                 10,  17,  23,  15,  20,  45,  44,  13,
                 -8,  22,  24,  27,  26,  33,  26,   3,
                -18,  -4,  21,  24,  27,  23,   9, -11,
                -19,  -3,  11,  21,  23,  16,   7,  -9,
                -27, -11,   4,  13,  14,   4,  -5, -17,
                -53, -34, -21, -11, -28, -14, -24, -43,
            ],

            doubled_pawn_penalty: (-12, -24),
            isolated_pawn_penalty: (-10, -20),
            passed_pawn_bonus: [0, 5, 10, 20, 40, 70, 100, 0],
            backward_pawn_penalty: (-8, -16),
            bishop_pair_bonus: (25, 45),
            rook_open_file_bonus: (25, 10),
            rook_semi_open_file_bonus: (12, 5),
            knight_mobility: [-20, -8, 0, 4, 8, 12, 15, 18, 20],
            bishop_mobility: [-20, -8, 0, 4, 8, 12, 15, 18, 20, 22, 24, 26, 27, 28],
            rook_mobility: [-20, -8, 0, 4, 8, 12, 15, 18, 20, 22, 24, 26, 28, 29, 30],
            queen_mobility: [
                -20, -8, 0, 4, 8, 12, 15, 18, 20, 22, 24, 26, 28, 29,
                30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43,
            ],
            king_shield_missing_penalty: -12,
            king_open_file_penalty: -20,
            outpost_knight_bonus: (18, 8),
            connected_passer_bonus: 20,
            rook_behind_passer_bonus: (20, 30),
            king_passer_proximity_bonus: 10,
            trapped_bishop_penalty: (-40, -50),
        }
    }
}

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
        for &(_, piece, _) in &board.piece_list {
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

        if board.side_to_move == Color::White { score } else { -score }
    }

    fn evaluate_side(&self, board: &Board, color: Color) -> (i32, i32) {
        let mut mg_score = 0i32;
        let mut eg_score = 0i32;

        let enemy = color.flip();
        let us_bb = board.colors_bb[color.index()];
        let enemy_bb = board.colors_bb[enemy.index()];
        let pawns_bb = board.pieces_bb[Piece::Pawn as usize] & us_bb;
        let enemy_pawns_bb = board.pieces_bb[Piece::Pawn as usize] & enemy_bb;
        let occ = board.occupancy;
        let king_sq = board.king_square[color.index()];

        // material + PST
        for &(sq, piece, pc) in &board.piece_list {
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
        self.eval_pawns(pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // bishop pair
        let bishops_bb = board.pieces_bb[Piece::Bishop as usize] & us_bb;
        if bishops_bb.count_ones() >= 2 {
            mg_score += self.bishop_pair_bonus.0;
            eg_score += self.bishop_pair_bonus.1;
        }

        // trapped bishops
        self.eval_trapped_bishops(board, color, &mut mg_score, &mut eg_score);

        // rook open/semi-open files
        self.eval_rooks(board, pawns_bb, enemy_pawns_bb, color, &mut mg_score, &mut eg_score);

        // outpost knights
        self.eval_outpost_knights(board, color, enemy_pawns_bb, &mut mg_score, &mut eg_score);

        // passed pawn bonuses
        let my_passers = self.passed_pawns(pawns_bb, enemy_pawns_bb, color);
        self.eval_connected_passers(my_passers, &mut mg_score, &mut eg_score);
        self.eval_rook_behind_passer(board, color, my_passers, &mut mg_score, &mut eg_score);
        self.eval_king_passer_proximity(board, color, my_passers, &mut mg_score, &mut eg_score);

        // mobility (applied to mg only; outer phase blend in evaluate()
        // handles mg→eg transition)
        let mobility_mg = self.eval_mobility(board, color, enemy, occ);
        mg_score += mobility_mg;

        // king safety (MG only; outer phase blend handles taper)
        let king_safety_mg = self.eval_king_safety(board, color, king_sq, pawns_bb, enemy_bb);
        mg_score += king_safety_mg;

        (mg_score, eg_score)
    }

    fn eval_pawns(&self, pawns_bb: u64, enemy_pawns_bb: u64, color: Color, mg: &mut i32, eg: &mut i32) {
        let white_rank = |r: u8| if color == Color::White { r } else { 7 - r };
        let mut pawns = pawns_bb;
        while pawns != 0 {
            let sq_idx = pawns.trailing_zeros() as u8;
            let sq = Square::new(sq_idx).unwrap();
            let file = sq.file();
            let rank = sq.rank();
            let fwd_rank = white_rank(rank);

            let ahead_on_file = file_mask(file) & rank_mask_forward(sq, color) & pawns_bb;
            if ahead_on_file != 0 {
                *mg += self.doubled_pawn_penalty.0;
                *eg += self.doubled_pawn_penalty.1;
            }

            if adjacent_files_mask(file) & pawns_bb == (1u64 << sq_idx) & adjacent_files_mask(file) {
                *mg += self.isolated_pawn_penalty.0;
                *eg += self.isolated_pawn_penalty.1;
            }

            let ahead = rank_mask_forward(sq, color);
            if ahead & enemy_pawns_bb & adjacent_files_mask(file) == 0 {
                *mg += self.passed_pawn_bonus[fwd_rank as usize];
                *eg += self.passed_pawn_bonus[fwd_rank as usize] * 2;
            }

            if fwd_rank > 0 && fwd_rank < 6 {
                if (adjacent_files_mask(file) & pawns_bb & rank_mask_forward(sq, color.flip())) == 0 {
                    if (adjacent_files_mask(file) & enemy_pawns_bb & rank_mask_forward(sq, color.flip())) != 0 {
                        *mg += self.backward_pawn_penalty.0;
                        *eg += self.backward_pawn_penalty.1;
                    }
                }
            }

            pawns &= pawns - 1;
        }
    }

    fn eval_rooks(&self, board: &Board, pawns_bb: u64, enemy_pawns_bb: u64, color: Color, mg: &mut i32, eg: &mut i32) {
        let mut rooks = board.pieces_bb[Piece::Rook as usize] & board.colors_bb[color.index()];
        while rooks != 0 {
            let file = (rooks.trailing_zeros() as u8) & 7;
            let fm = file_mask(file);
            let our_pawns = pawns_bb & fm;
            let enemy_pawns = enemy_pawns_bb & fm;
            if our_pawns == 0 {
                if enemy_pawns == 0 {
                    *mg += self.rook_open_file_bonus.0;
                    *eg += self.rook_open_file_bonus.1;
                } else {
                    *mg += self.rook_semi_open_file_bonus.0;
                    *eg += self.rook_semi_open_file_bonus.1;
                }
            }
            rooks &= rooks - 1;
        }
    }

    fn eval_trapped_bishops(&self, board: &Board, color: Color, mg: &mut i32, eg: &mut i32) {
        // Bishop on a1/a8/h1/h8 blocked by own pawn on b2/b7/g2/g7
        let my_bishops = board.pieces_bb[Piece::Bishop as usize] & board.colors_bb[color.index()];
        let my_pawns = board.pieces_bb[Piece::Pawn as usize] & board.colors_bb[color.index()];

        let traps: [(u8, u8); 2] = if color == Color::White {
            [(0, 9), (7, 14)]
        } else {
            [(56, 49), (63, 54)]
        };

        for &(b_sq, p_sq) in &traps {
            let b_bit = 1u64 << b_sq;
            let p_bit = 1u64 << p_sq;
            if (my_bishops & b_bit) != 0 && (my_pawns & p_bit) != 0 {
                *mg += self.trapped_bishop_penalty.0;
                *eg += self.trapped_bishop_penalty.1;
            }
        }
    }

    fn eval_outpost_knights(&self, board: &Board, color: Color, enemy_pawns_bb: u64, mg: &mut i32, eg: &mut i32) {
        let my_knights = board.pieces_bb[Piece::Knight as usize] & board.colors_bb[color.index()];
        let my_pawns = board.pieces_bb[Piece::Pawn as usize] & board.colors_bb[color.index()];
        let mut knights = my_knights;
        while knights != 0 {
            let sq_idx = knights.trailing_zeros() as u8;
            let sq = Square::new(sq_idx).unwrap();
            knights &= knights - 1;

            // Outpost: knight in opponent's half (rank 3+ for White, rank 0-4 for Black)
            let rank = sq.rank();
            let in_enemy_half = if color == Color::White { rank >= 4 } else { rank <= 3 };
            if !in_enemy_half { continue; }

            // Cannot be attacked by enemy pawns
            let enemy_attacks = crate::attack::pawn_attacks(sq, color);
            if enemy_attacks & enemy_pawns_bb != 0 { continue; }

            // Supported by a friendly pawn on an adjacent file?
            let file = sq.file();
            let adj = adjacent_files_mask(file) & !(1u64 << sq_idx);
            if adj & my_pawns != 0 {
                *mg += self.outpost_knight_bonus.0;
                *eg += self.outpost_knight_bonus.1;
            }
        }
    }

    fn passed_pawns(&self, pawns_bb: u64, enemy_pawns_bb: u64, color: Color) -> u64 {
        let mut passed = 0u64;
        let mut bb = pawns_bb;
        while bb != 0 {
            let sq_idx = bb.trailing_zeros() as u8;
            let sq = Square::new(sq_idx).unwrap();
            let ahead = rank_mask_forward(sq, color);
            let file = sq.file();
            if ahead & enemy_pawns_bb & adjacent_files_mask(file) == 0 {
                passed |= 1u64 << sq_idx;
            }
            bb &= bb - 1;
        }
        passed
    }

    fn eval_connected_passers(&self, passers: u64, mg: &mut i32, eg: &mut i32) {
        let mut bb = passers;
        while bb != 0 {
            let sq_idx = bb.trailing_zeros() as u8;
            bb &= bb - 1;
            let file = (sq_idx & 7) as u8;
            // Check if there's another passer on an adjacent file
            let adj = if file == 0 {
                file_mask(1)
            } else if file == 7 {
                file_mask(6)
            } else {
                file_mask(file - 1) | file_mask(file + 1)
            };
            if passers & adj != 0 {
                *mg += self.connected_passer_bonus;
                *eg += self.connected_passer_bonus * 2;
            }
        }
    }

    fn eval_rook_behind_passer(&self, board: &Board, color: Color, passers: u64, mg: &mut i32, eg: &mut i32) {
        if passers == 0 { return; }
        let my_rooks = board.pieces_bb[Piece::Rook as usize] & board.colors_bb[color.index()];
        let enemy_rooks = board.pieces_bb[Piece::Rook as usize] & board.colors_bb[color.flip().index()];
        let mut rooks = my_rooks | enemy_rooks;

        while rooks != 0 {
            let sq_idx = rooks.trailing_zeros() as u8;
            rooks &= rooks - 1;
            let rook_file = (sq_idx & 7) as u8;
            let rook_rank = (sq_idx >> 3) as u8;

            // Check if there's a passer on the same file
            let file_passers = passers & file_mask(rook_file);
            if file_passers == 0 { continue; }

            let passer_sq = Square::new(file_passers.trailing_zeros() as u8).unwrap();
            let passer_rank = passer_sq.rank();

            // Rook "behind" the passed pawn: between pawn and its promotion/home rank
            let is_mine = (my_rooks >> sq_idx) & 1 != 0;
            let behind = if is_mine {
                // Own rook behind own passer (toward our side)
                if color == Color::White { rook_rank < passer_rank } else { rook_rank > passer_rank }
            } else {
                // Enemy rook behind our passer (blocking it from our side)
                if color == Color::White { rook_rank < passer_rank } else { rook_rank > passer_rank }
            };

            if behind {
                if is_mine {
                    *mg += self.rook_behind_passer_bonus.0;
                    *eg += self.rook_behind_passer_bonus.1;
                } else {
                    *mg -= self.rook_behind_passer_bonus.0;
                    *eg -= self.rook_behind_passer_bonus.1;
                }
            }
        }
    }

    fn eval_king_passer_proximity(&self, board: &Board, color: Color, passers: u64, mg: &mut i32, eg: &mut i32) {
        if passers == 0 { return; }
        let my_king = board.king_square[color.index()];
        let enemy_king = board.king_square[color.flip().index()];

        // King distance to own passed pawns (endgame: king supports passers)
        let mut bb = passers;
        while bb != 0 {
            let sq_idx = bb.trailing_zeros() as u8;
            bb &= bb - 1;
            let passer_sq = Square::new(sq_idx).unwrap();
            let dist_own = king_distance(my_king, passer_sq) as i32;
            let dist_enemy = king_distance(enemy_king, passer_sq) as i32;

            // Bonus: our king is closer than enemy king to our passer
            if dist_own < dist_enemy {
                *eg += self.king_passer_proximity_bonus * (dist_enemy as i32 - dist_own as i32);
            }
        }
    }

    fn eval_mobility(&self, board: &Board, color: Color, enemy: Color, occ: u64) -> i32 {
        let us_bb = board.colors_bb[color.index()];
        let enemy_bb = board.colors_bb[enemy.index()];
        let mut score = 0i32;

        let mut knights = board.pieces_bb[Piece::Knight as usize] & us_bb;
        while knights != 0 {
            let sq = knights.trailing_zeros() as u8;
            let attacks = crate::attack::knight_attacks(Square::new(sq).unwrap());
            let safe = (attacks & !us_bb).count_ones() as usize;
            score += self.knight_mobility[safe.min(8)];
            knights &= knights - 1;
        }

        let mut bishops = board.pieces_bb[Piece::Bishop as usize] & us_bb;
        while bishops != 0 {
            let sq = bishops.trailing_zeros() as u8;
            let attacks = crate::attack::bishop_attacks(sq, occ);
            let safe = (attacks & !us_bb & !enemy_bb).count_ones() as usize;
            score += self.bishop_mobility[safe.min(13)];
            bishops &= bishops - 1;
        }

        let mut rooks = board.pieces_bb[Piece::Rook as usize] & us_bb;
        while rooks != 0 {
            let sq = rooks.trailing_zeros() as u8;
            let attacks = crate::attack::rook_attacks(sq, occ);
            let safe = (attacks & !us_bb).count_ones() as usize;
            score += self.rook_mobility[safe.min(14)];
            rooks &= rooks - 1;
        }

        let mut queens = board.pieces_bb[Piece::Queen as usize] & us_bb;
        while queens != 0 {
            let sq = queens.trailing_zeros() as u8;
            let attacks = crate::attack::queen_attacks(sq, occ);
            let safe = (attacks & !us_bb).count_ones() as usize;
            score += self.queen_mobility[safe.min(27)];
            queens &= queens - 1;
        }

        score
    }

    fn eval_king_safety(&self, board: &Board, color: Color, king_sq: Square, pawns_bb: u64, enemy_bb: u64) -> i32 {
        let mut penalty = 0i32;
        let kf = king_sq.file();
        let kr = king_sq.rank();

        let shield_offsets: [(i32, i32); 6] = if color == Color::White {
            [(0, 1), (1, 1), (-1, 1), (0, 2), (1, 2), (-1, 2)]
        } else {
            [(0, -1), (1, -1), (-1, -1), (0, -2), (1, -2), (-1, -2)]
        };

        for &(df, dr) in &shield_offsets {
            let ff = kf as i32 + df;
            let rr = kr as i32 + dr;
            if ff >= 0 && ff < 8 && rr >= 0 && rr < 8 {
                let sq_mask = 1u64 << (rr * 8 + ff) as u64;
                if sq_mask & pawns_bb == 0 {
                    penalty += self.king_shield_missing_penalty;
                }
            }
        }

        let start_file = kf.saturating_sub(1);
        let end_file = (kf + 1).min(7);
        for f in start_file..=end_file {
            let fm = file_mask(f);
            if fm & pawns_bb == 0 && fm & enemy_bb != 0 {
                penalty += self.king_open_file_penalty;
            }
        }

        let king_zone = crate::attack::king_attacks(king_sq) | (1u64 << king_sq.index());
        let enemy_queens = board.pieces_bb[Piece::Queen as usize] & enemy_bb;
        let enemy_rooks = board.pieces_bb[Piece::Rook as usize] & enemy_bb;
        let zone_attackers = (enemy_queens | enemy_rooks) & king_zone;
        penalty += (zone_attackers.count_ones() as i32) * -15;

        penalty
    }
}

// ---- convenience free function (uses static Default Eval) ----

use std::sync::OnceLock;
static DEFAULT_EVAL: OnceLock<Eval> = OnceLock::new();

/// Evaluate the board using the default Eval (PeSTO weights).
/// For custom piece values or PST tables, use `Eval::evaluate()` instead.
pub fn evaluate(board: &Board) -> i32 {
    DEFAULT_EVAL.get_or_init(Eval::default).evaluate(board)
}

// ---- file / rank helpers ----

const FILE_A: u64 = 0x0101010101010101;

fn file_mask(file: u8) -> u64 {
    FILE_A << file
}

fn adjacent_files_mask(file: u8) -> u64 {
    let mut mask: u64 = 0;
    if file > 0 { mask |= file_mask(file - 1); }
    mask |= file_mask(file);
    if file < 7 { mask |= file_mask(file + 1); }
    mask
}

fn rank_mask_forward(sq: Square, color: Color) -> u64 {
    let rank = sq.rank();
    if color == Color::White {
        let mut m: u64 = 0;
        for r in (rank + 1)..8 { m |= 0xFFu64 << (r * 8); }
        m
    } else {
        let mut m: u64 = 0;
        for r in 0..rank { m |= 0xFFu64 << (r * 8); }
        m
    }
}

fn king_distance(a: Square, b: Square) -> u8 {
    let df = (a.file() as i32 - b.file() as i32).unsigned_abs() as u8;
    let dr = (a.rank() as i32 - b.rank() as i32).unsigned_abs() as u8;
    df.max(dr)
}

// ---- Static Exchange Evaluation (SEE) ----

fn see_piece_value(p: Option<Piece>) -> i32 {
    match p {
        Some(Piece::Pawn) => 100,
        Some(Piece::Knight) => 320,
        Some(Piece::Bishop) => 330,
        Some(Piece::Rook) => 500,
        Some(Piece::Queen) => 900,
        Some(Piece::King) => 20000,
        None => 0,
    }
}

fn attackers_to(board: &Board, sq: Square, occ: u64) -> u64 {
    let si = sq.index();
    let knights = board.pieces_bb[Piece::Knight as usize] & crate::attack::knight_attacks(sq);
    let kings = board.pieces_bb[Piece::King as usize] & crate::attack::king_attacks(sq);
    let pawns_w = board.pieces_bb[Piece::Pawn as usize]
        & board.colors_bb[Color::White.index()]
        & crate::attack::pawn_attacks(sq, Color::Black);
    let pawns_b = board.pieces_bb[Piece::Pawn as usize]
        & board.colors_bb[Color::Black.index()]
        & crate::attack::pawn_attacks(sq, Color::White);
    let rooks = (board.pieces_bb[Piece::Rook as usize] | board.pieces_bb[Piece::Queen as usize])
        & crate::attack::rook_attacks(si, occ);
    let bishops = (board.pieces_bb[Piece::Bishop as usize] | board.pieces_bb[Piece::Queen as usize])
        & crate::attack::bishop_attacks(si, occ);
    (knights | kings | pawns_w | pawns_b | rooks | bishops) & occ
}

fn smallest_attacker(board: &Board, sq: Square, side: Color, occ: u64) -> Option<(Square, Piece)> {
    let attackers = attackers_to(board, sq, occ) & board.colors_bb[side.index()];
    if attackers == 0 { return None; }
    let p = attackers & board.pieces_bb[Piece::Pawn as usize];
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Pawn)); }
    let p = attackers & board.pieces_bb[Piece::Knight as usize];
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Knight)); }
    let p = attackers & board.pieces_bb[Piece::Bishop as usize];
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Bishop)); }
    let p = attackers & board.pieces_bb[Piece::Rook as usize];
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Rook)); }
    let p = attackers & board.pieces_bb[Piece::Queen as usize];
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Queen)); }
    let p = attackers & board.pieces_bb[Piece::King as usize];
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::King)); }
    None
}

/// Recursive SEE: evaluate whether `side` can profitably capture the piece on `sq`.
/// Returns the net material gain from `side`'s perspective (>= 0).
fn see_rec(board: &Board, sq: Square, side: Color, occ: u64, piece_on_sq: Option<Piece>) -> i32 {
    let att = smallest_attacker(board, sq, side, occ);
    if att.is_none() { return 0; }
    let (att_sq, att_piece) = att.unwrap();
    let captured_val = see_piece_value(piece_on_sq);
    let new_occ = occ ^ att_sq.bit();
    let opp_gain = see_rec(board, sq, side.flip(), new_occ, Some(att_piece));
    0i32.max(captured_val - opp_gain)
}

/// Static Exchange Evaluation for the given capture.
/// Returns net material gain from the perspective of the side that made the capture
/// (positive = winning exchange, negative = losing).
pub fn see(board: &Board, mv: Move) -> i32 {
    let from = mv.from();
    let to = mv.to();
    let moving = board.piece_at(from);
    let victim = board.piece_at(to);

    let mut base_gain = see_piece_value(victim);
    if let Some(pp) = mv.promotion_piece() {
        base_gain += see_piece_value(Some(pp)) - see_piece_value(Some(Piece::Pawn));
    }

    let occ = board.occupancy ^ from.bit() ^ to.bit();
    let opp_gain = see_rec(board, to, board.side_to_move.flip(), occ, moving);
    base_gain - opp_gain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_initial_position_near_zero() {
        let board = Board::from_initial();
        let score = evaluate(&board);
        assert!(score.abs() <= 50);
    }

    #[test]
    fn test_initial_position_symmetric() {
        let board = Board::from_initial();
        let score_white = evaluate(&board);
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board2 = Board::from_fen(fen).unwrap();
        let score_black = evaluate(&board2);
        assert!(score_white.abs() <= 50);
        assert!(score_black.abs() <= 50);
    }

    #[test]
    fn test_white_advantage_positive() {
        let board = Board::from_initial();
        let score_initial = evaluate(&board);
        let fen_up = "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board_up = Board::from_fen(fen_up).unwrap();
        let score_up = evaluate(&board_up);
        assert!(score_up > score_initial);
    }

    #[test]
    fn test_black_checkmate_scores_negative() {
        let fen = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";
        let board = Board::from_fen(fen).unwrap();
        let score = evaluate(&board);
        assert!(score <= 0);
    }

    #[test]
    fn test_symmetric_position_evaluates_equal() {
        let board = Board::from_initial();
        let score = evaluate(&board);
        assert!(score.abs() < 100);
    }

    #[test]
    fn test_bishop_pair_bonus() {
        let fen_b = "k7/8/8/8/8/8/8/2B1B2K w - - 0 1";
        let fen_n = "k7/8/8/8/8/8/8/2N1N2K w - - 0 1";
        let score_b = evaluate(&Board::from_fen(fen_b).unwrap());
        let score_n = evaluate(&Board::from_fen(fen_n).unwrap());
        assert!(score_b > score_n + 20);
    }

    #[test]
    fn test_rook_open_file() {
        let fen_open = "k7/8/8/8/8/8/8/K1R5 w - - 0 1";
        let fen_closed = "k7/8/8/8/8/2p5/8/K1R5 w - - 0 1";
        let score_open = evaluate(&Board::from_fen(fen_open).unwrap());
        let score_closed = evaluate(&Board::from_fen(fen_closed).unwrap());
        assert!(score_open > score_closed);
    }

    #[test]
    fn test_passed_pawn_bonus() {
        let fen = "k7/8/8/8/P7/8/8/K7 w - - 0 1";
        let score = evaluate(&Board::from_fen(fen).unwrap());
        assert!(score > 60);
    }

    #[test]
    fn test_doubled_pawns_penalty() {
        let fen_dbl = "k7/8/8/8/8/P7/P7/K7 w - - 0 1";
        let fen_spread = "k7/8/8/8/8/8/P1P5/K7 w - - 0 1";
        let score_dbl = evaluate(&Board::from_fen(fen_dbl).unwrap());
        let score_spread = evaluate(&Board::from_fen(fen_spread).unwrap());
        assert!(score_spread > score_dbl);
    }

    #[test]
    fn test_king_safety_pawn_shield() {
        let fen_shield = "k7/8/8/8/8/8/2P1K3/8 w - - 0 1";
        let fen_bare = "k7/8/8/8/8/8/4K3/8 w - - 0 1";
        let score_shield = evaluate(&Board::from_fen(fen_shield).unwrap());
        let score_bare = evaluate(&Board::from_fen(fen_bare).unwrap());
        assert!(score_shield > score_bare);
    }

    // --- Custom Eval tests ---

    #[test]
    fn test_custom_piece_values() {
        // Changing knight value shifts the score
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
        // White pawn on e4 captures black knight on d5, no recapture
        let board = Board::from_fen("8/8/8/3n4/4P3/8/8/8 w - -").unwrap();
        let e4 = Square::from_file_rank(4, 3).unwrap();
        let d5 = Square::from_file_rank(3, 4).unwrap();
        let mv = Move::capture(e4, d5);
        assert_eq!(see(&board, mv), 320);
    }

    #[test]
    fn test_see_losing_capture() {
        crate::attack::init_slider_tables();
        // White queen on e2 captures black rook on e5, black pawn on d6 recaptures
        let board = Board::from_fen("8/8/3p4/4r3/8/8/4Q3/8 w - -").unwrap();
        let e2 = Square::from_file_rank(4, 1).unwrap();
        let e5 = Square::from_file_rank(4, 4).unwrap();
        let mv = Move::capture(e2, e5);
        assert!(see(&board, mv) < 0, "QxR with pawn recapture should be losing");
    }

    #[test]
    fn test_see_winning_capture() {
        crate::attack::init_slider_tables();
        // White rook on e1 captures black knight on e5, no recapture
        let board = Board::from_fen("8/8/8/4n3/8/8/8/4R3 w - -").unwrap();
        let e1 = Square::E1;
        let e5 = Square::from_file_rank(4, 4).unwrap();
        let mv = Move::capture(e1, e5);
        assert!(see(&board, mv) > 0);
    }

    #[test]
    fn test_see_promotion_capture() {
        crate::attack::init_slider_tables();
        // White pawn on e7 captures black rook on d8, promoting to queen
        let board = Board::from_fen("3r4/4P3/8/8/8/8/8/8 w - -").unwrap();
        let e7 = Square::from_file_rank(4, 6).unwrap();
        let d8 = Square::from_file_rank(3, 7).unwrap();
        let mv = Move::promotion(e7, d8, Piece::Queen);
        assert!(see(&board, mv) > 0);
    }

    #[test]
    fn test_see_even_exchange() {
        crate::attack::init_slider_tables();
        // White rook on e1 captures black rook on e8, protected by black rook on a8
        let board = Board::from_fen("r3r3/8/8/8/8/8/8/4R3 w - -").unwrap();
        let e1 = Square::E1;
        let e8 = Square::from_file_rank(4, 7).unwrap();
        let mv = Move::capture(e1, e8);
        // Rook takes rook, opponent rook recaptures — even exchange
        assert_eq!(see(&board, mv), 0);
    }
}
