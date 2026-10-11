# World Growth and Infrastructure

The city occupies a finite geographic basin. A seeded master plan fixes the skeleton of the mature city: transport corridors, arterials, local streets, lanes, blocks, and lots. Growth is the visible, demand-driven realization of that plan. Each road opens before its lots exist, each lot is built before it is occupied, and mature districts keep changing through renewal, lot assembly, and redevelopment.

The previous model placed each building wherever a free rectangle cleared every road and parcel. That model topped out near 20% gross coverage with detached buildings, lawn gaps, and stranded block interiors. Its checkpoint log is archived in [growth history](../validation/growth-history-2026-10.md), and its planner is retired.

## Landscape and boundary

Proposed production landscape: a 2.4 km by 2.4 km square simulation domain, divided into 24 by 24 chunks of 100 m. Prototype: 800 m by 800 m. The chunk grid supports storage and indexing; roads and parcels need not follow it.

The seed defines terrain, river course, hills, protected open space, gateways, and developable land. Preserve meaningful parks, banks, and steep terrain rather than filling every surface. A ring of undeveloped land inside the domain edge (about 20 m in the prototype) gives the distant backdrop somewhere to start.

Outside the domain, render inexpensive atmospheric hills, water, or distant silhouettes. They have no parcels, residents, routing, or growth. Camera bounds and terrain composition keep the edge legible without suggesting reachable districts. External gateways connect to abstract regional demand; they do not generate an infinite world.

Every seed must have at least one viable founding area and enough reachable land for several neighborhoods. Reject or repair invalid geography with deterministic rules before simulation starts.

## Target urban form

The goal is a dense Tokyo-like fabric, not a suburb with tall buildings. Visual references: the [dense concept boards](../concepts/05-dense-last-train-first-light.png) and the [photo references](../concepts/photo-references/README.md). The numbers come from the [PLATEAU pilot](../validation/plateau-pilot/README.md).

What makes the fabric read as Tokyo:

- **Superblocks** of roughly 150–250 m, bounded by arterials. Mid-rise buildings line the arterials; finer, lower fabric fills the interior.
- **A fine, irregular local mesh.** Local streets are about 6.5 m wide shared surfaces without raised sidewalks, and lanes are 4 m (the Building Standards Act minimum). Blocks are irregular polygons about 35–90 m deep. T-junctions outnumber four-way crossings, and streets rarely continue straight across an intersection. The fabric should not read as a grid or as rows of trapezoids.
- **Narrow, deep lots with no gaps between them.** Typical frontage is 6–10 m and depth 12–20 m, mostly 60–180 m². Buildings sit against shared lot lines (party walls). Front setbacks are 0–1.5 m.
- **A skyline organized by transit.** Taller clusters sit at working stations, and small old buildings stay within the clusters.
- **Transport that structures the city:** elevated or at-grade rail with stations, an elevated expressway, river crossings, and arterials that follow the river and rail. These corridors are planned at generation time, not grown accidentally.

### Street-network targets

Measured from PLATEAU road surfaces and OpenStreetMap centerlines for 1 km meshes in Kyojima, Togoshi, Taito, Shinjuku and Minato; see the [street reference](../validation/street-reference/README.md). A generated plan must fall within Tokyo's range and must not be more grid-aligned than its most regular district:

| Metric | Target | Tokyo reference range |
| --- | --- | --- |
| Street orientation order (1 = grid) | 0.3–0.65 | 0.14 Kyojima to 0.64 Taito |
| Non-four-sided blocks | ≥ 55% | 44–91% |
| T-junction share | 65–90% | 65–83% |
| Dead ends per km² | 40–100 | 16–98 |
| Junctions per km² | 400–550 | 217–502 |

### Fabric targets

Measure over 250 m crops using the rendered footprint union, as in `scripts/audit-rendered-coverage.mjs`. Report gross and net-of-road coverage separately.

| Metric | Compact district | Station district | Waterfront | Source |
| --- | --- | --- | --- | --- |
| Gross rendered footprint coverage | 40–50% | 38–48% | 10–30% | PLATEAU Taito 48.5%, Shinjuku station edge 46.0%, Minato waterfront 10.1% |
| Median footprint | 55–140 m² | 100–300 m² | — | Taito 58 m², station 115 m² |
| p90 footprint | ≤ 350 m² | ≤ 900 m² | — | Taito 137 m², station 651 m² |
| Median floors | 2–4 | 4–7 | — | Taito 3, station 5 |
| Median nearest-building gap | < 1 m | < 1.5 m | — | Taito 0.23 m roof-edge gap |
| Road share of crop area (arterial sidewalks included) | 18–28% | 20–30% | — | Tokyo ward road share is commonly cited around 16–20% |

These are acceptance ranges for the generator, not runtime controllers. The renderer may stylize, but a rendered district must not fall below the lower bound for its type at maturity. Whole-map gross coverage also counts the river, the ring, and protected land; at maturity it should exceed 30% for the prototype basin.

## Master plan

The plan is generated deterministically from the seed and the engine version before simulation starts. It is pure data: it does not mutate world state and is never altered by camera, style, or rendering. Saves store the seed and the realized progress, and restore regenerates the same plan. Generation order:

1. **Geography:** river polyline, banks, protected land, and the developable mask.
2. **Transport skeleton:**
   - A rail line crosses the basin with a dedicated corridor (about 18 m wide) and is elevated over arterials and the river.
   - Stations sit on the line, at least 400 m apart in the production domain; the prototype has one active station.
   - An elevated expressway runs above one arterial row and one arterial column (Tokyo's Shuto pattern). It crosses the river only on a crossing row and never runs along the water. The east–west route is at 11 m and the north–south route at 20 m. Each has its own nodes, including separate nodes at their shared horizontal crossing. They pass over/under each other without a turn connection; no interchange ramps are implied. Neither is flattened into the street graph.
   - Gateways connect the arterial and rail layers to the region.
3. **Arterial lattice:** a quad lattice of major lines. Tokyo arterials are far apart, so only the two river-bank roads, the gateway row south of the rail corridor, the expressway row north of it and the expressway column are arterials. The other lattice lines are local collector streets, and every crossing row gets a bridge. Columns follow the river course as offset polylines, so bank arterials frame the river. Rows include every river bend and the rail corridor's frontage streets. Interior arterial junctions get seeded slopes (up to about 15°) and offsets (up to about 30 m), so arterials bend and run diagonally across the fabric. A convexity guard keeps every superblock convex. The river-bank roads, the rail corridor frontage and the outer ring stay fixed. Lattice edges are straight segments between shared junction nodes. Every lattice cell is a superblock or a reserved corridor band.
4. **Local streets:** each superblock gets its own street orientation: the superblock's long axis turned by a seeded angle, up to about 35° in compact districts and nearly none in the station core. It is then subdivided by recursive street cuts (hierarchical domain splitting) along that orientation, so blocks meet arterials at angles. A polygon whose short side exceeds twice the district's minimum block depth is cut along its long axis; otherwise one longer than the district's maximum block length is cut across it. Each cut is tilted and offset by seeded jitter, and it ends where it meets an existing street, creating a T-junction. A cut is rejected if it would land within 14 m of an existing junction or leave a block below its minimum size. District patterns, from station distance and a seeded draw:
   - station core: nearly straight cuts and deep blocks, for larger mid-rise lots;
   - compact: strongly jittered cuts and blocks at least 36 m deep;
   - strips: long narrow blocks whose cross cuts are mostly lanes;
   - waterfront: coarse blocks.
5. **Blocks:** each final polygon is a convex block. Inset every edge by its street's lot setback: half the carriageway plus the sidewalk on arterials, or a small margin on local streets and lanes. The result is the buildable block.
6. **Lot partition (street-aligned OBB subdivision):** split each buildable block parallel to its longest street edge into two back-to-back rows. Cut each row perpendicular to its street at seeded frontage widths, so lots line up with the street they face and oblique corners become irregular corner lots. Each lot's building rectangle is the largest rectangle inside the lot fronting its street. Each lot stores its frontage road, access node and orientation. Lots in corridor bands, river setbacks or protected land are never generated, and pieces without street access are dropped unless they are flag lots.

Lots in deep rows may be flag lots (旗竿地): rear lots reached by a passage beside a front lot and addressed to the same street. They keep fine grain without adding streets.

Plan generation must be bit-identical on every supported target. Plan decisions use only IEEE-exact arithmetic (no trigonometry or `hypot`). Saves record a plan digest, and restore rejects progress that does not match the regenerated plan.

Plan geometry is the authority for collisions. Lots from one partition are disjoint by construction, so they skip pairwise clearance tests. Generation must still validate that no lot overlaps any road right-of-way, rail corridor, river setback, or protected land, and that no two lots overlap at all.

Deep blocks in compact and strip districts often get a dead-end roji: a 4 m alley from a local street about two-thirds of the way into the block, replacing the lots in its path.

Variety comes from district orientation, jittered cuts, T-junctions, roji, lot widths, flag lots, building families and heights. Future work: bent streets, dead-end roji, retained old diagonal roads, and lot assembly, all through the same partition contract.

## Growth: realizing the plan

Stages describe measured conditions, not scripted unlocks.

| Stage | Typical change |
| --- | --- |
| Founding | Gateway, first arterial segment, small homes, shop, construction crew |
| Settlement | Occupied homes generate jobs, service demand, and deliveries |
| Neighborhoods | Local streets open around demand; businesses and community spaces establish centers |
| Transit city | Rail service, bridges, and the expressway open; station districts intensify |
| Metropolis | Most blocks are built out; lots are assembled for mid-rise and towers |
| Maturity | Outward development stops; renewal, tenant turnover, and redevelopment continue |

Different seeds can emphasize river frontage, several centers, a linear rail corridor, or an older compact core. Do not force identical skylines.

### Road realization

A planned street segment is eligible when at least one endpoint is on the open connected network and its corridor is not blocked by an unfinished project. Score eligible segments by:

- unmet demand near the segment (occupied buildings and vacancy within about 120 m);
- station proximity once service runs;
- arterial priority;
- the number of plan lots the segment would give frontage to;
- a seeded tie-break.

Admit a bounded number of road projects at a time, gated on sustained demand and low vacancy. Roads follow the normal visible lifecycle (survey → clearing → structure → surface → finishing) and become routable only when complete. A river crossing is admitted only when the opposite bank has planned lots and the near bank has an active catchment.

### Lot release and building admission

When a road opens, every plan lot whose frontage road is now open becomes a parcel. Building projects pick from released, unbuilt parcels using the existing local-infill selection: prefer sites with active neighbors, then proximity, then stable ID. Selection is gated on demand, vacancy, and finite construction capacity.

Use and height come from district character, as an initial calibration:

- **Station core:** offices and apartments, 4–12 floors. At most eight towers of 16–32 floors per city go on large served station-core lots (at least 280 m²), forming a recognizable cluster.
- **Station fringe:** mixed 2–6 floors.
- **Arterial frontage:** mid-rise 3–8 floors.
- **Compact interior:** homes and shop-houses of 2–3 floors (almost no single-storey buildings), with scattered 5–6 storey apartment and office blocks.
- **Waterfront:** larger, lower-coverage lots and parks.

### Densification and renewal

Mature districts densify on the lots they already have. Renewal replaces an aging low-rise building with a taller one where district demand supports it. Lot assembly merges 2–4 adjacent lots on one frontage into a single site for a mid-rise or tower, but only around working stations and on arterials. Lot splitting may divide an oversized lot on inheritance-style turnover. Every redevelopment needs relocation capacity, keeps a seeded fraction of character buildings (including the founding shop when feasible), and uses hysteresis so no site oscillates.

### Determinism and bounded work

Plan generation runs once, with bounded size: lots at most 3,000 in the prototype. Growth evaluations inspect a bounded window of eligible segments and lots through persisted cursors. Stable ordering and seeded tie-breaks preserve replay. Cached eligibility is updated incrementally when roads open or lots are built.

## Roads being built

Road project lifecycle:

1. Select an eligible planned segment from the open network.
2. Validate it against current geometry: no unfinished conflicting project, river and bank rules, and finite segment capacity.
3. Reserve the corridor.
4. Survey and clear; expose earthworks and construction access.
5. Build drainage, surface, bridge structure if needed, and markings.
6. Open the completed connected segment atomically to routing.
7. Release the plan lots that front it and recalculate local accessibility.

At-grade ordinary streets meet at explicit junctions. Elevated rail, the expressway, and bridges are separate layers and must not be flattened into street-face topology. Only completed, connected sections carry normal traffic. Work crews may use explicit construction access. A disconnected completed segment waits for its connection; travelers never teleport onto it.

Road hierarchy, by rendered carriageway: `expressway` (20 m wide, elevated at 11 m or 20 m according to its planned corridor), `arterial` (16 m, four lanes, 2.5 m sidewalks; arterial bridges match), `local` (6.5 m shared surface with painted edge lines), `lane` (alley, 4 m, walking and deliveries, no through traffic). Also: `bridge`, `rail`, and `expressway` (elevated, grade-separated, no frontage). A buildable lot needs road access suitable for its use.

## Rail and transit

The rail corridor is reserved in the plan and built visibly in segments. Passenger service begins only after a connected line, usable station access, and a schedule exist. The prototype runs one served segment between the station and the regional rail gateway; the line's remaining corridor is built as infrastructure for later multi-station service. Next transit slice: multi-station lines with trains serving each open station, plus bus routes along arterials connecting stations and centers.

Station catchments influence development only after service begins; announced projects may have a small, capped anticipatory effect. The expressway is built when the city reaches the transit-city stage. It carries ambient regional traffic only and creates no frontage.

## Construction and redevelopment

Buildings progress through clearing, foundations, structure, exterior, and opening. Road projects and building projects compete for finite crews and deliveries. An initial crew may arrive from the external gateway; later capacity grows within limits.

Redevelopment requires destination capacity for displaced residents or businesses. Keep a minimum stock of occupied homes and key services. Never demolish all recognizable anchors at once.

## Finite capacity

Proposed default ceilings:

| Resource | Ceiling |
| --- | --- |
| Buildings and reserved building sites | 3,000 |
| Plan lots (parcels) | 4,000 |
| Logical road and rail segments | 5,000 |
| Named persistent residents | 256 |
| Active individual travelers | 2,000 |
| Represented population | 30,000 |
| Simultaneous construction projects | 12 |

These are bounds, subject to profiling. A building ceiling of 3,000 requires renderer distance-based detail for low-rise fabric: distant low-rise buildings collapse to merged boxes with roof color. Reserve capacity before approving projects. If admission would exceed a ceiling, select renewal or defer; never silently delete entities to fit.

At maturity, jobs and shops change occupants, buildings renovate, transit service adjusts, and occasional sites redevelop. Protect stability with project cooldowns and hysteresis. A city can remain quiet without inventing disruptive events.

## Research references

- [Parish & Müller, Procedural Modeling of Cities](https://people.eecs.berkeley.edu/~sequin/CS285/PAPERS/Parish_Muller01.pdf): global goals with local road constraints.
- [Chen et al., Interactive Procedural Street Modeling](https://peterwonka.net/Publications/pdfs/2008.SG.Chen.InteractiveProceduralStreetModeling.pdf): direction fields for district orientation; future source of irregular superblocks.
- [Vanegas et al., Procedural Generation of Parcels in Urban Modeling (2012)](https://doi.org/10.1111/j.1467-8659.2012.03034.x): oriented-bounding-box and straight-skeleton block subdivision into gap-free lots; the basis of lot partition.
- [Yang et al., Urban Pattern: Layout Design by Hierarchical Domain Splitting (2013)](https://doi.org/10.1145/2508363.2508405): street and block layouts by recursive splitting of domains.
- [Lipp et al., Interactive Modeling of City Layouts using Layers of Procedural Content (2011)](https://doi.org/10.1111/j.1467-8659.2011.01858.x): editing layered plans without regenerating everything.
- [Müller et al., Procedural Modeling of Buildings](https://doi.org/10.1145/1179352.1141931): shape grammars for massing and façades.
- [Weber et al., Interactive Geometric Simulation of 4D Cities](https://peterwonka.net/Publications/pdfs/2009.EG.Weber.UrbanSimulation.FinalVersion.pdf): geometric urban evolution over time, including land-use-driven lot development.
- [Vanegas et al., Interactive Design of Urban Spaces](https://www.cs.purdue.edu/cgvlab/www/resources/papers/Vanegas-2009-Interactive_Design_of_Urban_Spaces_Using_Geometrical_and_Behavio.pdf): feedback between behavior, demand, and geometry.
- [tmwhere, Procedural City Generation](https://www.tmwhere.com/city_generation.html): bounded priority-queue road proposals.
- Urban form: André Sorensen, *The Making of Urban Japan*; Jorge Almazán, *Emergent Tokyo*; Barrie Shelton, *Learning from the Japanese City*. Lessons: superblocks with fine interiors, narrow lots, fast building replacement, roji lanes, and station-centered intensity.

## Acceptance examples

- A new lane is visibly built before its adjacent buildings start construction.
- A bridge connects two active networks before cross-river trips begin.
- A station creates demand around actual entrances and walking routes.
- No seed produces inaccessible occupied buildings or overlapping lots.
- A mature compact crop meets the fabric targets above in at least three seeds.
- A full city continues generating everyday activity without crossing its bounds or caps.

## Implementation status (2026-10-11)

The master plan, its growth realization, narrow streets, paved blocks, party-wall rendering, the elevated rail viaduct and the expressway are implemented. Three mature seeds measure 49.6–51.6% rendered coverage in compact crops and 39.9–42.2% in station crops, against 28.9% and 20.1% for the legacy planner. Evidence, captures and open items are in the [master-plan review](../validation/masterplan-review/README.md). Not yet implemented: lot assembly and height renewal, multi-station rail service, bus routes on arterials, distant low-rise LOD, and removal of the legacy planner.
