# 01 — Encapsulate Board internals

**Status**: `completed`
**Category**: `enhancement`

## Problem

`Board` has 13 pub fields directly accessed by eval, search, book, and UCI. ~30 field accesses in eval.rs alone. No invariants enforced. Any change to board representation ripples through all callers.

## What to change

Make all `Board` fields private. Add accessor methods:

| Current pub field | Replace with |
|-------------------|--------------|
| `squares: [Option<Piece>; 64]` | `fn piece_at(sq) -> Option<Piece>` (exists) |
| `colors: [Option<Color>; 64]` | Delete — not used outside board.rs |
| `piece_list: Vec<(Square, Piece, Color)>` | `fn pieces(color) -> Iter` or `fn piece_list() -> &[(Square, Piece, Color)]` |
| `pieces_bb: [Bitboard; 6]` | `fn pieces_bb(piece) -> Bitboard`, `fn piece_count(piece) -> u32` |
| `colors_bb: [Bitboard; 2]` | `fn colors_bb(color) -> Bitboard` |
| `occupancy: Bitboard` | `fn occupancy() -> Bitboard` |
| `side_to_move: Color` | `fn side_to_move() -> Color` (exists) |
| `castling_rights: CastlingRights` | `fn castling_rights() -> CastlingRights`, `fn can_castle(color, side)` |
| `en_passant: Option<Square>` | `fn en_passant() -> Option<Square>` (exists) |
| `halfmove_clock: u8` | `fn halfmove_clock() -> u8` |
| `fullmove_number: u16` | `fn fullmove_number() -> u16` |
| `hash: u64` | `fn hash() -> u64` (exists) |
| `king_square: [Square; 2]` | `fn king_square(color) -> Square` (exists) |
| `history: Vec<u64>` | Not directly exposed — hash repetition check stays internal |

Keep mutation through `make_move`/`unmake_move`/`make_null_move`/`unmake_null_move` — no setter methods needed.

## Key interfaces

- `Board` struct — all fields become private
- Accessor methods as listed above — zero-allocation inline getters
- Existing mutation methods unchanged
- `Piece` enum / `Color` — helpers like `pieces_bb(piece)` and `colors_bb(color)` delegate to internal arrays
- `Board::from_fen()` — unchanged (construction path)
- `Board::from_initial()` — unchanged

## Acceptance criteria

- [ ] All 13 Board fields are private
- [ ] All ~30 direct field accesses in eval.rs replaced with accessor calls
- [ ] All field accesses in search.rs, book.rs, uci.rs replaced
- [ ] All 86 unit tests pass
- [ ] All 10 tactical integration tests pass
- [ ] No NPS regression (benchmarks at ≥100K at depth 6 release)
- [ ] Accessor methods are `#[inline]` — no function-call overhead in release

## Out of scope

- Changing Board representation (mailbox → full bitboard, adding incremental eval, etc.)
- Extracting Board sub-modules (pin detection, draw detection)
- Changing the make_move/unmake_move interface

## Comments
