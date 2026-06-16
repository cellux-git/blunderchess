# 06 — game_phase() use bitboard popcount instead of piece_list iteration

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## Problem

`evaluate()` calls `game_phase()` which iterates `board.piece_list()` to count pieces by type. Then `evaluate_side()` iterates `piece_list()` again for the material+PST loop. The piece list is iterated 3 times per `evaluate()` call.

## Solution

Replace `game_phase()` with bitboard popcounts:

```rust
fn game_phase(&self, board: &Board) -> i32 {
    let knights = board.pieces_bb(Piece::Knight).count_ones() as i32;
    let bishops = board.pieces_bb(Piece::Bishop).count_ones() as i32;
    let rooks = board.pieces_bb(Piece::Rook).count_ones() as i32;
    let queens = board.pieces_bb(Piece::Queen).count_ones() as i32;
    (knights + bishops + rooks * 2 + queens * 4).min(24)
}
```

`count_ones()` is a single CPU instruction (POPCNT on x86_64). For 4 bitboards, this is ~4 cycles, vs iterating a Vec of up to 32 entries.

## Acceptance criteria

- [ ] `game_phase()` uses bitboard popcounts, not piece_list iteration
- [ ] Same result as before (test with a few positions)
- [ ] All 86 unit tests pass
- [ ] No NPS regression (expected improvement)

## Comments
