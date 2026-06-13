# 01 — Replace Eval::default() with static reference

**Status**: `completed`
**Category**: `enhancement`

## Problem

`Eval::default()` is called 6+ times per search node, each creating a ~4000-byte stack copy. At 270K NPS, this is ~6.5 GB/sec of useless memory copying.

Call sites in `src/search.rs`:
- Line ~268: horizon eval
- Line ~320: static eval for futility pruning
- Line ~470: SEE in `order_moves` (called per capture)
- Line ~489: quiescence horizon
- Line ~498: stand-pat eval
- Line ~544: SEE in `order_moves_q` (called per capture)

## Solution

1. Add a `LazyLock` static in `src/eval/params.rs`:
```rust
pub fn default_eval() -> &'static Eval {
    static DEFAULT: std::sync::LazyLock<Eval> = std::sync::LazyLock::new(Eval::default);
    &DEFAULT
}
```

2. Replace all `Eval::default().evaluate(board)` → `default_eval().evaluate(board)` in search.rs.
3. Replace all `Eval::default().see(board, mv)` → `default_eval().see(board, mv)`.

Also update the `see()` method to accept `&self` (already done). Update `tests/benchmarks.rs` and integration tests if they call `Eval::default()` directly.

## Acceptance criteria

- [ ] Single `LazyLock` static holds the default Eval
- [ ] All search hot-path calls use `&'static Eval` (pointer load, not 4KB copy)
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] No NPS regression (expected improvement)
- [ ] Zero warnings

## Comments
