use crate::*;

pub(crate) const RIVER_HALF_WIDTH: f32 = 32.0;

pub(crate) const RIVER_BANK_BUFFER: f32 = 6.0;

impl Engine {
    pub(crate) fn parcel_clear_of_river(&self, x: f32, z: f32, width: f32, depth: f32) -> bool {
        let bounding_radius = (width * 0.5).hypot(depth * 0.5);
        self.distance_to_river(x, z) >= RIVER_HALF_WIDTH + RIVER_BANK_BUFFER + bounding_radius
    }

    pub(crate) fn ordinary_road_clear_of_river(
        &self,
        ax: f32,
        az: f32,
        bx: f32,
        bz: f32,
        half_width: f32,
    ) -> bool {
        let required = RIVER_HALF_WIDTH + RIVER_BANK_BUFFER + half_width;
        let points = self.river_control_points();
        (0..points.len() - 1).all(|index| {
            segments_distance_coords(
                ax,
                az,
                bx,
                bz,
                points[index].x,
                points[index].z,
                points[index + 1].x,
                points[index + 1].z,
            ) >= required
        })
    }

    pub(crate) fn distance_to_river(&self, x: f32, z: f32) -> f32 {
        let points = self.river_control_points();
        (0..points.len() - 1)
            .map(|index| {
                point_segment_distance(
                    x,
                    z,
                    points[index].x,
                    points[index].z,
                    points[index + 1].x,
                    points[index + 1].z,
                )
            })
            .min_by(f32::total_cmp)
            .unwrap_or(f32::INFINITY)
    }

    pub(crate) fn river_center_x_at(&self, z: f32) -> f32 {
        let points = self.river_control_points();
        for segment in points.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            if z >= a.z && z <= b.z {
                let t = ((z - a.z) / (b.z - a.z)).clamp(0.0, 1.0);
                return a.x + (b.x - a.x) * t;
            }
        }
        if z < points[0].z {
            points[0].x
        } else {
            points[points.len() - 1].x
        }
    }

    pub(crate) fn river_points(&self) -> Vec<Point> {
        self.river_control_points().to_vec()
    }

    pub(crate) fn river_control_points(&self) -> [Point; world::river_course::SAMPLES] {
        world::river_course::sampled(self.state.seed).map(|p| Point { x: p.x, z: p.z })
    }
}
