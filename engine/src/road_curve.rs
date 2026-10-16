//! Deterministic cubic Bézier helpers for planned road centerlines.
//!
//! `previous` and `next` are the endpoint positions immediately before `a`
//! and after `b` on a connected route. Missing neighbors produce a chord
//! tangent. The construction uses scalar arithmetic and `sqrt` only so it can
//! be shared by native and WASM plan generation.

use crate::masterplan::P;

/// Fair the lateral offsets of an ordered corridor without moving its street
/// graph. Keep the two outer interior anchors and longitudinal coordinates.
/// Fixed Jacobi passes avoid order-dependent updates and unbounded convergence.
pub(crate) fn fair_route(points: &mut [P], along_x: bool) {
    if points.len() < 5 {
        return;
    }
    let axis = |p: P| if along_x { p.x } else { p.z };
    let lateral = |p: P| if along_x { p.z } else { p.x };
    for _ in 0..3 {
        let before = points.to_vec();
        for i in 2..points.len() - 2 {
            let span = axis(before[i + 1]) - axis(before[i - 1]);
            if span.abs() < 1.0e-6 {
                continue;
            }
            let t = (axis(before[i]) - axis(before[i - 1])) / span;
            let value = 0.5 * lateral(before[i])
                + 0.5 * ((1.0 - t) * lateral(before[i - 1]) + t * lateral(before[i + 1]));
            if along_x {
                points[i].z = value;
            } else {
                points[i].x = value;
            }
        }
    }
    // The short regional tails continue the new interior bearing exactly.
    for (end, outer, inner) in [
        (0, 1, 2),
        (points.len() - 1, points.len() - 2, points.len() - 3),
    ] {
        let span = axis(points[outer]) - axis(points[inner]);
        if span.abs() < 1.0e-6 {
            continue;
        }
        let value = lateral(points[outer])
            + (lateral(points[outer]) - lateral(points[inner]))
                * (axis(points[end]) - axis(points[outer]))
                / span;
        if along_x {
            points[end].z = value;
        } else {
            points[end].x = value;
        }
    }
}

fn sub(a: P, b: P) -> P {
    P {
        x: a.x - b.x,
        z: a.z - b.z,
    }
}

fn add(a: P, b: P) -> P {
    P {
        x: a.x + b.x,
        z: a.z + b.z,
    }
}

fn scale(a: P, s: f32) -> P {
    P {
        x: a.x * s,
        z: a.z * s,
    }
}

fn dot(a: P, b: P) -> f32 {
    a.x * b.x + a.z * b.z
}

fn length(v: P) -> f32 {
    (v.x * v.x + v.z * v.z).sqrt()
}

fn unit(v: P) -> P {
    let len = length(v);
    if len <= 1.0e-6 {
        P { x: 0.0, z: 0.0 }
    } else {
        scale(v, 1.0 / len)
    }
}

/// Return the two interior control points for the segment `a` to `b`.
///
/// At each endpoint, adjacent route directions are averaged only for turns
/// whose dot product with this segment's direction is greater than -0.5.
/// Handles span one third of the chord; the planner reserves the resulting
/// curve envelope rather than forcing bends back into a straight corridor.
pub(crate) fn controls(a: P, b: P, previous: Option<P>, next: Option<P>) -> [P; 2] {
    let chord = sub(b, a);
    let chord_len = length(chord);
    if chord_len <= 1.0e-6 {
        return [a, b];
    }
    let direction = scale(chord, 1.0 / chord_len);

    let endpoint_tangent = |neighbor: Option<P>, at_start: bool| {
        let Some(neighbor) = neighbor else {
            return direction;
        };
        let adjacent = if at_start {
            unit(sub(a, neighbor))
        } else {
            unit(sub(neighbor, b))
        };
        if dot(adjacent, direction) <= -0.5 {
            return direction;
        }
        let average = unit(add(adjacent, direction));
        if length(average) <= 1.0e-6 {
            direction
        } else {
            average
        }
    };

    let start = endpoint_tangent(previous, true);
    let end = endpoint_tangent(next, false);
    let handle = chord_len / 3.0;
    [add(a, scale(start, handle)), sub(b, scale(end, handle))]
}

/// Evaluate a cubic Bézier road centerline at `t` (clamped to `[0, 1]`).
pub(crate) fn sample(a: P, control: [P; 2], b: P, t: f32) -> P {
    let t = t.clamp(0.0, 1.0);
    let s = 1.0 - t;
    let aa = s * s * s;
    let bb = 3.0 * s * s * t;
    let cc = 3.0 * s * t * t;
    let dd = t * t * t;
    P {
        x: aa * a.x + bb * control[0].x + cc * control[1].x + dd * b.x,
        z: aa * a.z + bb * control[0].z + cc * control[1].z + dd * b.z,
    }
}

#[cfg(test)]
mod tests {
    use super::{controls, fair_route, sample};
    use crate::masterplan::P;

    fn p(x: f32, z: f32) -> P {
        P { x, z }
    }

    #[test]
    fn cubic_sample_preserves_endpoints_and_clamps_parameter() {
        let a = p(0.0, 0.0);
        let b = p(10.0, 0.0);
        let c = controls(a, b, Some(p(-10.0, 0.0)), Some(p(20.0, 0.0)));
        assert_eq!(sample(a, c, b, 0.0), a);
        assert_eq!(sample(a, c, b, 1.0), b);
        assert_eq!(sample(a, c, b, -1.0), a);
        assert_eq!(sample(a, c, b, 2.0), b);
    }

    #[test]
    fn neighboring_directions_make_a_continuous_shallow_turn() {
        let a = p(0.0, 0.0);
        let b = p(10.0, 0.0);
        let c = controls(a, b, Some(p(-10.0, -10.0)), Some(p(20.0, 5.0)));
        assert!(c[0].x > a.x && c[0].z > a.z);
        assert!(c[1].x < b.x && c[1].z < b.z);
        let reversed = controls(b, a, Some(p(20.0, 0.0)), Some(p(-10.0, 0.0)));
        assert!((reversed[0].x - 20.0 / 3.0).abs() < 1e-5);
        assert!((reversed[1].x - 10.0 / 3.0).abs() < 1e-5);
    }

    #[test]
    fn handles_span_a_third_of_the_chord_for_broad_bends() {
        let a = p(0.0, 0.0);
        let b = p(30.0, 0.0);
        let c = controls(a, b, Some(p(-10.0, 0.0)), Some(p(60.0, 15.0)));
        assert!((super::length(super::sub(c[0], a)) - 10.0).abs() < 1e-5);
        assert!((super::length(super::sub(c[1], b)) - 10.0).abs() < 1e-5);
    }

    #[test]
    fn fair_route_keeps_longitudinal_coordinates_and_outer_interior_anchors() {
        let mut route = vec![
            p(-20.0, -8.0),
            p(-10.0, -4.0),
            p(0.0, 9.0),
            p(10.0, -7.0),
            p(20.0, 8.0),
            p(30.0, -6.0),
            p(40.0, 5.0),
            p(50.0, 2.0),
        ];
        let original = route.clone();
        fair_route(&mut route, true);

        for (before, after) in original.iter().zip(&route) {
            assert_eq!(before.x, after.x);
        }
        assert_eq!(route[1].z, original[1].z);
        assert_eq!(route[route.len() - 2].z, original[original.len() - 2].z);
        assert_ne!(route[3].z, original[3].z);
    }

    #[test]
    fn fair_route_extends_both_boundary_tails_on_the_interior_bearings() {
        let mut route = vec![
            p(-20.0, 5.0),
            p(-10.0, 2.0),
            p(0.0, 0.0),
            p(10.0, 8.0),
            p(20.0, -5.0),
            p(30.0, 4.0),
            p(40.0, 7.0),
            p(50.0, 9.0),
        ];
        fair_route(&mut route, true);

        let first_slope = (route[1].z - route[2].z) / (route[1].x - route[2].x);
        let first_tail_slope = (route[0].z - route[1].z) / (route[0].x - route[1].x);
        let last_slope = (route[route.len() - 2].z - route[route.len() - 3].z)
            / (route[route.len() - 2].x - route[route.len() - 3].x);
        let last_tail_slope = (route[route.len() - 1].z - route[route.len() - 2].z)
            / (route[route.len() - 1].x - route[route.len() - 2].x);
        assert!((first_slope - first_tail_slope).abs() < 1.0e-5);
        assert!((last_slope - last_tail_slope).abs() < 1.0e-5);
    }

    #[test]
    fn fair_route_is_equivalent_when_the_route_order_is_reversed() {
        let original = vec![
            p(-30.0, -7.0),
            p(-20.0, -3.0),
            p(-10.0, 8.0),
            p(0.0, -6.0),
            p(10.0, 9.0),
            p(20.0, -5.0),
            p(30.0, 4.0),
            p(40.0, 6.0),
        ];
        let mut forward = original.clone();
        let mut reverse: Vec<_> = original.iter().copied().rev().collect();
        fair_route(&mut forward, true);
        fair_route(&mut reverse, true);
        reverse.reverse();

        for (a, b) in forward.iter().zip(&reverse) {
            assert!((a.x - b.x).abs() < 1.0e-5);
            assert!((a.z - b.z).abs() < 1.0e-5);
        }
    }

    #[test]
    fn fair_route_preserves_an_already_linear_route() {
        let mut route: Vec<_> = (0..8)
            .map(|i| p(i as f32 * 10.0, 2.0 + i as f32 * 3.0))
            .collect();
        let original = route.clone();
        fair_route(&mut route, true);
        assert_eq!(route, original);
    }
}
