# 04 — Replace unsafe statics in attack.rs

**Status:** `completed`

**Status**: `completed`
**Category**: `bug`

## Problem

Five `static mut` arrays in `attack.rs` are accessed through `unsafe` blocks on every sliding-piece lookup. `init_slider_tables()` is called as a guard inside every attack function, adding a branch on every access. This is a Rust anti-pattern — `OnceLock` was designed for exactly this case.

Current state:
```rust
static mut ROOK_TABLE: [u64; 102400] = [0; 102400];
static mut BISHOP_TABLE: [u64; 5248] = [0; 5248];
// ... 3 more statics
```

Access pattern:
```rust
pub fn rook_attacks(sq: u8, occ: u64) -> Bitboard {
    init_slider_tables();  // branch on every call
    let idx = unsafe { ROOK_OFFSETS[sq] + ... };
    unsafe { ROOK_TABLE[idx] }
}
```

## What to change

1. Define a struct holding all tables:
```rust
struct AttackTables {
    rook_table: Box<[u64]>,
    rook_offsets: [u16; 64],
    bishop_table: Box<[u64]>,
    bishop_offsets: [u16; 64],
    bishop_magics: [u64; 64],
}
```

2. Store in a `OnceLock<AttackTables>`:
```rust
static TABLES: OnceLock<AttackTables> = OnceLock::new();
```

3. Access functions use `TABLES.get().unwrap()` (safe, panics if not initialized):
```rust
pub fn rook_attacks(sq: u8, occ: u64) -> Bitboard {
    let tables = TABLES.get().expect("attack tables not initialized");
    // safe indexing
}
```

4. Remove per-call `init_slider_tables()` guard. Initialize once in `main()` or via `get_or_init`.

## Key interfaces

- `init_slider_tables()` — call once at startup; becomes `TABLES.get_or_init(|| AttackTables::new())`
- `rook_attacks()`, `bishop_attacks()`, `queen_attacks()` — unchanged return types; no unsafe blocks
- `knight_attacks()`, `king_attacks()`, `pawn_attacks()` — unchanged (already const-compiled)

## Acceptance criteria

- [ ] Zero `static mut` in attack.rs
- [ ] Zero `unsafe` blocks in attack functions
- [ ] `init_slider_tables()` called exactly once at startup
- [ ] No branch on every attack function call
- [ ] All 86 unit tests pass
- [ ] All 10 tactical integration tests pass
- [ ] No NPS regression (may improve slightly due to removed branch)

## Out of scope

- Changing magic number generation (bishop magics still runtime-generated, rook magics still CPW constants)
- Moving attack tables to a different module

## Comments
