# Extending the procedural architecture pipeline

The apartment release supplies a reusable offline asset pipeline, finite shared material palettes, Blender component attachment conventions, deterministic recipes, construction state, room data, distance detail and city batching. Runtime generation combines geometry and shader inputs; it never calls an LLM, Blender or an image provider.

## Next family: offices

Start with five structurally different recipes rather than five colors of one facade:

1. Older compact tiled office: punched windows, recessed entry, discrete tenant signs, exposed service roof.
2. Older concrete/ALC office: wider horizontal window groups, solid end walls and exterior equipment/service access.
3. Contemporary framed office: deep structural grid, shaded glazing and a taller reception floor.
4. Contemporary curtain-wall tower: office-size floor plates, mullion/spandrel rhythm, opaque service core and screened mechanical crown.
5. Contemporary stepped office: setbacks, terrace planting, differentiated podium and shaft proportions.

Use `art.window('office')` and tag real pane width/height/activity so the existing daytime work schedule controls rooms. Retail in a podium follows `shop`, not office lighting. Reuse the current stone/metal/concrete/ceramic maps; add ALC/metal finishes only when a reference comparison shows a material gap. Existing main-branch cladding, signage, rooftop services and access systems remain the default sources of those details.

Implementation boundary: a `supportsOffice` predicate and `buildOfficeBuilding(Building, ArtMaterials, {recipe, detail}) -> Group | null`, supported by separate recipe descriptors, family component slots, lifecycle tests, surface bindings and city policy selection. Do not copy the apartment builder and rename its kind: office floor plates, fenestration, entry scale and service arrangements differ.

## Next family: mixed use

Five useful initial compositions:

1. Neighborhood shop with apartments above.
2. Narrow vertical tenant building with separate upper-floor access.
3. Corner cafe with apartments or small offices above.
4. Retail podium with office floors above.
5. Residential tower with a commercial podium and separate resident lobby.

Model use as floor bands and frontage slots, for example `{fromFloor:0,count:1,use:'retail'}` and `{fromFloor:1,count:6,use:'residential'}`. Each band selects its own windows, signs, service access and lighting schedule. Use separate street-level retail and residential/office entrances without blocking the authoritative parcel access connector. Reserve loading, refuse, bicycle parking, mechanical plant and egress by composition rather than scattering props.

A visual mixed-use recipe can reuse the current `shop`/`office`/`apartment` records initially. **True simulated mixed use requires engine/schema work**: residential capacity, jobs/business occupancy, schedules and access destinations must coexist. Adding a new renderer kind string alone will not implement that. Existing engine demand kinds are `home`, `shop`, `cafe`, `office` and `apartment`; station and open-space systems are separate. Keep defaults for old saves when adding optional use-band data.

## Other classes, ordered by city impact

| Family | Useful variants | Scope |
|---|---|---|
| Detached homes / row houses | Traditional timber, postwar plaster, compact modern 3-floor, paired/row units, small courtyard | Extend existing `home` simulation; new massing/components |
| Shops / food / neighborhood services | Convenience shop, narrow shop, corner cafe, restaurant, small shopping strip | Extend existing shop/cafe families and tenant signage |
| Business hotels / inns | Compact business hotel, larger hotel podium/tower, small inn | New facade/service grammar; lodging demand if simulated |
| Workshops / logistics | Machikoba workshop, small warehouse, delivery depot, service yard | Loading/service access, larger doors/roof spans; new economic use if simulated |
| Civic / community | School, clinic, community center, post office, koban/fire station | Distinct public entrances, grounds and activity schedules |
| Parking / utilities | Structured parking, bicycle parking, substation, pump/service building | Scene/service systems before new demand categories |
| Religious / heritage | Small shrine, temple, neighborhood hall | Coordinate with existing protected/open-space systems |
| Transport facilities | Station frontage, concourse/podium, bus depot | Extend existing transport ownership and circulation contracts |

Keep one-off landmarks separately authored. They do not need the same five-variant catalog as ordinary neighborhood buildings.

## Reuse contract and acceptance

Every family must preserve metres/Y-up/+Z-front placement, authoritative width/depth/floors, stable appearance seeds, non-overlapping circulation/service zones, shared material ownership, room and weather attributes through merged batches, construction/demolition continuity, footprint fallbacks, picking and disposal. New asset packs carry source prompts/hashes and offline reproducible preparation; city clients receive only approved small derivatives.

Promote each family after a fixed complete-street comparison, variable-parcel/lifecycle tests, both styles/backends, night/vacancy checks and a mature-city regression. The apartment option to use the existing renderer is the rollout/fallback model for future families.
