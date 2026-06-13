# 10 — Only zero used PV length entries per iteration

**Status**: `completed`
**Category**: `enhancement`

## Problem

`search_worker` does `state.pv_length = [0; MAX_DEPTH as usize]` (128 × 8 = 1024 bytes zeroed) every iterative deepening iteration. Most iterations use only a fraction of this (depth 1 uses 1 entry, depth 10 uses 10 entries).

## Solution

Replace the full array reset with a targeted clear:

```rust
state.pv_length[..(depth as usize + 1)].fill(0);
```

This zeros only the entries actually used by the current depth.

## Acceptance criteria

- [ ] Only used PV length entries zeroed per iteration
- [ ] All 13 search tests pass (PV collection test especially)
- [ ] PV collection correctness unchanged
- [ ] No functional change

## Comments
