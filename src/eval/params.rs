#![allow(unused_imports)]
use crate::board::Board;
use crate::types::{Color, Move, Piece, Square};
use crate::attack::{file_mask, adjacent_files_mask, rank_mask_forward, king_distance};

// ---- Sub-structs: domain-grouped eval parameters ----

#[derive(Debug, Clone)]
pub(crate) struct MaterialValues {
    pub(crate) pawn_value: i32,
    pub(crate) knight_value: i32,
    pub(crate) bishop_value: i32,
    pub(crate) rook_value: i32,
    pub(crate) queen_value: i32,
    pub(crate) king_value: i32,
}

impl Default for MaterialValues {
    fn default() -> Self {
        Self {
            pawn_value: 100,
            knight_value: 320,
            bishop_value: 330,
            rook_value: 500,
            queen_value: 900,
            king_value: 20000,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PieceSquareTables {
    pub(crate) mg_pawn_table: [i32; 64],
    pub(crate) eg_pawn_table: [i32; 64],
    pub(crate) mg_knight_table: [i32; 64],
    pub(crate) eg_knight_table: [i32; 64],
    pub(crate) mg_bishop_table: [i32; 64],
    pub(crate) eg_bishop_table: [i32; 64],
    pub(crate) mg_rook_table: [i32; 64],
    pub(crate) eg_rook_table: [i32; 64],
    pub(crate) mg_queen_table: [i32; 64],
    pub(crate) eg_queen_table: [i32; 64],
    pub(crate) mg_king_table: [i32; 64],
    pub(crate) eg_king_table: [i32; 64],
}

impl Default for PieceSquareTables {
    fn default() -> Self {
        Self {
            mg_pawn_table: [
                 0,   0,   0,   0,   0,   0,  0,   0,
                 5,  10,   0,   5,  10,   5,  0, -10,
                30,   7,  26,  50,  65,  56, 60, -20,
                 0,  45,  38,  55,  38,  24, 29, -10,
               -27,  -2,  -5,  12,  17,   6, 10, -25,
               -26,  -4,  -4, -10,   3,   3, 33, -12,
               -35,  -1, -20, -23, -15,  24, 38, -22,
                 0,   0,   0,   0,   0,   0,  0,   0,
            ],
            eg_pawn_table: [
                 0,   0,   0,   0,   0,   0,  0,   0,
               107, 104,  95,  80,  88,  79,  99, 112,
                94, 100,  85,  67,  56,  53,  82,  84,
                32,  24,  13,   5,  -2,   4,  17,  17,
                13,   9,  -3,  -7,  -7,  -8,   3,  -1,
                 4,   7,  -6,   1,   0,  -5,  -1,  -8,
                13,   8,   8,  10,  13,   0,   2,  -7,
                 0,   0,   0,   0,   0,   0,  0,   0,
            ],
            mg_knight_table: [
                -30, -15, -10,  -5,  -5, -10, -15, -30,
                -20, -10,   0,  10,  10,   0, -10, -20,
                -10,   5,  15,  20,  20,  15,   5, -10,
                 -5,  10,  20,  30,  30,  20,  10,  -5,
                 -5,  10,  20,  30,  30,  20,  10,  -5,
                 -5,  10,  15,  25,  25,  15,  10,  -5,
                -15,   0,  10,  15,  15,  10,   0, -15,
                -25, -10,  -5,   0,   0,  -5, -10, -25,
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
                -25, -15, -10,  -5,  -5, -10, -15, -25,
                -15,  -5,   5,  10,  10,   5,  -5, -15,
                -10,   5,  15,  20,  20,  15,   5, -10,
                 -5,  10,  20,  25,  25,  20,  10,  -5,
                 -5,  10,  20,  25,  25,  20,  10,  -5,
                 -5,  10,  20,  25,  25,  20,  10,  -5,
                -10,   5,  15,  20,  20,  15,   5, -10,
                -20, -10,   0,   5,   5,   0,   5, -10,
            ],
            eg_bishop_table: [
                -14, -21, -11,  -8, -7,  -9,  -5, -24,
                 -8,  -4,   7, -12, -3, -13,  -4, -14,
                  2,  -8,   0,  -1, -2,   6,   0,   4,
                 -3,   9,  12,   9, 14,  10,   3,   2,
                 -6,   3,  13,  19,  7,  10,  -3,  -9,
                -12,  -3,   8,  10, 13,   3,  -7, -15,
                -14, -18,  -7,  -1,  4,  -9, -15, -27,
                -23,  -9, -23,  -5, -9, -16,   0, -17,
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
                -80, -70, -60, -55, -55, -60, -70, -80,
                -70, -60, -50, -45, -45, -50, -60, -70,
                -60, -50, -40, -35, -35, -40, -50, -60,
                -50, -40, -30, -25, -25, -30, -40, -50,
                -40, -30, -20, -15, -15, -20, -30, -40,
                -30, -20, -10,  -5,  -5, -10, -20, -30,
                -25, -15,  -5,   0,   0,  -5, -15, -25,
                -25, -15,   5,  -5, -10,  -5,  10, -15,
            ],
            eg_king_table: [
                -100, -60, -40, -30, -30, -40, -60, -100,
                 -60, -20,   0,  10,  10,   0, -20,  -60,
                 -40,   0,  20,  30,  30,  20,   0,  -40,
                 -30,  10,  30,  50,  50,  30,  10,  -30,
                 -30,  10,  30,  50,  50,  30,  10,  -30,
                 -40,   0,  20,  30,  30,  20,   0,  -40,
                 -60, -20,   0,  10,  10,   0, -20,  -60,
                -100, -60, -40, -30, -30, -40, -60, -100,
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MobilityTables {
    pub(crate) knight_mobility: [i32; 9],
    pub(crate) bishop_mobility: [i32; 14],
    pub(crate) rook_mobility: [i32; 15],
    pub(crate) queen_mobility: [i32; 28],
    pub(crate) knight_mobility_eg: [i32; 9],
    pub(crate) bishop_mobility_eg: [i32; 14],
    pub(crate) rook_mobility_eg: [i32; 15],
    pub(crate) queen_mobility_eg: [i32; 28],
}

impl Default for MobilityTables {
    fn default() -> Self {
        Self {
            knight_mobility: [-20, -6, 6, 14, 19, 22, 24, 25, 25],
            bishop_mobility: [-20, -6, 6, 14, 20, 24, 27, 29, 31, 32, 33, 34, 35, 35],
            rook_mobility: [-20, -6, 6, 14, 20, 25, 28, 31, 33, 34, 35, 36, 37, 37, 37],
            queen_mobility: [
                -20, -6, 6, 14, 20, 25, 29, 32, 35, 37, 39, 41, 43, 44,
                45, 46, 47, 48, 49, 50, 50, 50, 50, 50, 50, 50, 50, 50,
            ],
            knight_mobility_eg: [-20, -3, 3, 7, 10, 11, 12, 13, 13],
            bishop_mobility_eg: [-20, -3, 3, 7, 10, 12, 14, 15, 16, 16, 17, 17, 18, 18],
            rook_mobility_eg: [-20, -3, 3, 7, 10, 13, 14, 16, 17, 17, 18, 18, 19, 19, 19],
            queen_mobility_eg: [
                -20, -3, 3, 7, 10, 13, 15, 16, 18, 19, 20, 21, 22, 22,
                23, 23, 24, 24, 25, 25, 25, 25, 25, 25, 25, 25, 25, 25,
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PawnEval {
    pub(crate) doubled_pawn_penalty: (i32, i32),
    pub(crate) isolated_pawn_penalty: (i32, i32),
    pub(crate) passed_pawn_bonus: [i32; 8],
    pub(crate) backward_pawn_penalty: (i32, i32),
    pub(crate) pawn_phalanx_bonus: (i32, i32),
    pub(crate) pawn_chain_bonus: (i32, i32),
    pub(crate) candidate_passer_bonus: [i32; 8],
    pub(crate) passer_blocker_bonus: (i32, i32),
    pub(crate) space_bonus: (i32, i32),
    pub(crate) pawn_majority_bonus: (i32, i32),
}

impl Default for PawnEval {
    fn default() -> Self {
        Self {
            doubled_pawn_penalty: (-12, -24),
            isolated_pawn_penalty: (-10, -20),
            passed_pawn_bonus: [0, 5, 10, 20, 40, 70, 100, 0],
            backward_pawn_penalty: (-8, -16),
            pawn_phalanx_bonus: (8, 12),
            pawn_chain_bonus: (5, 8),
            candidate_passer_bonus: [0, 2, 5, 10, 20, 35, 50, 0],
            passer_blocker_bonus: (10, 15),
            space_bonus: (5, 3),
            pawn_majority_bonus: (8, 14),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PieceEval {
    pub(crate) bishop_pair_bonus: (i32, i32),
    pub(crate) rook_open_file_bonus: (i32, i32),
    pub(crate) rook_semi_open_file_bonus: (i32, i32),
    pub(crate) rook_closed_file_penalty: (i32, i32),
    pub(crate) rook_seventh_rank_bonus: (i32, i32),
    pub(crate) rook_queen_battery_bonus: (i32, i32),
    pub(crate) outpost_knight_bonus: (i32, i32),
    pub(crate) knight_rim_penalty: (i32, i32),
    pub(crate) knight_trapped_penalty: (i32, i32),
    pub(crate) bad_bishop_penalty: (i32, i32),
    pub(crate) bad_bishop_fixed_multiplier: i32,
    pub(crate) queen_fork_bonus: (i32, i32),
    pub(crate) queen_attack_count_bonus: [i32; 8],
    pub(crate) exchange_open_file_bonus: (i32, i32),
    pub(crate) exchange_bishop_pair_penalty: (i32, i32),
    pub(crate) exchange_minor_activity_bonus: (i32, i32),
}

impl Default for PieceEval {
    fn default() -> Self {
        Self {
            bishop_pair_bonus: (25, 45),
            rook_open_file_bonus: (25, 10),
            rook_semi_open_file_bonus: (12, 5),
            rook_closed_file_penalty: (-15, -20),
            rook_seventh_rank_bonus: (30, 40),
            rook_queen_battery_bonus: (15, 20),
            outpost_knight_bonus: (18, 8),
            knight_rim_penalty: (-10, -15),
            knight_trapped_penalty: (-25, -35),
            bad_bishop_penalty: (-20, -30),
            bad_bishop_fixed_multiplier: 2,
            queen_fork_bonus: (30, 25),
            queen_attack_count_bonus: [0, 4, 8, 12, 16, 20, 24, 28],
            exchange_open_file_bonus: (10, 15),
            exchange_bishop_pair_penalty: (-20, -30),
            exchange_minor_activity_bonus: (15, 20),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct KingEval {
    pub(crate) king_shield_missing_penalty: i32,
    pub(crate) king_open_file_penalty: i32,
    pub(crate) king_opposition_bonus: i32,
    pub(crate) connected_passer_bonus: i32,
    pub(crate) rook_behind_passer_bonus: (i32, i32),
    pub(crate) king_passer_proximity_bonus: i32,
    pub(crate) king_passer_proximity_bonus_mg: i32,
}

impl Default for KingEval {
    fn default() -> Self {
        Self {
            king_shield_missing_penalty: -12,
            king_open_file_penalty: -20,
            king_opposition_bonus: 50,
            connected_passer_bonus: 20,
            rook_behind_passer_bonus: (20, 30),
            king_passer_proximity_bonus: 10,
            king_passer_proximity_bonus_mg: 5,
        }
    }
}

// ---- Eval struct: container for all six sub-structs ----

#[derive(Debug, Clone)]
pub(crate) struct Eval {
    pub(crate) material: MaterialValues,
    pub(crate) pst: PieceSquareTables,
    pub(crate) mobility: MobilityTables,
    pub(crate) pawn: PawnEval,
    pub(crate) piece: PieceEval,
    pub(crate) king: KingEval,
}

impl Default for Eval {
    fn default() -> Self {
        Self {
            material: MaterialValues::default(),
            pst: PieceSquareTables::default(),
            mobility: MobilityTables::default(),
            pawn: PawnEval::default(),
            piece: PieceEval::default(),
            king: KingEval::default(),
        }
    }
}
