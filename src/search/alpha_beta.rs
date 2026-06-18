use crate::board::{Board, MAX_MOVES};
use crate::draw;
use crate::eval::EVAL;
use crate::movegen;
use crate::search::params::{SearchAlgorithmParams, CHECKMATE};
use crate::search::quiescence::quiescence;
use crate::search::worker::SearchState;
use crate::tt::{NodeType, TT};
use crate::types::{Move, MoveKind, Piece, MAX_DEPTH};
use std::sync::Arc;

pub(crate) fn alpha_beta(
    board: &mut Board,
    mut alpha: i32,
    beta: i32,
    depth: u8,
    ply: u8,
    state: &mut SearchState,
    tt: &Arc<TT>,
    is_pv: bool,
    thread_id: u8,
    alg: &SearchAlgorithmParams,
) -> i32 {
    state.nodes += 1;

    if ply >= MAX_DEPTH - 1 { return EVAL.evaluate(board); }
    state.pv_length[ply as usize] = ply as usize;

    if state.should_stop() || depth == 0 {
        return quiescence(board, alpha, beta, ply, 0, state, tt);
    }

    let hash = board.hash();
    tt.prefetch(hash);
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

    if draw::is_terminal_draw(board) { return 0; }

    let mut hash_move = tt_entry.and_then(|e| e.best_move);

    // IIR: reduced-depth search to find a good move when TT misses
    if !is_pv && hash_move.is_none() && depth >= 4 {
        alpha_beta(board, alpha, beta, depth - 2, ply, state, tt, false, thread_id, alg);
        hash_move = tt.probe(hash).and_then(|e| e.best_move);
    }

    let in_check = board.in_check();

    let can_null_move = !is_pv && depth >= alg.null_move.min_depth && ply > 0 && !in_check;
    let non_pawn_king = board.occupancy()
        & !board.pieces_bb(Piece::Pawn)
        & !board.pieces_bb(Piece::King);
    let has_big_pieces = non_pawn_king != 0;

    if can_null_move && has_big_pieces {
        let r = if depth >= alg.null_move.deep_threshold { alg.null_move.r_deep } else { alg.null_move.r_shallow };
        let null_depth = if depth > r { depth - r } else { 0 };
        if null_depth > 0 {
            let undo_null = board.make_null_move();
            let null_score = -alpha_beta(board, -beta, -beta + 1, null_depth, ply + 1, state, tt, false, thread_id, alg);
            board.unmake_null_move(&undo_null);
            if null_score >= beta { return null_score; }
        }
    }

    let static_eval = if depth <= alg.futility.max_depth { Some(EVAL.evaluate(board)) } else { None };

    // Razor pruning: at depth 1, if eval is far below alpha, skip to QS
    if depth == 1 && !is_pv && !in_check {
        if let Some(se) = static_eval {
            if se + alg.razor_margin <= alpha {
                return quiescence(board, alpha, beta, ply, 0, state, tt);
            }
        }
    }

    let mut moves_buf = [Move::NULL; MAX_MOVES];
    let mut move_count: usize = 0;
    movegen::generate_pseudo_legal(board, &mut moves_buf, &mut move_count);
    let moves = &mut moves_buf[..move_count];
    state.move_ordering.order_moves(moves, board, hash_move, ply, thread_id);

    let side = board.side_to_move();
    let pinned = board.pinned_pieces(side);
    let mut best_move: Option<Move> = None;
    let mut best_score = -(CHECKMATE + 200);
    let mut node_type = NodeType::UpperBound;
    let mut moves_searched = 0u32;

    for i in 0..moves.len() {
        if state.should_stop() { break; }

        let mv = moves[i];

        if ply == 0 && state.excluded_moves.contains(&mv) {
            continue;
        }

        let from = mv.from();

        let is_trivially_legal = !in_check && {
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
        tt.prefetch(board.hash());

        if !is_trivially_legal {
            let king_sq = board.king_square(side);
            if board.is_attacked_by(king_sq, board.side_to_move()) {
                board.unmake_move(&undo);
                continue;
            }
        }

        // Futility pruning
        if let Some(se) = static_eval {
            if depth <= alg.futility.max_depth {
                let mv_kind = mv.kind();
                let is_quiet = mv_kind != MoveKind::Capture && mv_kind != MoveKind::Promotion;
                if is_quiet && !board.in_check() {
                    let margin: i32 = if depth == 2 { alg.futility.margin_d2 } else { alg.futility.margin_d1 };
                    if se + margin <= alpha {
                        board.unmake_move(&undo);
                        continue;
                    }
                }
            }
        }

        let mut score: i32;
        if moves_searched == 0 {
            score = -alpha_beta(board, -beta, -alpha, depth - 1, ply + 1, state, tt, is_pv, thread_id, alg);
        } else {
            score = alpha + 1;
            let mv_kind = mv.kind();
            let is_quiet = mv_kind != MoveKind::Capture && mv_kind != MoveKind::Promotion;
            if depth >= alg.lmr.min_depth && moves_searched >= alg.lmr.min_moves_searched as u32 && is_quiet {
                let is_killer = state.move_ordering.is_killer(mv, ply);
                let gives_check = board.in_check();
                if !is_killer && !gives_check {
                    let base_r: u8 = if moves_searched >= 8 { alg.lmr.reduction[2] } else if moves_searched >= 5 { alg.lmr.reduction[1] } else { alg.lmr.reduction[0] };
                    let hist = state.move_ordering.history_score(mv);
                    let r = if hist > 2000 { base_r.saturating_sub(1) } else if hist < 200 { base_r + 1 } else { base_r };
                    if depth > r + 1 {
                        let r_depth = depth - 1 - r;
                        score = -alpha_beta(board, -alpha - 1, -alpha, r_depth, ply + 1, state, tt, false, thread_id, alg);
                    }
                }
            }
            if score > alpha {
                score = -alpha_beta(board, -alpha - 1, -alpha, depth - 1, ply + 1, state, tt, false, thread_id, alg);
                if score > alpha && score < beta {
                    score = -alpha_beta(board, -beta, -alpha, depth - 1, ply + 1, state, tt, true, thread_id, alg);
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

                    let k = mv.kind();
                    if k != MoveKind::Capture && k != MoveKind::Promotion {
                        state.move_ordering.record_beta_cutoff(mv, depth, ply);
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
