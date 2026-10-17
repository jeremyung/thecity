use crate::*;

pub(crate) fn rng_next(state: &mut u64) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x >> 16) as u32
}

pub(crate) fn random_unit(state: &mut u64) -> f32 {
    (rng_next(state) as f64 / u32::MAX as f64) as f32
}

pub(crate) fn clock_at(tick: u64) -> (u32, f32) {
    let day = (tick / TICKS_PER_DAY) as u32;
    let elapsed = (tick % TICKS_PER_DAY) as f32 * 1440.0 / TICKS_PER_DAY as f32;
    (day, (360.0 + elapsed) % 1440.0)
}

pub(crate) fn next_clock_deadline(tick: u64, minute: u16) -> u64 {
    let minute = minute as u64 % 1440;
    let minute_from_dawn = (minute + 1440 - 360) % 1440;
    let offset = minute_from_dawn * TICKS_PER_DAY / 1440;
    let day_start = tick / TICKS_PER_DAY * TICKS_PER_DAY;
    let current_offset = tick % TICKS_PER_DAY;
    if current_offset < offset {
        day_start + offset
    } else {
        day_start + TICKS_PER_DAY + offset
    }
}

pub(crate) fn push_history(history: &mut Vec<String>, entry: String) {
    history.push(entry);
    if history.len() > 8 {
        history.remove(0);
    }
}
