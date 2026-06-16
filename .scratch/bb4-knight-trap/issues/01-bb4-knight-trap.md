# Engine plays Bb4+ — trapped knight after c3 block

**Status:** `completed`

## Position

```
rnbqkb1r/pppp1ppp/4p3/4P3/3Pn3/7N/PPP2PPP/RNBQKB1R b KQkq - 2 4
```

Side to move: **Black**. The engine plays **Bb4+**.

## Problem

Bb4+ is a bad move. White responds **c3**, blocking the check with tempo (the c3 pawn attacks the bishop). After the bishop retreats, Black's knight on e4 is:
- **Undefended** — no pawn or piece protects it
- **Attacked by d4/c3 pawns and can be targeted by White's pieces** (Ng5, Qg4, f3, etc.)
- **No good escape squares** — the knight attacks squares that are either controlled by White pawns (d6, f6 via e5 pawn), or positions where it can be further harassed

The engine does not see that Bb4+ leads to a materially and positionally lost knight.

## Investigation: why the engine doesn't catch this

### 1. Outpost bonus — false positive (`src/eval/pieces.rs:141-149`)

The outpost knight bonus fires for the knight on e4:

| Condition | Status | Why |
|-----------|--------|-----|
| In enemy half | ✅ | Rank 3 ≤ 3 (for Black) |
| Not attacked by enemy pawn | ✅ | d4 attacks e5/c5; e5 attacks d6/f6. Neither attacks e4 |
| Friendly pawn on adjacent file | ✅ | d7 pawn on d-file, f7 pawn on f-file |

However, the outpost check at `pieces.rs:145-146` only tests for the **existence** of a friendly pawn on an adjacent file — it does **not** check whether that pawn actually **defends** the knight's square. Neither d7 nor f7 defends e4. The knight is completely undefended but gets an outpost bonus of **+18 mg / +8 eg**.

### 2. Mobility doesn't catch the danger (`src/eval/mobility.rs:11-19`)

The mobility eval counts "safe" squares as any square not occupied by a friendly piece. The knight on e4 attacks ~6 such squares (c5, d2, f2, g3, g5, etc.). The mobility table gives **+19 mg** for 6 knight moves.

Mobility does not distinguish between:
- Squares controlled by enemy pawns/attacks
- Squares where the knight can be trapped
- Useless squares vs productive squares

### 3. Trapped-knight detection only fires at safe==0 (`src/eval/pieces.rs:157-162`)

The trapped-knight penalty (`-25 mg / -35 eg`) only activates when the knight has **zero** safe squares. The knight on e4 still has 6+ squares, so this never triggers.

### 4. Material loss not detectable via SEE

SEE (Static Exchange Evaluation, `src/eval/see.rs`) only evaluates capture sequences. Bb4+ is a non-capture check, so SEE cannot flag it.

## What should catch this

Per the user's analysis, this position **should not require a new evaluation rule**. The combination of:

1. **Material** — an undefended knight that will be lost should show up as a material deficit deep enough in search. But the search horizon may be too shallow, and leaf-node eval misjudges the knight's value.

2. **Mobility** — if mobility correctly discounted squares that are attacked by enemy pawns or where the piece can be trapped, the knight's poor mobility would surface.

3. **Outpost fix** — the outpost bonus should require the knight to be **defended** by the pawn on the adjacent file, not merely that a pawn exists somewhere on that file.

## Action items

- [ ] Fix outpost detection: require the knight square to be actually defended (`pawn_attacks(sq, color) & my_pawns != 0`), not just that a pawn exists on an adjacent file
- [ ] Consider whether mobility should filter out squares attacked by enemy pawns (not just friendly pieces)
- [ ] Investigate whether a "hanging piece" / "undefended piece" term is needed in evaluation, or whether the search + existing terms should suffice after the outpost fix
- [x] Write a tactical test for this position verifying the engine does **not** play Bb4+ above some depth threshold

## Comments

### 2026-06-16 — implemented

**Two changes to evaluation:**

1. **Outpost fix** (`src/eval/pieces.rs:144-148`): Replaced `adjacent_files_mask(file) & my_pawns != 0` (mere existence of any pawn on adjacent file) with `crate::attack::pawn_attacks(sq, color.flip()) & my_pawns != 0` (actual pawn defense). The knight on e4 in the bug position no longer incorrectly gets the +18/+8 outpost bonus.

2. **Mobility fix** (`src/eval/mobility.rs`): Added `enemy_pawn_attacks` mask — squares attacked by enemy pawns are excluded from the safe-square count for all piece types (N/B/R/Q). The knight on e4 in the bug position gets lower mobility because squares controlled by White's pawns (e5, d4) are no longer counted as safe.

**Tests added:**
- `test_outpost_requires_pawn_defense` — verifies that a knight on e5 with pawn on c4 (adjacent file, but not defending) scores lower than with pawn on d4 (actually defends)
- `test_avoid_bb4_knight_trap` — verifies engine does not play `f8b4` (Bb4+) at depth 5-6

**All 134 unit tests + 12 tactical integration tests pass.**
