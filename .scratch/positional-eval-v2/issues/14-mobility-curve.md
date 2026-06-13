# 14 — Logarithmic mobility tables

**Status**: `completed`
**Category**: `enhancement`

## What's missing

Mobility tables are currently linear: `[-20, -8, 0, 4, 8, 12, 15, 18, 20]` for knights. The jump from 0 to 1 safe squares is +8cp, same as from 6 to 7.

Real mobility follows a logarithmic curve: the first few safe squares matter most (a piece going from 0 to 2 squares is a big improvement), while additional squares beyond ~6-8 have diminishing returns (a knight with 7 moves instead of 6 is only marginally better).

## What to change

Replace the linear mobility tables with logarithmic curves. Example for knights (9 entries):

```
Current (linear):  [-20, -8, 0, 4, 8, 12, 15, 18, 20]
Logarithmic:       [-20, -4, 8, 14, 18, 20, 21, 22, 22]
```

Where each additional safe square gives less bonus than the previous one. Same treatment for bishops, rooks, queens.

```rust
// No new Eval fields needed — just change the default values of existing tables
```

## Acceptance criteria

- [ ] Mobility tables follow logarithmic curve (diminishing returns per additional safe square)
- [ ] Baseline: 0 safe squares still penalized similarly (don't change the floor)
- [ ] Peak mobility (~max safe squares) still rewarded but the curve is flatter at the top
- [ ] All existing tests pass
- [ ] No NPS regression

## References

PRD gap #14.

## Comments
