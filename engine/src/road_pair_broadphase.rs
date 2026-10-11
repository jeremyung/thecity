//! Deterministic, quota-stepped broad phase for segment AABB pairs.
//!
//! The index uses 128 x-axis buckets. It emits conservative candidates in
//! `(i, j)` lexicographic order and never materializes the full pair graph.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_SEGMENTS: usize = 5_000;
const BUCKETS: usize = 128;
const MAX_COORDINATE: f64 = 100_000.0;
const EPSILON: f64 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub min_x: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    TooManySegments,
    InvalidBounds,
    InvalidCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Advance {
    pub work_used: usize,
    pub pair: Option<(usize, usize)>,
    pub done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    schema: u8,
    bounds: Vec<Bounds>,
    phase: u8,
    min_x: f64,
    max_x: f64,
    cursor: usize,
    bucket_cursor: usize,
    buckets: Vec<Vec<usize>>,
    query_i: usize,
    query_bucket: usize,
    query_member: usize,
    candidates: Vec<usize>,
    emit_cursor: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct BroadPhase {
    bounds: Vec<Bounds>,
    phase: u8, // 0 min/max, 1 bucket insertion, 2 gather, 4 emit, 3 done
    min_x: f64,
    max_x: f64,
    cursor: usize,
    bucket_cursor: usize,
    buckets: Vec<Vec<usize>>,
    query_i: usize,
    query_bucket: usize,
    query_member: usize,
    candidates: BTreeSet<usize>,
    emit_cursor: Option<usize>,
}

impl Bounds {
    fn valid(self) -> bool {
        [self.min_x, self.min_z, self.max_x, self.max_z]
            .into_iter()
            .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE)
            && self.min_x <= self.max_x
            && self.min_z <= self.max_z
    }
}

impl BroadPhase {
    /// Validation is synchronous and capped at 5,000 input records.
    pub fn new(bounds: Vec<Bounds>) -> Result<Self, Error> {
        if bounds.len() > MAX_SEGMENTS {
            return Err(Error::TooManySegments);
        }
        if bounds.iter().any(|b| !b.valid()) {
            return Err(Error::InvalidBounds);
        }
        Ok(Self {
            bounds,
            phase: 0,
            min_x: 0.0,
            max_x: 0.0,
            cursor: 0,
            bucket_cursor: 0,
            buckets: Vec::new(),
            query_i: 0,
            query_bucket: 0,
            query_member: 0,
            candidates: BTreeSet::new(),
            emit_cursor: None,
        })
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            schema: 1,
            bounds: self.bounds.clone(),
            phase: self.phase,
            min_x: self.min_x,
            max_x: self.max_x,
            cursor: self.cursor,
            bucket_cursor: self.bucket_cursor,
            buckets: self.buckets.clone(),
            query_i: self.query_i,
            query_bucket: self.query_bucket,
            query_member: self.query_member,
            candidates: self.candidates.iter().copied().collect(),
            emit_cursor: self.emit_cursor,
        }
    }

    /// Restore validates stored memberships and cursors synchronously. Its cost
    /// is separate from `advance` and bounded by the 640,000 membership cap.
    pub fn restore(c: Checkpoint) -> Result<Self, Error> {
        if c.schema != 1
            || c.bounds.len() > MAX_SEGMENTS
            || c.bounds.iter().any(|b| !b.valid())
            || c.buckets.len() > BUCKETS
            || c.candidates.len() > c.bounds.len()
        {
            return Err(Error::InvalidCheckpoint);
        }
        let phase_ok = match c.phase {
            0 => c.buckets.is_empty() && c.cursor <= c.bounds.len(),
            1 => c.buckets.len() == BUCKETS && c.cursor <= c.bounds.len(),
            5 => c.buckets.len() == c.cursor && c.cursor <= BUCKETS,
            2 | 4 => c.buckets.len() == BUCKETS && c.query_i < c.bounds.len().saturating_sub(1),
            3 => {
                (c.bounds.is_empty() && c.buckets.is_empty() && c.query_i == 0)
                    || (c.buckets.len() == BUCKETS && c.query_i == c.bounds.len().saturating_sub(1))
            }
            _ => false,
        };
        if !phase_ok || c.buckets.iter().any(|b| b.len() > MAX_SEGMENTS) {
            return Err(Error::InvalidCheckpoint);
        }
        if c.phase != 0 {
            let actual_min = c.bounds.iter().map(|b| b.min_x).fold(0.0, f64::min);
            let actual_max = c.bounds.iter().map(|b| b.max_x).fold(0.0, f64::max);
            if c.min_x != actual_min || c.max_x != actual_max {
                return Err(Error::InvalidCheckpoint);
            }
        }
        if c.phase == 0 {
            let prefix = &c.bounds[..c.cursor];
            let expected_min = prefix.iter().map(|b| b.min_x).fold(0.0, f64::min);
            let expected_max = prefix.iter().map(|b| b.max_x).fold(0.0, f64::max);
            if c.min_x != expected_min || c.max_x != expected_max {
                return Err(Error::InvalidCheckpoint);
            }
        }
        // Enforce stage-local cursors, including fields unused by that stage.
        // Otherwise a malformed checkpoint can bypass index construction or
        // jump past unvisited buckets while still containing plausible pairs.
        let idle_query = c.query_i == 0 && c.query_bucket == 0 && c.query_member == 0;
        let no_emit = c.emit_cursor.is_none();
        let canonical = match c.phase {
            0 => c.bucket_cursor == 0 && idle_query && no_emit,
            5 => {
                !c.bounds.is_empty()
                    && c.bucket_cursor == 0
                    && idle_query
                    && no_emit
                    && c.buckets.iter().all(Vec::is_empty)
            }
            1 => {
                !c.bounds.is_empty()
                    && idle_query
                    && no_emit
                    && if c.cursor == c.bounds.len() {
                        c.bucket_cursor == 0
                    } else {
                        let (lo, hi) = bucket_range(c.bounds[c.cursor], c.min_x, c.max_x);
                        (lo..=hi + 1).contains(&c.bucket_cursor)
                    }
            }
            2 | 4 => {
                let (lo, hi) = bucket_range(c.bounds[c.query_i], c.min_x, c.max_x);
                let initial = c.query_bucket == 0 && c.query_member == 0;
                let in_scan = (lo..=hi).contains(&c.query_bucket)
                    && c.query_member <= c.buckets[c.query_bucket].len();
                let scanned = c.query_bucket == hi + 1 && c.query_member == 0;
                c.cursor == c.bounds.len()
                    && c.bucket_cursor == 0
                    && if c.phase == 2 {
                        no_emit && (initial || in_scan || scanned)
                    } else {
                        scanned
                            && !no_emit
                            && c.emit_cursor.unwrap_or(0) >= c.query_i + 2
                            && c.emit_cursor.unwrap_or(0) <= c.bounds.len()
                            && overlaps(c.bounds[c.query_i], c.bounds[c.emit_cursor.unwrap() - 1])
                    }
            }
            3 => {
                c.cursor == c.bounds.len()
                    && c.bucket_cursor == 0
                    && c.query_bucket == 0
                    && c.query_member == 0
                    && no_emit
            }
            _ => false,
        };
        if !canonical {
            return Err(Error::InvalidCheckpoint);
        }
        let mut total = 0usize;
        for (bi, bucket) in c.buckets.iter().enumerate() {
            total = total.saturating_add(bucket.len());
            if total > MAX_SEGMENTS * BUCKETS || bucket.iter().any(|&idx| idx >= c.bounds.len()) {
                return Err(Error::InvalidCheckpoint);
            }
            if bucket.windows(2).any(|w| w[0] >= w[1]) {
                return Err(Error::InvalidCheckpoint);
            }
            if matches!(c.phase, 1 | 2 | 3 | 4) {
                let expected: Vec<_> = c
                    .bounds
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, b)| {
                        let (lo, hi) = bucket_range(*b, c.min_x, c.max_x);
                        let built = matches!(c.phase, 2 | 3 | 4)
                            || idx < c.cursor
                            || (idx == c.cursor && bi < c.bucket_cursor);
                        (built && bi >= lo && bi <= hi).then_some(idx)
                    })
                    .collect();
                if *bucket != expected {
                    return Err(Error::InvalidCheckpoint);
                }
            }
            // Memberships must correspond to the deterministic full build.
            for &idx in bucket {
                let (lo, hi) = bucket_range(c.bounds[idx], c.min_x, c.max_x);
                if bi < lo || bi > hi {
                    return Err(Error::InvalidCheckpoint);
                }
            }
        }
        if c.candidates.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::InvalidCheckpoint);
        }
        if matches!(c.phase, 2 | 4) {
            if c.query_i >= c.bounds.len().saturating_sub(1)
                || c.query_bucket > BUCKETS
                || c.query_member > MAX_SEGMENTS
            {
                return Err(Error::InvalidCheckpoint);
            }
            if c.query_bucket < BUCKETS && c.query_member > c.buckets[c.query_bucket].len() {
                return Err(Error::InvalidCheckpoint);
            }
        }
        for &j in &c.candidates {
            if j <= c.query_i || j >= c.bounds.len() || !overlaps(c.bounds[c.query_i], c.bounds[j])
            {
                return Err(Error::InvalidCheckpoint);
            }
        }
        let emit_cursor = c.emit_cursor;
        if let Some(j) = emit_cursor {
            if j < c.query_i + 1 || j > c.bounds.len() {
                return Err(Error::InvalidCheckpoint);
            }
        }
        // Prove the candidate set is the complete prefix of the query scan.
        // This costs at most the bounded bucket membership count on restore.
        if c.phase == 2 || c.phase == 4 {
            let (lo, hi) = bucket_range(c.bounds[c.query_i], c.min_x, c.max_x);
            let through_bucket = if c.phase == 4 {
                hi
            } else {
                c.query_bucket.min(hi)
            };
            let mut expected = BTreeSet::new();
            if lo <= through_bucket {
                for bi in lo..=through_bucket {
                    let members = &c.buckets[bi];
                    let limit = if c.phase == 2 && bi == c.query_bucket {
                        c.query_member.min(members.len())
                    } else {
                        members.len()
                    };
                    for &j in &members[..limit] {
                        if j > c.query_i && overlaps(c.bounds[c.query_i], c.bounds[j]) {
                            expected.insert(j);
                        }
                    }
                }
            }
            if c.phase == 4 {
                let emitted_before = emit_cursor.unwrap_or(0);
                expected.retain(|&j| j >= emitted_before);
            }
            if expected.iter().copied().collect::<Vec<_>>() != c.candidates {
                return Err(Error::InvalidCheckpoint);
            }
        } else if !c.candidates.is_empty() {
            return Err(Error::InvalidCheckpoint);
        }
        Ok(Self {
            bounds: c.bounds,
            phase: c.phase,
            min_x: c.min_x,
            max_x: c.max_x,
            cursor: c.cursor,
            bucket_cursor: c.bucket_cursor,
            buckets: c.buckets,
            query_i: c.query_i,
            query_bucket: c.query_bucket,
            query_member: c.query_member,
            candidates: c.candidates.into_iter().collect(),
            emit_cursor,
        })
    }

    pub fn advance(&mut self, quota: usize) -> Advance {
        let mut used = 0;
        if quota == 0 {
            return Advance {
                work_used: 0,
                pair: None,
                done: self.phase == 3,
            };
        }
        while used < quota && self.phase != 3 {
            match self.phase {
                0 => {
                    if self.cursor < self.bounds.len() {
                        let b = self.bounds[self.cursor];
                        self.min_x = self.min_x.min(b.min_x);
                        self.max_x = self.max_x.max(b.max_x);
                        self.cursor += 1;
                        used += 1;
                    } else {
                        if self.bounds.is_empty() {
                            self.phase = 3;
                            continue;
                        }
                        self.cursor = 0;
                        self.buckets.clear();
                        self.phase = 5;
                    }
                }
                5 => {
                    if self.cursor < BUCKETS {
                        self.buckets.push(Vec::new());
                        self.cursor += 1;
                        used += 1;
                    } else {
                        self.cursor = 0;
                        self.bucket_cursor = bucket_range(self.bounds[0], self.min_x, self.max_x).0;
                        self.phase = 1;
                    }
                }
                1 => {
                    if self.cursor == self.bounds.len() {
                        self.phase = 2;
                        self.query_i = 0;
                        continue;
                    }
                    let (_, hi) = bucket_range(self.bounds[self.cursor], self.min_x, self.max_x);
                    if self.bucket_cursor <= hi {
                        self.buckets[self.bucket_cursor].push(self.cursor);
                        self.bucket_cursor += 1;
                        used += 1;
                    } else {
                        self.cursor += 1;
                        self.bucket_cursor = if self.cursor < self.bounds.len() {
                            bucket_range(self.bounds[self.cursor], self.min_x, self.max_x).0
                        } else {
                            0
                        };
                    }
                }
                2 => {
                    if self.query_i + 1 >= self.bounds.len() {
                        self.phase = 3;
                        continue;
                    }
                    if self.query_bucket == 0 && self.query_member == 0 {
                        self.query_bucket =
                            bucket_range(self.bounds[self.query_i], self.min_x, self.max_x).0;
                    }
                    let (_, hi) = bucket_range(self.bounds[self.query_i], self.min_x, self.max_x);
                    if self.query_bucket <= hi {
                        let bucket = &self.buckets[self.query_bucket];
                        if self.query_member < bucket.len() {
                            let j = bucket[self.query_member];
                            self.query_member += 1;
                            used += 1;
                            if j > self.query_i
                                && overlaps(self.bounds[self.query_i], self.bounds[j])
                            {
                                self.candidates.insert(j);
                            }
                        } else {
                            self.query_bucket += 1;
                            self.query_member = 0;
                            used += 1;
                        }
                    } else {
                        self.phase = 4;
                        self.emit_cursor = Some(0);
                    }
                }
                4 => {
                    if let Some(j) = self.emit_cursor {
                        if let Some(&next) = self.candidates.range(j..).next() {
                            self.candidates.remove(&next);
                            self.emit_cursor = Some(next + 1);
                            used += 1;
                            return Advance {
                                work_used: used,
                                pair: Some((self.query_i, next)),
                                done: false,
                            };
                        }
                    }
                    self.query_i += 1;
                    self.query_bucket = 0;
                    self.query_member = 0;
                    self.emit_cursor = None;
                    self.phase = if self.query_i + 1 >= self.bounds.len() {
                        3
                    } else {
                        2
                    };
                }
                _ => self.phase = 3,
            }
        }
        Advance {
            work_used: used,
            pair: None,
            done: self.phase == 3,
        }
    }
}

fn bucket_range(b: Bounds, min: f64, max: f64) -> (usize, usize) {
    if max <= min {
        return (0, 0);
    }
    let scale = BUCKETS as f64 / (max - min);
    let lo = (((b.min_x - EPSILON - min) * scale).floor() as isize).clamp(0, BUCKETS as isize - 1)
        as usize;
    let hi = (((b.max_x + EPSILON - min) * scale).floor() as isize).clamp(0, BUCKETS as isize - 1)
        as usize;
    (lo, hi)
}

fn overlaps(a: Bounds, b: Bounds) -> bool {
    a.min_x <= b.max_x + EPSILON
        && b.min_x <= a.max_x + EPSILON
        && a.min_z <= b.max_z + EPSILON
        && b.min_z <= a.max_z + EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn random_small_sets_match_oracle_and_resume() {
        let mut seed = 17u64;
        for n in 0..48 {
            let mut bs = Vec::new();
            for _ in 0..n {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let x = ((seed >> 32) % 80) as f64 - 40.0;
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let z = ((seed >> 32) % 80) as f64 - 40.0;
                let w = (seed % 12) as f64;
                bs.push(Bounds {
                    min_x: x,
                    max_x: x + w,
                    min_z: z,
                    max_z: z + w,
                });
            }
            for quota in [1, 7, 48] {
                let mut expected = Vec::new();
                for i in 0..n {
                    for j in i + 1..n {
                        let (a, b) = (bs[i], bs[j]);
                        if a.min_x <= b.max_x + EPSILON
                            && b.min_x <= a.max_x + EPSILON
                            && a.min_z <= b.max_z + EPSILON
                            && b.min_z <= a.max_z + EPSILON
                        {
                            expected.push((i, j));
                        }
                    }
                }
                let mut bp = BroadPhase::new(bs.clone()).unwrap();
                let mut got = Vec::new();
                bp = BroadPhase::restore(bp.checkpoint()).unwrap();
                loop {
                    let before = serde_json::to_vec(&bp.checkpoint()).unwrap();
                    assert_eq!(bp.advance(0).work_used, 0);
                    assert_eq!(serde_json::to_vec(&bp.checkpoint()).unwrap(), before);
                    let a = bp.advance(quota);
                    assert!(a.work_used <= quota);
                    if let Some(p) = a.pair {
                        got.push(p);
                    }
                    if a.done {
                        BroadPhase::restore(bp.checkpoint()).unwrap();
                        break;
                    }
                    let bytes = serde_json::to_vec(&bp.checkpoint()).unwrap();
                    let cp: Checkpoint = serde_json::from_slice(&bytes).unwrap();
                    bp = BroadPhase::restore(cp).unwrap_or_else(|e| panic!("restore {e:?}: phase={} cursor={} bucket_cursor={} query_i={} query_bucket={} query_member={} emit={}", bp.phase, bp.cursor, bp.bucket_cursor, bp.query_i, bp.query_bucket, bp.query_member, bp.emit_cursor.unwrap_or(usize::MAX)));
                }
                assert_eq!(got, expected);
            }
        }
    }

    #[test]
    fn five_thousand_segment_street_grid_streams_candidates() {
        let mut bs = Vec::with_capacity(MAX_SEGMENTS);
        // 50 by 50 intersections: 2,500 horizontal and 2,500 vertical streets.
        for z in 0..50 {
            for x in 0..50 {
                bs.push(Bounds {
                    min_x: (x * 2) as f64,
                    max_x: (x * 2 + 2) as f64,
                    min_z: (z * 2) as f64,
                    max_z: (z * 2) as f64,
                });
            }
        }
        for x in 0..50 {
            for z in 0..50 {
                bs.push(Bounds {
                    min_x: (x * 2) as f64,
                    max_x: (x * 2) as f64,
                    min_z: (z * 2) as f64,
                    max_z: (z * 2 + 2) as f64,
                });
            }
        }
        let mut bp = BroadPhase::new(bs.clone()).unwrap();
        let mut pairs = Vec::new();
        let mut work = 0usize;
        loop {
            let a = bp.advance(48);
            work += a.work_used;
            if let Some(pair) = a.pair {
                pairs.push(pair);
            }
            if a.done {
                break;
            }
        }
        let mut expected = Vec::new();
        for i in 0..bs.len() {
            for j in i + 1..bs.len() {
                let (a, b) = (bs[i], bs[j]);
                if a.min_x <= b.max_x + EPSILON
                    && b.min_x <= a.max_x + EPSILON
                    && a.min_z <= b.max_z + EPSILON
                    && b.min_z <= a.max_z + EPSILON
                {
                    expected.push((i, j));
                }
            }
        }
        eprintln!(
            "grid: {} candidates, {work} work units, {} checkpoint bytes",
            pairs.len(),
            serde_json::to_vec(&bp.checkpoint()).unwrap().len()
        );
        assert_eq!(pairs, expected);
        assert!(pairs.len() < bs.len() * (bs.len() - 1) / 2);
        assert!(work < bs.len() * (bs.len() - 1) / 2);
    }

    #[test]
    fn epsilon_touching_extents_and_corrupt_checkpoint() {
        let bs = vec![
            Bounds {
                min_x: 0.0,
                max_x: 10_000.0,
                min_z: 0.0,
                max_z: 0.0,
            },
            Bounds {
                min_x: 10_000.0 + 0.5e-5,
                max_x: 10_000.0 + 0.5e-5,
                min_z: 0.0,
                max_z: 0.0,
            },
            Bounds {
                min_x: 50.0,
                max_x: 50.0,
                min_z: -10_000.0,
                max_z: 10_000.0,
            },
        ];
        let mut bp = BroadPhase::new(bs).unwrap();
        let mut got = Vec::new();
        loop {
            let a = bp.advance(7);
            if let Some(p) = a.pair {
                got.push(p);
            }
            if a.done {
                break;
            }
        }
        assert_eq!(got, vec![(0, 1), (0, 2)]);
        let mut c = bp.checkpoint();
        c.query_i = usize::MAX;
        assert_eq!(BroadPhase::restore(c).err(), Some(Error::InvalidCheckpoint));
    }

    #[test]
    fn restore_rejects_missing_candidates_and_inconsistent_stage_fields() {
        let b = Bounds {
            min_x: 0.0,
            max_x: 0.0,
            min_z: 0.0,
            max_z: 0.0,
        };
        let mut bp = BroadPhase::new(vec![b; 4]).unwrap();
        let mut checked_gather = false;
        let mut checked_emit = false;
        loop {
            let a = bp.advance(1);
            let c = bp.checkpoint();
            BroadPhase::restore(c.clone()).unwrap();
            if c.phase == 5 {
                let mut bad = c.clone();
                bad.buckets[0].push(0);
                assert_eq!(
                    BroadPhase::restore(bad).err(),
                    Some(Error::InvalidCheckpoint)
                );
            }
            if matches!(c.phase, 2 | 4) && !c.candidates.is_empty() {
                let mut bad = c.clone();
                bad.candidates.pop();
                assert_eq!(
                    BroadPhase::restore(bad).err(),
                    Some(Error::InvalidCheckpoint)
                );
                checked_gather |= c.phase == 2;
                checked_emit |= c.phase == 4;
            }
            if c.phase == 2 {
                let mut bad = c.clone();
                bad.query_bucket = BUCKETS;
                bad.query_member = 1;
                assert_eq!(
                    BroadPhase::restore(bad).err(),
                    Some(Error::InvalidCheckpoint)
                );
                let mut bad = c.clone();
                bad.phase = 3;
                assert_eq!(
                    BroadPhase::restore(bad).err(),
                    Some(Error::InvalidCheckpoint)
                );
            }
            if a.done {
                break;
            }
        }
        assert!(checked_gather && checked_emit);
    }

    #[test]
    fn coincident_bounds_stream_first_pair_without_pair_graph() {
        let b = Bounds {
            min_x: 2.0,
            max_x: 2.0,
            min_z: 3.0,
            max_z: 3.0,
        };
        let mut bp = BroadPhase::new(vec![b; MAX_SEGMENTS]).unwrap();
        let mut work = 0usize;
        loop {
            let a = bp.advance(48);
            work += a.work_used;
            if let Some(pair) = a.pair {
                assert_eq!(pair, (0, 1));
                break;
            }
            assert!(!a.done);
        }
        assert!(work < 20_000, "first pair needed {work} logical operations");
    }
}
