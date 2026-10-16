//! Bounded, isolated infill across artificial convex-decomposition seams.
//! Existing buildings remain fixed. New sites and their finite access corridors
//! are checked against the entire reserved parent ground before being accepted.
use super::*;

pub(super) struct Piece {
    pub parent: u32,
    pub block: u32,
    pub district: District,
    pub poly: Poly,
}

fn intersection(a: &Poly, b: &Poly) -> Poly {
    let mut result = a.clone();
    for i in 0..b.len() {
        let p = b[i].0;
        let d = b[(i + 1) % b.len()].0.sub(p);
        result = clip(&result, p, P::new(-d.z, d.x), None);
        if result.len() < 3 {
            break;
        }
    }
    result
}
fn covered(poly: &Poly, pieces: &[&Piece]) -> bool {
    let expected = area(poly);
    let covered: f32 = pieces
        .iter()
        .map(|p| area(&intersection(poly, &p.poly)).max(0.0))
        .sum();
    // Only float-scale seam/clip tolerance, never a buildable land allowance.
    (expected - covered).abs() < 0.02
}
fn corridor(a: P, b: P) -> Poly {
    let u = b.sub(a).unit();
    let v = P::new(-u.z, u.x).scale(1.1);
    ccw(vec![
        (a.sub(v), None),
        (b.sub(v), None),
        (b.add(v), None),
        (a.add(v), None),
    ])
}
fn road_for_entry(
    line: &Line,
    entry: P,
    tangent: P,
    nodes: &[PlanNode],
    roads: &[PlanRoad],
) -> Option<usize> {
    line.roads
        .iter()
        .map(|(_, _, i)| *i)
        .filter(|&i| {
            let r = roads[i];
            let a = nodes[r.a].p;
            let d = nodes[r.b].p.sub(a);
            let t = entry.sub(a).dot(d) / d.dot(d);
            t >= 0.0 && t <= 1.0 && d.unit().dot(tangent).abs() > 0.999
        })
        .min_by(|&a, &b| {
            let distance =
                |i: usize| point_segment_distance(entry, nodes[roads[i].a].p, nodes[roads[i].b].p);
            distance(a).total_cmp(&distance(b)).then(a.cmp(&b))
        })
}

pub(super) fn append(
    pieces: &[Piece],
    lines: &[Line],
    nodes: &[PlanNode],
    roads: &[PlanRoad],
    lots: &mut Vec<PlanLot>,
    ground: &mut Vec<Vec<P>>,
    legal: impl Fn(&PlanLot, Option<usize>) -> bool,
) {
    if lots.len() >= MAX_PLAN_LOTS {
        return;
    }
    let mut groups: BTreeMap<u32, Vec<&Piece>> = BTreeMap::new();
    for piece in pieces {
        groups.entry(piece.parent).or_default().push(piece);
    }
    for group in groups.values() {
        if group
            .iter()
            .map(|p| p.block)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            < 2
        {
            continue;
        }
        let mut entries = Vec::new();
        let mut centers = Vec::new();
        for piece in group {
            let c = centroid(&piece.poly);
            centers.push((c, piece.block, piece.district));
            for vertex in &piece.poly {
                centers.push((c.lerp(vertex.0, 0.55), piece.block, piece.district));
            }
            for i in 0..piece.poly.len() {
                let Some(line) = piece.poly[i].1 else {
                    continue;
                };
                if !matches!(lines[line].kind, RoadKind::Local | RoadKind::Lane) {
                    continue;
                }
                let a = piece.poly[i].0;
                let b = piece.poly[(i + 1) % piece.poly.len()].0;
                let tangent = b.sub(a).unit();
                let inward = P::new(-tangent.z, tangent.x);
                if b.sub(a).len() < 5.0 {
                    continue;
                }
                for fraction in [0.25, 0.5, 0.75] {
                    let boundary = a.lerp(b, fraction);
                    if let Some(road) =
                        road_for_entry(&lines[line], boundary, tangent, nodes, roads)
                    {
                        entries.push((boundary.add(inward.scale(1.11)), boundary, road));
                    }
                }
            }
        }
        let mut accepted = 0;
        'centers: for &(center, block, district) in centers.iter().take(64) {
            for &(entry, boundary, road) in entries.iter().take(24) {
                let length = entry.sub(center).len();
                if !(8.0..=30.0).contains(&length) {
                    continue;
                }
                let facing = entry.sub(center).unit();
                for depth in [18.0, 14.0, 10.0] {
                    if length < depth / 2.0 + 2.0 {
                        continue;
                    }
                    let threshold = center.add(facing.scale(depth / 2.0 + 0.25));
                    let mut lot = PlanLot {
                        x: center.x,
                        z: center.z,
                        width: 7.0,
                        depth,
                        rotation: facing.x.atan2(facing.z),
                        facing,
                        road,
                        district,
                        arterial_frontage: false,
                        flag: true,
                        passage: None,
                        block,
                    };
                    if !legal(&lot, None) {
                        continue;
                    }
                    let footprint: Poly =
                        rect_corners(&lot).into_iter().map(|p| (p, None)).collect();
                    if !covered(&footprint, group) {
                        continue;
                    }
                    let path = corridor(threshold, entry);
                    if !covered(&path, group) {
                        continue;
                    }
                    let r = roads[road];
                    let a = nodes[r.a].p;
                    let d = nodes[r.b].p.sub(a);
                    let projection = a.add(d.scale(boundary.sub(a).dot(d) / d.dot(d)));
                    let offset = crate::pedestrian_road_offset(r.kind.as_str());
                    let walk = projection.add(boundary.sub(projection).unit().scale(offset));
                    // Include the curb/setback connection in collision checks.
                    let connector = corridor(entry, walk);
                    let path_legal = |from: P, to: P| {
                        let direction = to.sub(from).unit();
                        let midpoint = from.lerp(to, 0.5);
                        let envelope = PlanLot {
                            x: midpoint.x,
                            z: midpoint.z,
                            width: 2.2,
                            depth: to.sub(from).len(),
                            facing: direction,
                            ..lot
                        };
                        legal(&envelope, Some(road))
                    };
                    if !path_legal(threshold, entry) || !path_legal(entry, walk) {
                        continue;
                    }
                    let mut blocked = false;
                    for existing in lots.iter() {
                        if (existing.x - center.x).abs() > existing.width + existing.depth + 45.0
                            || (existing.z - center.z).abs()
                                > existing.width + existing.depth + 45.0
                        {
                            continue;
                        }
                        let expanded = PlanLot {
                            width: existing.width + 0.5,
                            depth: existing.depth + 0.5,
                            ..*existing
                        };
                        let obstacle: Poly = rect_corners(&expanded)
                            .into_iter()
                            .map(|p| (p, None))
                            .collect();
                        if [&footprint, &path, &connector]
                            .iter()
                            .any(|p| area(&intersection(p, &obstacle)) > 0.001)
                        {
                            blocked = true;
                            break;
                        }
                        if let Some(passage) = existing.passage {
                            if passage.windows(2).any(|w| {
                                area(&intersection(&footprint, &corridor(w[0], w[1]))) > 0.001
                            }) {
                                blocked = true;
                                break;
                            }
                        }
                    }
                    if blocked {
                        continue;
                    }
                    lot.passage = Some([threshold, entry, boundary]);
                    ground.push(footprint.iter().map(|v| v.0).collect());
                    lots.push(lot);
                    accepted += 1;
                    if accepted >= 3 || lots.len() >= MAX_PLAN_LOTS {
                        break 'centers;
                    }
                    continue 'centers;
                }
            }
        }
        if lots.len() >= MAX_PLAN_LOTS {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn union_coverage_crosses_only_shared_ground() {
        let piece = |x: f32| Piece {
            parent: 0,
            block: x as u32,
            district: District::Compact,
            poly: vec![
                (P::new(x, 0.0), None),
                (P::new(x + 10.0, 0.0), None),
                (P::new(x + 10.0, 20.0), None),
                (P::new(x, 20.0), None),
            ],
        };
        let a = piece(0.0);
        let b = piece(10.0);
        let gap = piece(12.0);
        let path = corridor(P::new(5.0, 10.0), P::new(15.0, 10.0));
        assert!(covered(&path, &[&a, &b]));
        assert!(!covered(&path, &[&a, &gap]));
    }
}
