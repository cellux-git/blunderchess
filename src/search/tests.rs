#[cfg(test)]
mod tests {
    use crate::board::Board;
    use crate::search::params::{SearchParams, CHECKMATE};
    use crate::search::mt::search;
    use crate::tt::TT;
    use crate::types::{Color, Square};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn make_tt() -> Arc<TT> { Arc::new(TT::new(16)) }

    #[test]
    fn test_search_returns_valid_move() {
        let board = Board::from_initial();
        let params = SearchParams::with_depth(2);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.best_move.is_some());
    }

    #[test]
    fn test_search_mat_in_one() {
        let fen = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 2 4";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(3);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.best_move.is_some());
        assert!(result.score > 1000,
            "Score should be high for mate in 1, got {}", result.score);
    }

    #[test]
    fn test_search_deeper_finds_better_move() {
        let board = Board::from_initial();
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let r1 = search(&board, &SearchParams::with_depth(1), &stop, &tt, None);
        let r2 = search(&board, &SearchParams::with_depth(3), &stop, &tt, None);
        assert!(r1.depth <= r2.depth);
    }

    #[test]
    fn test_search_stop_flag_works() {
        let board = Board::from_initial();
        let params = SearchParams::with_depth(20);
        let stop = Arc::new(AtomicBool::new(true));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.best_move.is_some() || result.depth == 0);
    }

    #[test]
    fn test_search_multi_threaded() {
        let board = Board::from_initial();
        let mut params = SearchParams::with_depth(4);
        params.threads = 2;
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.best_move.is_some());
    }

    #[test]
    fn test_shallow_search_with_capture() {
        crate::attack::init_slider_tables();
        let fen = "7k/8/8/8/8/8/8/Kq6 w - -";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(1);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.best_move.is_some(), "Shallow search should return a move");
    }

    #[test]
    fn test_pv_collection_from_startpos() {
        crate::attack::init_slider_tables();
        let board = Board::from_initial();
        let params = SearchParams::with_depth(3);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
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
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.score >= 9000, "Expected mate score >= 9000, got {}", result.score);
        assert!(result.best_move.is_some(), "Should have a best move");
        let best = result.best_move.unwrap();
        assert_eq!(best.to_string(), "h5f7",
            "Best move should be Qh5xf7, got {}", best);
    }

    #[test]
    fn test_null_move_smoke() {
        crate::attack::init_slider_tables();
        let board = Board::from_initial();
        let params = SearchParams::with_depth(4);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result1 = search(&board, &params, &stop, &tt, None);
        assert!(result1.best_move.is_some(), "First search should return a move");
        let result2 = search(&board, &params, &stop, &tt, None);
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
        let result = search(&board, &params, &stop, &tt, None);
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
        let result = search(&board, &params, &stop, &tt, None);
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
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.score.abs() < 100,
            "Score should be close to 0 for two kings, got {}", result.score);
    }

    #[test]
    fn test_qsearch_captures_hanging_piece() {
        crate::attack::init_slider_tables();
        let fen = "7k/8/8/8/8/8/8/Kq6 w - -";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(1);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.best_move.is_some(), "Should find a move");
        assert!(result.score > -500,
            "Score after capturing queen should be much better than -900, got {}",
            result.score);
    }

    #[test]
    fn test_search_hanging_position() {
        crate::attack::init_slider_tables();
        let fen = "r1b1k2r/pp1p1ppp/1qn1pn2/8/1bPN4/2N1P1P1/PPQ2P1P/R1B1KB1R b KQkq - 2 8";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(4);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(result.best_move.is_some(), "search should return a move");
    }

    #[test]
    fn test_avoid_allowing_mate_in_one() {
        crate::attack::init_slider_tables();
        let fen = "5r1k/p1Q5/1p1p3r/6p1/2P1B3/6P1/7P/2R3K1 b - - 2 38";
        let board = Board::from_fen(fen).unwrap();
        for depth in 1..=5 {
            let params = SearchParams::with_depth(depth);
            let stop = Arc::new(AtomicBool::new(false));
            let tt = make_tt();
            let result = search(&board, &params, &stop, &tt, None);
            if let Some(bm) = result.best_move {
                let mv_str = bm.to_string();
                assert_ne!(
                    mv_str, "h6e6",
                    "depth {depth}: engine should not play Re6 (allows Qh7#)"
                );
                assert!(
                    result.score > -(CHECKMATE - 100),
                    "depth {depth}: score {} indicates engine sees forced mate",
                    result.score
                );
            }
        }
    }

    #[test]
    fn test_checkmate_score_negative_for_mated_side() {
        crate::attack::init_slider_tables();
        let fen = "6rk/5Npp/8/8/8/8/8/2K5 b - - 0 1";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(1);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(
            result.score <= -(CHECKMATE - 50),
            "Checkmated side must have score <= -MATE; got {}",
            result.score
        );
        assert!(
            result.best_move.is_none(),
            "Checkmated side must have no legal move"
        );
    }

    #[test]
    fn test_quiescence_detects_quiet_mate() {
        crate::attack::init_slider_tables();
        let fen = "5r1k/p1Q5/1p1p4/6p1/2P1B3/4r1P1/7P/2R3K1 w - - 0 39";
        let board = Board::from_fen(fen).unwrap();
        let params = SearchParams::with_depth(1);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        assert!(
            result.score >= CHECKMATE - 50,
            "Quiet mate Qh7# must score near +MATE at depth 1; got {} (best={})",
            result.score,
            result.best_move.map(|m| m.to_string()).unwrap_or_default()
        );
        assert_eq!(
            result.best_move.map(|m| m.to_string()),
            Some("c7h7".to_string()),
            "Best move must be Qh7#"
        );
    }

    #[test]
    fn test_alpha_beta_fallthrough_matches_early_exit() {
        crate::attack::init_slider_tables();
        let fen = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";
        let board = Board::from_fen(fen).unwrap();
        let stop = Arc::new(AtomicBool::new(false));

        let r2 = search(&board, &SearchParams::with_depth(2), &stop, &make_tt(), None);
        let r1 = search(&board, &SearchParams::with_depth(1), &stop, &make_tt(), None);

        assert!(
            r1.score <= -(CHECKMATE - 50) && r2.score <= -(CHECKMATE - 50),
            "Both depth 1 (QS) and depth 2 (early-exit) must score near -MATE; got {} and {}",
            r1.score, r2.score
        );
        assert!(
            r1.best_move.is_none() && r2.best_move.is_none(),
            "Checkmated side must have no legal move at any depth"
        );
    }

    #[test]
    fn test_search_hanging_position_movetime() {
        crate::attack::init_slider_tables();
        let fen = "r1b1k2r/pp1p1ppp/1qn1pn2/8/1bPN4/2N1P1P1/PPQ2P1P/R1B1KB1R b KQkq - 2 8";
        let board = Board::from_fen(fen).unwrap();
        let mut params = SearchParams::new();
        params.movetime = Some(500);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let start = std::time::Instant::now();
        let result = search(&board, &params, &stop, &tt, None);
        let elapsed = start.elapsed().as_millis();
        assert!(result.best_move.is_some(), "search should return a move");
        assert!(elapsed < 5000, "search with 500ms movetime took {elapsed}ms, should stop quickly");
    }

    #[test]
    fn test_king_in_check_illegal_move() {
        crate::attack::init_slider_tables();
        let fen = "8/p3r3/1p4k1/3B1Qp1/2P5/6P1/7P/4rRK1 b - - 14 47";
        let board = Board::from_fen(fen).unwrap();
        let king = board.king_square(Color::Black);
        assert!(board.is_attacked_by(king, Color::White), "black king should be in check");
        let moves = crate::movegen::generate_legal_vec(&board);
        let e1 = Square::from_file_rank(4, 0).unwrap();
        let f1 = Square::from_file_rank(5, 0).unwrap();
        let has_e1f1 = moves.iter().any(|m| m.from() == e1 && m.to() == f1);
        assert!(!has_e1f1, "e1f1 should be illegal (does not resolve check)");
        let params = SearchParams::with_depth(1);
        let stop = Arc::new(AtomicBool::new(false));
        let tt = make_tt();
        let result = search(&board, &params, &stop, &tt, None);
        if let Some(bm) = result.best_move {
            assert_ne!((bm.from(), bm.to()), (e1, f1),
                "search picked illegal move e1f1 (does not resolve check)");
        }
    }
}
