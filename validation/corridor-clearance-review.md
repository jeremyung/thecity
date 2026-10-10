# Road corridor clearance review

## Finding

`Engine::road_corridor_clear_of_parcels_with_context` currently requires both a radial center-distance clearance and exact rectangle clearance (`engine/src/lib.rs:1888-1908`). The radial test rejects some corridors that are clear of the parcel envelope by the exact 5 m margin.

For an axis-aligned parcel centered at `(0, 0)` with width 20 m and depth 12 m, a road segment parallel to its long side at `z = 14` is 8 m from the parcel edge. `geometry::rect_segment_distance` therefore permits it under the current exact `>= 5 m` test. The radial threshold is `max(20, 12) / 2 + 5 = 15 m`; the road is only 14 m from the parcel center, so the earlier center-distance test rejects it. For a 40×10 m parcel, a road at `z = 11` has 6 m of exact clearance but fails the 25 m radial threshold.

The exact helper computes rectangle-to-segment distance and returns zero for containment or edge crossing (`engine/src/geometry.rs:127-168`). It provides the direct geometric collision test; the radial circle is a more conservative spacing policy, especially for elongated lots.

## Legacy and rendered parcel shapes

For parcels without a saved rotation, `collision_context()` intentionally stores both their legacy axis-aligned envelope and inferred rendered envelope (`engine/src/lib.rs:1822-1842`). The regression test `oriented_admission_respects_legacy_saved_and_rendered_envelopes` explicitly checks that both shapes remain protected (`engine/src/lib.rs:15850-15887`). This is intentional compatibility behavior, not an accidental duplicate. The road clearance check should continue testing both shapes unless save validation policy changes.

The rotated-corner corridor test proves the exact check catches a parcel corner beyond the old center radius (`engine/src/lib.rs:12802-12837`). The connected-local-chord invariant also asserts the radial parcel spacing rule (`engine/src/lib.rs:15906-15912`), so removing the road radial guard requires updating that test to assert exact clearance instead.

## Proposed bounded change

Consider removing only the radial parcel-center predicate from the road corridor helper, retaining the exact `rect_segment_distance >= 5 m` checks against both legacy and rendered envelopes. Keep road-to-road, river, protected-land, neighborhood-plan, and interior-plan checks intact. Update the local-chord test to verify exact parcel-envelope clearance and add the narrow-parcel examples above as regression coverage. This is a proposal for the engine owner to evaluate; no runtime code was changed as part of this review.
