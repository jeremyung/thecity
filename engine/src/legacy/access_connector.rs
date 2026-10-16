//! Join a reserved door-to-curb passage to the street walking route.
use crate::WorldPoint;

pub(crate) fn pedestrian_points(
    passage: &[WorldPoint],
    street_end: WorldPoint,
    route_end: WorldPoint,
    departing: bool,
) -> Vec<WorldPoint> {
    if departing {
        let mut points = passage.to_vec();
        points.extend([street_end, route_end]);
        points
    } else {
        let mut points = vec![route_end, street_end];
        points.extend(passage.iter().rev().copied());
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrival_and_departure_preserve_every_passage_bend() {
        let passage = [
            WorldPoint { x: 0.0, z: 0.0 },
            WorldPoint { x: 0.0, z: 8.0 },
            WorldPoint { x: 9.0, z: 8.0 },
        ];
        let street = WorldPoint { x: 14.0, z: 8.0 };
        let route = WorldPoint { x: 14.0, z: 10.0 };
        let departing = pedestrian_points(&passage, street, route, true);
        let arriving = pedestrian_points(&passage, street, route, false);
        let coordinates =
            |p: Vec<WorldPoint>| p.into_iter().map(|p| (p.x, p.z)).collect::<Vec<_>>();
        assert_eq!(
            coordinates(departing.clone()),
            vec![
                (0.0, 0.0),
                (0.0, 8.0),
                (9.0, 8.0),
                (14.0, 8.0),
                (14.0, 10.0)
            ]
        );
        assert_eq!(
            coordinates(arriving),
            coordinates(departing.into_iter().rev().collect())
        );
    }
}
