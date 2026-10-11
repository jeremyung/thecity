use std::collections::{BTreeMap, BTreeSet};

const CELL_WIDTH: f64 = 2.0e-5;
const MAX_COORDINATE: f32 = 100_000.0;

/// Deterministic spatial broadphase for source-ordered node coincidence checks.
///
/// This is derived state: callers rebuild it from the validated node prefix.
/// Unsupported input makes the index unusable so callers can fall back to the
/// historical full scan without changing its admission semantics.
/// Storage is bounded by the caller's node cap; each query visits nine cells
/// with ordered-tree lower bounds, never scanning an entire dense bucket.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NodeCoincidenceIndex {
    cells: BTreeMap<(i64, i64), BTreeSet<usize>>,
    fallback: bool,
}

impl NodeCoincidenceIndex {
    pub(crate) fn insert(&mut self, index: usize, x: f32, z: f32) {
        let Some(cell) = cell_for(x, z) else {
            self.fallback = true;
            return;
        };
        self.cells.entry(cell).or_default().insert(index);
    }

    /// Return the lowest source index at or after `min_index` that could be
    /// coincident with `(x, z)`. `Err(())` requests the legacy full scan.
    pub(crate) fn next_candidate(
        &self,
        x: f32,
        z: f32,
        min_index: usize,
    ) -> Result<Option<usize>, ()> {
        if self.fallback {
            return Err(());
        }
        let (cx, cz) = cell_for(x, z).ok_or(())?;
        let mut candidate = None;
        for dx in -1..=1 {
            for dz in -1..=1 {
                let key = (cx + dx, cz + dz);
                if let Some(indices) = self.cells.get(&key) {
                    if let Some(&index) = indices.range(min_index..).next() {
                        candidate = Some(candidate.map_or(index, |best: usize| best.min(index)));
                    }
                }
            }
        }
        Ok(candidate)
    }
}

fn cell_for(x: f32, z: f32) -> Option<(i64, i64)> {
    if !eligible_coordinate(x) || !eligible_coordinate(z) {
        return None;
    }
    Some((cell_coordinate(x), cell_coordinate(z)))
}

fn eligible_coordinate(value: f32) -> bool {
    value.is_finite() && value.abs() <= MAX_COORDINATE
}

fn cell_coordinate(value: f32) -> i64 {
    ((value as f64) / CELL_WIDTH).floor() as i64
}

#[cfg(test)]
mod tests {
    use super::NodeCoincidenceIndex;

    const DISTANCE: f32 = 1.0e-5;

    fn is_coincident(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).hypot(a.1 - b.1) <= DISTANCE
    }

    fn assert_candidates_contain_all_pairs(points: &[(f32, f32)]) {
        let mut index = NodeCoincidenceIndex::default();
        for (i, &(x, z)) in points.iter().enumerate() {
            index.insert(i, x, z);
        }
        for (i, &(x, z)) in points.iter().enumerate() {
            let mut cursor = i + 1;
            let mut found = Vec::new();
            while let Some(candidate) = index.next_candidate(x, z, cursor).unwrap() {
                assert!(found.last().map_or(true, |&previous| candidate > previous));
                found.push(candidate);
                cursor = candidate + 1;
            }
            for j in i + 1..points.len() {
                if is_coincident(points[i], points[j]) {
                    assert!(found.contains(&j), "missed near pair ({i}, {j})");
                }
            }
        }
    }

    #[test]
    fn cell_boundaries_and_threshold_neighbors_preserve_f32_pairs() {
        let mut points = Vec::new();
        for cell in -40..=40 {
            let boundary = (cell as f64 * super::CELL_WIDTH) as f32;
            for offset in [-DISTANCE, -DISTANCE * 0.7, 0.0, DISTANCE * 0.7, DISTANCE] {
                let value = boundary + offset;
                points.push((value, 0.0));
                points.push((value, DISTANCE * 0.7));
                if value != 0.0 {
                    points.push((f32::from_bits(value.to_bits() - 1), 0.0));
                    points.push((f32::from_bits(value.to_bits() + 1), 0.0));
                }
            }
        }
        assert_candidates_contain_all_pairs(&points);
    }

    #[test]
    fn randomized_points_cover_every_near_pair_and_candidates_are_ordered() {
        let mut points = Vec::new();
        let mut state = 0x1234_5678_u32;
        for i in 0..1_200 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let x = (state as i32 as f32) * 0.000_001;
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let z = (state as i32 as f32) * 0.000_001;
            points.push((x, z));
            if i % 23 == 0 {
                points.push((x + 0.000_009, z));
            }
        }
        points.extend([
            (-2.0e-5, -1.0e-5),
            (-1.0e-5, -1.0e-5),
            (-f32::EPSILON, 0.0),
            (0.0, 0.0),
            (1.0e-5, 0.0),
            (99_999.0, -99_999.0),
            (99_999.0, -99_999.0),
        ]);
        assert_candidates_contain_all_pairs(&points);
    }

    #[test]
    fn unsupported_coordinates_request_legacy_fallback() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 100_001.0] {
            let mut index = NodeCoincidenceIndex::default();
            index.insert(0, bad, 0.0);
            assert_eq!(index.next_candidate(0.0, 0.0, 0), Err(()));

            let mut query = NodeCoincidenceIndex::default();
            query.insert(0, 0.0, 0.0);
            assert_eq!(query.next_candidate(bad, 0.0, 0), Err(()));
        }
    }

    #[test]
    fn candidates_advance_inside_a_dense_cell() {
        let mut index = NodeCoincidenceIndex::default();
        for i in 0..512 {
            index.insert(i, 0.0, 0.0);
        }
        for min_index in 0..512 {
            assert_eq!(
                index.next_candidate(0.0, 0.0, min_index),
                Ok(Some(min_index))
            );
        }
        assert_eq!(index.next_candidate(0.0, 0.0, 512), Ok(None));
    }

    #[test]
    fn sparse_grid_queries_find_no_candidates() {
        let mut index = NodeCoincidenceIndex::default();
        for i in 0..5_000 {
            let x = (i % 100) as f32 * 1.0;
            let z = (i / 100) as f32 * 1.0;
            index.insert(i, x, z);
        }
        for i in 0..5_000 {
            let x = (i % 100) as f32 * 1.0 + 0.25;
            let z = (i / 100) as f32 * 1.0 + 0.25;
            assert_eq!(index.next_candidate(x, z, 0), Ok(None));
            assert_eq!(index.next_candidate(x - 0.25, z - 0.25, i + 1), Ok(None));
        }
    }
}
