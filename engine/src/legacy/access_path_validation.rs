use crate::Engine;

impl Engine {
    /// A finite polyline is not sufficient: it must actually meet the route
    /// where the traveler enters/leaves it. Reject, rather than repair, saves.
    /// The 1 cm tolerance covers numeric roundoff within the current format, not
    /// compatibility with older walking policies. Policy changes require an
    /// explicit save-schema decision and updated regression fixtures; silently
    /// snapping a connector would also change its length and saved progress.
    pub(crate) fn validate_access_route_joins(&self) -> Result<(), String> {
        for traveler in &self.state.travelers {
            for (at_start, path) in [
                (true, &traveler.origin_access),
                (false, &traveler.destination_access),
            ] {
                let Some(path) = path else { continue };
                let label = if at_start { "origin" } else { "destination" };
                let expected = self
                    .route_endpoint_position(&traveler.route, at_start, traveler.id, traveler.kind)
                    .ok_or_else(|| {
                        format!(
                            "traveler {} has no route endpoint for its {label} connector",
                            traveler.id
                        )
                    })?;
                let actual = if at_start {
                    path.points.last()
                } else {
                    path.points.first()
                }
                .ok_or_else(|| {
                    format!("traveler {} has an empty {label} connector", traveler.id)
                })?;
                if !actual.x.is_finite()
                    || !actual.z.is_finite()
                    || (actual.x - expected.x).hypot(actual.z - expected.z) > 0.01
                {
                    return Err(format!(
                        "traveler {} {label} connector does not meet its route endpoint",
                        traveler.id
                    ));
                }
            }
        }
        Ok(())
    }
}
