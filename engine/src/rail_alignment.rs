//! Shared curved rail geometry for corridor reservation, train motion and rendering.
use crate::masterplan::P;

const PLATFORM_APPROACH: f32 = crate::RAIL_PLATFORM_END + 2.0;
// Keep the entire inward platform, stairs and apron on straight track.
const TERMINAL_APPROACH: f32 = crate::RAIL_GATEWAY_STAIR_BOTTOM + 5.0;
const SAMPLE_STEP: f32 = 2.0;
const MIN_RADIUS: f32 = 120.0;

#[derive(Clone, Debug)]
pub(crate) struct Alignment {
    knots: [P; 6],
    /// Exact analytic positions with monotonically increasing chord distances.
    samples: Vec<(f32, P)>,
}

impl Alignment {
    /// Keep the complete platform and terminal approaches inside the city, while
    /// staying clear of street columns. Preserve the existing choice when legal.
    pub(crate) fn station_x(preferred: f32, edge: f32, columns: &[f32]) -> f32 {
        let min = 110.0;
        let max = edge - TERMINAL_APPROACH - PLATFORM_APPROACH - 1.0;
        let legal = |x: f32| x >= min && x <= max && columns.iter().all(|c| (c - x).abs() > 22.0);
        if legal(preferred) {
            return preferred;
        }
        let mut candidates = vec![min, max];
        for c in columns {
            candidates.extend([c - 22.1, c + 22.1]);
        }
        candidates
            .into_iter()
            .filter(|x| legal(*x))
            .min_by(|a, b| (a - preferred).abs().total_cmp(&(b - preferred).abs()))
            .expect("station corridor has no legal platform approach")
    }

    pub(crate) fn new(station: P, edge: f32, seed: u32) -> Self {
        let left = -edge + TERMINAL_APPROACH;
        let right = edge - TERMINAL_APPROACH;
        let station_left = station.x - PLATFORM_APPROACH;
        let station_right = station.x + PLATFORM_APPROACH;
        assert!(
            left < station_left && station_right < right,
            "station needs a straight platform approach inside the terminal approaches"
        );
        // Quintic smoothstep has |second derivative| < 6. Bounding amplitude
        // by span²/(6R) therefore bounds curvature by 1/R, even on short arms.
        let offset = |span: f32, desired: f32| desired.min(span * span / (6.0 * MIN_RADIUS));
        let west = offset(station_left - left, 30.0 + (seed % 4) as f32 * 4.0);
        let east = offset(right - station_right, 24.0 + ((seed >> 2) % 4) as f32 * 4.0);
        let knots = [
            P {
                x: -edge,
                z: station.z - west,
            },
            P {
                x: left,
                z: station.z - west,
            },
            P {
                x: station_left,
                z: station.z,
            },
            P {
                x: station_right,
                z: station.z,
            },
            P {
                x: right,
                z: station.z + east,
            },
            P {
                x: edge,
                z: station.z + east,
            },
        ];
        let mut path = Self {
            knots,
            samples: vec![(0.0, knots[0])],
        };
        for span in knots.windows(2) {
            let steps = ((span[1].x - span[0].x) / SAMPLE_STEP).ceil() as usize;
            for i in 1..=steps {
                let x = span[0].x + (span[1].x - span[0].x) * i as f32 / steps as f32;
                let p = path.at_x(x).0;
                let (distance, previous) = *path.samples.last().unwrap();
                let dx = p.x - previous.x;
                let dz = p.z - previous.z;
                path.samples
                    .push((distance + (dx * dx + dz * dz).sqrt(), p));
            }
        }
        path
    }

    /// Position and eastward unit tangent, with C2 joins and no overshoot.
    pub(crate) fn at_x(&self, x: f32) -> (P, P) {
        let x = x.clamp(self.knots[0].x, self.knots[5].x);
        let i = self
            .knots
            .partition_point(|p| p.x < x)
            .saturating_sub(1)
            .min(4);
        let a = self.knots[i];
        let b = self.knots[i + 1];
        let t = ((x - a.x) / (b.x - a.x)).clamp(0.0, 1.0);
        let smooth = t * t * t * (10.0 + t * (-15.0 + 6.0 * t));
        let slope = (b.z - a.z) / (b.x - a.x) * 30.0 * t * t * (1.0 - t) * (1.0 - t);
        let norm = (1.0 + slope * slope).sqrt();
        (
            P {
                x,
                z: a.z + (b.z - a.z) * smooth,
            },
            P {
                x: 1.0 / norm,
                z: slope / norm,
            },
        )
    }

    pub(crate) fn length(&self) -> f32 {
        self.samples.last().unwrap().0
    }

    /// Parallel corridor edge evaluated at a fixed world x. Inverting the
    /// normal offset keeps its width perpendicular to the track on bends.
    pub(crate) fn boundary_at_x(&self, x: f32, offset: f32) -> P {
        let mut source_x = x;
        for _ in 0..16 {
            let (p, tangent) = self.at_x(source_x);
            source_x += x - (p.x - tangent.z * offset);
        }
        let (p, tangent) = self.at_x(source_x);
        P {
            x,
            z: p.z + tangent.x * offset,
        }
    }

    /// Arc-distance lookup. Each coach can sample its own distance; reversing
    /// service must reverse its travel direction, not the authored coach order.
    pub(crate) fn at_distance(&self, distance: f32) -> (P, P) {
        let distance = distance.clamp(0.0, self.length());
        let i = self
            .samples
            .partition_point(|s| s.0 < distance)
            .saturating_sub(1)
            .min(self.samples.len() - 2);
        let (lo, a) = self.samples[i];
        let (hi, b) = self.samples[i + 1];
        self.at_x(a.x + (b.x - a.x) * (distance - lo) / (hi - lo))
    }

    /// Shared sampled corridor for reservation/export. Includes exact join
    /// sections so neighboring render sweeps need not infer their own tangents.
    pub(crate) fn samples(&self) -> &[(f32, P)] {
        &self.samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_and_station_approaches_fit_the_whole_consist() {
        for seed in [0, 5, 87231, u32::MAX] {
            for preferred in [110.0, 190.0, 260.0] {
                let x = Alignment::station_x(preferred, 392.0, &[]);
                let station = P { x, z: -85.0 };
                let path = Alignment::new(station, 392.0, seed);
                for offset in [
                    -crate::RAIL_PLATFORM_END,
                    -crate::TRAIN_HALF_LENGTH,
                    0.0,
                    crate::TRAIN_HALF_LENGTH,
                    crate::RAIL_PLATFORM_END,
                ] {
                    let (p, tangent) = path.at_x(x + offset);
                    assert_eq!(p.z, station.z);
                    assert_eq!(tangent, P { x: 1.0, z: 0.0 });
                }
                for distance in [
                    0.0,
                    crate::TRAIN_HALF_LENGTH,
                    crate::RAIL_GATEWAY_STAIR_BOTTOM,
                    path.length() - crate::RAIL_GATEWAY_STAIR_BOTTOM,
                    path.length() - crate::TRAIN_HALF_LENGTH,
                    path.length(),
                ] {
                    assert_eq!(path.at_distance(distance).1, P { x: 1.0, z: 0.0 });
                }
                assert!(path.length() > 784.0);
                assert!(path.samples.len() <= 400);
            }
        }
    }

    #[test]
    fn bends_are_monotone_and_respect_radius_bound() {
        for seed in 0..16 {
            for preferred in [110.0, 190.0, 260.0] {
                let x = Alignment::station_x(preferred, 392.0, &[]);
                let path = Alignment::new(P { x, z: 115.0 }, 392.0, seed);
                for pair in path.samples.windows(2) {
                    assert!(pair[1].0 > pair[0].0 && pair[1].1.x > pair[0].1.x);
                    assert!(pair[1].1.z + 0.0001 >= pair[0].1.z);
                    let a = path.at_x(pair[0].1.x).1;
                    let b = path.at_x(pair[1].1.x).1;
                    let turn = ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
                    assert!(turn / (pair[1].0 - pair[0].0) <= 1.0 / MIN_RADIUS + 0.00002);
                }
            }
        }
    }

    #[test]
    fn arc_sampling_tracks_fine_integration_without_chord_shortcuts() {
        let path = Alignment::new(P { x: 150.0, z: -80.0 }, 392.0, 5);
        let mut previous = path.at_x(-392.0).0;
        let mut length = 0.0;
        for i in 1..=15680 {
            let p = path.at_x(-392.0 + i as f32 * 0.05).0;
            length += ((p.x - previous.x).powi(2) + (p.z - previous.z).powi(2)).sqrt();
            let sampled = path.at_distance(length).0;
            assert!((sampled.x - p.x).abs() < 0.04 && (sampled.z - p.z).abs() < 0.04);
            previous = p;
        }
        assert!((length - path.length()).abs() < 0.04);
        let (p, tangent) = path.at_distance(200.0);
        assert!(p.z < -80.0 && tangent.z > 0.0);
    }

    #[test]
    #[ignore]
    fn export_current_plan_alignment_prototypes() {
        let output = std::env::var("RAIL_ALIGNMENT_REPORT").expect("RAIL_ALIGNMENT_REPORT path");
        let mut cases = Vec::new();
        for seed in [87231, 0, 5] {
            let engine = crate::Engine::new(seed);
            let plan = &engine.plan.as_ref().unwrap().plan;
            let station = plan.nodes[plan.station].p;
            let path = Alignment::new(station, crate::masterplan::EDGE, seed);
            cases.push(
                serde_json::json!({"seed":seed,"station":[station.x,station.z],
                "length":path.length(),"samples":path.samples().iter().map(|(d,p)|
                    [*d,p.x,p.z]).collect::<Vec<_>>() }),
            );
        }
        std::fs::write(
            output,
            serde_json::to_string_pretty(&serde_json::json!({
                "scope":"Geometry prototype only; not wired to live rail or parcel reservations",
                "cases":cases
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

#[cfg(test)]
mod station_bounds_tests {
    use super::*;
    #[test]
    fn shifted_station_stays_inside_terminal_approaches() {
        let columns = [112.0, 180.0, 246.0, 320.0];
        for preferred in 110..310 {
            let x = Alignment::station_x(preferred as f32, 392.0, &columns);
            assert!(columns.iter().all(|c| (c - x).abs() > 22.0));
            assert!(x + PLATFORM_APPROACH < 392.0 - TERMINAL_APPROACH);
            let _ = Alignment::new(P { x, z: 0.0 }, 392.0, 0);
        }
        assert_eq!(Alignment::station_x(150.0, 392.0, &columns), 150.0);
    }
}
