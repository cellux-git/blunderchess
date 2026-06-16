# 06 — Bug fixes: mobility consistency + king-passer MG

**Status:** `completed`

**Status**: `completed`
**Category**: `bug`

## 1. Bishop mobility excludes enemy squares (asymmetry)

In `eval_mobility` (line 541–548), bishop safe-square counting excludes enemy-occupied squares:

```rust
let safe = (attacks & !us_bb & !enemy_bb).count_ones() as usize;
```

But knights (line 536), rooks (line 554), and queens (line 563) all use:

```rust
let safe = (attacks & !us_bb).count_ones() as usize;
```

This undervalues bishops relative to other pieces — attacking an enemy piece is a valid "mobile" square. Fix: make bishop use `& !us_bb` like the others. Update mobility table bounds if needed (bishops currently cap at 13 — may need to increase to match new max safe-square count including enemy squares).

## 2. King-passer proximity MG is a dead parameter

`eval_king_passer_proximity` takes `_mg: &mut i32` (unused, line 506). The bonus is only applied to `eg`. In queenless middlegames, king proximity to a passer matters. Apply a reduced MG bonus (e.g. 50% of EG).

```rust
pub king_passer_proximity_bonus_mg: i32, // separate from EG bonus (currently 10)
```

## Acceptance criteria

- [ ] Bishop mobility counts enemy-occupied squares in safe-square count
- [ ] Bishop mobility table bounds adjusted if needed
- [ ] King-passer proximity applies bonus to both MG and EG
- [ ] All 89 existing tests pass
- [ ] No NPS regression

## Comments
