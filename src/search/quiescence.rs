use crate::board::{Board, GameResult, MAX_MOVES};
use crate::eval::Eval;
use crate::movegen;
use crate::search::params::CHECKMATE;
use crate::search::worker::SearchState;
use crate::types::{MoveKind, MAX_DEPTH};

pub(crate) fn quiescence(board: &mut Board, mut alpha: i32, beta: i32, ply: u8, qs_depth: u8, state: &mut SearchState) -> i32 {
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

    let mut moves_buf = [crate::types::Move::NULL; MAX_MOVES];
    let move_count = movegen::generate_legal_moves(board, &mut moves_buf);
    let side = board.side_to_move();
    let mut filtered = 0;
    for i in 0..move_count {
        let mv = moves_buf[i];
        let k = mv.kind();
        let is_cap_or_promo = k == MoveKind::Capture || k == MoveKind::Promotion;
        let include = is_cap_or_promo || qs_depth == 0;
        if include {
            let undo = board.make_move(mv);
            let king = board.king_square(side);
            let own_king_safe = !board.is_attacked_by(king, board.side_to_move());
            let gives_check = board.in_check();
            board.unmake_move(&undo);
            if own_king_safe && (is_cap_or_promo || gives_check) {
                moves_buf[filtered] = mv;
                filtered += 1;
            }
        }
    }
    if filtered == 0 { return alpha; }
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
