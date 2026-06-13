use crate::board::{Board, GameResult, MAX_MOVES};
use crate::eval::Eval;
use crate::movegen;
use crate::tt::{NodeType, TT};
use crate::types::{Color, Move, MoveKind, Piece};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub const CHECKMATE: i32 = 1_000_000;
pub const MAX_DEPTH: u8 = 128;

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
    pub time_ms: u64,
    pub multi_pv_lines: Vec<(u8, i32, Vec<Move>)>,
}

struct SearchState {
    nodes: u64,
    pv: [[Option<Move>; MAX_DEPTH as usize]; MAX_DEPTH as usize],
    pv_length: [usize; MAX_DEPTH as usize],
    stop: *const AtomicBool,
    start_time: Instant,
    movetime: Option<u64>,
    killers: [[Option<Move>; 2]; MAX_DEPTH as usize],
    history: [[i32; 64]; 64],
    excluded_moves: Vec<Move>,
}

impl SearchState {
    fn should_stop(&self) -> bool {
        if unsafe { &*self.stop }.load(Ordering::Relaxed) { return true; }
        if let Some(limit) = self.movetime {
            if self.start_time.elapsed().as_millis() as u64 >= limit { return true; }
        }
        false
    }
}

pub fn search(
    board: &Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
) -> SearchResult {
    if params.threads.max(1) == 1 {
        return search_single(board, params, stop, tt, 0);
    }
    search_mt(board, params, stop, tt)
}

pub fn search_single(
    board: &Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
    thread_id: u8,
) -> SearchResult {
    let mut b = board.clone();
    search_worker(&mut b, params, stop, tt, thread_id)
}

pub fn search_mt(
    board: &Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
) -> SearchResult {
    let num_threads = params.threads.max(1) as usize;
    if num_threads == 1 {
        return search_single(board, params, stop, tt, 0);
    }

    let start = Instant::now();
    let board = board.clone();
    let params = params.clone();
    let stop = Arc::clone(stop);
    let tt = tt.clone();
    let best_result = Arc::new(std::sync::Mutex::new(SearchResult {
        best_move: None, score: 0, depth: 0, pv: Vec::new(), nodes: 0, time_ms: 0, multi_pv_lines: Vec::new(),
    }));

    let mut handles = Vec::with_capacity(num_threads);

    for tid in 0..num_threads as u8 {
        let mut b = board.clone();
        let p = params.clone();
        let real_stop = if tid == 0 { Arc::clone(&stop) } else { Arc::new(AtomicBool::new(false)) };
        let tt = tt.clone();
        let best = Arc::clone(&best_result);

        let handle = std::thread::spawn(move || {
            let result = search_worker(&mut b, &p, &real_stop, &tt, tid);
            let mut best = best.lock().unwrap();
            if result.depth > best.depth || (result.depth == best.depth && result.best_move.is_some()) {
                *best = result;
            }
        });
        handles.push(handle);
    }

    for handle in handles { let _ = handle.join(); }

    let mut result = best_result.lock().unwrap().clone();
    if result.nodes == 0 { result.nodes = 1; }
    result.time_ms = start.elapsed().as_millis() as u64;
    if result.best_move.is_none() {
        let mut buf = [Move::NULL; MAX_MOVES];
        let count = movegen::generate_legal_moves(&board, &mut buf);
        result.best_move = if count > 0 { Some(buf[0]) } else { None };
    }
    result
}

fn search_worker(
    board: &mut Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
    thread_id: u8,
) -> SearchResult {
    let max_depth = params.depth.unwrap_or(MAX_DEPTH);
    let start = Instant::now();
    let mut best_result = SearchResult {
        best_move: None, score: 0, depth: 0, pv: Vec::new(), nodes: 0, time_ms: 0, multi_pv_lines: Vec::new(),
    };

    let mut state = SearchState {
        nodes: 0,
        pv: [[None; MAX_DEPTH as usize]; MAX_DEPTH as usize],
        pv_length: [0; MAX_DEPTH as usize],
        stop: &**stop,
        start_time: start,
        movetime: params.movetime,
        killers: [[None; 2]; MAX_DEPTH as usize],
        history: [[0; 64]; 64],
        excluded_moves: Vec::new(),
    };

    let multi_pv = params.multi_pv.max(1) as usize;
    let mut prev_scores = vec![0i32; multi_pv];
    let mut delta = 25i32;

    for depth in 1..=max_depth {
        let mut excluded_moves: Vec<Move> = Vec::new();
        let mut depth_results: Vec<(Move, i32, Vec<Move>)> = Vec::new();

        for mpv_idx in 0..multi_pv {
            if state.should_stop() { break; }

            state.pv_length = [0; MAX_DEPTH as usize];
            state.excluded_moves = excluded_moves.clone();

            let mut alpha = -(CHECKMATE + 100);
            let mut beta = CHECKMATE + 100;

            if depth >= 4 && prev_scores[mpv_idx].abs() < CHECKMATE - 500 {
                alpha = prev_scores[mpv_idx] - delta;
                beta = prev_scores[mpv_idx] + delta;
            }

            let score = loop {
                let score = alpha_beta(board, alpha, beta, depth, 0, &mut state, tt, true, thread_id);

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
                excluded_moves.push(best);
                depth_results.push((best, score, pv_moves));
            } else if depth_results.is_empty() {
                // No PV (e.g. immediate checkmate/draw) — record score anyway
                depth_results.push((Move::NULL, score, Vec::new()));
            }

            if state.should_stop() { break; }
        }

        if state.should_stop() && depth > 1 { break; }

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
    best_result.time_ms = start.elapsed().as_millis() as u64;
    best_result
}

fn alpha_beta(
    board: &mut Board,
    mut alpha: i32,
    beta: i32,
    depth: u8,
    ply: u8,
    state: &mut SearchState,
    tt: &Arc<TT>,
    is_pv: bool,
    thread_id: u8,
) -> i32 {
    state.nodes += 1;

    if ply >= MAX_DEPTH - 1 { return Eval::default().evaluate(board); }
    state.pv_length[ply as usize] = ply as usize;

    if state.should_stop() || depth == 0 {
        return quiescence(board, alpha, beta, ply, state);
    }

    if let Some(result) = board.check_result() {
        return match result {
            GameResult::Checkmate(winner) => {
                let sign: i32 = if winner == Color::White { 1 } else { -1 };
                let color_sign: i32 = if board.side_to_move() == Color::White { 1 } else { -1 };
                -(CHECKMATE - ply as i32) * sign * color_sign
            }
            GameResult::Stalemate | GameResult::Draw => 0,
        };
    }

    let hash = board.hash();
    let tt_entry = tt.probe(hash);
    let tt_score = tt_entry.as_ref().map(|e| {
        if e.score.abs() >= CHECKMATE - 100 {
            if e.score > 0 { e.score - ply as i32 } else { e.score + ply as i32 }
        } else { e.score }
    });

    if let Some(ref entry) = tt_entry {
        if !is_pv && entry.depth >= depth {
            match entry.node_type {
                NodeType::Exact => return tt_score.unwrap(),
                NodeType::LowerBound => { if tt_score.unwrap() >= beta { return tt_score.unwrap(); } }
                NodeType::UpperBound => { if tt_score.unwrap() <= alpha { return tt_score.unwrap(); } }
            }
        }
    }

    let hash_move = tt_entry.and_then(|e| e.best_move);

    let can_null_move = !is_pv && depth >= 3 && ply > 0 && !board.in_check();
    let has_big_pieces = board.piece_list().iter().any(|&(_, p, _)| p != crate::types::Piece::Pawn && p != crate::types::Piece::King);

    if can_null_move && has_big_pieces {
        let r = if depth >= 6 { 4 } else { 3 };
        let null_depth = if depth > r { depth - r } else { 0 };
        if null_depth > 0 {
            let undo_null = board.make_null_move();
            let null_score = -alpha_beta(board, -beta, -beta + 1, null_depth, ply + 1, state, tt, false, thread_id);
            board.unmake_null_move(&undo_null);
            if null_score >= beta { return null_score; }
        }
    }

    let static_eval = if depth <= 2 { Some(Eval::default().evaluate(board)) } else { None };

    let mut moves_buf = [Move::NULL; MAX_MOVES];
    let mut move_count: usize = 0;
    movegen::generate_pseudo_legal(board, &mut moves_buf, &mut move_count);
    let moves = &mut moves_buf[..move_count];
    order_moves(moves, board, hash_move, ply, state, thread_id);

    let side = board.side_to_move();
    let pinned = board.pinned_pieces(side);
    let mut best_move: Option<Move> = None;
    let mut best_score = -(CHECKMATE + 200);
    let mut node_type = NodeType::UpperBound;
    let mut moves_searched = 0u32;

    for i in 0..moves.len() {
        let mv = moves[i];

        // MultiPV: skip excluded moves at root
        if ply == 0 && state.excluded_moves.contains(&mv) {
            continue;
        }

        let from = mv.from();

        // Trivially legal: non-king, non-ep, non-castle, non-pinned
        let is_trivially_legal = {
            if let Some(piece) = board.piece_at(from) {
                let is_ep = mv.kind() == MoveKind::Capture
                    && board.en_passant() == Some(mv.to());
                piece != Piece::King
                    && !is_ep
                    && mv.kind() != MoveKind::Castle
                    && (from.bit() & pinned) == 0
            } else {
                false
            }
        };

        let undo = board.make_move(mv);

        if !is_trivially_legal {
            let king_sq = board.king_square(side);
            if board.is_attacked_by(king_sq, board.side_to_move()) {
                board.unmake_move(&undo);
                continue;
            }
        }

        // Futility pruning: skip quiet moves near horizon when eval is far below alpha
        if let Some(se) = static_eval {
            if depth <= 2 {
                let mv_kind = mv.kind();
                let is_quiet = mv_kind != MoveKind::Capture && mv_kind != MoveKind::Promotion;
                if is_quiet && !board.in_check() {
                    let margin: i32 = if depth == 2 { 400 } else { 200 };
                    if se + margin <= alpha {
                        board.unmake_move(&undo);
                        continue;
                    }
                }
            }
        }

        let mut score: i32;
        if moves_searched == 0 {
            score = -alpha_beta(board, -beta, -alpha, depth - 1, ply + 1, state, tt, is_pv, thread_id);
        } else {
            score = alpha + 1;
            // LMR: reduce depth for late quiet, non-killer moves
            let mv_kind = mv.kind();
            let is_quiet = mv_kind != MoveKind::Capture && mv_kind != MoveKind::Promotion;
            if depth >= 3 && moves_searched >= 3 && is_quiet {
                let is_killer = state.killers[ply as usize][0] == Some(mv)
                    || state.killers[ply as usize][1] == Some(mv);
                // Don't reduce checks (board state has the move already applied)
                let gives_check = board.in_check();
                if !is_killer && !gives_check {
                    let r: u8 = if moves_searched >= 8 { 3 } else if moves_searched >= 5 { 2 } else { 1 };
                    if depth > r + 1 {
                        let r_depth = depth - 1 - r;
                        score = -alpha_beta(board, -alpha - 1, -alpha, r_depth, ply + 1, state, tt, false, thread_id);
                    }
                }
            }
            if score > alpha {
                score = -alpha_beta(board, -alpha - 1, -alpha, depth - 1, ply + 1, state, tt, false, thread_id);
                if score > alpha && score < beta {
                    score = -alpha_beta(board, -beta, -alpha, depth - 1, ply + 1, state, tt, true, thread_id);
                }
            }
        }

        board.unmake_move(&undo);
        moves_searched += 1;

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
            if score > alpha {
                alpha = score;
                node_type = NodeType::Exact;
                state.pv[ply as usize][ply as usize] = Some(mv);
                for j in (ply + 1) as usize..state.pv_length[(ply + 1) as usize] {
                    state.pv[ply as usize][j] = state.pv[(ply + 1) as usize][j];
                }
                state.pv_length[ply as usize] = state.pv_length[(ply + 1) as usize];
                if score >= beta {
                    node_type = NodeType::LowerBound;

                    // killer move: store quiet move that caused cutoff
                    let k = mv.kind();
                    if k != MoveKind::Capture && k != MoveKind::Promotion {
                        let ki = ply as usize;
                        state.killers[ki][1] = state.killers[ki][0];
                        state.killers[ki][0] = Some(mv);
                        let from = mv.from().index() as usize;
                        let to = mv.to().index() as usize;
                        state.history[from][to] += (depth as i32) * (depth as i32);
                    }

                    break;
                }
            }
        }
        if state.should_stop() { return best_score; }
    }

    if best_move.is_none() {
        return if board.in_check() { -(CHECKMATE - ply as i32) } else { 0 };
    }

    let skip_store = node_type == NodeType::UpperBound && depth <= 1;
    if !skip_store {
        tt.store(hash, best_score, depth, node_type, best_move);
    }
    best_score
}

fn order_moves(
    moves: &mut [Move], board: &Board, hash_move: Option<Move>, ply: u8, state: &SearchState, thread_id: u8,
) {
    let hash = hash_move.unwrap_or(Move::NULL);
    let k0 = state.killers[ply as usize][0].unwrap_or(Move::NULL);
    let k1 = state.killers[ply as usize][1].unwrap_or(Move::NULL);

    moves.sort_by_cached_key(|mv| {
        if *mv == hash { return i32::MAX; }
        let k = mv.kind();
        if k == MoveKind::Capture {
            let see_val = Eval::default().see(board, *mv);
            return if see_val > 0 { 10_000 + see_val }
            else { 2_000 + see_val }; // SEE <= 0: still search, but low priority
        }
        if k == MoveKind::Promotion { return 30_000; }
        if *mv == k0 { return 9_000; }
        if *mv == k1 { return 8_999; }
        // history heuristic for quiets
        let hist = state.history[mv.from().index() as usize][mv.to().index() as usize];
        let perturb = if ply == 0 && thread_id > 0 {
            ((mv.from().index().wrapping_mul(thread_id)) % 16) as i32
        } else { 0 };
        hist.clamp(0, 8_000) + perturb
    });
    moves.reverse();
}

fn quiescence(board: &mut Board, mut alpha: i32, beta: i32, ply: u8, state: &mut SearchState) -> i32 {
    state.nodes += 1;
    if state.should_stop() || ply >= MAX_DEPTH - 1 { return Eval::default().evaluate(board); }

    if let Some(result) = board.check_result() {
        return match result {
            GameResult::Checkmate(_) => -(CHECKMATE - ply as i32),
            GameResult::Stalemate | GameResult::Draw => 0,
        };
    }

    let stand_pat = Eval::default().evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves_buf = [Move::NULL; MAX_MOVES];
    let move_count = movegen::generate_legal_moves(board, &mut moves_buf);
    let side = board.side_to_move();
    let mut filtered = 0;
    for i in 0..move_count {
        let mv = moves_buf[i];
        let k = mv.kind();
        if k == MoveKind::Capture || k == MoveKind::Promotion {
            // pre-filter: only captures and promotions
            let undo = board.make_move(mv);
            let king = board.king_square(side);
            let ok = !board.is_attacked_by(king, board.side_to_move());
            board.unmake_move(&undo);
            if ok {
                moves_buf[filtered] = mv;
                filtered += 1;
            }
        }
    }
    if filtered == 0 { return alpha; }
    order_moves_q(&mut moves_buf[..filtered], board, state);

    for i in 0..filtered {
        let mv = moves_buf[i];
        let undo = board.make_move(mv);
        let king_sq = board.king_square(side);
        if board.is_attacked_by(king_sq, board.side_to_move()) {
            board.unmake_move(&undo);
            continue;
        }
        let score = -quiescence(board, -beta, -alpha, ply + 1, state);
        board.unmake_move(&undo);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
        if state.should_stop() { break; }
    }
    alpha
}

fn order_moves_q(moves: &mut [Move], board: &Board, _state: &SearchState) {
    moves.sort_by_cached_key(|mv| {
        if mv.kind() == MoveKind::Capture {
            let see_val = Eval::default().see(board, *mv);
            if see_val > 0 { 10_000 + see_val }
            else { 2_000 + see_val }
        } else if mv.kind() == MoveKind::Promotion {
            30_000
        } else { 0 }
    });
    moves.reverse();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    fn make_tt() -> Arc<TT> { Arc::new(TT::new(16)) }

    #[test]
    fn test_search_returns_valid_move() {
        let board = Board::from_initial();
        let params = SearchParams::with_depth(2);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.best_move.is_some());
    }

    #[test]
    fn test_search_mat_in_one() {
        let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 2 4";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(3);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.best_move.is_some());
        // Qh5xf7 should score very high (mate in 1 or near-mate)
        assert!(result.score > 1000,
            "Score should be high for mate in 1, got {}", result.score);
    }

    #[test]
    fn test_search_deeper_finds_better_move() {
        let board = Board::from_initial();
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let r1 = search(&board, &SearchParams::with_depth(1), &stop, &tt);
        let r2 = search(&board, &SearchParams::with_depth(3), &stop, &tt);
        assert!(r1.depth <= r2.depth);
    }

    #[test]
    fn test_search_stop_flag_works() {
        let board = Board::from_initial();
        let params = SearchParams::with_depth(20);
        let stop = Arc::new(AtomicBool::new(true));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.best_move.is_some() || result.depth == 0);
    }

    #[test]
    fn test_search_multi_threaded() {
        let board = Board::from_initial();
        let mut params = SearchParams::with_depth(4);
        params.threads = 2;
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.best_move.is_some());
    }

    #[test]
    fn test_shallow_search_with_capture() {
        crate::attack::init_slider_tables();
        // White king on a1, Black queen on b1 (undefended), Black king on h8.
        // White to move: Kxb1 wins the queen.
        let fen = "7k/8/8/8/8/8/8/Kq6 w - -";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(1);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.best_move.is_some(), "Shallow search should return a move");
    }

    #[test]
    fn test_pv_collection_from_startpos() {
        crate::attack::init_slider_tables();
        let board = Board::from_initial();
        let params = SearchParams::with_depth(3);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(!result.pv.is_empty(), "PV should not be empty after depth-3 search");
        assert!(result.pv.len() >= 3, "PV length should be >= 3, got {}", result.pv.len());
        assert_eq!(result.pv[0], result.best_move.unwrap(),
            "First PV move should equal bestmove");
    }

    #[test]
    fn test_scholars_mate_score_and_move() {
        crate::attack::init_slider_tables();
        let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 2 4";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(3);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.score >= 9000, "Expected mate score >= 9000, got {}", result.score);
        assert!(result.best_move.is_some(), "Should have a best move");
        let best = result.best_move.unwrap();
        assert_eq!(best.to_string(), "h5f7",
            "Best move should be Qh5xf7, got {}", best);
    }

    #[test]
    fn test_null_move_smoke() {
        crate::attack::init_slider_tables();
        // Null-move pruning is always enabled when conditions are met.
        // Smoke test: two depth-4 searches from startpos both complete.
        let board = Board::from_initial();
        let params = SearchParams::with_depth(4);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result1 = search(&board, &params, &stop, &tt);
        assert!(result1.best_move.is_some(), "First search should return a move");
        let result2 = search(&board, &params, &stop, &tt);
        assert!(result2.best_move.is_some(), "Second search should also return a move");
    }

    #[test]
    fn test_search_stop_flag_pre_set() {
        crate::attack::init_slider_tables();
        let board = Board::from_initial();
        let params = SearchParams::with_depth(20);
        let stop = Arc::new(AtomicBool::new(true));
        let tt = make_tt();
        let start = std::time::Instant::now();
        let result = search(&board, &params, &stop, &tt);
        let elapsed_ms = start.elapsed().as_millis();
        assert!(elapsed_ms < 5000,
            "Search with pre-set stop flag should finish quickly, took {}ms", elapsed_ms);
        assert!(result.best_move.is_some() || result.depth == 0,
            "Result should be valid with pre-set stop flag");
    }

    #[test]
    fn test_iterative_deepening() {
        crate::attack::init_slider_tables();
        let board = Board::from_initial();
        let params = SearchParams::with_depth(3);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.depth >= 1, "Iterative deepening should reach at least depth 1");
        assert!(result.best_move.is_some(), "Should have a best move");
        assert!(result.nodes > 0, "Should have searched nodes");
        assert!(result.time_ms < 30000, "Search should not take too long");
    }

    #[test]
    fn test_draw_detection_kings_only() {
        crate::attack::init_slider_tables();
        let fen = "k7/8/8/8/8/8/8/K7 w - -";
        let board = Board::from_fen(fen).unwrap();
        assert!(board.check_result().is_some(), "Two kings only should be terminal");
        let params = SearchParams::with_depth(2);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.score.abs() < 100,
            "Score should be close to 0 for two kings, got {}", result.score);
    }

    #[test]
    fn test_qsearch_captures_hanging_piece() {
        crate::attack::init_slider_tables();
        // White king on a1, Black queen on b1 (undefended), Black king on h8.
        // White to move: Kxb1 captures the hanging queen.
        // After capture only kings remain (drawn); before, White is down a queen.
        let fen = "7k/8/8/8/8/8/8/Kq6 w - -";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(1);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt);
        assert!(result.best_move.is_some(), "Should find a move");
        // Capturing the queen is much better than not capturing
        // Score should be significantly better than losing a queen (~ -900)
        assert!(result.score > -500,
            "Score after capturing queen should be much better than -900, got {}",
            result.score);
    }
}
