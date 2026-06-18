use crate::types::Move;

pub const CHECKMATE: i32 = 1_000_000;

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub depth: Option<u8>,
    pub movetime: Option<u64>,
    pub infinite: bool,
    pub threads: u8,
    pub multi_pv: u8,
    pub ponder: bool,
}

impl SearchParams {
    pub fn new() -> SearchParams {
        SearchParams { depth: None, movetime: None, infinite: false, threads: 1, multi_pv: 1, ponder: false }
    }

    pub fn with_depth(depth: u8) -> SearchParams {
        SearchParams { depth: Some(depth), movetime: None, infinite: false, threads: 1, multi_pv: 1, ponder: false }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub pv: Vec<Move>,
    pub nodes: u64,
    pub total_nodes: u64,
    pub time_ms: u64,
    pub multi_pv_lines: Vec<(u8, i32, Vec<Move>)>,
}

#[derive(Debug, Clone)]
pub struct LmrConfig {
    pub min_depth: u8,
    pub min_moves_searched: u8,
    pub reduction: [u8; 3],
}
impl Default for LmrConfig {
    fn default() -> Self { Self { min_depth: 3, min_moves_searched: 3, reduction: [1, 2, 3] } }
}

#[derive(Debug, Clone)]
pub struct NullMoveConfig {
    pub min_depth: u8,
    pub r_shallow: u8,
    pub r_deep: u8,
    pub deep_threshold: u8,
}
impl Default for NullMoveConfig {
    fn default() -> Self { Self { min_depth: 3, r_shallow: 3, r_deep: 4, deep_threshold: 6 } }
}

#[derive(Debug, Clone)]
pub struct AspirationConfig {
    pub initial_delta: i32,
    pub depth_threshold: u8,
}
impl Default for AspirationConfig {
    fn default() -> Self { Self { initial_delta: 25, depth_threshold: 4 } }
}

#[derive(Debug, Clone)]
pub struct FutilityConfig {
    pub max_depth: u8,
    pub margin_d1: i32,
    pub margin_d2: i32,
}
impl Default for FutilityConfig {
    fn default() -> Self { Self { max_depth: 2, margin_d1: 200, margin_d2: 400 } }
}

#[derive(Debug, Clone)]
pub struct SearchAlgorithmParams {
    pub lmr: LmrConfig,
    pub null_move: NullMoveConfig,
    pub aspiration: AspirationConfig,
    pub futility: FutilityConfig,
    pub razor_margin: i32,
    pub soft_time_divisor: u64,
}
impl Default for SearchAlgorithmParams {
    fn default() -> Self {
        Self {
            lmr: LmrConfig::default(),
            null_move: NullMoveConfig::default(),
            aspiration: AspirationConfig::default(),
            futility: FutilityConfig::default(),
            razor_margin: 350,
            soft_time_divisor: 2,
        }
    }
}
