use crate::types::{Move, MoveKind, Piece};
use crate::types::Square;
use std::sync::atomic::{AtomicU64, Ordering};

const ENTRY_SIZE: usize = 3;

fn pack_move(mv: Move) -> u32 {
    let mut packed: u32 = 0;
    packed |= (mv.from().index() as u32) & 0x3F;
    packed |= ((mv.to().index() as u32) & 0x3F) << 6;
    packed |= ((mv.kind() as u32) & 0x3) << 12;
    let promo_bits: u32 = match mv.promotion_piece() {
        Some(Piece::Knight) => 1,
        Some(Piece::Bishop) => 2,
        Some(Piece::Rook) => 3,
        _ => 0,
    };
    packed | (promo_bits << 15)
}

fn unpack_move(packed: u32) -> Option<Move> {
    if packed == 0 { return None; }
    let from = Square::new((packed & 0x3F) as u8)?;
    let to = Square::new(((packed >> 6) & 0x3F) as u8)?;
    let kind_val = ((packed >> 12) & 0x3) as u8;
    let promo_val = ((packed >> 15) & 0x3) as u8;

    let kind = match kind_val {
        0 => MoveKind::Normal,
        1 => MoveKind::Capture,
        2 => MoveKind::Castle,
        _ => MoveKind::Promotion,
    };
    let piece = match promo_val {
        0 => None,
        1 => Some(Piece::Knight),
        2 => Some(Piece::Bishop),
        3 => Some(Piece::Rook),
        4 => Some(Piece::Queen),
        _ => return None,
    };

    let raw = (from.index() as u16)
        | ((to.index() as u16) << 6)
        | ((promo_raw(piece) as u16) << 12)
        | ((kind as u16) << 14);
    Some(Move::from_raw(raw))
}

const fn promo_raw(piece: Option<Piece>) -> u8 {
    match piece {
        Some(Piece::Knight) => 1,
        Some(Piece::Bishop) => 2,
        Some(Piece::Rook) => 3,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

pub struct TT {
    table: Box<[AtomicU64]>,
    entries_mask: usize,
    age: u8,
}

unsafe impl Send for TT {}
unsafe impl Sync for TT {}

impl TT {
    pub fn new(mega_bytes: usize) -> TT {
        let entry_bytes = ENTRY_SIZE * 8;
        let max_entries = (mega_bytes * 1024 * 1024) / entry_bytes;
        let size = max_entries.next_power_of_two().max(1024);
        let cap = size >> 1;
        let total = cap * ENTRY_SIZE;

        let mut vec = Vec::with_capacity(total);
        vec.resize_with(total, || AtomicU64::new(0));
        let table = vec.into_boxed_slice();

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        unsafe {
            let ptr = table.as_ptr() as u64;
            let len = (table.len() * 8) as u64;
            std::arch::asm!(
                "syscall",
                in("rax") 28u64,
                in("rdi") ptr,
                in("rsi") len,
                in("rdx") 14u64,
                lateout("rax") _,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }

        TT {
            table,
            entries_mask: cap - 1,
            age: 0,
        }
    }

    pub fn new_game(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    #[inline]
    fn entry_offset(&self, hash: u64) -> usize {
        ((hash as usize) & self.entries_mask) * ENTRY_SIZE
    }

    pub fn probe(&self, hash: u64) -> Option<TTProbe> {
        let base = self.entry_offset(hash);
        let stored_hash = self.table[base].load(Ordering::Acquire);

        if stored_hash != hash {
            return None;
        }

        let data = self.table[base + 1].load(Ordering::Acquire);
        let mv_packed = self.table[base + 2].load(Ordering::Acquire);

        let score = data as i32;
        let depth = ((data >> 32) & 0xFF) as u8;
        let node_type = match ((data >> 40) & 0x3) as u8 {
            0 => NodeType::Exact,
            1 => NodeType::LowerBound,
            _ => NodeType::UpperBound,
        };
        let best_move = unpack_move(mv_packed as u32);

        Some(TTProbe {
            score,
            depth,
            node_type,
            best_move,
        })
    }

    pub fn store(
        &self,
        hash: u64,
        score: i32,
        depth: u8,
        node_type: NodeType,
        best_move: Option<Move>,
    ) {
        let base = self.entry_offset(hash);

        let existing_hash = self.table[base].load(Ordering::Relaxed);
        if existing_hash == hash {
            let existing_data = self.table[base + 1].load(Ordering::Relaxed);
            let existing_depth = ((existing_data >> 32) & 0xFF) as u8;
            let existing_age = ((existing_data >> 42) & 0xFF) as u8;
            if existing_age == self.age && depth < existing_depth {
                return;
            }
        }

        let data = (score as u64 & 0xFFFF_FFFF)
            | ((depth as u64) << 32)
            | (((node_type as u64) & 0x3) << 40)
            | (((self.age as u64) & 0xFF) << 42);

        let mv_packed = best_move.map(|m| pack_move(m) as u64).unwrap_or(0);

        self.table[base + 1].store(data, Ordering::Release);
        self.table[base + 2].store(mv_packed, Ordering::Release);
        self.table[base].store(hash, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub struct TTProbe {
    pub score: i32,
    pub depth: u8,
    pub node_type: NodeType,
    pub best_move: Option<Move>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Square;

    #[test]
    fn test_probe_store_roundtrip() {
        let tt = TT::new(1);
        let mv = crate::types::Move::new(
            Square::from_file_rank(4, 1).unwrap(),
            Square::from_file_rank(4, 3).unwrap(),
        );
        let hash = 0x1234_5678_9ABC_DEF0;
        tt.store(hash, 42, 5, NodeType::Exact, Some(mv));
        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.score, 42);
        assert_eq!(entry.depth, 5);
        assert_eq!(entry.node_type, NodeType::Exact);
        let unpacked = entry.best_move.unwrap();
        assert_eq!(unpacked.from(), mv.from());
        assert_eq!(unpacked.to(), mv.to());
    }

    #[test]
    fn test_probe_miss() {
        let tt = TT::new(1);
        tt.store(0xAAAA, 10, 3, NodeType::LowerBound, None);
        assert!(tt.probe(0xBBBB).is_none());
    }

    #[test]
    fn test_depth_preferred_replacement() {
        let tt = TT::new(1);
        let mv1 = crate::types::Move::new(
            Square::from_file_rank(4, 1).unwrap(),
            Square::from_file_rank(4, 3).unwrap(),
        );
        let mv2 = crate::types::Move::new(
            Square::from_file_rank(3, 1).unwrap(),
            Square::from_file_rank(3, 3).unwrap(),
        );
        let hash = 0xDEAD_BEEF;

        tt.store(hash, 30, 3, NodeType::Exact, Some(mv1));
        tt.store(hash, 50, 6, NodeType::Exact, Some(mv2));

        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.depth, 6);
        assert_eq!(entry.score, 50);
    }

    #[test]
    fn test_age_based_replacement() {
        let mut tt = TT::new(1);
        let mv1 = crate::types::Move::new(
            Square::from_file_rank(4, 1).unwrap(),
            Square::from_file_rank(4, 3).unwrap(),
        );
        let mv2 = crate::types::Move::new(
            Square::from_file_rank(3, 1).unwrap(),
            Square::from_file_rank(3, 3).unwrap(),
        );
        let hash = 0xBEEF;

        tt.store(hash, 30, 5, NodeType::Exact, Some(mv1));
        tt.new_game();
        tt.store(hash, 50, 2, NodeType::Exact, Some(mv2));

        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.score, 50, "New game entry should replace stale");
    }

    #[test]
    fn test_move_pack_roundtrip() {
        let original = crate::types::Move::promotion(
            Square::from_file_rank(4, 6).unwrap(),
            Square::from_file_rank(4, 7).unwrap(),
            crate::types::Piece::Queen,
        );
        let packed = pack_move(original);
        let unpacked = unpack_move(packed).unwrap();
        assert_eq!(unpacked.from(), original.from());
        assert_eq!(unpacked.to(), original.to());
        assert_eq!(unpacked.promotion_piece(), original.promotion_piece());
        assert_eq!(unpacked.kind(), original.kind());
    }
}
