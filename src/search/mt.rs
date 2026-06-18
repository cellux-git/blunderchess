use crate::board::{Board, MAX_MOVES};
use crate::movegen;
use crate::search::params::{SearchParams, SearchResult};
use crate::search::worker::search_worker;
use crate::thread_pool::ThreadPool;
use crate::tt::TT;
use crate::types::Move;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Instant;

pub fn search(
    board: &Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
    pool: Option<&ThreadPool>,
) -> SearchResult {
    if params.threads.max(1) == 1 {
        return search_single(board, params, stop, tt, 0);
    }
    search_mt(board, params, stop, tt, pool)
}

pub(crate) fn search_single(
    board: &Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
    thread_id: u8,
) -> SearchResult {
    let mut b = board.clone();
    let mut result = search_worker(&mut b, params, stop, tt, thread_id);
    result.total_nodes = result.nodes;
    result
}

pub(crate) fn search_mt(
    board: &Board,
    params: &SearchParams,
    stop: &Arc<AtomicBool>,
    tt: &Arc<TT>,
    pool: Option<&ThreadPool>,
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
        best_move: None, score: 0, depth: 0, pv: Vec::new(), nodes: 0, total_nodes: 0, time_ms: 0, multi_pv_lines: Vec::new(),
    }));
    let total_nodes = Arc::new(AtomicU64::new(0));

    if let Some(pool) = pool.filter(|p| p.size() >= num_threads) {
        let barrier = Arc::new(Barrier::new(num_threads + 1));
        let mut jobs: Vec<Box<dyn FnOnce() + Send + 'static>> = Vec::with_capacity(num_threads);

        for tid in 0..num_threads as u8 {
            let mut b = board.clone();
            let p = params.clone();
            let real_stop = if tid == 0 { Arc::clone(&stop) } else { Arc::new(AtomicBool::new(false)) };
            let tt = tt.clone();
            let best = Arc::clone(&best_result);
            let bar = Arc::clone(&barrier);
            let tn = Arc::clone(&total_nodes);

            jobs.push(Box::new(move || {
                let result = search_worker(&mut b, &p, &real_stop, &tt, tid);
                tn.fetch_add(result.nodes, Ordering::Relaxed);
                {
                    let mut best = best.lock().unwrap();
                    if result.depth > best.depth || (result.depth == best.depth && result.best_move.is_some()) {
                        *best = result;
                    }
                }
                bar.wait();
            }));
        }

        pool.execute_batch(jobs);
        barrier.wait();
    } else {
        let mut handles = Vec::with_capacity(num_threads);

        for tid in 0..num_threads as u8 {
            let mut b = board.clone();
            let p = params.clone();
            let real_stop = if tid == 0 { Arc::clone(&stop) } else { Arc::new(AtomicBool::new(false)) };
            let tt = tt.clone();
            let best = Arc::clone(&best_result);
            let tn = Arc::clone(&total_nodes);

            let handle = std::thread::spawn(move || {
                let result = search_worker(&mut b, &p, &real_stop, &tt, tid);
                tn.fetch_add(result.nodes, Ordering::Relaxed);
                let mut best = best.lock().unwrap();
                if result.depth > best.depth || (result.depth == best.depth && result.best_move.is_some()) {
                    *best = result;
                }
            });
            handles.push(handle);
        }

        for handle in handles { let _ = handle.join(); }
    }

    let mut result = best_result.lock().unwrap().clone();
    result.total_nodes = total_nodes.load(Ordering::Relaxed);
    if result.nodes == 0 { result.nodes = 1; }
    if result.total_nodes == 0 { result.total_nodes = result.nodes; }
    result.time_ms = start.elapsed().as_millis() as u64;
    if result.best_move.is_none() {
        let mut buf = [Move::NULL; MAX_MOVES];
        let count = movegen::generate_legal_moves(&board, &mut buf);
        result.best_move = if count > 0 { Some(buf[0]) } else { None };
    }
    result
}
