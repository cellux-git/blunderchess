use crate::move_ordering::MoveOrdering;
use crate::board::Board;
use crate::eval::Eval;
use crate::movegen::{self, MAX_MOVES};
use crate::tt::TT;
use crate::types::{Move, MAX_DEPTH};
use crate::search::params::{SearchParams, SearchResult, SearchAlgorithmParams, CHECKMATE};
use crate::search::alpha_beta::alpha_beta;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Instant;

pub(crate) const MAX_EXCLUDED: usize = 8;

pub(crate) struct SearchState {
    pub nodes: u64,
    pub pv: [[Option<Move>; MAX_DEPTH as usize]; MAX_DEPTH as usize],
    pub pv_length: [usize; MAX_DEPTH as usize],
    pub stop: *const AtomicBool,
    pub start_time: Instant,
    pub movetime: Option<u64>,
    pub soft_time: Option<u64>,
    pub move_ordering: MoveOrdering,
    pub excluded_moves: [Move; MAX_EXCLUDED],
    pub excluded_count: u8,
}

impl SearchState {
    pub fn should_stop(&self) -> bool {
        if unsafe { &*self.stop }.load(AtomicOrdering::Acquire) { return true; }
        if self.nodes & 1023 == 0 {
            if let Some(limit) = self.movetime {
                if self.start_time.elapsed().as_millis() as u64 >= limit { return true; }
            }
        }
        false
    }

    pub fn soft_time_exceeded(&self) -> bool {
        if let Some(soft) = self.soft_time {
            self.start_time.elapsed().as_millis() as u64 >= soft
        } else {
            false
        }
    }
}

pub(crate) fn search_worker(
    board: &mut Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
    thread_id: u8,
    eval: &Eval,
) -> SearchResult {
    let max_depth = params.depth.unwrap_or(MAX_DEPTH);
    let start = Instant::now();
    let mut best_result = SearchResult {
        best_move: None, score: 0, depth: 0, pv: Vec::new(), nodes: 0, total_nodes: 0, time_ms: 0, multi_pv_lines: Vec::new(),
    };

    let alg = SearchAlgorithmParams::default();

    let mut state = SearchState {
        nodes: 0,
        pv: [[None; MAX_DEPTH as usize]; MAX_DEPTH as usize],
        pv_length: [0; MAX_DEPTH as usize],
        stop: &**stop,
        start_time: start,
        movetime: params.movetime,
        soft_time: params.movetime.map(|t| t / alg.soft_time_divisor),
        move_ordering: MoveOrdering::new(),
        excluded_moves: [Move::NULL; MAX_EXCLUDED],
        excluded_count: 0,
    };

    let multi_pv = params.multi_pv.max(1) as usize;
    let mut prev_scores = vec![0i32; multi_pv];
    let mut delta = alg.aspiration.initial_delta;

    for depth in 1..=max_depth {
        let mut excluded_moves: [Move; MAX_EXCLUDED] = [Move::NULL; MAX_EXCLUDED];
        let mut excluded_count: u8 = 0;
        let mut depth_results: Vec<(Move, i32, Vec<Move>)> = Vec::new();

        for mpv_idx in 0..multi_pv {
            if state.should_stop() { break; }

            state.pv_length = [0; MAX_DEPTH as usize];
            state.excluded_moves = excluded_moves;
            state.excluded_count = excluded_count;

            let mut alpha = -(CHECKMATE + 100);
            let mut beta = CHECKMATE + 100;

            if depth >= alg.aspiration.depth_threshold && prev_scores[mpv_idx].abs() < CHECKMATE - 500 {
                alpha = prev_scores[mpv_idx] - delta;
                beta = prev_scores[mpv_idx] + delta;
            }

            let score = loop {
                let score = alpha_beta(board, alpha, beta, depth, 0, &mut state, tt, true, thread_id, &alg, eval);

                if state.should_stop() { break score; }

                if score <= alpha {
                    alpha = -(CHECKMATE + 100);
                    delta += delta / 2;
                    if state.should_stop() { break score; }
                    continue;
                }
                if score >= beta {
                    beta = CHECKMATE + 100;
                    delta += delta / 2;
                    if state.should_stop() { break score; }
                    continue;
                }
                prev_scores[mpv_idx] = score;
                break score;
            };

            let pv_moves: Vec<Move> = (0..state.pv_length[0]).filter_map(|i| state.pv[0][i]).collect();

            if let Some(best) = pv_moves.first().copied() {
                if (excluded_count as usize) < MAX_EXCLUDED {
                    excluded_moves[excluded_count as usize] = best;
                    excluded_count += 1;
                }
                depth_results.push((best, score, pv_moves));
            } else if depth_results.is_empty() {
                depth_results.push((Move::NULL, score, Vec::new()));
            }

            if state.should_stop() { break; }
        }

        if state.should_stop() && depth > 1 { break; }
        if depth > 1 && state.soft_time_exceeded() { break; }

        if let Some((best_mv, best_score, pv)) = depth_results.first() {
            if *best_mv != Move::NULL {
                best_result.best_move = Some(*best_mv);
            }
            best_result.score = *best_score;
            best_result.pv = pv.clone();
            best_result.depth = depth;
            best_result.nodes = state.nodes;
            best_result.time_ms = start.elapsed().as_millis() as u64;
            best_result.multi_pv_lines = depth_results.iter()
                .filter(|(m, _, _)| *m != Move::NULL)
                .enumerate()
                .map(|(i, (_, s, p))| ((i + 1) as u8, *s, p.clone()))
                .collect();
        }

        if depth_results.first().map_or(false, |&(_, s, _)| s.abs() >= CHECKMATE - 100) {
            break;
        }
    }

    if best_result.best_move.is_none() {
        let mut buf = [Move::NULL; MAX_MOVES];
        let count = movegen::generate_legal_moves(board, &mut buf);
        best_result.best_move = if count > 0 { Some(buf[0]) } else { None };
    }
    best_result.nodes = state.nodes;
    best_result.total_nodes = state.nodes;
    best_result.time_ms = start.elapsed().as_millis() as u64;
    best_result
}
