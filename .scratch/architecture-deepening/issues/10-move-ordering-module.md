# 10 — Extract MoveOrdering module from search.rs

**Category:** improvement
****Status:** completed

## Summary

Move ordering (`order_moves`, `order_moves_q`), the killer-move table (2 slots × MAX_DEPTH), and the 64×64 history heuristic are embedded in `src/search.rs`. They are tested only indirectly through integration search tests. Extract them into a standalone module that owns its state and can be unit-tested.

## Current state

- `order_moves()` (line 499) — scores and sorts moves: hash move (∞), SEE-based captures, promotions, killers, history
- `order_moves_q()` (line 583) — scores captures + promotions for quiescence
- `SearchState.killers: [[Option<Move>; 2]; MAX_DEPTH]` — heap-allocated killer table
- `SearchState.history: [[i32; 64]; 64]` — from-square → to-square history scores
- Killer updates (lines 471-479) — interleaved with alpha_beta cutoff logic

## Desired interface

```rust
pub struct MoveOrdering {
    killers: [[Option<Move>; 2]; MAX_DEPTH],
    history: [[i32; 64]; 64],
}

impl MoveOrdering {
    pub fn new() -> Self;
    pub fn order_moves(&self, moves: &mut [Move], board: &Board, hash_move: Option<Move>, ply: u8, thread_id: u8);
    pub fn order_moves_q(&self, moves: &mut [Move], board: &Board);
    pub fn record_killer(&mut self, mv: Move, ply: u8);
    pub fn record_history(&mut self, from: Square, to: Square, depth: u8);
}
```

## Acceptance criteria

- [ ] `src/move_ordering.rs` created with `MoveOrdering` struct + interface above
- [ ] `order_moves`, `order_moves_q`, killer logic, and history logic moved out of search.rs
- [ ] `SearchState` no longer holds `killers` or `history` fields
- [ ] `alpha_beta` passes `&mut MoveOrdering` and calls `record_killer`/`record_history` on beta cutoff
- [ ] Unit tests added: killer outranks history, SEE winning capture outranks quiet, history perturbation by thread_id
- [ ] `cargo test --lib --test benchmarks` passes cleanly
- [ ] No NPS regression
