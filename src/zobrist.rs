use crate::types::{CastlingRights, Color, Piece, Square};

const PIECE_COUNT: usize = 6;
const COLOR_COUNT: usize = 2;
const SQUARE_COUNT: usize = 64;
const CASTLING_COUNT: usize = 4;
const EP_FILE_COUNT: usize = 8;

const TOTAL_KEYS: usize =
    PIECE_COUNT * COLOR_COUNT * SQUARE_COUNT + 1 + CASTLING_COUNT + EP_FILE_COUNT;

const ZOBRIST_KEYS: [u64; TOTAL_KEYS] = {
    let mut keys = [0u64; TOTAL_KEYS];
    let mut seed: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let mut i = 0;
    while i < TOTAL_KEYS {
        seed = lcg_next(seed);
        keys[i] = seed;
        i += 1;
    }
    keys
};

const fn lcg_next(state: u64) -> u64 {
    state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407)
}

const fn piece_square_index(color: Color, piece: Piece, square: Square) -> usize {
    color as usize * PIECE_COUNT * SQUARE_COUNT
        + piece as usize * SQUARE_COUNT
        + square.index() as usize
}

const SIDE_TO_MOVE_INDEX: usize = PIECE_COUNT * COLOR_COUNT * SQUARE_COUNT;

const fn castling_index(right: usize) -> usize {
    SIDE_TO_MOVE_INDEX + 1 + right
}

const fn ep_file_index(file: u8) -> usize {
    SIDE_TO_MOVE_INDEX + 1 + CASTLING_COUNT + file as usize
}

#[inline]
pub fn zobrist_piece_square(color: Color, piece: Piece, square: Square) -> u64 {
    ZOBRIST_KEYS[piece_square_index(color, piece, square)]
}

#[inline]
pub fn zobrist_side_to_move() -> u64 {
    ZOBRIST_KEYS[SIDE_TO_MOVE_INDEX]
}

#[inline]
pub fn zobrist_castling(rights: CastlingRights) -> u64 {
    let mut hash = 0u64;
    if rights.white_kingside {
        hash ^= ZOBRIST_KEYS[castling_index(0)];
    }
    if rights.white_queenside {
        hash ^= ZOBRIST_KEYS[castling_index(1)];
    }
    if rights.black_kingside {
        hash ^= ZOBRIST_KEYS[castling_index(2)];
    }
    if rights.black_queenside {
        hash ^= ZOBRIST_KEYS[castling_index(3)];
    }
    hash
}

#[inline]
pub fn zobrist_en_passant(file: Option<u8>) -> u64 {
    match file {
        Some(f) if f < 8 => ZOBRIST_KEYS[ep_file_index(f)],
        _ => 0,
    }
}

pub fn compute_initial_hash(
    squares: &[Option<Piece>; 64],
    colors: &[Option<Color>; 64],
    side_to_move: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
) -> u64 {
    let mut hash = 0u64;

    for sq_idx in 0..64 {
        if let Some(piece) = squares[sq_idx] {
            let color = colors[sq_idx].unwrap();
            hash ^= zobrist_piece_square(color, piece, Square::new(sq_idx as u8).unwrap());
        }
    }

    if side_to_move == Color::Black {
        hash ^= zobrist_side_to_move();
    }

    hash ^= zobrist_castling(castling_rights);

    if let Some(ep) = en_passant {
        hash ^= zobrist_en_passant(Some(ep.file()));
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_incremental_hash_matches_full() {
        let board = Board::from_initial();
        let initial_hash = board.hash();

        let full = compute_initial_hash(
            board.squares(),
            board.colors(),
            board.side_to_move(),
            board.castling_rights(),
            board.en_passant(),
        );
        assert_eq!(initial_hash, full);

        let moves = crate::movegen::generate_legal_vec(&board);
        for mv in &moves {
            let mut child = board.clone();
            child.make_move(*mv);
            let recomputed = compute_initial_hash(
                child.squares(),
                child.colors(),
                child.side_to_move(),
                child.castling_rights(),
                child.en_passant(),
            );
            assert_eq!(child.hash(), recomputed, "Hash mismatch after move {mv}");
        }
    }

    #[test]
    fn test_hash_changes_after_move() {
        let board = Board::from_initial();
        let moves = crate::movegen::generate_legal_vec(&board);
        for mv in &moves {
            let mut child = board.clone();
            child.make_move(*mv);
            assert_ne!(child.hash(), board.hash(), "Hash should change after move {mv}");
        }
    }

    #[test]
    fn test_side_to_move_toggle() {
        let fen_btm = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let board_btm = Board::from_fen(fen_btm).unwrap();

        let fen_wtm = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e3 0 1";
        let board_wtm = Board::from_fen(fen_wtm).unwrap();

        assert_ne!(board_btm.hash(), board_wtm.hash(),
            "Hash should differ when side to move differs");
    }
}
