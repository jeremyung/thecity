//! Bounded, state-free subdivision for rectangular city blocks.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct BlockPoint {
    pub(crate) x: f32,
    pub(crate) z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct BlockLot {
    pub(crate) center: BlockPoint,
    pub(crate) frontage_edge: usize,
    pub(crate) frontage_t: f32,
    pub(crate) width: f32,
    pub(crate) depth: f32,
    pub(crate) rotation: f32,
}

const MAX_LOTS: usize = 64;
const MAX_LOTS_PER_ROW: usize = MAX_LOTS / 2;
const RECTANGLE_RELATIVE_TOLERANCE: f32 = 1.0e-4;

/// Split a rectangular block into two back-to-back frontage rows.
///
/// `corners` must be a finite, convex rectangle in perimeter order. Each row
/// fronts one of the two long edges. `street_inset` reserves a uniform buffer
/// inside the block perimeter; `gap` separates adjacent lots along the street
/// and the backs of the opposing rows. The returned frontage parameter is
/// measured on the original, directed edge `corners[i] -> corners[i + 1]`.
///
/// `rotation` follows Three.js Y rotation: local +X maps to `(cos(r), -sin(r))`.
/// Local +Z points outward toward the frontage road, so the opposite row's
/// rotation differs by π.
pub(crate) fn subdivide_rectangular_block(
    corners: [BlockPoint; 4],
    street_inset: f32,
    target_frontage: f32,
    gap: f32,
) -> Vec<BlockLot> {
    if corners
        .iter()
        .any(|point| !point.x.is_finite() || !point.z.is_finite())
        || !street_inset.is_finite()
        || street_inset < 0.0
        || !target_frontage.is_finite()
        || target_frontage <= 0.0
        || !gap.is_finite()
        || gap < 0.0
    {
        return Vec::new();
    }

    let edges = [
        subtract(corners[1], corners[0]),
        subtract(corners[2], corners[1]),
        subtract(corners[3], corners[2]),
        subtract(corners[0], corners[3]),
    ];
    let lengths = edges.map(|edge| edge.x.hypot(edge.z));
    if lengths
        .iter()
        .any(|length| !length.is_finite() || *length <= f32::EPSILON)
    {
        return Vec::new();
    }

    let block_scale = lengths.iter().copied().fold(0.0_f32, f32::max).max(1.0);
    let length_tolerance = block_scale * RECTANGLE_RELATIVE_TOLERANCE;
    let area_twice = cross(edges[0], edges[1]);
    if !area_twice.is_finite() || area_twice.abs() <= f32::EPSILON {
        return Vec::new();
    }
    if (lengths[0] - lengths[2]).abs() > length_tolerance
        || (lengths[1] - lengths[3]).abs() > length_tolerance
        || dot(edges[0], edges[1]).abs() > lengths[0] * lengths[1] * RECTANGLE_RELATIVE_TOLERANCE
        || dot(edges[1], edges[2]).abs() > lengths[1] * lengths[2] * RECTANGLE_RELATIVE_TOLERANCE
        || dot(edges[2], edges[3]).abs() > lengths[2] * lengths[3] * RECTANGLE_RELATIVE_TOLERANCE
        || dot(edges[3], edges[0]).abs() > lengths[3] * lengths[0] * RECTANGLE_RELATIVE_TOLERANCE
        || vector_distance(edges[0], scale_neg(edges[2])) > length_tolerance
        || vector_distance(edges[1], scale_neg(edges[3])) > length_tolerance
    {
        return Vec::new();
    }

    let long_axis_is_0 = lengths[0] >= lengths[1];
    let long_length = if long_axis_is_0 {
        lengths[0]
    } else {
        lengths[1]
    };
    let short_length = if long_axis_is_0 {
        lengths[1]
    } else {
        lengths[0]
    };
    let usable_frontage = long_length - 2.0 * street_inset;
    let usable_depth = short_length - 2.0 * street_inset;
    if !usable_frontage.is_finite() || !usable_depth.is_finite() || usable_frontage <= 0.0 {
        return Vec::new();
    }
    let row_depth = (usable_depth - gap) / 2.0;
    if !row_depth.is_finite() || row_depth <= 0.0 {
        return Vec::new();
    }

    let natural_count = ((usable_frontage + gap) / (target_frontage + gap)).floor();
    if !natural_count.is_finite() || natural_count < 1.0 {
        return Vec::new();
    }
    let lot_count = (natural_count as usize).min(MAX_LOTS_PER_ROW);
    let lot_width = (usable_frontage - (lot_count - 1) as f32 * gap) / lot_count as f32;
    if !lot_width.is_finite() || lot_width <= 0.0 {
        return Vec::new();
    }
    let occupied_length = lot_count as f32 * lot_width + (lot_count - 1) as f32 * gap;
    let leading_margin = (usable_frontage - occupied_length) * 0.5;

    let frontage_edges: [usize; 2] = if long_axis_is_0 { [0, 2] } else { [1, 3] };
    let winding_positive = area_twice > 0.0;
    let mut lots = Vec::with_capacity(lot_count * 2);
    for edge_index in frontage_edges {
        let start = corners[edge_index];
        let edge = edges[edge_index];
        let length = lengths[edge_index];
        let tangent = BlockPoint {
            x: edge.x / length,
            z: edge.z / length,
        };
        let inward = if winding_positive {
            BlockPoint {
                x: -tangent.z,
                z: tangent.x,
            }
        } else {
            BlockPoint {
                x: tangent.z,
                z: -tangent.x,
            }
        };
        let outward = scale_neg(inward);
        let local_x = if winding_positive {
            scale_neg(tangent)
        } else {
            tangent
        };
        let rotation = (-local_x.z).atan2(local_x.x);
        let front_origin = add(start, scale(inward, street_inset));
        for slot in 0..lot_count {
            let along =
                street_inset + leading_margin + slot as f32 * (lot_width + gap) + lot_width * 0.5;
            let frontage_t = along / length;
            let center = add(
                add(front_origin, scale(tangent, along)),
                scale(inward, row_depth * 0.5),
            );
            if !center.x.is_finite()
                || !center.z.is_finite()
                || !frontage_t.is_finite()
                || !(0.0..=1.0).contains(&frontage_t)
            {
                return Vec::new();
            }
            // Keep the orientation calculation tied to the actual outward
            // frontage normal even when the input winding is reversed.
            debug_assert!((local_x.x * outward.z - local_x.z * outward.x - 1.0).abs() < 1.0e-4);
            lots.push(BlockLot {
                center,
                frontage_edge: edge_index,
                frontage_t,
                width: lot_width,
                depth: row_depth,
                rotation,
            });
        }
    }
    lots
}

fn add(a: BlockPoint, b: BlockPoint) -> BlockPoint {
    BlockPoint {
        x: a.x + b.x,
        z: a.z + b.z,
    }
}

fn subtract(a: BlockPoint, b: BlockPoint) -> BlockPoint {
    BlockPoint {
        x: a.x - b.x,
        z: a.z - b.z,
    }
}

fn scale(vector: BlockPoint, factor: f32) -> BlockPoint {
    BlockPoint {
        x: vector.x * factor,
        z: vector.z * factor,
    }
}

fn scale_neg(vector: BlockPoint) -> BlockPoint {
    BlockPoint {
        x: -vector.x,
        z: -vector.z,
    }
}

fn dot(a: BlockPoint, b: BlockPoint) -> f32 {
    a.x * b.x + a.z * b.z
}

fn cross(a: BlockPoint, b: BlockPoint) -> f32 {
    a.x * b.z - a.z * b.x
}

fn vector_distance(a: BlockPoint, b: BlockPoint) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

#[cfg(test)]
mod tests {
    use super::{subdivide_rectangular_block, BlockLot, BlockPoint};
    use std::collections::BTreeSet;
    use std::f32::consts::PI;

    fn point(x: f32, z: f32) -> BlockPoint {
        BlockPoint { x, z }
    }

    fn rectangle(length: f32, depth: f32, rotation: f32, clockwise: bool) -> [BlockPoint; 4] {
        let center = point(23.0, -17.0);
        let axis_x = point(rotation.cos(), -rotation.sin());
        let axis_z = point(rotation.sin(), rotation.cos());
        let p0 = add_test(
            center,
            add_test(
                scale_test(axis_x, -length * 0.5),
                scale_test(axis_z, -depth * 0.5),
            ),
        );
        let p1 = add_test(p0, scale_test(axis_x, length));
        let p2 = add_test(p1, scale_test(axis_z, depth));
        let p3 = add_test(p0, scale_test(axis_z, depth));
        if clockwise {
            [p0, p3, p2, p1]
        } else {
            [p0, p1, p2, p3]
        }
    }

    fn add_test(a: BlockPoint, b: BlockPoint) -> BlockPoint {
        point(a.x + b.x, a.z + b.z)
    }

    fn scale_test(a: BlockPoint, factor: f32) -> BlockPoint {
        point(a.x * factor, a.z * factor)
    }

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 1.0e-4, "{a} != {b}");
    }

    fn lot_corners(lot: BlockLot) -> [BlockPoint; 4] {
        let axis_x = point(lot.rotation.cos(), -lot.rotation.sin());
        let axis_z = point(lot.rotation.sin(), lot.rotation.cos());
        [
            add_test(
                lot.center,
                add_test(
                    scale_test(axis_x, -lot.width * 0.5),
                    scale_test(axis_z, -lot.depth * 0.5),
                ),
            ),
            add_test(
                lot.center,
                add_test(
                    scale_test(axis_x, lot.width * 0.5),
                    scale_test(axis_z, -lot.depth * 0.5),
                ),
            ),
            add_test(
                lot.center,
                add_test(
                    scale_test(axis_x, lot.width * 0.5),
                    scale_test(axis_z, lot.depth * 0.5),
                ),
            ),
            add_test(
                lot.center,
                add_test(
                    scale_test(axis_x, -lot.width * 0.5),
                    scale_test(axis_z, lot.depth * 0.5),
                ),
            ),
        ]
    }

    fn project(point: BlockPoint, origin: BlockPoint, axis: BlockPoint) -> f32 {
        (point.x - origin.x) * axis.x + (point.z - origin.z) * axis.z
    }

    fn assert_inside_inset(corners: [BlockPoint; 4], lots: &[BlockLot], inset: f32) {
        let area = (0..4)
            .map(|index| {
                let a = corners[index];
                let b = corners[(index + 1) % 4];
                a.x * b.z - a.z * b.x
            })
            .sum::<f32>();
        for lot in lots {
            for corner in lot_corners(*lot) {
                for index in 0..4 {
                    let start = corners[index];
                    let end = corners[(index + 1) % 4];
                    let edge = point(end.x - start.x, end.z - start.z);
                    let length = edge.x.hypot(edge.z);
                    let tangent = point(edge.x / length, edge.z / length);
                    let inward = if area > 0.0 {
                        point(-tangent.z, tangent.x)
                    } else {
                        point(tangent.z, -tangent.x)
                    };
                    let offset = point(corner.x - start.x, corner.z - start.z);
                    let along = offset.x * tangent.x + offset.z * tangent.z;
                    let across = offset.x * inward.x + offset.z * inward.z;
                    assert!(along >= inset - 1.0e-4 && along <= length - inset + 1.0e-4);
                    let opposite_span = (corners[(index + 2) % 4].x - corners[(index + 1) % 4].x)
                        .hypot(corners[(index + 2) % 4].z - corners[(index + 1) % 4].z);
                    assert!(across >= inset - 1.0e-4 && across <= opposite_span - inset + 1.0e-4);
                }
            }
        }
    }

    #[test]
    fn eighty_two_by_fifty_block_produces_two_gapped_rows_inside_inset() {
        let corners = rectangle(82.0, 50.0, 0.0, false);
        let lots = subdivide_rectangular_block(corners, 6.0, 18.0, 2.0);
        assert_eq!(lots.len(), 6);
        assert_inside_inset(corners, &lots, 6.0);
        assert!(lots.iter().all(|lot| lot.width > 0.0 && lot.depth > 0.0));
        let area = lots.iter().map(|lot| lot.width * lot.depth).sum::<f32>();
        approx(area, 6.0 * (66.0 / 3.0) * 18.0);
        for edge in [0, 2] {
            let edge_span = lots
                .iter()
                .filter(|lot| lot.frontage_edge == edge)
                .map(|lot| lot.width)
                .sum::<f32>()
                + 2.0 * (lots.iter().filter(|lot| lot.frontage_edge == edge).count() - 1) as f32;
            approx(edge_span, 70.0);
        }

        let origin = corners[0];
        let long_axis = point(1.0, 0.0);
        let short_axis = point(0.0, 1.0);
        for lot in &lots {
            assert!(lot.frontage_edge == 0 || lot.frontage_edge == 2);
            assert!((0.0..=1.0).contains(&lot.frontage_t));
            let edge_start = corners[lot.frontage_edge];
            let edge_end = corners[(lot.frontage_edge + 1) % 4];
            let edge = point(edge_end.x - edge_start.x, edge_end.z - edge_start.z);
            let edge_length = edge.x.hypot(edge.z);
            let tangent = point(edge.x / edge_length, edge.z / edge_length);
            let front = add_test(
                lot.center,
                scale_test(
                    point(lot.rotation.sin(), lot.rotation.cos()),
                    lot.depth * 0.5,
                ),
            );
            let front_t = project(front, edge_start, tangent) / edge_length;
            approx(front_t, lot.frontage_t);
            let frontage_distance = project(front, edge_start, point(-tangent.z, tangent.x));
            approx(frontage_distance.abs(), 6.0);
            for corner in lot_corners(*lot) {
                let along = project(corner, origin, long_axis);
                let across = project(corner, origin, short_axis);
                assert!(along >= 6.0 - 1.0e-4 && along <= 82.0 - 6.0 + 1.0e-4);
                assert!(across >= 6.0 - 1.0e-4 && across <= 50.0 - 6.0 + 1.0e-4);
            }
        }
        approx((lots[0].rotation - lots[3].rotation).sin(), 0.0);
        assert!((lots[0].rotation - lots[3].rotation).cos() < -0.9999);

        for (index, a) in lots.iter().enumerate() {
            for b in lots.iter().skip(index + 1) {
                let dx = (a.center.x - b.center.x).abs();
                let dz = (a.center.z - b.center.z).abs();
                assert!(
                    dx >= (a.width + b.width) * 0.5 - 1.0e-4
                        || dz >= (a.depth + b.depth) * 0.5 - 1.0e-4
                );
            }
        }
    }

    #[test]
    fn arbitrary_rotation_and_reversed_winding_preserve_lot_geometry_and_facing() {
        let angle = 0.73;
        let mut forward =
            subdivide_rectangular_block(rectangle(82.0, 50.0, angle, false), 6.0, 18.0, 2.0);
        let mut reverse =
            subdivide_rectangular_block(rectangle(82.0, 50.0, angle, true), 6.0, 18.0, 2.0);
        assert_eq!(forward.len(), reverse.len());
        assert_inside_inset(rectangle(82.0, 50.0, angle, false), &forward, 6.0);
        assert_inside_inset(rectangle(82.0, 50.0, angle, true), &reverse, 6.0);
        let sort_lots = |lots: &mut Vec<BlockLot>| {
            lots.sort_by(|a, b| {
                a.center
                    .x
                    .total_cmp(&b.center.x)
                    .then_with(|| a.center.z.total_cmp(&b.center.z))
            });
        };
        sort_lots(&mut forward);
        sort_lots(&mut reverse);
        for (a, b) in forward.iter().zip(&reverse) {
            approx(a.center.x, b.center.x);
            approx(a.center.z, b.center.z);
            approx(a.width, b.width);
            approx(a.depth, b.depth);
            approx(a.rotation.cos(), b.rotation.cos());
            approx(a.rotation.sin(), b.rotation.sin());
            let outward = point(a.rotation.sin(), a.rotation.cos());
            let lot_center_from_block = point(a.center.x - 23.0, a.center.z + 17.0);
            let side = if lot_center_from_block.x * angle.sin()
                + lot_center_from_block.z * angle.cos()
                > 0.0
            {
                point(angle.sin(), angle.cos())
            } else {
                point(-angle.sin(), -angle.cos())
            };
            assert!(outward.x * side.x + outward.z * side.z > 0.999);
        }
        let axis_x = point(angle.cos(), -angle.sin());
        let axis_z = point(angle.sin(), angle.cos());
        for (index, a) in forward.iter().enumerate() {
            for b in forward.iter().skip(index + 1) {
                let dx = b.center.x - a.center.x;
                let dz = b.center.z - a.center.z;
                let along = dx * axis_x.x + dz * axis_x.z;
                let across = dx * axis_z.x + dz * axis_z.z;
                assert!(
                    along.abs() >= (a.width + b.width) * 0.5 - 1.0e-4
                        || across.abs() >= (a.depth + b.depth) * 0.5 - 1.0e-4
                );
            }
        }
    }

    #[test]
    fn invalid_rectangles_and_parameters_are_rejected() {
        let valid = rectangle(82.0, 50.0, 0.2, false);
        assert!(subdivide_rectangular_block(valid, -1.0, 18.0, 2.0).is_empty());
        assert!(subdivide_rectangular_block(valid, 6.0, 0.0, 2.0).is_empty());
        assert!(subdivide_rectangular_block(valid, 6.0, 18.0, f32::NAN).is_empty());
        let degenerate = [
            point(0.0, 0.0),
            point(0.0, 0.0),
            point(1.0, 1.0),
            point(0.0, 1.0),
        ];
        assert!(subdivide_rectangular_block(degenerate, 1.0, 10.0, 1.0).is_empty());
        let skewed = [
            point(0.0, 0.0),
            point(82.0, 0.0),
            point(75.0, 50.0),
            point(0.0, 50.0),
        ];
        assert!(subdivide_rectangular_block(skewed, 6.0, 18.0, 2.0).is_empty());
        let nonfinite = [
            point(0.0, 0.0),
            point(f32::INFINITY, 0.0),
            point(1.0, 1.0),
            point(0.0, 1.0),
        ];
        assert!(subdivide_rectangular_block(nonfinite, 1.0, 10.0, 1.0).is_empty());
    }

    #[test]
    fn oversized_blocks_are_bounded_to_sixty_four_lots() {
        let lots =
            subdivide_rectangular_block(rectangle(2_000.0, 50.0, -1.17, false), 4.0, 8.0, 1.0);
        assert_eq!(lots.len(), 64);
        assert!(lots.len() <= 64);
    }

    #[test]
    fn rotations_use_three_y_convention_and_opposite_rows_face_outward() {
        let corners = rectangle(82.0, 50.0, PI / 4.0, false);
        let lots = subdivide_rectangular_block(corners, 6.0, 18.0, 2.0);
        assert!(!lots.is_empty());
        for lot in &lots {
            let local_x_world = point(lot.rotation.cos(), -lot.rotation.sin());
            let edge_start = corners[lot.frontage_edge];
            let edge_end = corners[(lot.frontage_edge + 1) % 4];
            let edge = point(edge_end.x - edge_start.x, edge_end.z - edge_start.z);
            let tangent = point(edge.x / edge.x.hypot(edge.z), edge.z / edge.x.hypot(edge.z));
            let alignment = local_x_world.x * tangent.x + local_x_world.z * tangent.z;
            approx(alignment.abs(), 1.0);
            let local_z_world = point(lot.rotation.sin(), lot.rotation.cos());
            let to_edge = point(edge_start.x - lot.center.x, edge_start.z - lot.center.z);
            assert!(local_z_world.x * to_edge.x + local_z_world.z * to_edge.z > 0.0);
        }
    }

    #[test]
    fn frontage_metadata_is_finite_and_stably_indexed() {
        let corners = rectangle(82.0, 50.0, 0.41, false);
        let lots = subdivide_rectangular_block(corners, 6.0, 18.0, 2.0);
        let first = subdivide_rectangular_block(corners, 6.0, 18.0, 2.0);
        assert_eq!(lots, first);
        let edges: BTreeSet<usize> = lots.iter().map(|lot| lot.frontage_edge).collect();
        assert_eq!(edges.len(), 2);
        assert!(lots
            .iter()
            .all(|lot| lot.frontage_t.is_finite() && (0.0..=1.0).contains(&lot.frontage_t)));
    }
}
