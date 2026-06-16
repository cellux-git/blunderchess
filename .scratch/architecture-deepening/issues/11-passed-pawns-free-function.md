# 11 — Make passed_pawns a free function (remove dead &self)

**Status:** `completed`

**Category:** improvement
****Status:** completed

## Summary

`passed_pawns()` in `src/eval/pawns.rs` takes `&self` but reads zero fields from `self`. It is a pure bitboard computation — count enemy pawns ahead on adjacent files. The dead parameter couples callers to the full `Eval` struct unnecessarily and prevents callers from using the function without constructing an `Eval`.

## Change

```rust
// Before
pub(crate) fn passed_pawns(&self, pawns_bb: u64, enemy_pawns_bb: u64, color: Color) -> u64

// After
pub(crate) fn passed_pawns(pawns_bb: u64, enemy_pawns_bb: u64, color: Color) -> u64
```

Update the two call sites to remove `self.`:

- `evaluate_side` (mod.rs line ~120): `self.passed_pawns(...)` → `passed_pawns(...)`
- `eval_passer_blocker` (pawns.rs): `self.passed_pawns(...)` → `passed_pawns(...)`

## Acceptance criteria

- [ ] `passed_pawns` is a free function (no `&self`)
- [ ] Both call sites updated
- [ ] `cargo test --lib --test benchmarks` passes cleanly
- [ ] Audit: no other `impl Eval` methods have unused `&self` parameters
