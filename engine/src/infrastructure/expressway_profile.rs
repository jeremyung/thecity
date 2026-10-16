//! Cross sections shared with the renderer; compact remains an isolated experiment.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct ExpresswayProfile {
    pub width: f32,
    pub lane_centers: [f32; 2],
    pub inner_mark: f32,
    pub lane_mark: f32,
    pub outer_mark: f32,
    pub median_width: f32,
}
#[derive(Deserialize)]
struct Profiles {
    current: ExpresswayProfile,
    compact: ExpresswayProfile,
}
fn profiles() -> &'static Profiles {
    static PROFILES: OnceLock<Profiles> = OnceLock::new();
    PROFILES.get_or_init(|| {
        serde_json::from_str(include_str!("../../../shared/expressway-profiles.json"))
            .expect("checked-in cross sections")
    })
}
pub(crate) fn active() -> ExpresswayProfile {
    if cfg!(feature = "compact-expressway-candidate") {
        profiles().compact
    } else {
        profiles().current
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_lanes_fit_trucks_between_markings_and_barriers() {
        let p = profiles().compact;
        assert_eq!(p.width, 16.0);
        for (center, lo, hi) in [
            (p.lane_centers[0], p.inner_mark, p.lane_mark),
            (p.lane_centers[1], p.lane_mark, p.outer_mark),
        ] {
            // 2.5m truck body, plus at least 0.3m on each side to lane marks.
            assert!(center - 1.25 - lo >= 0.3 && hi - center - 1.25 >= 0.3);
        }
        assert!(p.inner_mark - p.median_width / 2.0 > 0.4);
        assert!(p.width / 2.0 - p.outer_mark >= 0.69);
    }
}
