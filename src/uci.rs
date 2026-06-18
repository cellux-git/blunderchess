use crate::board::Board;
use crate::book::Book;
use crate::search::{SearchParams, SearchResult};
use crate::thread_pool::ThreadPool;
use crate::tt::TT;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Engine {
    board: Board,
    tt: Arc<TT>,
    pool: Arc<ThreadPool>,
    stop_flag: Arc<AtomicBool>,
    search_handles: Vec<std::thread::JoinHandle<SearchResult>>,
    multi_pv: u8,
    pondering: bool,
    ponderhit_received: Arc<AtomicBool>,
    book: Option<Book>,
    own_book: bool,
    threads: u8,
    hash_size: usize,
}

impl Engine {
    pub fn new() -> Engine {
        let hash_size = 64;
        let tt = TT::new(hash_size);
        let default_threads = std::thread::available_parallelism()
            .map(|n| n.get().max(1))
            .unwrap_or(4);
        Engine {
            board: Board::from_initial(),
            tt: Arc::new(tt),
            pool: Arc::new(ThreadPool::new(default_threads)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            search_handles: Vec::new(),
            multi_pv: 1,
            pondering: false,
            ponderhit_received: Arc::new(AtomicBool::new(false)),
            book: None,
            own_book: false,
            threads: 1,
            hash_size,
        }
    }

    pub fn search_position(&self, board: &Board, depth: u8) -> SearchResult {
        let params = SearchParams::with_depth(depth);
        let stop = Arc::new(AtomicBool::new(false));
        crate::search::search(board, &params, &stop, &self.tt, Some(&self.pool))
    }

    pub fn process_command(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return true;
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = tokens[0];

        match cmd {
            "uci" => self.cmd_uci(),
            "isready" => self.cmd_isready(),
            "ucinewgame" => self.cmd_ucinewgame(),
            "position" => self.cmd_position(&tokens[1..]),
            "go" => self.cmd_go(&tokens[1..]),
            "stop" => self.cmd_stop(),
            "quit" => {
                self.cmd_stop();
                return false;
            }
            "ponderhit" => self.cmd_ponderhit(),
            "setoption" => self.cmd_setoption(&tokens[1..]),
            _ => {
                log::debug!("unknown command: {trimmed}");
            }
        }

        true
    }

    fn cmd_uci(&self) {
        println!("id name BlunderChess 0.1.0");
        println!("id author Cellux");
        println!("option name Hash type spin default 64 min 1 max 65536");
        println!("option name MultiPV type spin default 1 min 1 max 64");
        println!("option name OwnBook type check default false");
        println!("option name BookFile type string default <empty>");
        println!("option name Threads type spin default 1 min 1 max 255");
        println!("uciok");
    }

    fn cmd_isready(&self) {
        println!("readyok");
    }

    fn cmd_ucinewgame(&mut self) {
        if let Some(tt) = Arc::get_mut(&mut self.tt) {
            tt.new_game();
        }
    }

    fn cmd_position(&mut self, args: &[&str]) {
        if args.is_empty() {
            return;
        }

        let mut idx = 0;

        if args[0] == "startpos" {
            self.board = Board::from_initial();
            idx = 1;
        } else if args[0] == "fen" {
            let mut fen_parts = Vec::new();
            for i in 1..args.len() {
                if args[i] == "moves" {
                    break;
                }
                fen_parts.push(args[i]);
                idx = i;
            }
            let fen = fen_parts.join(" ");
            match Board::from_fen(&fen) {
                Ok(b) => {
                    self.board = b;
                    idx += 1;
                }
                Err(e) => {
                    log::error!("Invalid FEN: {e}");
                    return;
                }
            }
        }

        if args.len() > idx && args[idx] == "moves" {
            for &mv_str in &args[idx + 1..] {
                if let Some(mv) = parse_uci_move(&self.board, mv_str) {
                    self.board.make_move(mv);
                } else {
                    log::error!("Unknown move: {mv_str}");
                    break;
                }
            }
        }
    }

    fn cmd_go(&mut self, args: &[&str]) {
        if args.first() == Some(&"ponder") {
            self.pondering = true;
            self.ponderhit_received.store(false, Ordering::SeqCst);
            let args = &args[1..];
            self.cmd_go_impl(args);
        } else {
            self.pondering = false;
            self.cmd_go_impl(args);
        }
    }

    fn cmd_go_impl(&mut self, args: &[&str]) {
        self.cmd_stop();

        let mut params = SearchParams::new();
        params.multi_pv = self.multi_pv;
        params.ponder = self.pondering;
        params.threads = self.threads;
        let mut i = 0;
        let mut wtime: Option<u64> = None;
        let mut btime: Option<u64> = None;
        let mut winc: Option<u64> = None;
        let mut binc: Option<u64> = None;
        let mut movestogo: Option<u8> = None;
        while i < args.len() {
            match args[i] {
                "depth" => {
                    if i + 1 < args.len() {
                        if let Ok(d) = args[i + 1].parse::<u8>() {
                            params.depth = Some(d);
                        }
                        i += 1;
                    }
                }
                "movetime" => {
                    if i + 1 < args.len() {
                        if let Ok(ms) = args[i + 1].parse::<u64>() {
                            params.movetime = Some(ms);
                        }
                        i += 1;
                    }
                }
                "wtime" => {
                    if i + 1 < args.len() {
                        if let Ok(ms) = args[i + 1].parse::<u64>() { wtime = Some(ms); }
                        i += 1;
                    }
                }
                "btime" => {
                    if i + 1 < args.len() {
                        if let Ok(ms) = args[i + 1].parse::<u64>() { btime = Some(ms); }
                        i += 1;
                    }
                }
                "winc" => {
                    if i + 1 < args.len() {
                        if let Ok(ms) = args[i + 1].parse::<u64>() { winc = Some(ms); }
                        i += 1;
                    }
                }
                "binc" => {
                    if i + 1 < args.len() {
                        if let Ok(ms) = args[i + 1].parse::<u64>() { binc = Some(ms); }
                        i += 1;
                    }
                }
                "movestogo" => {
                    if i + 1 < args.len() {
                        if let Ok(n) = args[i + 1].parse::<u8>() { movestogo = Some(n); }
                        i += 1;
                    }
                }
                "infinite" => {
                    params.infinite = true;
                }
                "threads" => {
                    if i + 1 < args.len() {
                        if let Ok(n) = args[i + 1].parse::<u8>() {
                            params.threads = n.max(1);
                        }
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        // If no explicit movetime/depth, compute from time control
        if params.movetime.is_none() && params.depth.is_none() && !params.infinite {
            params.movetime = crate::time_control::allocate_time(
                self.board.side_to_move(),
                wtime, btime, winc, binc, movestogo,
            );
            if params.movetime.is_none() {
                params.depth = Some(6); // fallback when no time info at all
            }
        }

        if self.pondering {
            params.infinite = true;
            params.movetime = None;
        }

        // Book probe — if enabled and a move is found, play it immediately
        if self.own_book {
            if let Some(ref book) = self.book {
                if let Some(bm) = book.probe(&self.board) {
                    println!("info string book");
                    println!("bestmove {bm}");
                    return;
                }
            }
        }

        let pool_size = self.pool.size();
        params.threads = params.threads.min(pool_size as u8);

        if params.infinite || self.pondering {
            self.start_async_search(params);
        } else {
            let stop = Arc::new(AtomicBool::new(false));
            let result = crate::search::search(&self.board, &params, &stop, &self.tt, Some(&self.pool));
            self.report_result(&result);
        }
    }

    fn start_async_search(&mut self, params: SearchParams) {
        self.stop_flag.store(false, Ordering::SeqCst);

        let board = self.board.clone();
        let tt = self.tt.clone();
        let stop = self.stop_flag.clone();
        let pool = self.pool.clone();
        let pondering = self.pondering;
        let ponderhit = self.ponderhit_received.clone();

        let handle = std::thread::spawn(move || {
            let result = crate::search::search(&board, &params, &stop, &tt, Some(&pool));
            if pondering && !ponderhit.load(Ordering::SeqCst) {
                return SearchResult {
                    best_move: None, score: 0, depth: 0, pv: Vec::new(),
                    nodes: 0, total_nodes: 0, time_ms: 0, multi_pv_lines: Vec::new(),
                };
            }
            result
        });

        self.search_handles.push(handle);
    }

    fn cmd_stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        let results: Vec<SearchResult> = self.search_handles.drain(..)
            .filter_map(|h| h.join().ok())
            .collect();
        for result in &results {
            if result.best_move.is_some() {
                self.report_result(result);
            }
        }
        self.pondering = false;
    }

    fn cmd_ponderhit(&mut self) {
        self.ponderhit_received.store(true, Ordering::SeqCst);
        self.pondering = false;
    }

    fn cmd_setoption(&mut self, args: &[&str]) {
        let args: Vec<&str> = args.iter().copied().collect();
        let mut i = 0;
        while i < args.len() {
            match args.get(i).copied() {
                Some("name") => {
                    if i + 3 < args.len() && args[i + 1] == "Hash" && args[i + 2] == "value" {
                        if let Ok(n) = args[i + 3].parse::<usize>() {
                            let new_size = n.max(1).min(65536);
                            if new_size != self.hash_size {
                                self.hash_size = new_size;
                                self.tt = Arc::new(TT::new(new_size));
                            }
                        }
                        i += 3;
                    } else if i + 3 < args.len() && args[i + 1] == "MultiPV" && args[i + 2] == "value" {
                        if let Ok(n) = args[i + 3].parse::<u8>() {
                            self.multi_pv = n.max(1).min(64);
                        }
                        i += 3;
                    } else if i + 3 < args.len() && args[i + 1] == "BookFile" && args[i + 2] == "value" {
                        let path = args[i + 3];
                        match Book::load(path) {
                            Ok(book) => {
                                log::info!("Loaded book: {path} ({len} entries)", len = book.entry_count());
                                self.book = Some(book);
                            }
                            Err(e) => log::error!("Failed to load book: {e}"),
                        }
                        i += 3;
                    } else if i + 3 < args.len() && args[i + 1] == "OwnBook" && args[i + 2] == "value" {
                        self.own_book = args[i + 3] == "true";
                        i += 3;
                    } else if i + 3 < args.len() && args[i + 1] == "Threads" && args[i + 2] == "value" {
                        if let Ok(n) = args[i + 3].parse::<u8>() {
                            self.threads = n.max(1);
                        }
                        i += 3;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn report_result(&self, result: &SearchResult) {
        if let Some(best) = &result.best_move {
            if result.multi_pv_lines.is_empty() {
                let pv_str: Vec<String> = result.pv.iter().map(|m| m.to_string()).collect();
                println!(
                    "info depth {} score cp {} nodes {} time {} pv {}",
                    result.depth,
                    result.score,
                    result.nodes,
                    result.time_ms,
                    pv_str.join(" ")
                );
            } else {
                for (mpv, score, pv) in &result.multi_pv_lines {
                    let pv_str: Vec<String> = pv.iter().map(|m| m.to_string()).collect();
                    println!(
                        "info depth {} multipv {} score cp {} nodes {} time {} pv {}",
                        result.depth,
                        mpv,
                        score,
                        result.nodes,
                        result.time_ms,
                        pv_str.join(" ")
                    );
                }
            }
            println!("bestmove {best}");
        } else {
            println!("bestmove 0000");
        }
    }
}

pub fn parse_uci_move(board: &Board, s: &str) -> Option<crate::types::Move> {
    if s.len() < 4 {
        return None;
    }

    let from_file = s.as_bytes()[0].wrapping_sub(b'a');
    let from_rank = s.as_bytes()[1].wrapping_sub(b'1');
    let to_file = s.as_bytes()[2].wrapping_sub(b'a');
    let to_rank = s.as_bytes()[3].wrapping_sub(b'1');

    if from_file > 7 || from_rank > 7 || to_file > 7 || to_rank > 7 {
        return None;
    }

    let from = crate::types::Square::from_file_rank(from_file, from_rank)?;
    let to = crate::types::Square::from_file_rank(to_file, to_rank)?;

    let promotion = if s.len() == 5 {
        match s.as_bytes()[4] {
            b'n' => Some(crate::types::Piece::Knight),
            b'b' => Some(crate::types::Piece::Bishop),
            b'r' => Some(crate::types::Piece::Rook),
            b'q' => Some(crate::types::Piece::Queen),
            _ => None,
        }
    } else {
        None
    };

    let moves = crate::movegen::generate_legal_vec(board);
    for mv in moves {
        if mv.from() == from &&     mv.to() == to && mv.promotion_piece() == promotion {
            return Some(mv);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::movegen;
    use crate::types::{Color, Square};

    #[test]
    fn test_parse_uci_move_roundtrip() {
        let board = Board::from_initial();
        let moves = movegen::generate_legal_vec(&board);
        assert!(!moves.is_empty(), "Startpos should have legal moves");
        for mv in moves {
            let uci = mv.to_string();
            let parsed = parse_uci_move(&board, &uci);
            assert!(parsed.is_some(), "Failed to parse UCI string '{}'", uci);
            assert_eq!(parsed.unwrap(), mv, "Roundtrip mismatch for '{}'", uci);
        }
    }

    #[test]
    fn test_position_startpos() {
        let mut engine = Engine::new();
        assert!(engine.process_command("position startpos"));
        let initial = Board::from_initial();
        assert_eq!(engine.board.side_to_move(), initial.side_to_move());
        assert_eq!(engine.board.castling_rights(), initial.castling_rights());
        assert_eq!(engine.board.en_passant(), initial.en_passant());
    }

    #[test]
    fn test_position_startpos_with_moves() {
        let mut engine = Engine::new();
        assert!(engine.process_command("position startpos moves e2e4 e7e5"));
        let e4 = Square::from_file_rank(4, 3).unwrap(); // e4
        let e5 = Square::from_file_rank(4, 4).unwrap(); // e5
        assert!(engine.board.piece_at(e4).is_some(), "e4 should have a piece");
        assert!(engine.board.piece_at(e5).is_some(), "e5 should have a piece");
    }

    #[test]
    fn test_position_fen_en_passant() {
        let mut engine = Engine::new();
        assert!(engine.process_command("position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"));
        let ep = engine.board.en_passant();
        assert!(ep.is_some(), "En passant should be set");
        assert_eq!(ep.unwrap(), Square::from_file_rank(4, 2).unwrap()); // e3
    }

    #[test]
    fn test_go_depth_returns_bestmove() {
        let mut engine = Engine::new();
        assert!(engine.process_command("position startpos"));
        let params = SearchParams::with_depth(5);
        let stop = Arc::new(AtomicBool::new(false));
        let result = crate::search::search(&engine.board, &params, &stop, &engine.tt, None);
        assert!(result.best_move.is_some(), "go depth 5 should return a bestmove");
    }

    #[test]
    fn test_go_movetime_from_clock() {
        // Verify time allocation produces a movetime, and search with it works
        let mut engine = Engine::new();
        assert!(engine.process_command("position startpos"));
        // Fast test: use a tiny clock to get a short movetime
        let t = crate::time_control::allocate_time(
            engine.board.side_to_move(),
            Some(1000), Some(1000),  // 1s left
            Some(0), Some(0),
            Some(1),  // 1 move to go — uses most of the 1s
        );
        assert!(t.is_some(), "should compute a movetime from clock");
        let ms = t.unwrap();
        assert!(ms >= 10, "should allocate at least 10ms");
        assert!(ms <= 250, "with 1s left, cap is 250ms (1/4 of remaining)");
        let mut params = SearchParams::new();
        params.movetime = Some(ms);
        let stop = Arc::new(AtomicBool::new(false));
        let start = std::time::Instant::now();
        let result = crate::search::search(&engine.board, &params, &stop, &engine.tt, None);
        let elapsed = start.elapsed().as_millis() as u64;
        assert!(result.best_move.is_some(), "search should return a bestmove");
        // Search stops between depth iterations, so elapsed can overshoot movetime.
        // Just verify it didn't run forever.
        assert!(elapsed < 10_000, "search took {elapsed}ms, should stop within 10s");
    }

    #[test]
    fn test_parse_uci_invalid_input() {
        let board = Board::from_initial();
        assert!(parse_uci_move(&board, "").is_none());
        assert!(parse_uci_move(&board, "abc").is_none());
        assert!(parse_uci_move(&board, "e2e9").is_none()); // rank 9 invalid
        assert!(parse_uci_move(&board, "i2e4").is_none()); // file i invalid
    }

    #[test]
    fn test_allocate_time() {
        // 5min + 3s inc, 40 movestogo → ~10.5s, capped at 30s
        let t = crate::time_control::allocate_time(Color::White, Some(300_000), None, Some(3_000), None, Some(40));
        assert!(t.is_some());
        let t = t.unwrap();
        assert!(t >= 5_000, "expected at least 5s, got {t}ms");
        assert!(t <= 30_000, "should not exceed 30s absolute cap, got {t}ms");
    }

    #[test]
    fn test_allocate_time_no_data() {
        // no time info → None
        let t = crate::time_control::allocate_time(Color::White, None, None, None, None, None);
        assert!(t.is_none());
    }

    #[test]
    fn test_allocate_time_black() {
        // black's clock: 2min + 2s inc, default movestogo=30
        let t = crate::time_control::allocate_time(Color::Black, None, Some(120_000), None, Some(2_000), None);
        assert!(t.is_some());
        let t = t.unwrap();
        assert!(t >= 3_000 && t <= 30_000, "got {t}ms");
    }

    #[test]
    fn test_allocate_time_small_remaining() {
        // 200ms left, 0 inc, default movestogo=30 → minimum 10ms
        let t = crate::time_control::allocate_time(Color::White, Some(200), None, Some(0), None, None);
        assert!(t.is_some());
        assert_eq!(t.unwrap(), 10, "should return minimum 10ms");
    }

    #[test]
    fn test_allocate_time_cap_at_quarter() {
        // 2min remaining, movestogo=1 → base=120k, capped at min(30k, 30k) = 30k
        let t = crate::time_control::allocate_time(Color::White, Some(120_000), None, Some(0), None, Some(1));
        assert!(t.is_some());
        assert_eq!(t.unwrap(), 30_000);
    }

    #[test]
    fn test_allocate_time_wrong_color() {
        // white to move but only btime provided → None
        let t = crate::time_control::allocate_time(Color::White, None, Some(120_000), None, None, None);
        assert!(t.is_none(), "white needs wtime, not btime");
    }
}
