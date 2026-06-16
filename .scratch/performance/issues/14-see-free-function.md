# 14 — Decouple SEE from Eval entirely (hot-path optimization)

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## Problem

Even with the static `&Eval` ref from issue #01, SEE currently calls `self.material_value(piece)` which involves a method call and field access. SEE is called per capture in move ordering — dozens of times per node.

## Solution

Make SEE a pure free function that uses compile-time constant piece values:

```rust
const SEE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];

pub fn see(board: &Board, mv: Move) -> i32 {
    // Use SEE_VALUES[p as usize] instead of self.material_value(p)
}
```

This eliminates the `&Eval` dependency entirely from the move ordering hot path. SEE values match the default `Eval` values — tuning Eval doesn't affect SEE, but the consistency argument from architecture issue #05 still holds: if you tune Eval piece values, you should also update SEE_VALUES.

## Acceptance criteria

- [ ] SEE uses inline constants, not `Eval` method calls
- [ ] `see()` is a free function again (no `&self`)
- [ ] All 5 SEE tests pass
- [ ] All 86 unit tests pass
- [ ] No NPS regression (expected slight improvement)
- [ ] `Eval::see()` method removed or converted to delegate to free function

## Comments

Builds on #05 from architecture deepening. The inverse of that refactor — optimizing for speed over consistency.
