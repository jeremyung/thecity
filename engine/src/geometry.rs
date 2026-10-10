//! Small, pure two-dimensional geometry helpers used by the simulation.
//!
//! These functions use scalar inputs so they can be tested independently of
//! simulation entities. Segment intersection checks both orientation and
//! overlapping axis-aligned intervals; zero-length segments are treated as
//! points before the intersection precheck.

pub(crate) fn distance_coords(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    ((ax - bx).powi(2) + (az - bz).powi(2)).sqrt()
}

/// A rectangle whose width/depth axes rotate with the building's local frame.
/// Rotation follows Three.js `rotation.y`: local X becomes `(cos, -sin)` and
/// local +Z becomes `(sin, cos)` in world X/Z coordinates.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct OrientedRect {
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub rotation: f32,
}

impl OrientedRect {
    /// Returns whether the rectangle has finite position/orientation and
    /// strictly positive finite dimensions.
    pub(crate) fn finite_positive(self) -> bool {
        self.x.is_finite()
            && self.z.is_finite()
            && self.width.is_finite()
            && self.depth.is_finite()
            && self.rotation.is_finite()
            && self.width > 0.0
            && self.depth > 0.0
    }

    /// The four world-space corners in winding order, without allocating.
    pub(crate) fn corners(self) -> [(f32, f32); 4] {
        let (cos, sin) = (self.rotation.cos(), self.rotation.sin());
        let ux = (cos, -sin);
        let uz = (sin, cos);
        let hx = self.width * 0.5;
        let hz = self.depth * 0.5;
        [(-hx, -hz), (hx, -hz), (hx, hz), (-hx, hz)]
            .map(|(x, z)| (self.x + ux.0 * x + uz.0 * z, self.z + ux.1 * x + uz.1 * z))
    }

    /// Returns world-axis bounds. Invalid or overflowing rectangles return
    /// infinite bounds so callers conservatively treat them as covering space.
    pub(crate) fn bounds(self) -> (f32, f32, f32, f32) {
        if !self.finite_positive() {
            return (
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::INFINITY,
            );
        }
        let corners = self.corners();
        if corners
            .iter()
            .any(|(x, z)| !x.is_finite() || !z.is_finite())
        {
            return (
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::INFINITY,
            );
        }
        let mut min_x = f32::INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for (x, z) in corners {
            min_x = min_x.min(x);
            min_z = min_z.min(z);
            max_x = max_x.max(x);
            max_z = max_z.max(z);
        }
        (min_x, min_z, max_x, max_z)
    }

    fn axes(self) -> [(f32, f32); 2] {
        let (cos, sin) = (self.rotation.cos(), self.rotation.sin());
        [(cos, -sin), (sin, cos)]
    }
}

/// Tests overlap after expanding each rectangle's local half-width and
/// half-depth by `gap / 2`. This inflates the footprints along their own axes;
/// it is a conservative square-like expansion, not a constant-Euclidean
/// offset. Exact contact between the inflated footprints is allowed. Invalid
/// rectangles, gaps, or overflowing projections conservatively report overlap
/// (reject).
pub(crate) fn rects_overlap(a: &OrientedRect, b: &OrientedRect, gap: f32) -> bool {
    if !a.finite_positive() || !b.finite_positive() || !gap.is_finite() || gap < 0.0 {
        return true;
    }
    let center_dx = b.x - a.x;
    let center_dz = b.z - a.z;
    if !center_dx.is_finite() || !center_dz.is_finite() {
        return true;
    }
    let inflation = gap * 0.5;

    // Before evaluating sin/cos, reject rectangles separated on either world
    // axis by a rotation-independent upper bound for their inflated extents.
    // For any angle, a rectangle's X or Z half-extent is no larger than the
    // L1 sum of its local half-width and half-depth. A scale-aware margin
    // makes this filter deliberately generous near contact; uncertain or
    // overflowing cases fall through to the original SAT path below.
    let max_world_half_extent =
        |rect: &OrientedRect| (rect.width * 0.5 + inflation) + (rect.depth * 0.5 + inflation);
    let radius_bound = max_world_half_extent(a) + max_world_half_extent(b);
    let center_l1 = center_dx.abs() + center_dz.abs();
    if radius_bound.is_finite() && center_l1.is_finite() {
        let scale = radius_bound.max(center_l1).max(1.0);
        let margin = scale * (32.0 * f32::EPSILON);
        let separated_at = radius_bound + margin;
        if separated_at.is_finite()
            && (center_dx.abs() >= separated_at || center_dz.abs() >= separated_at)
        {
            return false;
        }
    }

    let axes_a = a.axes();
    let axes_b = b.axes();
    for axis in [axes_a[0], axes_a[1], axes_b[0], axes_b[1]] {
        let distance = (center_dx * axis.0 + center_dz * axis.1).abs();
        let radius_a = (a.width * 0.5 + inflation)
            * (axes_a[0].0 * axis.0 + axes_a[0].1 * axis.1).abs()
            + (a.depth * 0.5 + inflation) * (axes_a[1].0 * axis.0 + axes_a[1].1 * axis.1).abs();
        let radius_b = (b.width * 0.5 + inflation)
            * (axes_b[0].0 * axis.0 + axes_b[0].1 * axis.1).abs()
            + (b.depth * 0.5 + inflation) * (axes_b[1].0 * axis.0 + axes_b[1].1 * axis.1).abs();
        let required_separation = radius_a + radius_b;
        if !distance.is_finite() || !required_separation.is_finite() {
            return true;
        }
        if distance >= required_separation {
            return false;
        }
    }
    true
}

/// Exact distance between a rectangle and a segment in X/Z. Containment and
/// edge intersections return zero. Invalid input conservatively returns zero
/// distance, so callers cannot treat malformed geometry as clear space.
pub(crate) fn rect_segment_distance(
    rect: &OrientedRect,
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
) -> f32 {
    if !rect.finite_positive()
        || !ax.is_finite()
        || !az.is_finite()
        || !bx.is_finite()
        || !bz.is_finite()
    {
        return 0.0;
    }
    let axes = rect.axes();
    let axis_x = axes[0];
    let axis_z = axes[1];
    let inside = |x: f32, z: f32| {
        let dx = x - rect.x;
        let dz = z - rect.z;
        (dx * axis_x.0 + dz * axis_x.1).abs() <= rect.width * 0.5
            && (dx * axis_z.0 + dz * axis_z.1).abs() <= rect.depth * 0.5
    };
    if inside(ax, az) || inside(bx, bz) {
        return 0.0;
    }
    let corners = rect.corners();
    let mut minimum = f32::INFINITY;
    for index in 0..4 {
        let a = corners[index];
        let b = corners[(index + 1) % 4];
        minimum = minimum.min(segments_distance_coords(a.0, a.1, b.0, b.1, ax, az, bx, bz));
    }
    if minimum.is_finite() {
        minimum
    } else {
        0.0
    }
}

pub(crate) fn point_segment_distance(x: f32, z: f32, ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    let dx = bx - ax;
    let dz = bz - az;
    let denom = dx * dx + dz * dz;
    if denom <= f32::EPSILON {
        return (x - ax).hypot(z - az);
    }
    let t = (((x - ax) * dx + (z - az) * dz) / denom).clamp(0.0, 1.0);
    (x - (ax + t * dx)).hypot(z - (az + t * dz))
}

pub(crate) fn segments_distance_coords(
    ax: f32,
    az: f32,
    bx: f32,
    bz: f32,
    cx: f32,
    cz: f32,
    dx: f32,
    dz: f32,
) -> f32 {
    if ax == bx && az == bz {
        return point_segment_distance(ax, az, cx, cz, dx, dz);
    }
    if cx == dx && cz == dz {
        return point_segment_distance(cx, cz, ax, az, bx, bz);
    }
    let orient = |px: f32, pz: f32, qx: f32, qz: f32, rx: f32, rz: f32| {
        (qx - px) * (rz - pz) - (qz - pz) * (rx - px)
    };
    let ab_c = orient(ax, az, bx, bz, cx, cz);
    let ab_d = orient(ax, az, bx, bz, dx, dz);
    let cd_a = orient(cx, cz, dx, dz, ax, az);
    let cd_b = orient(cx, cz, dx, dz, bx, bz);
    let bounds_overlap = ax.min(bx) <= cx.max(dx)
        && cx.min(dx) <= ax.max(bx)
        && az.min(bz) <= cz.max(dz)
        && cz.min(dz) <= az.max(bz);
    if bounds_overlap && ab_c * ab_d <= 0.0 && cd_a * cd_b <= 0.0 {
        return 0.0;
    }
    point_segment_distance(ax, az, cx, cz, dx, dz)
        .min(point_segment_distance(bx, bz, cx, cz, dx, dz))
        .min(point_segment_distance(cx, cz, ax, az, bx, bz))
        .min(point_segment_distance(dx, dz, ax, az, bx, bz))
}

#[cfg(test)]
mod tests {
    use super::{
        rect_segment_distance, rects_overlap, segments_distance_coords as segments_distance,
        OrientedRect,
    };

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }

    fn rect(x: f32, z: f32, width: f32, depth: f32, rotation: f32) -> OrientedRect {
        OrientedRect {
            x,
            z,
            width,
            depth,
            rotation,
        }
    }

    // Frozen copy of the original predicate. Used to guard equivalence for
    // the fast-rejection path and to keep a deterministic microbenchmark.
    fn rects_overlap_sat_reference(a: &OrientedRect, b: &OrientedRect, gap: f32) -> bool {
        if !a.finite_positive() || !b.finite_positive() || !gap.is_finite() || gap < 0.0 {
            return true;
        }
        let axes_a = a.axes();
        let axes_b = b.axes();
        let center_dx = b.x - a.x;
        let center_dz = b.z - a.z;
        if !center_dx.is_finite() || !center_dz.is_finite() {
            return true;
        }
        let inflation = gap * 0.5;
        for axis in [axes_a[0], axes_a[1], axes_b[0], axes_b[1]] {
            let distance = (center_dx * axis.0 + center_dz * axis.1).abs();
            let radius_a = (a.width * 0.5 + inflation)
                * (axes_a[0].0 * axis.0 + axes_a[0].1 * axis.1).abs()
                + (a.depth * 0.5 + inflation) * (axes_a[1].0 * axis.0 + axes_a[1].1 * axis.1).abs();
            let radius_b = (b.width * 0.5 + inflation)
                * (axes_b[0].0 * axis.0 + axes_b[0].1 * axis.1).abs()
                + (b.depth * 0.5 + inflation) * (axes_b[1].0 * axis.0 + axes_b[1].1 * axis.1).abs();
            let required_separation = radius_a + radius_b;
            if !distance.is_finite() || !required_separation.is_finite() {
                return true;
            }
            if distance >= required_separation {
                return false;
            }
        }
        true
    }

    fn next_random(state: &mut u32) -> f32 {
        *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (*state >> 8) as f32 / 16_777_215.0
    }

    #[test]
    fn oriented_rectangle_uses_three_y_rotation_and_reports_world_bounds() {
        let shape = rect(2.0, 3.0, 4.0, 2.0, std::f32::consts::FRAC_PI_2);
        let corners = shape.corners();
        close(corners[0].0, 1.0);
        close(corners[0].1, 5.0);
        let (min_x, min_z, max_x, max_z) = shape.bounds();
        close(min_x, 1.0);
        close(min_z, 1.0);
        close(max_x, 3.0);
        close(max_z, 5.0);
    }

    #[test]
    fn rotated_nonsquare_rectangles_use_oriented_separating_axes() {
        let a = rect(0.0, 0.0, 6.0, 2.0, std::f32::consts::FRAC_PI_4);
        let overlap = rect(0.0, 0.0, 2.0, 6.0, std::f32::consts::FRAC_PI_4);
        let clear = rect(5.0, -5.0, 6.0, 2.0, std::f32::consts::FRAC_PI_4);
        assert!(rects_overlap(&a, &overlap, 0.0));
        assert!(!rects_overlap(&a, &clear, 0.0));
    }

    #[test]
    fn rectangle_overlap_handles_containment_and_exact_requested_gap() {
        let outer = rect(0.0, 0.0, 10.0, 10.0, 0.0);
        let contained = rect(0.0, 0.0, 2.0, 2.0, std::f32::consts::FRAC_PI_4);
        assert!(rects_overlap(&outer, &contained, 0.0));

        let touching = rect(3.0, 0.0, 2.0, 2.0, 0.0);
        assert!(!rects_overlap(
            &rect(0.0, 0.0, 4.0, 2.0, 0.0),
            &touching,
            0.0
        ));
        assert!(rects_overlap(
            &rect(0.0, 0.0, 4.0, 2.0, 0.0),
            &touching,
            0.1
        ));

        let exact_one_meter_gap = rect(4.0, 0.0, 2.0, 2.0, 0.0);
        assert!(!rects_overlap(
            &rect(0.0, 0.0, 4.0, 2.0, 0.0),
            &exact_one_meter_gap,
            1.0,
        ));
    }

    #[test]
    fn rotated_clearance_inflates_local_corners_before_sat_projection() {
        let a = rect(0.0, 0.0, 6.0, 2.0, 0.0);
        let b = rect(8.0, 0.0, 6.0, 2.0, std::f32::consts::FRAC_PI_4);
        // Adding a flat 2m to each tested projected radius would call these
        // clear on A's X axis (the uninflated projected radii sum to 5.828m).
        // Expanding each rectangle by 1m along its own X and Z axes increases
        // its rotated-corner projection, so the safety envelopes still overlap.
        assert!(rects_overlap(&a, &b, 2.0));
    }

    #[test]
    fn far_axis_aligned_separation_matches_the_original_sat_predicate() {
        let a = rect(-120.0, 48.0, 22.0, 7.0, 1.13);
        let b = rect(95.0, -31.0, 13.0, 18.0, -0.77);
        assert!(!rects_overlap(&a, &b, 3.0));
        assert_eq!(
            rects_overlap(&a, &b, 3.0),
            rects_overlap_sat_reference(&a, &b, 3.0)
        );
    }

    #[test]
    fn overlap_fast_path_matches_frozen_sat_for_seeded_random_geometry() {
        let mut seed = 0x6d2b_79f5;
        for index in 0..60_000 {
            let random = |state: &mut u32| next_random(state);
            let ax = random(&mut seed) * 20_000.0 - 10_000.0;
            let az = random(&mut seed) * 20_000.0 - 10_000.0;
            let width_a = 0.02 + random(&mut seed) * 600.0;
            let depth_a = 0.02 + random(&mut seed) * 600.0;
            let rotation_a = (random(&mut seed) * 2.0 - 1.0) * 32.0;
            let a = rect(ax, az, width_a, depth_a, rotation_a);

            let width_b = 0.02 + random(&mut seed) * 600.0;
            let depth_b = 0.02 + random(&mut seed) * 600.0;
            let rotation_b = (random(&mut seed) * 2.0 - 1.0) * 32.0;
            let gap = match index % 4 {
                0 => 0.0,
                1 => 0.5,
                2 => 2.0,
                _ => random(&mut seed) * 20.0,
            };
            let (bx, bz) = match index % 3 {
                // Include near contacts by placing centers just around the
                // largest possible inflated X/Z envelope.
                0 => (
                    ax + (width_a + depth_a + width_b + depth_b) * 0.25
                        + (random(&mut seed) - 0.5) * 4.0,
                    az + (random(&mut seed) - 0.5) * 6.0,
                ),
                1 => (
                    ax + (random(&mut seed) - 0.5) * 40.0,
                    az + (random(&mut seed) - 0.5) * 40.0,
                ),
                _ => (
                    random(&mut seed) * 20_000.0 - 10_000.0,
                    random(&mut seed) * 20_000.0 - 10_000.0,
                ),
            };
            let b = rect(bx, bz, width_b, depth_b, rotation_b);
            assert_eq!(
                rects_overlap(&a, &b, gap),
                rects_overlap_sat_reference(&a, &b, gap),
                "seeded geometry mismatch at iteration {index}: {a:?}, {b:?}, gap={gap}"
            );
        }
    }

    #[test]
    fn overlap_fast_path_preserves_invalid_and_extreme_float_policy() {
        let valid = rect(0.0, 0.0, 4.0, 2.0, 0.4);
        let cases = [
            (valid, rect(f32::INFINITY, 0.0, 4.0, 2.0, 0.0), 0.0),
            (valid, rect(1.0, 1.0, f32::NAN, 2.0, 0.0), 0.0),
            (valid, rect(1.0, 1.0, 4.0, 2.0, f32::INFINITY), 0.0),
            (valid, rect(1.0, 1.0, 4.0, 2.0, 0.0), -1.0),
            (valid, rect(1.0, 1.0, 4.0, 2.0, 0.0), f32::NAN),
            (valid, rect(1.0, 1.0, 4.0, 2.0, 0.0), f32::MAX),
            (
                rect(f32::MAX, f32::MAX, 1.0, 1.0, 0.2),
                rect(-f32::MAX, -f32::MAX, 1.0, 1.0, -0.2),
                0.0,
            ),
            (
                rect(0.0, 0.0, f32::MAX, f32::MAX, 0.8),
                rect(1.0e30, 1.0e30, 2.0, 3.0, -0.4),
                0.0,
            ),
        ];
        for (a, b, gap) in cases {
            assert_eq!(
                rects_overlap(&a, &b, gap),
                rects_overlap_sat_reference(&a, &b, gap)
            );
        }
    }

    #[test]
    #[ignore = "manual deterministic microbenchmark; run with --ignored --nocapture"]
    fn benchmark_overlap_against_frozen_sat_reference() {
        use std::hint::black_box;
        use std::time::Instant;

        let mut seed = 0x4f1b_bcdd;
        let pairs: Vec<_> = (0..4096)
            .map(|index| {
                let x = next_random(&mut seed) * 4_000.0 - 2_000.0;
                let z = next_random(&mut seed) * 4_000.0 - 2_000.0;
                let a = rect(
                    x,
                    z,
                    9.0 + next_random(&mut seed) * 18.0,
                    7.0 + next_random(&mut seed) * 15.0,
                    next_random(&mut seed) * 6.28,
                );
                let gap = [0.0, 1.0, 2.0][index % 3];
                let (bx, bz) = if index % 4 == 0 {
                    (x + 150.0, z - 90.0)
                } else {
                    (
                        x + next_random(&mut seed) * 36.0 - 18.0,
                        z + next_random(&mut seed) * 36.0 - 18.0,
                    )
                };
                let b = rect(
                    bx,
                    bz,
                    8.0 + next_random(&mut seed) * 18.0,
                    6.0 + next_random(&mut seed) * 12.0,
                    next_random(&mut seed) * 6.28,
                );
                (a, b, gap)
            })
            .collect();
        let rounds = 256;
        let run = |predicate: fn(&OrientedRect, &OrientedRect, f32) -> bool| {
            let started = Instant::now();
            let mut overlaps = 0_usize;
            for _ in 0..rounds {
                for (a, b, gap) in &pairs {
                    overlaps += usize::from(predicate(black_box(a), black_box(b), *gap));
                }
            }
            (started.elapsed().as_secs_f64() * 1000.0, overlaps)
        };
        let _ = run(rects_overlap_sat_reference);
        let _ = run(rects_overlap);
        let mut reference_samples = Vec::with_capacity(7);
        let mut production_samples = Vec::with_capacity(7);
        let mut reference_overlaps = 0;
        let mut production_overlaps = 0;
        for repetition in 0..7 {
            if repetition % 2 == 0 {
                let (elapsed, overlaps) = run(rects_overlap_sat_reference);
                reference_samples.push(elapsed);
                reference_overlaps = overlaps;
                let (elapsed, overlaps) = run(rects_overlap);
                production_samples.push(elapsed);
                production_overlaps = overlaps;
            } else {
                let (elapsed, overlaps) = run(rects_overlap);
                production_samples.push(elapsed);
                production_overlaps = overlaps;
                let (elapsed, overlaps) = run(rects_overlap_sat_reference);
                reference_samples.push(elapsed);
                reference_overlaps = overlaps;
            }
        }
        reference_samples.sort_by(f64::total_cmp);
        production_samples.sort_by(f64::total_cmp);
        assert_eq!(reference_overlaps, production_overlaps);
        eprintln!("overlap microbenchmark: pairs={} rounds={} repetitions=7 reference_median_ms={:.3} production_median_ms={:.3} reference_samples_ms={reference_samples:?} production_samples_ms={production_samples:?} overlap_count={production_overlaps}", pairs.len(), rounds, reference_samples[3], production_samples[3]);
    }

    #[test]
    fn rectangle_segment_distance_handles_crossing_inside_and_parallel_segments() {
        let shape = rect(0.0, 0.0, 4.0, 2.0, 0.0);
        close(rect_segment_distance(&shape, -3.0, 0.0, 3.0, 0.0), 0.0);
        close(rect_segment_distance(&shape, -1.0, 0.0, 1.0, 0.0), 0.0);
        close(rect_segment_distance(&shape, -3.0, 2.0, 3.0, 2.0), 1.0);

        let rotated = rect(0.0, 0.0, 4.0, 2.0, std::f32::consts::FRAC_PI_4);
        close(rect_segment_distance(&rotated, -4.0, 0.0, 4.0, 0.0), 0.0);
    }

    #[test]
    fn malformed_rectangles_conservatively_block_overlap_and_clearance() {
        let valid = rect(0.0, 0.0, 4.0, 2.0, 0.0);
        let invalid = rect(0.0, 0.0, 0.0, 2.0, 0.0);
        assert!(rects_overlap(&valid, &invalid, 0.0));
        close(rect_segment_distance(&invalid, 5.0, 5.0, 6.0, 5.0), 0.0);
        assert!(invalid.bounds().0.is_infinite());
    }

    #[test]
    fn disjoint_collinear_segments_have_their_endpoint_gap() {
        close(
            segments_distance(0.0, 0.0, 2.0, 0.0, 5.0, 0.0, 8.0, 0.0),
            3.0,
        );
        close(
            segments_distance(0.0, 0.0, 0.0, 2.0, 0.0, 5.0, 0.0, 8.0),
            3.0,
        );
        close(
            segments_distance(0.0, 0.0, 2.0, 2.0, 5.0, 5.0, 8.0, 8.0),
            2.0_f32.sqrt() * 3.0,
        );
    }

    #[test]
    fn collinear_overlap_touching_crossing_and_parallel_cases_are_distinguished() {
        close(
            segments_distance(0.0, 0.0, 5.0, 0.0, 2.0, 0.0, 8.0, 0.0),
            0.0,
        );
        close(
            segments_distance(0.0, 0.0, 2.0, 0.0, 2.0, 0.0, 5.0, 0.0),
            0.0,
        );
        close(
            segments_distance(1.0, 0.0, 5.0, 4.0, 1.0, 4.0, 5.0, 0.0),
            0.0,
        );
        close(
            segments_distance(0.0, 0.0, 4.0, 0.0, 0.0, 3.0, 4.0, 3.0),
            3.0,
        );
    }

    #[test]
    fn degenerate_segments_are_points_and_only_intersect_when_on_the_other_segment() {
        close(
            segments_distance(2.0, 2.0, 2.0, 2.0, 0.0, 0.0, 4.0, 4.0),
            0.0,
        );
        close(
            segments_distance(2.0, 3.0, 2.0, 3.0, 0.0, 0.0, 4.0, 4.0),
            2.0_f32.sqrt() / 2.0,
        );
        close(
            segments_distance(1.0, 1.0, 1.0, 1.0, 3.0, 3.0, 3.0, 3.0),
            2.0_f32.sqrt() * 2.0,
        );
        close(
            segments_distance(0.0, 0.0, 4.0, 4.0, 2.0, 3.0, 2.0, 3.0),
            2.0_f32.sqrt() / 2.0,
        );
    }

    #[test]
    fn segment_distance_is_symmetric_under_endpoint_reversal() {
        for (a, b, c, d, expected) in [
            ((0.0, 0.0), (2.0, 0.0), (5.0, 0.0), (8.0, 0.0), 3.0),
            (
                (0.0, 0.0),
                (4.0, 4.0),
                (2.0, 3.0),
                (2.0, 3.0),
                2.0_f32.sqrt() / 2.0,
            ),
            ((0.0, 0.0), (4.0, 0.0), (0.0, 3.0), (4.0, 3.0), 3.0),
            ((0.0, 0.0), (4.0, 4.0), (0.0, 4.0), (4.0, 0.0), 0.0),
        ] {
            for reverse_ab in [false, true] {
                for reverse_cd in [false, true] {
                    let (a, b) = if reverse_ab { (b, a) } else { (a, b) };
                    let (c, d) = if reverse_cd { (d, c) } else { (c, d) };
                    close(
                        segments_distance(a.0, a.1, b.0, b.1, c.0, c.1, d.0, d.1),
                        expected,
                    );
                    close(
                        segments_distance(c.0, c.1, d.0, d.1, a.0, a.1, b.0, b.1),
                        expected,
                    );
                }
            }
        }
    }
}
