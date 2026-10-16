//! Bounded rectangular subdivision with a shared center street spine.
//!
//! The center spine always follows input edge 0's direction. Each half-block is
//! passed through the ordinary two-row subdivider. This
//! yields two perimeter-facing rows and two rows facing the center spine,
//! while preserving the source perimeter's edge indices and directions.

use crate::infrastructure::blocks::{subdivide_rectangular_block, BlockLot, BlockPoint};

const MAX_SPINE_LOTS: usize = 64;
const RECTANGLE_RELATIVE_TOLERANCE: f32 = 1.0e-4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpineFrontage {
    /// The index refers to the original directed edge `corners[i] -> corners[(i+1)%4]`.
    Perimeter(usize),
    /// The sign identifies the lot's side of `spine[0] -> spine[1]`.
    Spine(i8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpineLot {
    pub(crate) center: BlockPoint,
    pub(crate) width: f32,
    pub(crate) depth: f32,
    pub(crate) rotation: f32,
    pub(crate) frontage: SpineFrontage,
    /// Normalized position along the actual directed frontage segment.
    pub(crate) frontage_t: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpineBlockLayout {
    /// Center street segment, directed parallel to the block's long axis.
    pub(crate) spine: [BlockPoint; 2],
    pub(crate) lots: Vec<SpineLot>,
}

/// Subdivide a rectangular perimeter into four rows around a center spine.
///
/// Each half uses `subdivide_rectangular_block` with the same inset, target
/// frontage, and gap. A result over 64 total lots is rejected rather than
/// truncated so the layout remains complete and symmetric.
pub(crate) fn subdivide_spine_block(
    corners: [BlockPoint; 4],
    street_inset: f32,
    target_frontage: f32,
    gap: f32,
    min_lot_depth: f32,
) -> Option<SpineBlockLayout> {
    if !street_inset.is_finite()
        || street_inset < 0.0
        || !target_frontage.is_finite()
        || target_frontage <= 0.0
        || !gap.is_finite()
        || gap < 0.0
        || !min_lot_depth.is_finite()
        || min_lot_depth <= 0.0
        || !valid_rectangle(corners)
    {
        return None;
    }

    let edge_lengths = edges(corners).map(length);
    // The standard helper chooses edge 0 as frontage only when it is at least
    // as long as edge 1. Each half halves edge 1, so reject extreme aspect
    // ratios rather than silently rotating the intended spine by 90 degrees.
    if edge_lengths[0] < edge_lengths[1] * 0.5 {
        return None;
    }
    let midpoint_1 = midpoint(corners[1], corners[2]);
    let midpoint_3 = midpoint(corners[3], corners[0]);
    let spine = [midpoint_3, midpoint_1];
    let halves = [
        [corners[0], corners[1], midpoint_1, midpoint_3],
        [midpoint_3, midpoint_1, corners[2], corners[3]],
    ];

    let mut lots = Vec::new();
    for half in halves {
        let half_lots = subdivide_rectangular_block(half, street_inset, target_frontage, gap);
        if half_lots.is_empty() || lots.len() + half_lots.len() > MAX_SPINE_LOTS {
            return None;
        }
        for lot in half_lots {
            if !lot.center.x.is_finite()
                || !lot.center.z.is_finite()
                || !lot.width.is_finite()
                || !lot.depth.is_finite()
                || !lot.rotation.is_finite()
                || !lot.frontage_t.is_finite()
                || lot.width <= 0.0
                || lot.depth < min_lot_depth
                || !(0.0..=1.0).contains(&lot.frontage_t)
            {
                return None;
            }
            lots.push(map_lot(lot, half, corners, spine)?);
        }
    }
    if lots.is_empty() {
        return None;
    }

    Some(SpineBlockLayout { spine, lots })
}

fn map_lot(
    lot: BlockLot,
    half: [BlockPoint; 4],
    perimeter: [BlockPoint; 4],
    spine: [BlockPoint; 2],
) -> Option<SpineLot> {
    let local_edge = lot.frontage_edge;
    let start = half[local_edge];
    let end = half[(local_edge + 1) % 4];
    for edge_index in 0..4 {
        let perimeter_start = perimeter[edge_index];
        let perimeter_end = perimeter[(edge_index + 1) % 4];
        if same_point(start, perimeter_start) && same_point(end, perimeter_end) {
            return Some(SpineLot {
                center: lot.center,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
                frontage: SpineFrontage::Perimeter(edge_index),
                frontage_t: lot.frontage_t,
            });
        }
    }

    // The only remaining frontage edge is the shared spine. Normalize its
    // parameter to the layout's direction, which may oppose the half edge.
    let same_direction = same_point(start, spine[0]) && same_point(end, spine[1]);
    if !same_direction && !(same_point(start, spine[1]) && same_point(end, spine[0])) {
        return None;
    }
    let frontage_t = if same_direction {
        lot.frontage_t
    } else {
        1.0 - lot.frontage_t
    };
    let tangent = subtract(spine[1], spine[0]);
    let from_spine = subtract(lot.center, midpoint(spine[0], spine[1]));
    let side = if cross(tangent, from_spine) >= 0.0 {
        1
    } else {
        -1
    };
    Some(SpineLot {
        center: lot.center,
        width: lot.width,
        depth: lot.depth,
        rotation: lot.rotation,
        frontage: SpineFrontage::Spine(side),
        frontage_t,
    })
}

fn valid_rectangle(corners: [BlockPoint; 4]) -> bool {
    if corners
        .iter()
        .any(|point| !point.x.is_finite() || !point.z.is_finite())
    {
        return false;
    }
    let edge = edges(corners);
    let lengths = edge.map(length);
    if lengths
        .iter()
        .any(|length| !length.is_finite() || *length <= f32::EPSILON)
    {
        return false;
    }
    let scale = lengths.iter().copied().fold(0.0_f32, f32::max).max(1.0);
    let length_tolerance = scale * RECTANGLE_RELATIVE_TOLERANCE;
    let area_twice = cross(edge[0], edge[1]);
    area_twice.is_finite()
        && area_twice.abs() > f32::EPSILON
        && (lengths[0] - lengths[2]).abs() <= length_tolerance
        && (lengths[1] - lengths[3]).abs() <= length_tolerance
        && dot(edge[0], edge[1]).abs() <= lengths[0] * lengths[1] * RECTANGLE_RELATIVE_TOLERANCE
        && dot(edge[1], edge[2]).abs() <= lengths[1] * lengths[2] * RECTANGLE_RELATIVE_TOLERANCE
        && dot(edge[2], edge[3]).abs() <= lengths[2] * lengths[3] * RECTANGLE_RELATIVE_TOLERANCE
        && dot(edge[3], edge[0]).abs() <= lengths[3] * lengths[0] * RECTANGLE_RELATIVE_TOLERANCE
        && distance(edge[0], scale_neg(edge[2])) <= length_tolerance
        && distance(edge[1], scale_neg(edge[3])) <= length_tolerance
}

fn edges(corners: [BlockPoint; 4]) -> [BlockPoint; 4] {
    [
        subtract(corners[1], corners[0]),
        subtract(corners[2], corners[1]),
        subtract(corners[3], corners[2]),
        subtract(corners[0], corners[3]),
    ]
}

fn midpoint(a: BlockPoint, b: BlockPoint) -> BlockPoint {
    BlockPoint {
        x: (a.x + b.x) * 0.5,
        z: (a.z + b.z) * 0.5,
    }
}

fn same_point(a: BlockPoint, b: BlockPoint) -> bool {
    (a.x - b.x).abs() <= 1.0e-4 && (a.z - b.z).abs() <= 1.0e-4
}

fn subtract(a: BlockPoint, b: BlockPoint) -> BlockPoint {
    BlockPoint {
        x: a.x - b.x,
        z: a.z - b.z,
    }
}

fn scale_neg(vector: BlockPoint) -> BlockPoint {
    BlockPoint {
        x: -vector.x,
        z: -vector.z,
    }
}

fn length(vector: BlockPoint) -> f32 {
    vector.x.hypot(vector.z)
}

fn dot(a: BlockPoint, b: BlockPoint) -> f32 {
    a.x * b.x + a.z * b.z
}

fn cross(a: BlockPoint, b: BlockPoint) -> f32 {
    a.x * b.z - a.z * b.x
}

fn distance(a: BlockPoint, b: BlockPoint) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

#[cfg(test)]
mod tests {
    use super::{subdivide_spine_block, SpineFrontage};
    use crate::infrastructure::blocks::BlockPoint;
    use crate::legacy::spine_plan::{SpinePlan, SpineStage, RECIPE_VERSION};
    use crate::world::geometry::{rects_overlap, OrientedRect};
    use std::f32::consts::FRAC_PI_4;

    fn point(x: f32, z: f32) -> BlockPoint {
        BlockPoint { x, z }
    }

    fn rectangle(length: f32, depth: f32, rotation: f32, clockwise: bool) -> [BlockPoint; 4] {
        let center = point(31.0, -22.0);
        let axis_x = point(rotation.cos(), -rotation.sin());
        let axis_z = point(rotation.sin(), rotation.cos());
        let c0 = add(
            center,
            add(scale(axis_x, -length * 0.5), scale(axis_z, -depth * 0.5)),
        );
        let c1 = add(c0, scale(axis_x, length));
        let c2 = add(c1, scale(axis_z, depth));
        let c3 = add(c0, scale(axis_z, depth));
        if clockwise {
            [c0, c3, c2, c1]
        } else {
            [c0, c1, c2, c3]
        }
    }

    fn add(a: BlockPoint, b: BlockPoint) -> BlockPoint {
        point(a.x + b.x, a.z + b.z)
    }

    fn scale(a: BlockPoint, factor: f32) -> BlockPoint {
        point(a.x * factor, a.z * factor)
    }

    fn point_segment_distance(point: BlockPoint, start: BlockPoint, end: BlockPoint) -> f32 {
        let edge = BlockPoint {
            x: end.x - start.x,
            z: end.z - start.z,
        };
        let length_squared = edge.x * edge.x + edge.z * edge.z;
        let t = (((point.x - start.x) * edge.x + (point.z - start.z) * edge.z) / length_squared)
            .clamp(0.0, 1.0);
        (point.x - (start.x + edge.x * t)).hypot(point.z - (start.z + edge.z * t))
    }

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() <= 1.0e-3, "{a} != {b}");
    }

    #[test]
    fn square_produces_four_balanced_rows_and_twenty_lots() {
        let layout = subdivide_spine_block(rectangle(82.0, 82.0, 0.0, false), 8.0, 11.0, 2.2, 8.0)
            .expect("square should fit four rows");
        assert_eq!(layout.lots.len(), 20);
        assert!(layout
            .lots
            .iter()
            .all(|lot| (lot.depth - 11.4).abs() < 1.0e-4));
        assert!(layout
            .lots
            .iter()
            .all(|lot| (lot.width - 11.44).abs() < 1.0e-4));
        assert_eq!(
            layout
                .lots
                .iter()
                .filter(|lot| matches!(lot.frontage, SpineFrontage::Spine(_)))
                .count(),
            10
        );
        assert_eq!(
            layout
                .lots
                .iter()
                .filter(|lot| matches!(lot.frontage, SpineFrontage::Perimeter(_)))
                .count(),
            10
        );
        let frontages: Vec<_> = layout
            .lots
            .iter()
            .filter_map(|lot| match lot.frontage {
                SpineFrontage::Perimeter(edge) => Some(edge),
                SpineFrontage::Spine(_) => None,
            })
            .collect();
        assert!(frontages.iter().all(|edge| *edge == 0 || *edge == 2));
    }

    #[test]
    fn deeper_spine_recipes_keep_twenty_contained_clear_lots_and_canonical_plans() {
        let expected_depths = [(82.0, 11.4), (100.0, 15.9), (110.0, 18.4), (114.0, 19.4)];
        for (block_depth, expected_lot_depth) in expected_depths {
            for clockwise in [false, true] {
                for rotation in [0.0, FRAC_PI_4] {
                    // Reversing the winding also reverses which physical
                    // dimension lies on edge 0; keep the 82 m spine axis on
                    // edge 0 so this remains the same recipe under reversal.
                    let corners = if clockwise {
                        rectangle(block_depth, 82.0, rotation, true)
                    } else {
                        rectangle(82.0, block_depth, rotation, false)
                    };
                    let layout = subdivide_spine_block(corners, 8.0, 11.0, 2.2, 8.0).unwrap();
                    assert_eq!(layout.lots.len(), 20, "depth {block_depth}");
                    assert!(layout
                        .lots
                        .iter()
                        .all(|lot| (lot.width - 11.44).abs() < 1.0e-4));
                    assert!(layout
                        .lots
                        .iter()
                        .all(|lot| (lot.depth - expected_lot_depth).abs() < 1.0e-4));

                    let block_center = point(
                        corners.iter().map(|p| p.x).sum::<f32>() * 0.25,
                        corners.iter().map(|p| p.z).sum::<f32>() * 0.25,
                    );
                    let edge_x = point(corners[1].x - corners[0].x, corners[1].z - corners[0].z);
                    let edge_z = point(corners[3].x - corners[0].x, corners[3].z - corners[0].z);
                    let edge_x_length = edge_x.x.hypot(edge_x.z);
                    let edge_z_length = edge_z.x.hypot(edge_z.z);
                    let axis_x = point(edge_x.x / edge_x_length, edge_x.z / edge_x_length);
                    let axis_z = point(edge_z.x / edge_z_length, edge_z.z / edge_z_length);
                    let mut nominal_area = 0.0;
                    for (index, lot) in layout.lots.iter().enumerate() {
                        nominal_area += lot.width * lot.depth;
                        let rect = OrientedRect {
                            x: lot.center.x,
                            z: lot.center.z,
                            width: lot.width,
                            depth: lot.depth,
                            rotation: lot.rotation,
                        };
                        let relative =
                            point(lot.center.x - block_center.x, lot.center.z - block_center.z);
                        let center_x = relative.x * axis_x.x + relative.z * axis_x.z;
                        let center_z = relative.x * axis_z.x + relative.z * axis_z.z;
                        for local_x in [-lot.width * 0.5, lot.width * 0.5] {
                            for local_z in [-lot.depth * 0.5, lot.depth * 0.5] {
                                let world = point(
                                    lot.center.x
                                        + lot.rotation.cos() * local_x
                                        + lot.rotation.sin() * local_z,
                                    lot.center.z - lot.rotation.sin() * local_x
                                        + lot.rotation.cos() * local_z,
                                );
                                let from_center =
                                    point(world.x - block_center.x, world.z - block_center.z);
                                let x = from_center.x * axis_x.x + from_center.z * axis_x.z;
                                let z = from_center.x * axis_z.x + from_center.z * axis_z.z;
                                assert!(
                                    x.abs() <= edge_x_length * 0.5 + 1.0e-3,
                                    "lot escaped frontage bounds"
                                );
                                assert!(
                                    z.abs() <= edge_z_length * 0.5 + 1.0e-3,
                                    "lot escaped cross-depth bounds"
                                );
                            }
                        }
                        assert!(
                            center_x.abs() <= edge_x_length * 0.5
                                && center_z.abs() <= edge_z_length * 0.5
                        );
                        for other in layout.lots.iter().skip(index + 1) {
                            let other_rect = OrientedRect {
                                x: other.center.x,
                                z: other.center.z,
                                width: other.width,
                                depth: other.depth,
                                rotation: other.rotation,
                            };
                            assert!(!rects_overlap(&rect, &other_rect, 0.0));
                        }

                        let (frontage_start, frontage_end) = match lot.frontage {
                            SpineFrontage::Spine(_) => (layout.spine[0], layout.spine[1]),
                            SpineFrontage::Perimeter(edge) => {
                                assert!(edge == 0 || edge == 2);
                                (corners[edge], corners[(edge + 1) % 4])
                            }
                        };
                        let edge = point(
                            frontage_end.x - frontage_start.x,
                            frontage_end.z - frontage_start.z,
                        );
                        let from_start = point(
                            lot.center.x - frontage_start.x,
                            lot.center.z - frontage_start.z,
                        );
                        let projected_t = (from_start.x * edge.x + from_start.z * edge.z)
                            / (edge.x * edge.x + edge.z * edge.z);
                        approx(projected_t, lot.frontage_t);
                        assert!((0.0..=1.0).contains(&lot.frontage_t));
                    }
                    let expected_area = 20.0 * 11.44 * expected_lot_depth;
                    assert!((nominal_area - expected_area).abs() <= 0.01);

                    let plan = SpinePlan {
                        corners,
                        recipe_version: RECIPE_VERSION,
                        stage: SpineStage::Reserved,
                    };
                    assert_eq!(plan.validate().unwrap(), layout);
                }
            }
        }
    }

    #[test]
    fn rotated_and_reversed_winding_keep_valid_spine_sides_and_parameters() {
        for clockwise in [false, true] {
            let corners = rectangle(82.0, 82.0, FRAC_PI_4, clockwise);
            let layout = subdivide_spine_block(corners, 8.0, 11.0, 2.2, 8.0).unwrap();
            assert_eq!(layout.lots.len(), 20);
            let edge0 = point(corners[1].x - corners[0].x, corners[1].z - corners[0].z);
            let spine_direction = point(
                layout.spine[1].x - layout.spine[0].x,
                layout.spine[1].z - layout.spine[0].z,
            );
            approx(
                edge0.x * spine_direction.z - edge0.z * spine_direction.x,
                0.0,
            );
            assert!(edge0.x * spine_direction.x + edge0.z * spine_direction.z > 0.0);
            assert!(layout
                .lots
                .iter()
                .all(|lot| (0.0..=1.0).contains(&lot.frontage_t)));
            let spine_sides: Vec<_> = layout
                .lots
                .iter()
                .filter_map(|lot| match lot.frontage {
                    SpineFrontage::Spine(side) => Some(side),
                    SpineFrontage::Perimeter(_) => None,
                })
                .collect();
            assert_eq!(spine_sides.iter().filter(|side| **side == -1).count(), 5);
            assert_eq!(spine_sides.iter().filter(|side| **side == 1).count(), 5);
            for lot in &layout.lots {
                let (start, end) = match lot.frontage {
                    SpineFrontage::Spine(_) => (layout.spine[0], layout.spine[1]),
                    SpineFrontage::Perimeter(edge) => (corners[edge], corners[(edge + 1) % 4]),
                };
                let edge = point(end.x - start.x, end.z - start.z);
                let from_start = point(lot.center.x - start.x, lot.center.z - start.z);
                let projected_t = (from_start.x * edge.x + from_start.z * edge.z)
                    / (edge.x * edge.x + edge.z * edge.z);
                approx(projected_t, lot.frontage_t);
            }
        }
    }

    #[test]
    fn shallow_invalid_and_over_cap_blocks_are_rejected() {
        assert!(
            subdivide_spine_block(rectangle(82.0, 50.0, 0.0, false), 8.0, 11.0, 2.2, 8.0).is_none()
        );
        assert!(
            subdivide_spine_block(rectangle(82.0, 82.0, 0.0, false), f32::NAN, 11.0, 2.2, 8.0)
                .is_none()
        );
        assert!(
            subdivide_spine_block(rectangle(50.0, 120.0, 0.0, false), 8.0, 11.0, 2.2, 8.0)
                .is_none()
        );
        let mut invalid = rectangle(82.0, 82.0, 0.0, false);
        invalid[2].x = f32::INFINITY;
        assert!(subdivide_spine_block(invalid, 8.0, 11.0, 2.2, 8.0).is_none());
        assert!(
            subdivide_spine_block(rectangle(400.0, 400.0, 0.0, false), 1.0, 1.0, 0.0, 1.0)
                .is_none()
        );
    }

    #[test]
    fn spine_minimum_cross_depth_preserves_eight_metre_rows() {
        let minimum = subdivide_spine_block(rectangle(82.0, 68.4, 0.0, false), 8.0, 11.0, 2.2, 8.0)
            .expect("68.4 m supports four 8 m minimum-depth rows");
        assert_eq!(minimum.lots.len(), 20);
        assert!(minimum
            .lots
            .iter()
            .all(|lot| (lot.depth - 8.0).abs() < 1.0e-4));
        assert!(
            subdivide_spine_block(rectangle(82.0, 68.3, 0.0, false), 8.0, 11.0, 2.2, 8.0,)
                .is_none()
        );
    }

    #[test]
    fn rows_respect_frontage_setbacks_do_not_overlap_and_conserve_lot_area() {
        let corners = rectangle(82.0, 82.0, FRAC_PI_4, false);
        let inset = 8.0;
        let layout = subdivide_spine_block(corners, inset, 11.0, 2.2, 8.0).unwrap();
        let mut lot_area = 0.0;
        for (index, lot) in layout.lots.iter().enumerate() {
            lot_area += lot.width * lot.depth;
            let rect = OrientedRect {
                x: lot.center.x,
                z: lot.center.z,
                width: lot.width,
                depth: lot.depth,
                rotation: lot.rotation,
            };
            for other in layout.lots.iter().skip(index + 1) {
                let other_rect = OrientedRect {
                    x: other.center.x,
                    z: other.center.z,
                    width: other.width,
                    depth: other.depth,
                    rotation: other.rotation,
                };
                assert!(!rects_overlap(&rect, &other_rect, 0.0));
            }
            let (road_start, road_end) = match lot.frontage {
                SpineFrontage::Spine(_) => (layout.spine[0], layout.spine[1]),
                SpineFrontage::Perimeter(edge) => (corners[edge], corners[(edge + 1) % 4]),
            };
            let local_z = point(lot.rotation.sin(), lot.rotation.cos());
            let front_center = point(
                lot.center.x + local_z.x * lot.depth * 0.5,
                lot.center.z + local_z.z * lot.depth * 0.5,
            );
            approx(
                point_segment_distance(front_center, road_start, road_end),
                inset,
            );
        }
        assert!(lot_area > 0.0 && lot_area < 82.0 * 82.0);
        assert!(layout
            .lots
            .iter()
            .all(|lot| (lot.width - 11.44).abs() < 1.0e-4));
        assert!(layout
            .lots
            .iter()
            .all(|lot| (lot.depth - 11.4).abs() < 1.0e-4));
        let expected_occupied_frontage = (82.0 - 2.0 * inset) - 4.0 * 2.2;
        let expected_lot_area = 4.0 * expected_occupied_frontage * 11.4;
        approx(lot_area, expected_lot_area);
    }
}
