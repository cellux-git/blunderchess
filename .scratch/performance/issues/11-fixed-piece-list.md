# 11 — Replace piece_list Vec with fixed-size array

**Status**: `completed`
**Category**: `enhancement`

## Problem

`Board::piece_list` is `Vec<(Square, Piece, Color)>`. At most 32 pieces exist. The Vec forces heap allocation on every `Board::clone()` — which happens in `generate_legal_moves` and per-thread in `search_mt`. While cloning is reduced by issue #03, it still happens during multi-threaded startup and perft.

## Solution

Replace with a fixed-size array:

```rust
// In board.rs
pub(crate) piece_list: [(Square, Piece, Color); 32],
pub(crate) piece_count: u8, // 0..32
```

Update all iteration patterns from `for &(sq, piece, pc) in &self.piece_list` to `for i in 0..self.piece_count as usize { let (sq, piece, pc) = self.piece_list[i]; }`.

`Board::clone()` becomes a simple `memcpy` of the fixed array (256 bytes for the array + 1 byte for count).

## Acceptance criteria

- [ ] `piece_list` is a fixed `[(Square, Piece, Color); 32]` with explicit count
- [ ] All piece_list iterations updated
- [ ] Board::clone() no longer heap-allocates (except `history` Vec)
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] Perft numbers unchanged

## Comments
