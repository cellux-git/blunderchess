# 02 — Eliminate heap allocation in order_moves

**Status**: `completed`
**Category**: `enhancement`

## Problem

`order_moves` (search.rs ~466) and `order_moves_q` (search.rs ~542) use `sort_by_cached_key`, which heap-allocates a `Vec` to store cached sort keys. Called at every interior node — one allocation per node.

## Solution

1. Add a pre-allocated scores buffer to `SearchState`:
```rust
scores_buf: [i32; MAX_MOVES], // 218 elements, 872 bytes on stack
```

2. Rewrite `order_moves` to score manually then `sort_unstable_by_key`:
```rust
for i in 0..moves.len() {
    state.scores_buf[i] = score_move(moves[i], board, hash_move, ply, state, thread_id);
}
moves.sort_unstable_by_key(|i| {
    let idx = moves.as_ptr().offset_from(i as *const Move) as usize;
    -state.scores_buf[idx] // descending
});
```

3. Same treatment for `order_moves_q`.

## Acceptance criteria

- [ ] Zero heap allocations in `order_moves` and `order_moves_q`
- [ ] Move ordering quality unchanged (same scores, same sort result)
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] No NPS regression (expected improvement from removed allocation)

## Out of scope

- Changing the scoring function itself

## Comments
