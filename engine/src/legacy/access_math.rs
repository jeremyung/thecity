//! Algebraic XZ-vector helpers used when constructing access headings.
//!
//! Avoiding an angle round-trip (`atan2`/`asin`/`sin`/`cos`) keeps equivalent
//! headings stable across native and WebAssembly math libraries.

/// Euclidean length with an explicit operation sequence for deterministic
/// normalization of access vectors.
#[inline]
pub(crate) fn length(x: f32, z: f32) -> f32 {
    (x * x + z * z).sqrt()
}

/// Return the directions at `-asin(clearance / distance)` and
/// `+asin(clearance / distance)` from `(dx, dz)`, in that order.
///
/// This is the vector equivalent of rotating the normalized input by the two
/// signed angles. Callers retain their existing distance and clearance gates.
#[inline]
#[cfg(feature = "legacy")]
pub(crate) fn rotate_clearance_sides(
    dx: f32,
    dz: f32,
    distance: f32,
    clearance: f32,
) -> [(f32, f32); 2] {
    let ux = dx / distance;
    let uz = dz / distance;
    let sine = (clearance / distance).clamp(-1.0, 1.0);
    let cosine = (1.0 - sine * sine).max(0.0).sqrt();
    [
        (ux * cosine + uz * sine, uz * cosine - ux * sine),
        (ux * cosine - uz * sine, uz * cosine + ux * sine),
    ]
}

#[cfg(test)]
mod tests {
    use super::{length, rotate_clearance_sides};

    fn angle_reference(dx: f32, dz: f32, distance: f32, clearance: f32) -> [(f32, f32); 2] {
        let bearing = dz.atan2(dx);
        let offset = (clearance / distance).clamp(-1.0, 1.0).asin();
        [
            ((bearing - offset).cos(), (bearing - offset).sin()),
            ((bearing + offset).cos(), (bearing + offset).sin()),
        ]
    }

    #[test]
    fn algebraic_rotations_match_angle_reference_for_cardinal_and_oblique_vectors() {
        for (dx, dz, clearance) in [
            (100.0, 0.0, 15.0),
            (-100.0, 0.0, 15.0),
            (0.0, 100.0, 15.0),
            (0.0, -100.0, 15.0),
            (83.25, -41.5, 11.0),
            (-23.75, 91.125, 8.0),
            (0.375, 0.625, 0.2),
        ] {
            let distance = length(dx, dz);
            let actual = rotate_clearance_sides(dx, dz, distance, clearance);
            let expected = angle_reference(dx, dz, distance, clearance);
            for side in 0..2 {
                assert!((actual[side].0 - expected[side].0).abs() <= 2.0e-6);
                assert!((actual[side].1 - expected[side].1).abs() <= 2.0e-6);
                assert!(actual[side].0.is_finite() && actual[side].1.is_finite());
                assert!((length(actual[side].0, actual[side].1) - 1.0).abs() <= 2.0e-6);
            }
        }
    }

    #[test]
    fn explicit_length_and_tangent_vectors_stay_finite_for_oblique_inputs() {
        for (dx, dz, clearance) in [(3.0, 4.0, 1.0), (-7.25, 2.5, 3.0), (0.03125, -0.0625, 0.01)] {
            let distance = length(dx, dz);
            assert!(distance.is_finite() && distance > clearance);
            for direction in rotate_clearance_sides(dx, dz, distance, clearance) {
                assert!(direction.0.is_finite() && direction.1.is_finite());
                let tangent = (-direction.1, direction.0);
                assert!((length(tangent.0, tangent.1) - 1.0).abs() < 2.0e-6);
            }
        }
    }
}
