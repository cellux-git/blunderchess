# 05 — Fast-path attack table access without branch

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## Problem

`rook_attacks()` and `bishop_attacks()` call `tables()` which uses `OnceLock::get_or_init()`. After initialization, this still does an atomic load + branch to check initialization status. These functions are in the absolute hottest path:

- `is_attacked_by` — every legality check
- `attackers_to` — SEE, queen fork detection
- `eval_mobility` — per piece per node
- `pinned_pieces` — per node

At 270K NPS with ~30 pieces × mobility + ~30 attacks per legality check, this branch executes millions of times per second.

## Solution

Since `init_slider_tables()` is guaranteed to be called in `main()` before any search, add an unchecked accessor:

```rust
#[inline]
fn tables_unchecked() -> &'static AttackTables {
    unsafe { TABLES.get().unwrap_unchecked() }
}
```

Use `tables_unchecked()` in the hot inline functions (`rook_attacks`, `bishop_attacks`, `queen_attacks`, `attackers_to`). Keep `tables()` for `init_slider_tables()` and test/diagnostic code.

The `unsafe` is minimal and well-justified: we guarantee initialization in `main()`.

## Acceptance criteria

- [ ] `tables_unchecked()` added as `#[inline]` private function
- [ ] `rook_attacks`, `bishop_attacks`, `queen_attacks`, `attackers_to` use `tables_unchecked()`
- [ ] `init_slider_tables()` called in `main.rs` before any search
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] No NPS regression (expected slight improvement)

## Comments
