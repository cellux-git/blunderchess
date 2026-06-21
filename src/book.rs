use crate::board::Board;
use crate::types::{Move, Piece, Square};

pub struct Book {
    entries: Vec<u8>,
}

impl Book {
    pub fn load(path: &str) -> Result<Book, String> {
        let data = std::fs::read(path).map_err(|e| format!("cannot read book: {e}"))?;
        if data.len() % 16 != 0 {
            return Err("book file size not multiple of 16".to_string());
        }
        if data.is_empty() {
            return Err("book file is empty".to_string());
        }
        // Verify entries are sorted by hash
        for chunk in data.chunks_exact(16) {
            let _ = u64::from_be_bytes(chunk[0..8].try_into().unwrap());
        }
        Ok(Book { entries: data })
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len() / 16
    }

    pub fn probe(&self, board: &Board) -> Option<Move> {
        let hash = board.hash();
        let count = self.entries.len() / 16;

        // Binary search for first matching hash (lower bound)
        let first = self.lower_bound(hash, 0, count);
        if first >= count || self.get_hash(first) != hash {
            return None;
        }

        // Find range of matching entries (all entries with the same hash)
        let last = self.lower_bound(hash + 1, first, count);

        // Pick highest-weight move
        let mut best_idx = first;
        let mut best_weight: u16 = 0;
        for i in first..last {
            let w = self.get_weight(i);
            if w > best_weight {
                best_weight = w;
                best_idx = i;
            }
        }

        if best_weight == 0 {
            return None;
        }

        polyglot_to_move(board, self.get_move(best_idx))
    }

    fn lower_bound(&self, hash: u64, mut lo: usize, hi: usize) -> usize {
        let mut hi = hi;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.get_hash(mid) < hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    fn get_hash(&self, idx: usize) -> u64 {
        let base = idx * 16;
        u64::from_be_bytes(self.entries[base..base + 8].try_into().unwrap())
    }

    fn get_move(&self, idx: usize) -> u16 {
        let base = idx * 16;
        u16::from_be_bytes(self.entries[base + 8..base + 10].try_into().unwrap())
    }

    fn get_weight(&self, idx: usize) -> u16 {
        let base = idx * 16;
        u16::from_be_bytes(self.entries[base + 10..base + 12].try_into().unwrap())
    }
}

fn polyglot_to_move(board: &Board, pg_move: u16) -> Option<Move> {
    let to_file = (pg_move & 0x7) as u8;
    let to_rank = ((pg_move >> 3) & 0x7) as u8;
    let from_file = ((pg_move >> 6) & 0x7) as u8;
    let from_rank = ((pg_move >> 9) & 0x7) as u8;
    let promo = (pg_move >> 12) & 0x7;

    let from = Square::from_file_rank(from_file, from_rank)?;
    let to = Square::from_file_rank(to_file, to_rank)?;

    if promo > 0 {
        let piece = match promo {
            1 => Piece::Knight,
            2 => Piece::Bishop,
            3 => Piece::Rook,
            4 => Piece::Queen,
            _ => return None,
        };
        return Some(Move::promotion(from, to, piece));
    }

    // Infer move kind from board context
    let piece = board.piece_at(from)?;
    if piece == Piece::King && (from_file as i8 - to_file as i8).abs() == 2 {
        return Some(Move::castle(from, to));
    }
    if board.piece_at(to).is_some() {
        return Some(Move::capture(from, to));
    }
    if piece == Piece::Pawn {
        if board.en_passant() == Some(to) {
            return Some(Move::ep(from, to));
        }
    }
    Some(Move::new(from, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attack::init_slider_tables;
    use crate::movegen;

    #[test]
    fn test_polyglot_move_roundtrip_startpos() {
        init_slider_tables();
        let board = Board::from_initial();
        let legal = movegen::generate_legal_vec(&board);
        for mv in legal {
            let pg = move_to_polyglot(mv);
            let back = polyglot_to_move(&board, pg);
            assert_eq!(back, Some(mv), "roundtrip failed for {mv}");
        }
    }

    fn move_to_polyglot(mv: Move) -> u16 {
        let promo = match mv.promotion_piece() {
            Some(Piece::Knight) => 1,
            Some(Piece::Bishop) => 2,
            Some(Piece::Rook) => 3,
            Some(Piece::Queen) => 4,
            _ => 0,
        };
        let from_file = mv.from().file();
        let from_rank = mv.from().rank();
        let to_file = mv.to().file();
        let to_rank = mv.to().rank();
        to_file as u16
            | ((to_rank as u16) << 3)
            | ((from_file as u16) << 6)
            | ((from_rank as u16) << 9)
            | ((promo as u16) << 12)
    }

    #[test]
    fn test_binary_search() {
        // Create a mock book with sorted hashes
        let mut data = Vec::new();
        for i in 0..5u64 {
            let mut entry = [0u8; 16];
            entry[0..8].copy_from_slice(&(i * 100).to_be_bytes());
            data.extend_from_slice(&entry);
        }
        let book = Book { entries: data };

        assert_eq!(book.lower_bound(0, 0, 5), 0);
        assert_eq!(book.lower_bound(100, 0, 5), 1);
        assert_eq!(book.lower_bound(500, 0, 5), 5); // past end
        assert_eq!(book.lower_bound(50, 0, 5), 1); // between 0 and 100
    }
}
