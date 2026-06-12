use crate::board::Board;
use crate::book::Book;
use crate::search::{SearchParams, SearchResult};
use crate::tt::TT;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Engine {
    pub board: Board,
    pub tt: Arc<TT>,
    pub stop_flag: Arc<AtomicBool>,
    pub search_handles: Vec<std::thread::JoinHandle<SearchResult>>,
    pub multi_pv: u8,
    pub pondering: bool,
    pub ponderhit_received: Arc<AtomicBool>,
    pub book: Option<Book>,
    pub own_book: bool,
}

impl Engine {
    pub fn new() -> Engine {
        let tt = TT::new(64);
        Engine {
            board: Board::from_initial(),
            tt: Arc::new(tt),
            stop_flag: Arc::new(AtomicBool::new(false)),
            search_handles: Vec::new(),
            multi_pv: 1,
            pondering: false,
            ponderhit_received: Arc::new(AtomicBool::new(false)),
            book: None,
            own_book: false,
        }
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
        println!("id author Zsolt Herpai");
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
        let mut i = 0;
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

        if params.depth.is_none() && params.movetime.is_none() && !params.infinite {
            params.depth = Some(6);
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

        if params.infinite || params.movetime.is_some() {
            self.start_async_search(params);
        } else {
            let stop = Arc::new(AtomicBool::new(false));
            let result = crate::search::search(&self.board, &params, &stop, &self.tt);
            self.report_result(&result);
        }
    }

    fn start_async_search(&mut self, params: SearchParams) {
        self.stop_flag.store(false, Ordering::SeqCst);

        let board = self.board.clone();
        let tt = self.tt.clone();
        let stop = self.stop_flag.clone();
        let movetime = params.movetime;
        let pondering = self.pondering;
        let ponderhit = self.ponderhit_received.clone();

        let handle = std::thread::spawn(move || {
            let result = crate::search::search(&board, &params, &stop, &tt);
            if pondering && !ponderhit.load(Ordering::SeqCst) {
                // Ponder search — opponent didn't play predicted move. Discard.
                return SearchResult {
                    best_move: None, score: 0, depth: 0, pv: Vec::new(),
                    nodes: 0, time_ms: 0, multi_pv_lines: Vec::new(),
                };
            }
            result
        });

        self.search_handles.push(handle);

        // Don't set movetime timer when pondering (we wait for ponderhit/stop)
        if !pondering {
            if let Some(limit) = movetime {
                let stop = self.stop_flag.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(limit + 50));
                    stop.store(true, Ordering::SeqCst);
                });
            }
        }
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
                    if i + 3 < args.len() && args[i + 1] == "MultiPV" && args[i + 2] == "value" {
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
    use crate::types::Square;

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
        assert_eq!(engine.board.side_to_move, initial.side_to_move);
        assert_eq!(engine.board.castling_rights, initial.castling_rights);
        assert_eq!(engine.board.en_passant, initial.en_passant);
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
        let ep = engine.board.en_passant;
        assert!(ep.is_some(), "En passant should be set");
        assert_eq!(ep.unwrap(), Square::from_file_rank(4, 2).unwrap()); // e3
    }

    #[test]
    fn test_go_depth_returns_bestmove() {
        let mut engine = Engine::new();
        assert!(engine.process_command("position startpos"));
        let params = SearchParams::with_depth(5);
        let stop = Arc::new(AtomicBool::new(false));
        let result = crate::search::search(&engine.board, &params, &stop, &engine.tt);
        assert!(result.best_move.is_some(), "go depth 5 should return a bestmove");
    }

    #[test]
    fn test_parse_uci_invalid_input() {
        let board = Board::from_initial();
        assert!(parse_uci_move(&board, "").is_none());
        assert!(parse_uci_move(&board, "abc").is_none());
        assert!(parse_uci_move(&board, "e2e9").is_none()); // rank 9 invalid
        assert!(parse_uci_move(&board, "i2e4").is_none()); // file i invalid
    }
}
