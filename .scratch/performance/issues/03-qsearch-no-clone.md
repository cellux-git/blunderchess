# 03 — Remove Board clone from quiescence move generation

**Status**: `completed`
**Category**: `enhancement`

## Problem

`generate_legal_moves` in `src/movegen.rs` does `let mut b = board.clone()` internally for legality verification. This is called from quiescence (`search.rs ~503`) — every quiescence node clones the Board (heap allocation for `piece_list` Vec + `history` Vec).

The alpha_beta function already uses `generate_pseudo_legal` + inline legality filtering (make/unmake on the live board). Quiescence should do the same.

## Solution

In `quiescence()`, replace:
```rust
let move_count = movegen::generate_legal_moves(board, &mut moves_buf);
```
with:
```rust
let mut count = 0;
movegen::generate_pseudo_legal(board, &mut moves_buf, &mut count);
// Filter in-place using make_move/unmake like alpha_beta does
```

Mirror the legality filtering pattern from `alpha_beta` (search.rs ~322-367). Only apply make/unmake to edge cases (king moves, en passant, castling, pinned pieces); pass trivially-legal moves directly.

## Acceptance criteria

- [ ] `generate_legal_moves` no longer called from quiescence
- [ ] Quiescence filters moves in-place without Board clone
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] Perft numbers unchanged (perft uses `generate_legal_moves` — don't touch that path)
- [ ] No NPS regression (expected improvement)
- [ ] No functional change in search behavior

## Comments
