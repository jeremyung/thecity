# Parcel-scale urban ground proposal

## What the current renderer is showing

The mature captures have two different ground conditions. `CityRenderer.buildTerrain()` in `web/src/render/city.ts` creates a continuous green meadow across the whole 840 m by 840 m basin, interrupted by the river, roads, and planted groves. It does not render `World.parcels` as a separate ground layer. The groves intentionally leave open meadows, and `updateTrees()` only clears canopies near buildings; neither function turns the space between developed streets into urban ground.

Developed lots do get a ground surface: `buildingKit()` in `web/src/render/kit.ts` adds one flat, uniform paving box at 5 cm high and 96% of the parcel width/depth for open or renovating buildings. That pad is a simple rectangle under the whole lot. Construction sites do not receive it. `BuildingAccessPaths.setWorld()` in `web/src/render/building-access.ts` adds a 1.65 m walkway and a 1.85 m curb landing along the authoritative door-to-curb segment, but this narrow connector does not shape the rest of the lot. The result is grass outside the developed parcels, plain paving islands on most open parcels, and temporary green ground at work sites. The screenshots therefore show both real undeveloped land and a lack of parcel-scale ground variation; paving every open patch would hide the former rather than solve the latter.

The available seed 87231 snapshot at tick 242000 has 850 open buildings and 850 parcels. Their nominal parcel envelopes total 160,348 m², about 25.1% of the 640,000 m² world bounds. That ratio is only whole-map context: it includes river, protected edges, and outer land, and does not classify the lawns visible in a particular camera frame. It does show why a renderer-only ground treatment cannot create continuous block fabric across the remaining map.

There is also intentional open area inside individual lots. The base kit starts most non-anchor footprints at 84% of parcel width and depth (`kit.ts:73–75`). Supported low-rise frontage masses then use about 92% of parcel width but only 60–64% of parcel depth (`lowrise-massing.ts:47–48`), with a separate rear service wing (`:53–79`). That is appropriate for compact Tokyo lots only when the leftover strip reads as a real entry court, service yard, or small garden. At present, most open lots use the same near-full-lot paving box, while the low-rise side garden is a single seeded patch; the ground plane does not consistently explain those open margins as a designed part of the parcel.

## Smallest coherent treatment

Replace the single full-lot pad with a deterministic `buildParcelGround(kit, building, access, materials)` helper called from `buildingKit()`. Use the building's saved parcel dimensions and rotation for its local frame, and the matching `BuildingAccess` segment when present to align the frontage apron with the actual entry. `Parcel` already carries its own optional rotation and reservation in `web/src/types.ts`; if ground generation moves into a world-level manager, join `World.buildings` to `World.parcels` by `parcel_id` and use the parcel record as the envelope authority. Keep all added surfaces within the parcel envelope and above the existing y=0 meadow plane. Do not generate a surface for a parcel with no built structure, and do not pave beyond the parcel boundary or across roads, sidewalks, protected land, or water.

Use a small set of related surfaces rather than a blanket slab:

- Shops and cafes receive a shallow frontage apron at the storefront and a compact service pad behind the building. Keep the authored door, the existing door-to-curb path, and street furniture clear.
- Homes receive a narrow threshold path and a small rear/service-yard pad; preserve the existing seeded side garden instead of converting the whole setback to concrete.
- Apartments and offices receive only the entry/service apron and a restrained rear utility pad. Their larger envelope should not become an uninterrupted paved plaza.
- Renovations retain a reduced, stage-aware work apron. Foundation and construction parcels remain visibly unfinished, with their existing barriers and worksite geometry.

Reuse the current paving, concrete, and garden material classes. Continue to merge the geometry through `Kit` and `BuildingChunks` so the details stay in the existing spatial batches. At most four rectangular plates per parcel add at most 48 triangles per lot; replacing the current 12-triangle pad with four plates is about 30,600 additional triangles for 850 buildings, roughly 1.5% of the measured 2.14 million building triangles. The shared materials add no per-lot material instances or draw calls. These are geometry-count estimates, not GPU timing measurements.

This treatment will make occupied parcel edges, entrances, and service space read as part of the street. It will not close vacant block interiors or broad meadows outside the parcel network; those remain engine/layout work and should stay visible until the simulation develops them.

## Review views

1. Capture the same mature seed, tick, and overview camera in Anime and Miniature. Compare paved frontage continuity around shops and homes while verifying the river, grove clearings, and unbuilt spaces remain green.
2. Capture a street-level block with a shop entry, home frontage, and an occupied building-access path. Check that apron, threshold, path, and curb landing meet without gaps or overlap, and that furniture and doors remain readable.
3. Capture a construction parcel and a neighboring open parcel. Confirm the worksite keeps its stage-specific surface, the open lot has the new treatment, and neither surface spills into the roadway or adjacent parcel.

For quantitative review, project ground-surface triangles and parcel polygons into the same half-meter grid, report coverage only inside parcels with open or renovating buildings, and separately report any changed cells outside those parcels. The outside-parcel change should be zero. Keep this metric separate from building-envelope or whole-map coverage.
