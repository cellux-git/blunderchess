use crate::board::{Board, MAX_MOVES};
use crate::draw;
use crate::eval::EVAL;
use crate::movegen;
use crate::search::params::CHECKMATE;
use crate::search::worker::SearchState;
use crate::types::{Move, MoveKind, Piece, MAX_DEPTH};

pub(crate) fn quiescence(board: &mut Board, mut alpha: i32, beta: i32, ply: u8, qs_depth: u8, state: &mut SearchState) -> i32 {
    state.nodes += 1;
    if state.should_stop() || ply >= MAX_DEPTH - 1 { return EVAL.evaluate(board); }

    if draw::is_terminal_draw(board) { return 0; }

    let stand_pat = EVAL.evaluate(board);
    let in_check = board.in_check();
    if !in_check {
        if stand_pat >= beta { return beta; }
        if stand_pat > alpha { alpha = stand_pat; }
    }

    let mut moves_buf = [Move::NULL; MAX_MOVES];
    let mut pseudo_count: usize = 0;
    movegen::generate_pseudo_legal(board, &mut moves_buf, &mut pseudo_count);

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
            if EVAL.see(board, mv) < 0 { continue; }
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
            let gives_check = board.in_check();
            board.unmake_move(&undo);
            if own_king_safe && (is_cap_or_promo || gives_check || in_check) {
                moves_buf[filtered] = mv;
                filtered += 1;
            }
        }
    }

    if filtered == 0 {
        return if board.in_check() { -(CHECKMATE - ply as i32) } else { alpha };
    }
    state.move_ordering.order_moves_q(&mut moves_buf[..filtered], board);

    for i in 0..filtered {
        let mv = moves_buf[i];
        let undo = board.make_move(mv);
        let king_sq = board.king_square(side);
        if board.is_attacked_by(king_sq, board.side_to_move()) {
            board.unmake_move(&undo);
            continue;
        }
        let score = -quiescence(board, -beta, -alpha, ply + 1, qs_depth + 1, state);
        board.unmake_move(&undo);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
        if state.should_stop() { break; }
    }
    alpha
}
