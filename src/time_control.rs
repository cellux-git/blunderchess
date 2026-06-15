use crate::types::Color;

pub fn allocate_time(
    side: Color,
    wtime: Option<u64>,
    btime: Option<u64>,
    winc: Option<u64>,
    binc: Option<u64>,
    movestogo: Option<u8>,
) -> Option<u64> {
    let (time_left, inc) = match side {
        Color::White => (wtime?, winc.unwrap_or(0)),
        Color::Black => (btime?, binc.unwrap_or(0)),
    };
    let moves_left = movestogo.unwrap_or(30).max(1) as u64;
    let base = time_left.saturating_div(moves_left);
    let allocation = (base + inc).saturating_sub(50);
    let cap = (time_left.saturating_div(4)).min(30_000);
    Some(allocation.clamp(10, cap.max(10)))
}
