use blunderchess::attack::init_slider_tables;
use blunderchess::board::Board;
use blunderchess::search::{search, SearchParams, SearchResult};
use blunderchess::thread_pool::ThreadPool;
use blunderchess::tt::TT;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

fn search_position(fen: &str, depth: u8) -> SearchResult {
    init_slider_tables();
    let board = Board::from_fen(fen).expect("valid FEN");
    let params = SearchParams::with_depth(depth);
    let stop = Arc::new(AtomicBool::new(false));
    let tt = Arc::new(TT::new(16));
    search(&board, &params, &stop, &tt, None)
}

fn assert_best_move_in(fen: &str, depth: u8, acceptable: &[&str], label: &str) {
    let result = search_position(fen, depth);
    let best = result.best_move.expect("search should return a move");
    let best_str = format!("{best}");
    assert!(
        acceptable.contains(&best_str.as_str()),
        "{label}: depth {depth} expected one of {acceptable:?}, got {best_str} (score {})",
        result.score
    );
}

fn assert_winning_score(fen: &str, depth: u8, min_score: i32, label: &str) {
    let result = search_position(fen, depth);
    let best_str = result.best_move.map(|m| format!("{m}")).unwrap_or_default();
    assert!(
        result.score >= min_score,
        "{label}: depth {depth} expected score >= {min_score}, got {} (move: {best_str})",
        result.score,
    );
}

// --- Tactical tests: exact best move matching ---

#[test]
fn tactical_mate_in_one_scholars() {
    assert_best_move_in(
        "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 0 1",
        3, &["h5f7"], "Scholar's Mate"
    );
}

#[test]
fn tactical_capture_hanging_queen() {
    assert_best_move_in(
        "rnb1kbnr/pppp1ppp/8/4p3/4P1q1/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1",
        3, &["d1g4"], "Capture hanging queen"
    );
}

#[test]
fn tactical_back_rank_mate() {
    assert_best_move_in(
        "6k1/5ppp/8/8/8/2q5/5PPP/3R2K1 w - - 0 1",
        3, &["d1d8"], "Back rank mate Rd8#"
    );
}

#[test]
fn tactical_promotion_threat() {
    assert_best_move_in(
        "k7/2P5/1K6/8/8/8/8/8 w - - 0 1",
        3, &["c7c8q", "c7c8r", "c7c8b", "c7c8n"], "Promotion"
    );
}

// --- Tactical tests: score thresholds ---

#[test]
fn tactical_mate_in_two_found() {
    assert_winning_score(
        "r1b1kb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 0 1",
        4, 500, "Mate in 2 found"
    );
}

#[test]
fn tactical_smothered_mate_found() {
    // Black is already checkmated (smothered mate). The score for the
    // mated side must be very negative and no legal move exists.
    let result = search_position(
        "6rk/5Npp/8/8/8/8/8/2K5 b - - 0 1", 3);
    assert!(
        result.score <= -9000,
        "Smothered mate: black in checkmate: expected score <= -9000, got {}",
        result.score
    );
    assert!(result.best_move.is_none(),
        "Smothered mate: black in checkmate: expected no legal move, got {:?}",
        result.best_move.map(|m| format!("{m}"))
    );
}

#[test]
fn tactical_discovered_attack_not_losing() {
    assert_winning_score(
        "r1bqk2r/pppp1ppp/2n2n2/2b1p1B1/2B1P3/8/PPPP1PPP/RN1QK1NR w KQkq - 0 1",
        4, -70, "Discovered attack: not losing"
    );
}

#[test]
fn tactical_pin_not_losing() {
    assert_winning_score(
        "r2qkbnr/ppp2ppp/2n5/3pp3/2B1P1b1/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1",
        4, -50, "Pin: not losing"
    );
}

// --- Convergence ---

#[test]
fn depth_convergence_scholars_mate() {
    let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 0 1";
    let r3 = search_position(fen, 3);
    let r5 = search_position(fen, 5);
    assert_eq!(
        r3.best_move, r5.best_move,
        "Scholar's Mate: depth 3 and 5 should agree"
    );
}

// --- Ra8+ skewer debug test ---

#[test]
fn debug_skewer_ra8_is_legal() {
    init_slider_tables();
    let fen = "3qk3/8/8/8/8/8/8/R3K3 w Q - 0 1";
    let board = Board::from_fen(fen).unwrap();
    let mut buf = [blunderchess::types::Move::NULL; 218];
    let count = blunderchess::movegen::generate_legal_moves(&board, &mut buf);
    let has_ra8 = (0..count).any(|i| {
        let mv = buf[i];
        format!("{mv}") == "a1a8"
    });
    assert!(has_ra8, "Ra8+ should be a legal move (found {count} legal moves)");
}

// --- Performance benchmarks (ignored by default) ---

#[test]
#[ignore]
fn bench_nps_vs_depth() {
    init_slider_tables();
    let is_release = !cfg!(debug_assertions);
    if !is_release {
        println!("=== NPS vs Depth [SKIPPED in debug — use --release] ===");
        return;
    }
    let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let tt = Arc::new(TT::new(16));
    let stop = Arc::new(AtomicBool::new(false));
    println!("=== NPS vs Depth (startpos, 1 thread, shared TT) ===");
    for depth in 3..=10 {
        let params = SearchParams::with_depth(depth);
        let start = Instant::now();
        let result = search(&board, &params, &stop, &tt, None);
        let ms = start.elapsed().as_millis() as u64;
        let nps = if ms > 0 { result.total_nodes * 1000 / ms } else { 0 };
        println!("  depth {:2}: {:>8} nodes {:>5}ms {:>8} nps", depth, result.total_nodes, ms, nps);
        if depth <= 5 { assert!(ms < 500, "Depth {depth} too slow: {ms}ms"); }
        if depth >= 6 { assert!(nps >= 100_000, "Depth {depth} NPS too low: {nps}"); }
    }
}

#[test]
#[ignore]
fn bench_thread_scaling() {
    init_slider_tables();
    let is_release = !cfg!(debug_assertions);
    if !is_release {
        println!("=== Thread Scaling (startpos, depth 8) [SKIPPED in debug] ===");
        return;
    }
    let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let pool = ThreadPool::new(16);
    println!("=== Thread Scaling (startpos, depth 8, TT scaled 8MB × threads, fresh per run) ===");
    for threads in [1, 2, 4, 8, 16] {
        let mut params = SearchParams::with_depth(8);
        params.threads = threads;
        let tt_mb = 8 * threads as usize;
        let stop = Arc::new(AtomicBool::new(false));
        let tt = Arc::new(TT::new(tt_mb));
        let start = Instant::now();
        let result = search(&board, &params, &stop, &tt, Some(&pool));
        let ms = start.elapsed().as_millis() as u64;
        let nps = if ms > 0 { result.total_nodes * 1000 / ms } else { 0 };
        let best_str = result.best_move.map(|m| format!("{m}")).unwrap_or_default();
        println!(
            "  t{:>2}: {:>2}MB TT  total={:>8} nodes {:>5}ms  total_nps={:>8}  best={}",
            threads, tt_mb, result.total_nodes, ms, nps, best_str,
        );
    }
}

#[test]
fn tactical_avoid_pawn_fork() {
    let fen = "r3k2r/pp2qpp1/2n1b2p/2Ppp3/8/2PBP3/P1P2PPP/1R2QKNR w kq - 2 13";
    let mut avoided = false;
    for depth in 4..=7 {
        let result = search_position(fen, depth);
        let best = result.best_move.map(|m| format!("{m}")).unwrap_or_default();
        if best != "g1f3" {
            avoided = true;
            break;
        }
    }
    assert!(avoided, "Engine should avoid Nf3 (walks into e4 pawn fork) by depth 7");
}

#[test]
fn tactical_avoid_queen_trap() {
    let fen = "rnb1k2r/pp2pp1p/3p1np1/3P4/2B1P3/2qQ1N2/P1P2PPP/R1B2RK1 b kq - 1 10";
    let mut reasonable = false;
    for depth in 4..=7 {
        let result = search_position(fen, depth);
        let score = result.score;
        let best = result.best_move.map(|m| format!("{m}")).unwrap_or_default();
        // Qxa1 can be playable (queen for two rooks), but score must not indicate a huge blunder
        if best == "c3a1" {
            assert!(score.abs() < 200, "Qxa1 at depth {depth}: score {score} indicates blunder");
        }
        if score > -200 {
            reasonable = true;
        }
    }
    assert!(reasonable, "Engine score should be reasonable (not catastrophic) in queen trap position");
}

#[test]
#[ignore]
fn bench_deep_thread_scaling() {
    init_slider_tables();
    let is_release = !cfg!(debug_assertions);
    if !is_release {
        println!("=== Deep Thread Scaling [SKIPPED in debug — use --release] ===");
        return;
    }
    let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let pool = ThreadPool::new(16);
    for depth in [10u8, 12] {
        for threads in [1u8, 16] {
            let tt_mb = 8 * threads as usize;
            let mut params = SearchParams::with_depth(depth);
            params.threads = threads;
            let stop = Arc::new(AtomicBool::new(false));
            let tt = Arc::new(TT::new(tt_mb));
            let start = Instant::now();
            let result = search(&board, &params, &stop, &tt, Some(&pool));
            let ms = start.elapsed().as_millis() as u64;
            let nps = if ms > 0 { result.total_nodes * 1000 / ms } else { 0 };
            println!("  depth {}  t{:>2}: {:>3}MB TT  total={:>10} nodes {:>6}ms  total_nps={:>10}  best={:?}",
                depth, threads, tt_mb, result.total_nodes, ms, nps,
                result.best_move.map(|m| format!("{m}")));
        }
    }
}

#[test]
#[ignore]
fn bench_perft_speed() {
    use blunderchess::movegen::perft;
    init_slider_tables();
    let is_release = !cfg!(debug_assertions);
    if !is_release {
        println!("=== Perft Speed (kiwipete) [SKIPPED in debug] ===");
        return;
    }
    let board = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    for depth in 1..=3 {
        let start = Instant::now();
        let nodes = perft(&board, depth);
        let ms = start.elapsed().as_millis() as u64;
        let nps = if ms > 0 { nodes * 1000 / ms } else { 0 };
        println!("  perft {depth}: {:>8} nodes {:>5}ms {:>8} nps", nodes, ms, nps);
        assert_eq!(nodes, [48, 2039, 97862][depth as usize - 1]);
    }
}
