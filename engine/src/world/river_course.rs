//! Shared bounded river and perpendicular bank alignments for terrain and planning.
use crate::infrastructure::masterplan::P;

pub(crate) const SAMPLES: usize = 33;

/// Four smooth lobes through coarse planning anchors, sampled eight times per
/// span. Zero slope at each extremum prevents overshoot and keeps boundary
/// extensions tangent. Scalar polynomial arithmetic is native/WASM portable.
pub(crate) fn sampled(seed: u32) -> [P; SAMPLES] {
    let offset = (seed % 5) as f32 * 9.0 - 18.0;
    let amplitude = 0.85 + ((seed >> 3) % 7) as f32 * 0.05;
    let z = [-400.0, -190.0, 20.0, 210.0, 400.0];
    let x = [-130.0, -75.0, -155.0, -65.0, -110.0];
    std::array::from_fn(|index| {
        let span = (index / 8).min(3);
        let t = (index - span * 8) as f32 / 8.0;
        let smooth = t * t * (3.0 - 2.0 * t);
        P {
            x: -110.0 + offset + ((x[span] + (x[span + 1] - x[span]) * smooth) + 110.0) * amplitude,
            z: z[span] + (z[span + 1] - z[span]) * t,
        }
    })
}

/// Parallel bank centerline. Intersect adjacent offset segments so joins share
/// one vertex; a fixed x offset loses perpendicular clearance on steep bends.
/// Keep the boundary stations at the river's z limits by extending the end
/// segments. Intended for the bounded, z-monotone course above, not arbitrary
/// paths with reversals or tight hairpins.
pub(crate) fn bank_alignment(course: &[P], signed_offset: f32) -> Vec<P> {
    assert!(course.len() >= 2);
    let normals: Vec<P> = course
        .windows(2)
        .map(|pair| {
            let dx = pair[1].x - pair[0].x;
            let dz = pair[1].z - pair[0].z;
            assert!(dz > 0.0, "river stations must increase");
            let length = (dx * dx + dz * dz).sqrt();
            P {
                x: dz / length,
                z: -dx / length,
            }
        })
        .collect();
    course
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == 0 || i == course.len() - 1 {
                let n = normals[if i == 0 { 0 } else { normals.len() - 1 }];
                // Offset supporting line evaluated at the original end station.
                P {
                    x: p.x + signed_offset / n.x,
                    z: p.z,
                }
            } else {
                let a = normals[i - 1];
                let b = normals[i];
                let scale = signed_offset / (1.0 + a.x * b.x + a.z * b.z);
                P {
                    x: p.x + (a.x + b.x) * scale,
                    z: p.z + (a.z + b.z) * scale,
                }
            }
        })
        .collect()
}

#[test]
fn bank_alignment_preserves_perpendicular_clearance() {
    // The course has 5 offsets x 7 amplitudes; cover every geometry variant,
    // including all three visual-review seeds and the largest u32 seed.
    for seed in (0..280).chain([87231, u32::MAX]) {
        let river = sampled(seed);
        for offset in [-47.0, 47.0] {
            let bank = bank_alignment(&river, offset);
            assert_eq!(bank.len(), river.len());
            assert_eq!(bank[0].z, river[0].z);
            assert_eq!(bank.last().unwrap().z, river.last().unwrap().z);
            for edge in bank.windows(2) {
                assert!(edge[1].z > edge[0].z, "bank folds at seed {seed}");
                for water in river.windows(2) {
                    let distance = crate::segments_distance_coords(
                        edge[0].x, edge[0].z, edge[1].x, edge[1].z, water[0].x, water[0].z,
                        water[1].x, water[1].z,
                    );
                    assert!(
                        distance >= 47.0 - 0.001,
                        "seed {seed}, offset {offset}: distance {distance}"
                    );
                }
            }
        }
    }
}

#[test]
fn diagonal_bank_uses_normal_distance_and_keeps_boundary_stations() {
    let river = [P { x: 0.0, z: 0.0 }, P { x: 100.0, z: 100.0 }];
    for offset in [-47.0, 47.0] {
        let bank = bank_alignment(&river, offset);
        for (p, q) in river.iter().zip(&bank) {
            assert!((q.x - p.x - offset * 2.0_f32.sqrt()).abs() < 0.0001);
            assert_eq!(p.z, q.z);
        }
    }
}

#[test]
fn sampled_river_has_ordered_stations_without_overshoot() {
    for seed in [0, 5, 7, 41, 87231, u32::MAX] {
        let points = sampled(seed);
        assert_eq!(points, sampled(seed));
        assert_eq!(points[0].z, -400.0);
        assert_eq!(points[32].z, 400.0);
        for span in 0..4 {
            let lo = points[span * 8].x.min(points[(span + 1) * 8].x);
            let hi = points[span * 8].x.max(points[(span + 1) * 8].x);
            for pair in points[span * 8..=(span + 1) * 8].windows(2) {
                assert!(pair[1].z > pair[0].z);
                assert!(pair[1].x >= lo - 0.001 && pair[1].x <= hi + 0.001);
            }
        }
    }
}
