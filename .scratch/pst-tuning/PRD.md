# PST Tuning: reduce positional overcompensation


**Status:** `completed`

## Problem

The piece-square tables (PST) plus other positional eval terms (mobility, king safety, pawn structure) are too aggressive — they give ~286 cp of positional credit to positions that don't warrant it. This causes material losses to be masked.

## Root cause

Positional eval terms (PST + mobility + king safety + pawn structure + space + etc.) sum to large values that can offset significant material deficits. A side down a knight (-320 cp) can still evaluate as +84 cp ahead because positional terms contribute +286 cp.

This is visible in the `a4b5 f6e5` test case (`src/search/tests.rs:test_a4b5_detected_as_material_loss`):

```
Position after a4b5 f6e5:
  Material: White 8P+0N+2B+2R+Q = 23360, Black 7P+1N+2B+2R+Q = 23580
  Material diff: -220 (Black up a knight for a pawn)
  Actual eval: White mg=23662, Black mg=23596, diff=+66
  PST + other positional: White +302, Black +16
  Net positional overcompensation: +286 cp
  Search result at depth 6: still plays a4b5 (score +218)
```

## Impact

- Search instability: depth 5 finds Ne5c4 (correct), depth 4 and 6 revert to a4b5 (bad)
- Tuning difficulty: changing individual PST values barely moves the aggregate score because the overcompensation is spread across 10+ eval terms
- Violates ADR-0010 (principles over specifics): the eval relies on many small terms adding up to large values, making it hard to control

## Contributing terms (diagnostic data from a4b5 position)

| Term | White advantage (approx) | Notes |
|------|--------------------------|-------|
| Pawn PST | ~+50 cp | d4 at +55 (row 3), e3 at +20 (row 2) |
| Bishop PST | ~+38 cp | Bf4 at +25, Be2 at +18; Bc8 at -15 |
| Rook PST | ~-35 cp | Rooks on back rank penalized |
| King PST | ~-20 cp | Black king castled (+10) vs White uncastled (-10) |
| Mobility | ~+50 cp (est.) | White pieces more active |
| King safety | ~+75 cp (est.) | Black king has exposed shield (f6 gone) |
| Space/other | ~+50 cp (est.) | White has more space |
| **Total** | **~286 cp** | Offsets -220 cp material deficit |

## Suggested fixes

1. **Further PST moderation**: reduce mg_pawn_table rows 2-4, mg_bishop_table development rows, mg_knight_table central squares, mg_rook_table activation values. All should form smooth gradients without spikes.

2. **Material scaling**: scale positional terms down when a side is down material. A side down a knight should not get full positional credit — material matters.

3. **Cap positional totals**: limit the total positional bonus per side to prevent individual terms from accumulating past a threshold.

4. **Term-by-term audit**: trace each eval term in the a4b5 position and verify it's calibrated correctly.

## Related

- `test_a4b5_detected_as_material_loss` in `src/search/tests.rs` (currently passes at depth 5, fails at depth 6)
- ADR-0010 (evaluation principles)
- Previous PST fixes: mg_pawn_table row 2, mg_bishop_table rows 0/1/6/7
