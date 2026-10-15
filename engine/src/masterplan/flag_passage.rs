//! Joint front/rear fitting: access occupies land before either building does.
use super::{inscribed_rect_frame, Poly, P};

pub(super) const PASSAGE_WIDTH: f32 = 2.2;

#[derive(Clone, Copy, Debug)]
pub(super) struct Cell {
    pub front: (P, f32, f32),
    pub rear: (P, f32, f32),
    /// Rear threshold, cross-passage bend, street-side entrance.
    pub passage: [P; 3],
}

/// Fit both buildings and an L-shaped passage inside one convex ownership cell.
/// `a/u/v` are the real frontage origin, tangent and inward normal. No synthetic
/// access is granted by a polygon cut. Returned rectangles face `-v`.
pub(super) fn fit(poly: &Poly, a: P, u: P, v: P, front_depth: f32, rear_max: f32) -> Option<Cell> {
    let (center, width, depth, _) =
        inscribed_rect_frame(poly, a, u, v, front_depth + PASSAGE_WIDTH + rear_max)?;
    let building_width = width - PASSAGE_WIDTH;
    let rear_depth = depth - front_depth - PASSAGE_WIDTH;
    if building_width < 5.0 || front_depth < 7.0 || rear_depth < 7.0 { return None; }
    let front_edge = center.sub(v.scale(depth * 0.5));
    let building_axis = front_edge.add(u.scale(PASSAGE_WIDTH * 0.5));
    let entrance = front_edge.sub(u.scale(width * 0.5 - PASSAGE_WIDTH * 0.5));
    let cross_depth = front_depth + PASSAGE_WIDTH * 0.5;
    Some(Cell {
        front: (building_axis.add(v.scale(front_depth * 0.5)), building_width, front_depth),
        // The side passage ends at the cross passage. The rear lot can use
        // the full cell width behind it without narrowing either walking leg.
        rear: (front_edge.add(v.scale(front_depth + PASSAGE_WIDTH + rear_depth * 0.5)), width, rear_depth),
        passage: [front_edge.add(v.scale(cross_depth)), entrance.add(v.scale(cross_depth)), entrance],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_cell_reserves_side_and_cross_passages_in_rotated_frames() {
        for u in [P::new(1.0,0.0),P::new(0.0,1.0),P::new(0.6,0.8)] {
            let v=P::new(-u.z,u.x); let a=P::new(27.0,-14.0);
            let poly=vec![(a,Some(0)),(a.add(u.scale(10.0)),None),
                (a.add(u.scale(10.0)).add(v.scale(40.0)),None),(a.add(v.scale(40.0)),None)];
            let cell=fit(&poly,a,u,v,15.0,28.0).unwrap();
            let project=|p:P| (p.sub(a).dot(u),p.sub(a).dot(v));
            let (side,entrance_depth)=project(cell.passage[2]);
            assert!((side-1.1).abs()<0.001 && entrance_depth.abs()<0.001);
            let front_min=project(cell.front.0).0-cell.front.1/2.0;
            assert!((front_min-(side+1.1)).abs()<0.001);
            let front_back=project(cell.front.0).1+cell.front.2/2.0;
            let rear_front=project(cell.rear.0).1-cell.rear.2/2.0;
            assert!((rear_front-front_back-PASSAGE_WIDTH).abs()<0.001);
            assert!((cell.rear.1 - 10.0).abs() < 0.001);
            assert!((project(cell.rear.0).0 - cell.rear.1 / 2.0).abs() < 0.001);
            assert!((project(cell.passage[0]).0 - project(cell.rear.0).0).abs() < 0.001);
            assert!((project(cell.passage[0]).1-(front_back+1.1)).abs()<0.001);
        }
    }

    #[test]
    fn narrow_cell_does_not_invent_a_passage_through_a_building() {
        let a=P::new(0.0,0.0);let u=P::new(1.0,0.0);let v=P::new(0.0,1.0);
        let poly=vec![(a,Some(0)),(P::new(6.0,0.0),None),(P::new(6.0,40.0),None),(P::new(0.0,40.0),None)];
        assert!(fit(&poly,a,u,v,15.0,28.0).is_none());
    }
}
