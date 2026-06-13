# 11 — Space / center control

**Status**: `completed`
**Category**: `enhancement`

## What's missing

Control of the center 4 squares (d4, e4, d5, e5) and the expanded center (c3-c6, f3-f6) is not evaluated. A space advantage — having more pawns advanced into the opponent's half and controlling more central squares — is a key middlegame concept.

Currently: PST tables give central squares better values, but there's no explicit "our side controls more space" bonus.

## What to add

Count how many center/expanded-center squares are:
- Occupied by own pawns
- Attacked by own pawns or pieces
- Not contested by enemy pawns/pieces

Apply a space bonus proportional to (own control − enemy control).

```rust
pub space_bonus: (i32, i32), // mg, eg — per controlled center square
pub space_center_squares: u64, // bitboard of squares to count (d4/e4/d5/e5)
```

Simpler approach: count own pawns on ranks 4-6 (white) or ranks 3-5 (black) as space-gaining. Each such pawn gets a small bonus. This is what many engines do and is much simpler to implement.

## References

PRD gap #11.

## Comments
