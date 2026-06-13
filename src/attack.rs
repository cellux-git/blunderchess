use crate::types::{Bitboard, Color, Square};
use crate::types::Piece;
use crate::board::Board;

const KNIGHT_ATTACKS: [Bitboard; 64] = precompute_knight_attacks();
const KING_ATTACKS: [Bitboard; 64] = precompute_king_attacks();
const PAWN_ATTACKS: [[Bitboard; 64]; 2] = precompute_pawn_attacks();

const fn precompute_knight_attacks() -> [Bitboard; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0;
    while sq < 64 {
        let f = sq % 8;
        let r = sq / 8;
        let offsets: [(i32, i32); 8] = [
            (-2, -1), (-2, 1), (-1, -2), (-1, 2),
            (1, -2), (1, 2), (2, -1), (2, 1),
        ];
        let mut attacks: u64 = 0;
        let mut i = 0;
        while i < 8 {
            let nf = f + offsets[i].0;
            let nr = r + offsets[i].1;
            if nf >= 0 && nf < 8 && nr >= 0 && nr < 8 {
                attacks |= 1u64 << (nr * 8 + nf);
            }
            i += 1;
        }
        table[sq as usize] = attacks;
        sq += 1;
    }
    table
}

const fn precompute_king_attacks() -> [Bitboard; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0;
    while sq < 64 {
        let f = sq % 8;
        let r = sq / 8;
        let offsets: [(i32, i32); 8] = [
            (-1, -1), (-1, 0), (-1, 1), (0, -1),
            (0, 1), (1, -1), (1, 0), (1, 1),
        ];
        let mut attacks: u64 = 0;
        let mut i = 0;
        while i < 8 {
            let nf = f + offsets[i].0;
            let nr = r + offsets[i].1;
            if nf >= 0 && nf < 8 && nr >= 0 && nr < 8 {
                attacks |= 1u64 << (nr * 8 + nf);
            }
            i += 1;
        }
        table[sq as usize] = attacks;
        sq += 1;
    }
    table
}

const fn precompute_pawn_attacks() -> [[Bitboard; 64]; 2] {
    let mut table = [[0u64; 64]; 2];
    let mut sq = 0;
    while sq < 64 {
        let f = sq % 8;
        let r = sq / 8;
        if r < 7 {
            if f > 0 { table[0][sq as usize] |= 1u64 << ((r + 1) * 8 + (f - 1)); }
            if f < 7 { table[0][sq as usize] |= 1u64 << ((r + 1) * 8 + (f + 1)); }
        }
        if r > 0 {
            if f > 0 { table[1][sq as usize] |= 1u64 << ((r - 1) * 8 + (f - 1)); }
            if f < 7 { table[1][sq as usize] |= 1u64 << ((r - 1) * 8 + (f + 1)); }
        }
        sq += 1;
    }
    table
}

use std::sync::OnceLock;

struct AttackTables {
    rook_table: Box<[Bitboard]>,
    bishop_table: Box<[Bitboard]>,
    rook_offsets: [usize; 64],
    bishop_offsets: [usize; 64],
    bishop_magics: [u64; 64],
}

static TABLES: OnceLock<AttackTables> = OnceLock::new();

pub fn init_slider_tables() {
    TABLES.get_or_init(|| init_slider_tables_inner());
}

fn tables() -> &'static AttackTables {
    TABLES.get_or_init(|| init_slider_tables_inner())
}

// Magic multiplier seeds (well-known)
const ROOK_MAGICS: [u64; 64] = [
    0x0080001020400080, 0x0040001000200040, 0x0080081000200080, 0x0080040800100080,
    0x0080020400080080, 0x0080010200040080, 0x0080008001000200, 0x0080002040800100,
    0x0000800020400080, 0x0000400020005000, 0x0000801000200080, 0x0000800800100080,
    0x0000800400080080, 0x0000800200040080, 0x0000800100020080, 0x0000800040800100,
    0x0000208000400080, 0x0000404000201000, 0x0000808010002000, 0x0000808008001000,
    0x0000808004000800, 0x0000808002000400, 0x0000010100020004, 0x0000020000408104,
    0x0000208080004000, 0x0000200040005000, 0x0000100080200080, 0x0000080080100080,
    0x0000040080080080, 0x0000020080040080, 0x0000010080800200, 0x0000800080004100,
    0x0000204000800080, 0x0000200040401000, 0x0000100080802000, 0x0000080080801000,
    0x0000040080800800, 0x0000020080800400, 0x0000020001010004, 0x0000800040800100,
    0x0000204000808000, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
    0x0000204000800080, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000800100020080, 0x0000800041000080,
    0x00FFFCDDFCED714A, 0x007FFCDDFCED714A, 0x003FFFCDFFD88096, 0x0000040810002101,
    0x0001000204080011, 0x0001000204000801, 0x0001000082000401, 0x0001FFFAABFAD1A2,
];

fn init_slider_tables_inner() -> AttackTables {
    let mut rng: u64 = 0x29A1B4C3D5E6F708;
    let mut bishop_magics = [0u64; 64];
    for sq in 0..64u8 {
        let mask = bishop_mask(sq);
        let bits = mask.count_ones() as usize;
        if bits == 0 {
            bishop_magics[sq as usize] = 0;
            continue;
        }
        let shift = 64 - bits;
        bishop_magics[sq as usize] = find_magic(&mut rng, sq, mask, bits, shift);
    }

    let rook_total: usize = (0..64).map(|s| 1usize << (64 - ROOK_SHIFTS[s] as usize)).sum();
    let bishop_total: usize = (0..64).map(|s| 1usize << (64 - BISHOP_SHIFTS[s] as usize)).sum();
    let mut rook_table = vec![0u64; rook_total];
    let mut bishop_table = vec![0u64; bishop_total];
    let mut rook_offsets = [0usize; 64];
    let mut bishop_offsets = [0usize; 64];
    let mut rook_offset = 0;
    let mut bishop_offset = 0;
    for sq in 0..64 {
        rook_offsets[sq] = rook_offset;
        let rshift = ROOK_SHIFTS[sq] as usize;
        rook_offset += 1usize << (64 - rshift);

        bishop_offsets[sq] = bishop_offset;
        let bshift = BISHOP_SHIFTS[sq] as usize;
        bishop_offset += 1usize << (64 - bshift);
    }
    // Fill rook table
    for sq in 0..64u8 {
        let mask = rook_mask(sq);
        let magic = ROOK_MAGICS[sq as usize];
        let shift = ROOK_SHIFTS[sq as usize] as usize;
        let offset = rook_offsets[sq as usize];
        let n = 1u64 << mask.count_ones();
        for i in 0..n {
            let blockers = index_to_blockers(i, mask);
            let attacks = rook_attacks_slow(sq, blockers);
            let idx = offset + ((blockers.wrapping_mul(magic)) >> shift) as usize;
            rook_table[idx] = attacks;
        }
    }

    // Fill bishop table
    for sq in 0..64u8 {
        let mask = bishop_mask(sq);
        let magic = bishop_magics[sq as usize];
        let shift = BISHOP_SHIFTS[sq as usize] as usize;
        let offset = bishop_offsets[sq as usize];
        let n = 1u64 << mask.count_ones();
        for i in 0..n {
            let blockers = index_to_blockers(i, mask);
            let attacks = bishop_attacks_slow(sq, blockers);
            let idx = offset + ((blockers.wrapping_mul(magic)) >> shift) as usize;
            bishop_table[idx] = attacks;
        }
    }

    AttackTables {
        rook_table: rook_table.into_boxed_slice(),
        bishop_table: bishop_table.into_boxed_slice(),
        rook_offsets,
        bishop_offsets,
        bishop_magics,
    }
}

fn rng_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn find_magic(_rng: &mut u64, _sq: u8, mask: u64, bits: usize, shift: usize) -> u64 {
    let n = 1u64 << bits;
    let mut used = vec![0u64; n as usize];

    loop {
        let candidate = rng_next(_rng) & rng_next(_rng) & rng_next(_rng);

        for entry in used.iter_mut() {
            *entry = u64::MAX;
        }

        let mut ok = true;
        for i in 0..n {
            let blockers = index_to_blockers(i, mask);
            let idx = (blockers.wrapping_mul(candidate)) >> shift;
            if idx >= n as u64 || used[idx as usize] != u64::MAX {
                ok = false;
                break;
            }
            used[idx as usize] = i;
        }

        if ok {
            return candidate;
        }
    }
}

const ROOK_SHIFTS: [u8; 64] = [
    52, 53, 53, 53, 53, 53, 53, 52,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    52, 53, 53, 53, 53, 53, 53, 52,
];

const BISHOP_SHIFTS: [u8; 64] = [
    58, 59, 59, 59, 59, 59, 59, 58,
    59, 59, 59, 59, 59, 59, 59, 59,
    59, 59, 57, 57, 57, 57, 59, 59,
    59, 59, 57, 55, 55, 57, 59, 59,
    59, 59, 57, 55, 55, 57, 59, 59,
    59, 59, 57, 57, 57, 57, 59, 59,
    59, 59, 59, 59, 59, 59, 59, 59,
    58, 59, 59, 59, 59, 59, 59, 58,
];

fn rook_mask(sq: u8) -> u64 {
    let f = sq % 8;
    let r = sq / 8;
    let mut mask: u64 = 0;
    for rf in (f + 1)..7 { mask |= 1u64 << (r * 8 + rf); }
    for lf in (1..f).rev() { mask |= 1u64 << (r * 8 + lf); }
    for ur in (r + 1)..7 { mask |= 1u64 << (ur * 8 + f); }
    for dr in (1..r).rev() { mask |= 1u64 << (dr * 8 + f); }
    mask
}

fn bishop_mask(sq: u8) -> u64 {
    let f = sq % 8;
    let r = sq / 8;
    let mut mask: u64 = 0;
    let mut ff = f + 1; let mut rr = r + 1;
    while ff < 7 && rr < 7 { mask |= 1u64 << (rr * 8 + ff); ff += 1; rr += 1; }
    if r > 0 && f < 7 {
        ff = f + 1; rr = r - 1;
        while ff < 7 && rr > 0 { mask |= 1u64 << (rr * 8 + ff); ff += 1; rr -= 1; }
    }
    if f > 0 && r < 7 {
        ff = f - 1; rr = r + 1;
        while ff > 0 && rr < 7 { mask |= 1u64 << (rr * 8 + ff); ff -= 1; rr += 1; }
    }
    if f > 0 && r > 0 {
        ff = f - 1; rr = r - 1;
        while ff > 0 && rr > 0 { mask |= 1u64 << (rr * 8 + ff); ff -= 1; rr -= 1; }
    }
    mask
}

fn index_to_blockers(index: u64, mask: u64) -> u64 {
    let mut blockers: u64 = 0;
    let bits = mask.count_ones();
    for i in 0..bits {
        let bit_pos = nth_set_bit(mask, i);
        if (index >> i) & 1 != 0 {
            blockers |= 1u64 << bit_pos;
        }
    }
    blockers
}

fn nth_set_bit(mut mask: u64, n: u32) -> u8 {
    let mut count = 0;
    let mut pos = 0;
    while pos < 64 {
        if mask & 1 != 0 {
            if count == n { return pos as u8; }
            count += 1;
        }
        mask >>= 1;
        pos += 1;
    }
    0
}

fn rook_attacks_slow(sq: u8, blockers: u64) -> u64 {
    let f = sq % 8;
    let r = sq / 8;
    let mut attacks: u64 = 0;

    for ff in (f + 1)..8 {
        let b = 1u64 << (r * 8 + ff);
        attacks |= b;
        if blockers & b != 0 { break; }
    }
    for ff in (0..f).rev() {
        let b = 1u64 << (r * 8 + ff);
        attacks |= b;
        if blockers & b != 0 { break; }
    }
    for rr in (r + 1)..8 {
        let b = 1u64 << (rr * 8 + f);
        attacks |= b;
        if blockers & b != 0 { break; }
    }
    for rr in (0..r).rev() {
        let b = 1u64 << (rr * 8 + f);
        attacks |= b;
        if blockers & b != 0 { break; }
    }
    attacks
}

fn bishop_attacks_slow(sq: u8, blockers: u64) -> u64 {
    let f = sq % 8;
    let r = sq / 8;
    let mut attacks: u64 = 0;
    let dirs: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
    for (df, dr) in dirs.iter() {
        let mut ff = f as i32 + df;
        let mut rr = r as i32 + dr;
        while ff >= 0 && ff < 8 && rr >= 0 && rr < 8 {
            let b = 1u64 << (rr as u8 * 8 + ff as u8);
            attacks |= b;
            if blockers & b != 0 { break; }
            ff += df;
            rr += dr;
        }
    }
    attacks
}

#[inline]
pub fn rook_attacks(sq: u8, occ: u64) -> u64 {
    let t = tables();
    let mask = rook_mask(sq);
    let magic = ROOK_MAGICS[sq as usize];
    let shift = ROOK_SHIFTS[sq as usize];
    let idx = t.rook_offsets[sq as usize] + (((occ & mask).wrapping_mul(magic)) >> shift) as usize;
    t.rook_table[idx]
}

#[inline]
pub fn bishop_attacks(sq: u8, occ: u64) -> u64 {
    let t = tables();
    let mask = bishop_mask(sq);
    let magic = t.bishop_magics[sq as usize];
    let shift = BISHOP_SHIFTS[sq as usize];
    let idx = t.bishop_offsets[sq as usize] + (((occ & mask).wrapping_mul(magic)) >> shift) as usize;
    t.bishop_table[idx]
}

#[inline]
pub fn queen_attacks(sq: u8, occ: u64) -> u64 {
    rook_attacks(sq, occ) | bishop_attacks(sq, occ)
}

#[inline]
pub fn knight_attacks(sq: Square) -> Bitboard { KNIGHT_ATTACKS[sq.index() as usize] }

#[inline]
pub fn king_attacks(sq: Square) -> Bitboard { KING_ATTACKS[sq.index() as usize] }

#[inline]
pub fn pawn_attacks(sq: Square, color: Color) -> Bitboard {
    PAWN_ATTACKS[color.index()][sq.index() as usize]
}

// ---- file / rank helpers ----

const FILE_A: u64 = 0x0101010101010101;

pub fn file_mask(file: u8) -> u64 {
    FILE_A << file
}

pub fn adjacent_files_mask(file: u8) -> u64 {
    let mut mask: u64 = 0;
    if file > 0 { mask |= file_mask(file - 1); }
    mask |= file_mask(file);
    if file < 7 { mask |= file_mask(file + 1); }
    mask
}

pub fn rank_mask_forward(sq: Square, color: Color) -> u64 {
    let rank = sq.rank();
    if color == Color::White {
        let mut m: u64 = 0;
        for r in (rank + 1)..8 { m |= 0xFFu64 << (r * 8); }
        m
    } else {
        let mut m: u64 = 0;
        for r in 0..rank { m |= 0xFFu64 << (r * 8); }
        m
    }
}

pub fn king_distance(a: Square, b: Square) -> u8 {
    let df = (a.file() as i32 - b.file() as i32).unsigned_abs() as u8;
    let dr = (a.rank() as i32 - b.rank() as i32).unsigned_abs() as u8;
    df.max(dr)
}

pub fn attackers_to(board: &Board, sq: Square, occ: u64) -> u64 {
    let si = sq.index();
    let knights = board.pieces_bb(Piece::Knight) & knight_attacks(sq);
    let kings = board.pieces_bb(Piece::King) & king_attacks(sq);
    let pawns_w = board.pieces_bb(Piece::Pawn)
        & board.colors_bb(Color::White)
        & pawn_attacks(sq, Color::Black);
    let pawns_b = board.pieces_bb(Piece::Pawn)
        & board.colors_bb(Color::Black)
        & pawn_attacks(sq, Color::White);
    let rooks = (board.pieces_bb(Piece::Rook) | board.pieces_bb(Piece::Queen))
        & rook_attacks(si, occ);
    let bishops = (board.pieces_bb(Piece::Bishop) | board.pieces_bb(Piece::Queen))
        & bishop_attacks(si, occ);
    (knights | kings | pawns_w | pawns_b | rooks | bishops) & occ
}

pub fn smallest_attacker(board: &Board, sq: Square, side: Color, occ: u64) -> Option<(Square, Piece)> {
    let attackers = attackers_to(board, sq, occ) & board.colors_bb(side);
    if attackers == 0 { return None; }
    let p = attackers & board.pieces_bb(Piece::Pawn);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Pawn)); }
    let p = attackers & board.pieces_bb(Piece::Knight);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Knight)); }
    let p = attackers & board.pieces_bb(Piece::Bishop);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Bishop)); }
    let p = attackers & board.pieces_bb(Piece::Rook);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Rook)); }
    let p = attackers & board.pieces_bb(Piece::Queen);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::Queen)); }
    let p = attackers & board.pieces_bb(Piece::King);
    if p != 0 { let lsb = p.trailing_zeros() as u8; return Some((Square::new(lsb).unwrap(), Piece::King)); }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_rook_tables_exhaustive() {
        init_slider_tables();
        for sq in 0..64u8 {
            let mask = rook_mask(sq);
            let n = 1u64 << mask.count_ones();
            for i in 0..n {
                let blockers = index_to_blockers(i, mask);
                let expected = rook_attacks_slow(sq, blockers);
                let got = rook_attacks(sq, blockers);
                assert_eq!(got, expected,
                    "Rook sq={} i={} blockers=0x{:016X} expected=0x{:016X} got=0x{:016X}",
                    sq, i, blockers, expected, got);
            }
        }
    }

    #[test]
    fn test_magic_bishop_tables_exhaustive() {
        init_slider_tables();
        let mut total_errors = 0;
        for sq in 0..64u8 {
            let mask = bishop_mask(sq);
            let n = 1u64 << mask.count_ones();
            let mut errors = 0;
            for i in 0..n {
                let blockers = index_to_blockers(i, mask);
                let expected = bishop_attacks_slow(sq, blockers);
                let got = bishop_attacks(sq, blockers);
                if got != expected {
                    errors += 1;
                }
            }
            total_errors += errors;
            if errors > 0 {
                use std::collections::HashMap;
                let t = tables();
                let magic = t.bishop_magics[sq as usize];
                let shift = BISHOP_SHIFTS[sq as usize] as usize;
                let offset = t.bishop_offsets[sq as usize];
                let mut seen: HashMap<usize, u64> = HashMap::new();
                let mut collisions = 0;
                for i in 0..n {
                    let blockers = index_to_blockers(i, mask);
                    let idx = offset + ((blockers.wrapping_mul(magic)) >> shift) as usize;
                    if let Some(&first_blockers) = seen.get(&idx) {
                        eprintln!(
                            "  sq={} COLLISION idx={}: blockers 0x{:016X} and 0x{:016X} map to same slot",
                            sq, idx, first_blockers, blockers
                        );
                        collisions += 1;
                    } else {
                        seen.insert(idx, blockers);
                    }
                    if blockers == 0 {
                        eprintln!("  sq={} blockers=0 → idx={} (in range {offset}..{})", sq, idx, offset + n as usize);
                    }
                }
                eprintln!(
                    "BISHOP ERRORS sq={} mask_bits={} errors={}/{} collisions={} table_size={}",
                    sq, mask.count_ones(), errors, n, collisions, n
                );
            }
        }
        assert_eq!(total_errors, 0, "Total bishop magic mismatches: {total_errors}");
    }
}
