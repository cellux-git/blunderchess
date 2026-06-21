use crate::board::Board;
use crate::draw;
use crate::eval::Eval;
use crate::movegen::{self, MAX_MOVES};
use crate::search::params::CHECKMATE;
use crate::search::worker::SearchState;
use crate::tt::{NodeType, TT};
use crate::types::{Move, MoveKind, Piece, MAX_DEPTH};
use std::sync::Arc;

pub(crate) fn quiescence(board: &mut Board, mut alpha: i32, beta: i32, ply: u8, qs_depth: u8, state: &mut SearchState, tt: &Arc<TT>, eval: &Eval) -> i32 {
    state.nodes += 1;
    if state.should_stop() || ply >= MAX_DEPTH - 1 { return eval.evaluate(board); }

    let hash = board.hash();
    let tt_entry = tt.probe(hash);
    let tt_score = tt_entry.as_ref().map(|e| {
        if e.score.abs() >= CHECKMATE - 100 {
            if e.score > 0 { e.score - ply as i32 } else { e.score + ply as i32 }
        } else { e.score }
    });

    if let Some(ref entry) = tt_entry {
        {
            let s = tt_score.unwrap();
            match entry.node_type {
                NodeType::Exact => return s,
                NodeType::LowerBound => { if s >= beta { return s; } }
                NodeType::UpperBound => { if s <= alpha { return s; } }
            }
        }
    }

    if draw::is_draw_by_rule(board) { return 0; }

    let stand_pat = eval.evaluate(board);
    let in_check = board.in_check();
    if !in_check {
        if stand_pat >= beta {
            tt.store(hash, beta, 0, NodeType::LowerBound, None);
            return beta;
        }
        if stand_pat > alpha { alpha = stand_pat; }
        if stand_pat + 900 <= alpha {
            return alpha;
        }
    }

    let mut moves_buf = [Move::NULL; MAX_MOVES];
    let mut pseudo_count: usize = 0;
    if qs_depth > 0 && !in_check {
        movegen::generate_captures_promotions(board, &mut moves_buf, &mut pseudo_count);
    } else {
        movegen::generate_pseudo_legal(board, &mut moves_buf, &mut pseudo_count);
    }

    let side = board.side_to_move();
    let pinned = board.pinned_pieces(side);
    let mut filtered = 0;

    for i in 0..pseudo_count {
        let mv = moves_buf[i];
        let k = mv.kind();
        let is_cap_or_promo = k == MoveKind::Capture || k == MoveKind::Promotion;
        if !is_cap_or_promo && qs_depth > 0 && !in_check { continue; }

        // SEE pruning: skip losing captures in quiescence
        if qs_depth > 0 && !in_check && k == MoveKind::Capture {
            if eval.see(board, mv) < 0 { continue; }
        }

        let from = mv.from();

        let is_trivially_legal = !in_check && {
            if let Some(piece) = board.piece_at(from) {
                let is_ep = k == MoveKind::Capture && board.en_passant() == Some(mv.to());
                piece != Piece::King && !is_ep && k != MoveKind::Castle && (from.bit() & pinned) == 0
            } else { false }
        };

        if is_trivially_legal {
            moves_buf[filtered] = mv;
            filtered += 1;
        } else {
            let undo = board.make_move(mv);
            let king = board.king_square(side);
            let own_king_safe = !board.is_attacked_by(king, board.side_to_move());
            let gives_check = if !is_cap_or_promo || in_check { board.in_check() } else { false };
            board.unmake_move(&undo);
            if own_king_safe && (is_cap_or_promo || gives_check || in_check) {
                moves_buf[filtered] = mv;
                filtered += 1;
            }
        }
    }

    if filtered == 0 {
        let score = if board.in_check() { -(CHECKMATE - ply as i32) } else { alpha };
        tt.store(hash, score, 0, NodeType::Exact, None);
        return score;
    }
    state.move_ordering.order_moves_q(&mut moves_buf[..filtered], board, eval);

    let mut best_score = -(CHECKMATE + 200);
    let mut node_type = NodeType::UpperBound;

    for i in 0..filtered {
        let mv = moves_buf[i];
        let undo = board.make_move(mv);
        tt.prefetch(board.hash());
        let score = -quiescence(board, -beta, -alpha, ply + 1, qs_depth + 1, state, tt, eval);
        board.unmake_move(&undo);
        if score > best_score { best_score = score; }
        if score >= beta {
            tt.store(hash, beta, 0, NodeType::LowerBound, None);
            return beta;
        }
        if score > alpha {
            alpha = score;
            node_type = NodeType::Exact;
        }
        if state.should_stop() { break; }
    }
    if node_type == NodeType::Exact {
        tt.store(hash, best_score, 0, node_type, None);
    }
    alpha
}
