//! Street cross-sections shared by planning, access and movement.
use crate::kinds::RoadKind;

/// Rendered carriageway width. Tokyo local streets are narrow shared surfaces;
/// only arterials carry raised sidewalks.
/// Carriageway width; `None` is an unresolved road.
pub fn road_width(kind: Option<RoadKind>) -> f32 {
    match kind {
        Some(RoadKind::Arterial | RoadKind::Bridge) => 16.0,
        Some(RoadKind::Local) => 6.5,
        Some(RoadKind::Lane) => 4.0,
        Some(RoadKind::Expressway | RoadKind::Rail) | None => 9.0,
    }
}

/// Shared streets carry walkers inside the painted edge; arterials use the
/// raised sidewalk outside the carriageway. All access and movement consumers
/// must use this same offset so their route endpoints meet.
pub fn pedestrian_road_offset(kind: Option<RoadKind>) -> f32 {
    let half_width = road_width(kind) * 0.5;
    if matches!(kind, Some(RoadKind::Local | RoadKind::Lane)) {
        half_width - 0.72
    } else {
        half_width + 0.72
    }
}
