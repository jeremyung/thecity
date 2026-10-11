# World Growth and Infrastructure

The city occupies a finite geographic basin. Expansion is visible and connected, while mature neighborhoods can continue changing indefinitely within fixed resource limits.

## Landscape and boundary

Proposed production landscape: a 2.4 km by 2.4 km square simulation domain, divided into 24 by 24 chunks of 100 m. Prototype: 800 m by 800 m. The chunk grid supports storage and indexing; roads and parcels need not follow the grid.

The seed defines terrain, river course, hills, protected open space, gateways, and developable land. Aim initially for 50–65% developable land; preserve meaningful parks, banks, and steep terrain rather than filling every surface.

Outside the domain, render inexpensive atmospheric hills, water, or distant silhouettes. They have no parcels, residents, routing, or growth. Camera bounds and terrain composition keep the edge legible without suggesting reachable districts. External gateways connect to abstract regional demand; they do not generate an infinite world.

Every seed must have at least one viable founding area and enough reachable land for several neighborhoods. Reject or repair invalid geography using deterministic rules before simulation starts.

### Metropolitan map composition reference

The [additional user map](../concepts/photo-references/metropolitan-waterways-block-patterns.png) reinforces a hierarchy of city form: a fine-grained continuous built fabric, distinct clusters of taller buildings, irregular older street patterns alongside more rectilinear districts, larger waterfront blocks, connected waterways, and concentrated open spaces. Use district-conditioned road directions, block dimensions and parcel subdivision rules instead of one global grid or independent random rotations. Major corridors should connect neighborhood networks and bridge crossings; infill should resolve leftover accessible spaces within those networks.

Preserve meaningful parks, banks and larger open areas while reducing unexplained lawn between ordinary developed parcels. Do not translate green map fills directly into grass materials or land-use categories: the source legend and scale are unknown. These are qualitative visual goals, not extracted numeric Tokyo targets. At wide camera scales the finite city should read as connected neighborhoods with a coherent river/corridor structure; nearer views must retain usable street frontage, service gaps and purposeful movement. Existing art styles, simulation bounds and performance requirements remain unchanged.

## Growth stages

Stages describe measured conditions, not scripted unlocks.

| Stage | Typical change |
| --- | --- |
| Founding | Gateway, initial lane, small homes, shop, construction crew |
| Settlement | Occupied homes generate jobs, service demand, and deliveries |
| Neighborhoods | Streets branch; businesses and community spaces establish centers |
| Transit city | Stations and bus links increase accessibility and density |
| Metropolis | Mixed districts, infill, larger buildings, and busy transport |
| Maturity | Outward development slows; renovations, reuse, and selective redevelopment continue |

Different seeds can emphasize river frontage, several centers, a linear rail corridor, or an older compact core. Do not force identical skylines.

## Development opportunities

### Density feedback and urban form

Active implementation task (2026-10-10): introduce district-aware project competition. Incremental measurement and persisted checkpoints are integrated. The current engine task integrates and tests bounded candidate competition through existing admission checks; production scheduling stays unchanged until that policy is reviewed. Keep the bridgehead bootstrap viable. Compare infill, interior access and frontier expansion using local unmet need and vacant capacity rather than unconditional planner priority.

Project competition experiment (in progress, not enabled in the normal app): compare the existing first-valid frontier proposal with up to 48 feasible interior-access proposals after local-chord planning. The selected bridgehead proposal retains its existing exception. Use actual feasible oriented site geometry and conserved clipped cell contributions. Forecasts remain separate from measured construction. Initial calibration uses gross nominal-envelope minima of 0.32 historic core, 0.40 station core, 0.35 station fringe and 0.28 outer neighborhoods; these are experimental inputs, not accepted Tokyo fidelity targets. Initial weights are coverage 1, demand 0.2, floor/frontage 0, road-cost scale 60 m and forecast utilization 0.5. Target maxima (minimum+0.10) are metadata, not enforced upper bounds.

Matched-seed experiment result (2026-10-10): at development rate 16 and tick 242,000, enabling this policy changes seed 87231 from 753 to 802 open buildings, and seed 0 from 845 to 813. Root's independent projected-kit audit measures gross coverage changing from 14.8662% to 15.8185% for seed 87231, but from 16.3703% to 16.0307% for seed 0. All four snapshots use the same pre-frontage-change rendering geometry. These mixed results do not justify production promotion. The policy remains default-off while district distribution and scoring are investigated; the figures are geometry measurements, not visual acceptance.

Exact corridor integration checkpoint (2026-10-10): new road admission uses at least 5 m exact clearance from all protected parcel envelopes, preserving both legacy and rendered orientations where required. Saved legacy parcels retain their former radial acceptance as a compatibility fallback; explicit-rotation validation retains its historical threshold. Pending access branches use exact clearance. The repaired public WASM passes the full 171-test web suite and the 149-test native suite; the two matched mature seeds retain their earlier trajectories, so this is a validated feasibility correction, not a measured city-density gain. See `validation/corridor-clearance-review.md` and `validation/spec-audit.md`.

Candidate diagnostic checkpoint (2026-10-10): bounded test-only traces reproduced the four prior save hashes exactly. Approximately 94% of policy comparisons had no usable geometry candidate. Interior access won all three comparisons where both categories were ranked across the two seeds, so the evidence does not support changing score weights to favor access. Next, audit corridor feasibility: the legacy max-dimension radial exclusion can reject a road that clears the exact parcel envelope by more than 5 m. The proposed correction keeps exact clearance, pending access reservations, and both legacy and rendered envelopes for parcels without stored rotation. The subsequent integration checkpoint above records the completed verification and compatibility rules.

Read current housing and commercial vacancy from connected open buildings within 110 m of the proposal's source, not the cached destination cells. For the experiment, positive housing vacancy at or above 25%, or commercial vacancy at or above 35%, suppresses new mixed-capacity proposals; zero existing capacity is bootstrap-neutral. Require current demand > 0.8. Cached density older than 400 ticks falls back to established planning. The experiment must preserve deterministic scan/RNG advancement and demonstrate its effects across matched seeds before any production promotion. This first comparison leaves earlier infill and coordinated-block precedence intact; it is not the full project-competition milestone.

Runtime measurement checkpoint (2026-10-10): the controller now runs in public WASM, capturing a frozen building snapshot, advancing at most 16 samples each fixed tick, and refreshing 200 ticks after completion. Pending and completed source checkpoints persist in saves; restored metrics are reconstructed from those inputs. Frontage remains explicitly unavailable. Root verifies two mature seeds over 10,000 ticks against the prior WASM: existing world-state text is identical after excluding only the appended controller field, and checkpoint save/restore continuation matches. See `validation/district-runtime-comparison.json`. Runtime project scoring is still inactive, so this is persistent measurement infrastructure, not a density improvement. Earlier statements that the controller or checkpoint are test-only are superseded by this checkpoint.

Proposal and scoring architecture (2026-10-10): interior-access and first-valid frontier searches expose read-only proposals. Legacy admission retains prior ordering, cursor and RNG advancement; captured native before/after saves match for frontier success and exhausted-search cases, with source-hash limitations documented in `validation/frontier-refactor.md`. `engine/src/district_density.rs` measures clipped nominal building envelopes, while `engine/src/project_scoring.rs` ranks multi-cell forecasts with project-level cost and demand terms. Housing vacancy and commercial activity remain separate. Overlapping nominal envelopes are summed, so coverage is a proxy rather than rendered footprint union or net developable coverage. Policy integration, calibration and visual validation remain in progress; upper targets and hysteresis are not implemented.

Acceptance sequence: unit-check district clipping and stable ranking; verify scheduler admission, connectivity and save/resume; compare matched native and public-WASM runs for multiple seeds; then inspect neighborhood and overhead browser views in both styles. Report actual projected coverage separately from nominal parcel-envelope and floor-area proxies. A pure module or a higher building count does not complete this task. Early access-corridor reservation, station-centered height clusters and occupied-site redevelopment with relocation remain subsequent required work. The next access slice is an optional deeper-block geometry recipe, not a replacement for shallow blocks: with 8 m street setbacks, an 82 × 82 m block and center street supports twenty 11.44 × 11.4 m nominal lots (2,608.32 m² total), while a two-row layout on the same area supports 3,649.36 m² of envelopes. The street buys internal access and finer frontage at an area and road-cost penalty. Reserve its geometry before perimeter construction, resolve road IDs only after those roads open, and compare feasible alternatives before selection. See `validation/early-access-design.md`; runtime integration is not yet implemented.

Density is a local development objective, not a timer or a fixed skyline to reproduce. Each seeded neighborhood has a target range for built coverage, floor area, continuous street frontage, and height distribution. Station districts can support taller, more intensive development while historic streets retain compact low-rise character. Demand, vacancy, working transport, land constraints, and construction capacity remain prerequisites; falling below a visual target never bypasses them.

Audit checkpoint (2026-10-10): the offline density report now clips rotated nominal parcel envelopes to world bounds and fixed cells using the renderer’s Y-rotation convention. It still sums parcel envelopes, including overlaps, rather than measuring actual rendered building footprints or net developable land. These reports support comparison; they are not yet the runtime district feedback controller.

Selection checkpoint (2026-10-10): construction now ranks a rotating window of up to 32 eligible sites, preferring sites with at least three active neighbors within 78 m, then nearest open-building proximity and stable parcel ID. The cursor advances past the inspected window; founding fallback remains possible. This implements local infill preference, not coverage/FAR targets, vacancy-based project competition, or block-interior access. Fresh-seed comparison lowered gross envelope coverage, so visual density improvement is not established.

Measure building footprint and total floor area independently. Gross coverage uses the full geographic region as its denominator and is useful for comparing the same seed over time. Net developable coverage excludes water, protected land, and reserved infrastructure; calculate it from actual land geometry before using it for admission decisions. Do not label a gross statistic as net coverage. Measure frontage continuity along usable streets, and retain vacant or underused capacity as a separate economic signal. Taller buildings increase floor area without necessarily improving coverage or street continuity.

The bounded development loop measures a neighborhood, identifies its unmet need, scores feasible projects, reserves one accepted project, and measures again after construction. Candidate actions include frontage infill, connected local access streets opening block interiors, parcel subdivision, and selective redevelopment with relocation capacity. An access street must provide usable new sites or a meaningful network connection; roads cannot be added merely to increase a density counter. Reserve future access corridors before parcels make them impossible. New interior buildings need the same explicit pedestrian and delivery access as roadside buildings.

Candidate comparison must distinguish the district receiving new buildings from the established catchment supplying demand. A frontier endpoint in an empty cell has zero measured vacancy; that alone must not override vacant capacity in the neighborhood it would serve. Attribute building-envelope and floor-area gains to the cells actually intersected by proposed lots, and charge each street's construction cost once. Do not count an entire cross-boundary project as new coverage in every touched cell. District-specific targets come from neighborhood character and working station access, with explicit calibration rather than one uniform city-wide number.

Proposal generation must be read-only with respect to authoritative geometry, IDs, projects and growth RNG. Speculative seeded draws and scan cursors are returned explicitly; the scheduler applies an agreed deterministic advancement rule once. Admission rechecks current feasibility before any mutation. Early integration preserves first-valid frontier selection and existing seed outcomes; changing selection policy requires a separately measured comparison. Cached measurements must be invalidated or updated after relevant construction, occupancy and business changes, and cache restore behavior must preserve identical decisions after save/resume.

Use persisted cursors and incremental measurements so density planning cannot trigger unbounded whole-city searches. Stable ordering and seeded tie breaks preserve replay. Use hysteresis and vacancy checks to prevent repeated demolition or oscillation around a target. Once a district approaches its range, favor renewal rather than filling protected space or forcing a uniform height.

Acceptance requires same-seed before/after measurements and browser views at neighborhood and overhead scales. Report coverage, floor area, accessible occupied sites, and infrastructure cost together. A maturity label, building count, or a few towers cannot establish dense urban form. Initial target ranges remain calibration work against the supplied overhead, crossing, and river references; no numerical target is approved solely because the current implementation can reach it.

Maintain a finite queue of candidate projects. A candidate has prerequisites, land reservation, resource requirements, benefit score, estimated completion time, and a reason suitable for the history inspector.

Score plausible projects from unmet demand, network access, local suitability, occupancy, service coverage, ecological constraints, and deterministic variation. Global checks enforce land, entity, infrastructure, and construction limits.

Approve only a bounded number per development evaluation. Respect vacancy and moving averages so short-lived demand spikes do not create whole empty districts. An external baseline can help establish the first settlement, but external migration is bounded and reacts to available homes and employment.

### PLATEAU-informed calibration

Use the [PLATEAU research](../validation/plateau-research.md) to build a small, provenance-tagged reference pack before changing density targets. Measure joint footprint, height, frontage and gap distributions for station-edge, ordinary compact-neighborhood and waterfront samples. Keep rendered footprint union, nominal parcel/envelope coverage, gross land and net developable-land denominators separate. Preserve missing attributes and source vintage; do not fill measurements from visual guesses. Station-cluster archetypes must depend on working pedestrian access, with smaller older buildings retained around concentrated taller groups. Calibrate after access discovery is accepted and compare multiple seeds without bypassing demand, collision, construction or finite-capacity rules. Numerical targets remain unset until source extraction is verified.

## Roads being built

Use a constrained procedural street planner combining terrain costs, nearby destinations, existing intersections, parcel geometry, and reserved arterial corridors. Begin with short local extensions; add larger connections when served demand justifies them.

Road project lifecycle:

1. Propose a corridor from an existing active network node toward reachable demand or a viable growth front.
2. Validate slope, width, water crossings, intersection spacing, parcel impact, and boundary clearance.
3. Reserve the corridor and prevent conflicting development.
4. Survey and clear; expose earthworks and construction access.
5. Build drainage, surface, bridge structure if needed, and markings in segments.
6. Open completed connected segments atomically to routing.
7. Create or update adjacent parcels and recalculate local accessibility.

At-grade ordinary streets must form an explicit junction wherever they meet: geometric crossings without a shared routing node are invalid. Validate the founding network as well as generated additions. Elevated rail and bridges remain separate layers and must not be flattened into street-face topology. Face polygons describe boundaries, not buildable land; internal dead-end streets, parcels, water and reservations remain collision obstacles even when they do not bound a face.

Only completed, connected sections carry normal traffic. Work crews can use explicit construction access. A disconnected completed segment waits for its connection; travelers never teleport onto it.

### Interior access streets

A new access street may branch from the interior of an existing open local street or arterial. At project admission, insert a collinear junction into the host road while preserving its usable geometry, frontage access and active journeys. This graph subdivision does not open the new branch. The branch follows the normal visible survey-to-surface construction lifecycle and becomes routable only on completion.

Before changing topology, validate and reserve both the proposed corridor and at least two compatible, precisely oriented frontage lots. Include reserved lots in parcel-capacity accounting and collision checks so unrelated development cannot consume the promised sites. Failed admission must leave IDs, roads, actors and reservations unchanged. Publish the reserved lots only after the branch is connected and open; a failed completion must retain a recoverable pending project rather than partially publish its parcels.

Persist the bounded candidate cursor and pending reservation. Restore must reject malformed geometry, stale project references, disconnected hosts and over-capacity reservations; older saves without an access-street reservation remain valid. Candidate evaluation retains river, protected-land, existing-parcel and road-clearance rules, and must not scan the entire map on every growth update. Test existing routes in both directions, active travelers, bus passengers, frontage access, pending save/resume and repeated deterministic continuation.

Implementation checkpoint (2026-10-10): live admission now examines at most 48 proposals per evaluation using a persisted cursor, reserves one branch and 2–16 lots, and synchronizes the branch's visible construction with its project. The branch stays disconnected until all-lot completion preflight succeeds. Restore checks host subdivision, branch geometry, references and reservations. Public-WASM continuation of two mature saves completes one and three branches respectively, with pending/open checkpoints and a save/resume regression. Gross nominal parcel-envelope coverage improves by less than 0.21 percentage points in each city. This mechanism complements, rather than replaces, neighborhood density feedback and earlier corridor planning; it does not yet satisfy the dense-city visual target.

Local streets can be narrow and irregular. Arterials connect centers. Pedestrian paths can open land for walking without making it accessible to delivery trucks. A buildable parcel needs the required access for its use.

Implementation checkpoint (2026-10-10): the former 160-open-road stop is removed while the finite segment ceiling and economic/access gates remain. Ordinary road extensions and local chords require at least two compatible potential frontage sites, evaluated across at most 34 positions. Both can be on one side of the street. These are admission opportunities, not reserved parcels or promises of immediate occupancy. A matched mature-save continuation grows646 to754 buildings and168 to202 roads; see the latest validation audit. District density feedback remains future work; the later interior-access checkpoint above supersedes the access-planning status of this road-extension comparison.

Road density has diminishing value. Prefer infill, loop completion, and connection improvements over repeatedly extending dead ends. Bridges are exceptional projects with higher resource requirements and clear demand. No arbitrary roads to the boundary simply to occupy space.

A completed connected bridge may seed one short approach street on its undeveloped bank when the opposite bank has an active catchment. This opportunity must not depend on already having occupied buildings beyond the crossing: that would make the bridge unable to open new land. Suppress duplicate approach projects, require ordinary road geometry and viable frontage, and let subsequent expansion return to the normal occupied-neighborhood rules. Closed or disconnected bridges confer no development access.

## Rail and service expansion

Reserve plausible rail corridors during generation to avoid destructive late routing through dense blocks. Construct rail, platforms, and station access visibly. Passenger service begins only after a connected line, usable station access, and a schedule exist. Electrification and utilities can be abstract capacity requirements rather than fully simulated networks.

Station catchments influence development only after service begins; announced projects may have a small, capped anticipatory effect. Bus routes use active roads and adjust gradually to connect centers.

## Construction and redevelopment

Buildings progress through clearing, foundations, structure, exterior, and opening. Road projects and building projects compete for finite crews and deliveries. An initial crew may arrive from the external gateway; later capacity grows within limits.

Redevelopment requires destination capacity for displaced residents or businesses. Keep a minimum stock of occupied homes and key services. Preserve a seeded fraction of character buildings, including the founding shop when feasible. Never demolish all recognizable anchors at once.

## Finite capacity

Proposed default ceilings:

| Resource | Ceiling |
| --- | --- |
| Buildings and reserved building sites | 2,000 |
| Logical road and rail segments | 5,000 |
| Named persistent residents | 256 |
| Active individual travelers | 2,000 |
| Represented population | 30,000 |
| Simultaneous construction projects | 12 |

These are gameplay-independent resource bounds, subject to profiling. Segment limits count logical routing segments, not mesh subdivisions. Reserve capacity before approving projects. If admission would exceed a ceiling, select renewal or defer the project; never silently delete existing entities to fit it.

Begin soft saturation around 75% of usable development capacity. Land scarcity, vacancy, transport capacity, and construction costs lower expansion rates before hard limits. Do not expose a progress bar toward filling the map.

At maturity, jobs and shops change occupants, buildings renovate, transit service adjusts, occasional sites redevelop, and seasonal or daily activity varies. Protect stability with project cooldowns and hysteresis. A city can remain quiet without inventing disruptive events to force novelty.

## Procedural generation research and proposed next work

Implementation checkpoint (2026-10-09): new rectangular neighborhood plans now reserve their interiors during perimeter construction. Once all four streets are open and connected, a pure bounded subdivision generates two back-to-back rows facing the long streets. The current 82 m × 50 m recipe uses an 8 m street inset, approximately 11 m target frontage, and 2.2 m parcel gaps; coordinated parcels then enter ordinary demand-driven building construction. Existing plans without the saved recipe marker retain their previous placement behavior. This is the first integrated rectangular-block slice, not general irregular-polygon subdivision, alleys, district-wide redevelopment, or artistic acceptance.

New coordinated parcels carry explicit local footprint rotation. Admission and validation use oriented footprint checks; building facing agrees with actual frontage metadata and access nodes. Legacy parcels keep their historical serialized envelope convention to avoid silently moving or invalidating saved cities. New-lot admission respects both that saved reservation and the rendered orientation of existing buildings. Unifying all legacy collision envelopes remains future migration work.

As of 2026-10-10, ordinary progressive frontage infill also stores and checks road-facing footprints; tower admission retains its established path. The existing bounded cursor, road offsets, spacing, centerline clearances, and demand/access prerequisites remain in force. In a matched 64000-tick continuation of the mature seed-87231 fixture, this recovered nine buildings on existing roads (629 to 638, unchanged 168 roads). This is a narrow infill correction, not density-target feedback, irregular-block planning, or a visual acceptance result.

A subsequent gap-aware fallback projects existing reserved footprints into each 12 m-deep roadside strip, unions their buffered frontage intervals, and proposes 8–14 m-wide parcels in remaining spans. It preserves the historical grid and tower priority, then appends at most eight gap proposals per side. Full oriented collision, demand, frontage, and access checks still govern admission. The persisted cursor accepts at most 50 positions per road, with at most 48 site checks and 24 roads visited per evaluation. Projections are cached once per visited road within the evaluation; footprint scans and interval sorting still scale with world size. In a matched continuation from 638 buildings, this adds 20 buildings without new roads (658/168). It does not open inaccessible block interiors or implement district density targets.

Scheduling checkpoint (2026-10-10): after usable frontage and progressive infill, a feasible coordinated block can now compete every80 ticks before ordinary road extension. It still requires no active projects, no outstanding infill quota, and no unbuilt historic block frontage. This replaces the former1600-tick opportunity; bounded anchor scans remain. All three road admission paths now protect exact cached parcel footprints as well as historical center clearances. Fresh same-seed coverage improves from19.15% to24.22% with fewer roads, but artistic acceptance remains open; see the validation audit for limitations.

Coordinated plans reserve parcel capacity for their entire deterministic layout before construction starts. Other parcel admissions cannot spend those reserved slots. Completion releases the reservation in the same synchronous operation that admits the lots. Road admission also protects the reserved block interior independently of scheduler order; planned perimeter segments remain permitted. Near-cap worlds defer new blocks instead of constructing infrastructure for a truncated layout.

Recorded 2026-10-09 following the 580-building visual review. These are proposed implementation directions, not completed features or accepted art.

Priority order: block subdivision and compact infill; architectural grammars; richer street layouts. Streets should enclose usable block polygons, excluding sidewalks, water, and protected land. Subdivide these into street-accessible parcels with district-specific frontage, depth, and setbacks. Interior development requires real access, including alleys where appropriate. Avoid treating leftover land as uniform lawn or increasing tower counts as a substitute for urban fabric.

Infrastructure efficiency follow-up: compare explicit developed frontage per 100 m of ordinary road alongside density. `scripts/audit-road-frontage.mjs` reports this for saved worlds, separating coordinated block streets from other ordinary streets. It excludes rail and bridges and does not estimate traffic or connection benefits. In the fresh seed87231 road-benefit checkpoint, coordinated streets support6.20 frontage buildings/100 m versus4.69 elsewhere; this is evidence for investigating project selection, not a universal target.

Shared-edge implementation checkpoint (2026-10-10): the planner can reuse an open connected ordinary street and construct the other three perimeter edges. It preserves the street and junction IDs, checks junction capacity and exact corridor clearance, and retains existing parcels/frontages. Up to24 candidate existing edges are inspected per evaluation within the existing bounded anchor scan; a four-new-edge fallback remains.

An optional saved `NeighborhoodPlan.selected_lots` stores the collision-free subset of the canonical block subdivision. At least four new sites are required, with benefit checks on new long edges. The selected count is reserved against parcel capacity. Completion preflights the complete promised layout before releasing the plan and admitting the lots; an invalidated promise leaves the plan pending rather than silently dropping lots. Restore rejects duplicate or noncanonical layouts; old plans without the field retain their original subdivision behavior. The public WASM naturally selects six retained-layout lots at tick7520 for seed87231 and verifies completion through restore.

This establishes connected reuse, not a universal efficiency improvement: the initial two fresh seed comparisons had mixed coverage/floor-area results and more total roads. Never overwrite occupied frontage to force a rectangle.

Ranking checkpoint (2026-10-10): shared-edge and full-block candidates now compete by admitted nominal parcel area per meter of new perimeter road, with quantized integer comparison and stable ties. This measures potential site area, not future occupied floors or actual construction cost. Each class has a 32-candidate ceiling, within 24 anchors and 24 reusable edges. The scan finishes the current anchor and stops when either class reaches its cap, preserving later anchors for the next evaluation. Only the winner allocates junctions. Existing demand, collision, frontage, capacity and construction gates remain.

Public-WASM comparisons at 242,000 ticks show slightly higher developed frontage per road length but lower coverage and nominal floor-area proxy in both tested seeds. Seed 87231 changes 803→767 buildings and 217→203 roads; seed 0 changes 766→771 buildings and 206→204 roads. This is not a density acceptance result. District feedback, net developable/actual footprint measurements, and access into irregular block interiors remain required; area-per-road ranking alone does not solve the target urban fabric.

Mid-edge access investigation (2026-10-10): a bounded offline geometry probe finds one feasible short access street with two lots in ranked seed87231 and three candidates with two or three lots in seed0. These require a junction inserted along an existing open road. This was an offline feasibility result at that checkpoint. The subsequent live implementation and two-seed WASM verification are recorded under Interior access streets above. Split-road migration preserves existing frontage, connected access, live traveler positions, bus routes and save compatibility without leaving the host segment as a bypass edge. See the validation audit for the corresponding tests, coordinates and rejected-candidate counts.

Correlate variation through city seed → district character → block pattern → building family → individual details. Districts share an architectural vocabulary and age distribution while retaining local variation. Station accessibility should support coherent height clusters. This hierarchy supplies proposals to the existing demand, capacity, and construction systems; it must not instantaneously replace developed geometry. At saturation, prioritize infill and feasible redevelopment while retaining recognizable places.

Measure actual building footprints and net developable block area separately from nominal parcel envelopes and whole-map coverage. Validate access, overlap, reproducibility, and construction history alongside reference-matched visual captures. Bound candidate work and update only affected blocks; camera or style changes must not affect generation.

Research references and intended lessons:

- [Parish & Müller, Procedural Modeling of Cities](https://people.eecs.berkeley.edu/~sequin/CS285/PAPERS/Parish_Muller01.pdf): global growth goals with local road constraints.
- [Chen et al., Interactive Procedural Street Modeling](https://peterwonka.net/Publications/pdfs/2008.SG.Chen.InteractiveProceduralStreetModeling.pdf): direction fields for distinct district orientations and river-sensitive streets.
- [Müller et al., Procedural Modeling of Buildings](https://doi.org/10.1145/1179352.1141931): architectural shape grammars for massing and façade detail, shared across rendering styles.
- [Weber et al., Interactive Geometric Simulation of 4D Cities](https://peterwonka.net/Publications/pdfs/2009.EG.Weber.UrbanSimulation.FinalVersion.pdf): geometric urban evolution through time.
- [Vanegas et al., Interactive Design of Urban Spaces](https://www.cs.purdue.edu/cgvlab/www/resources/papers/Vanegas-2009-Interactive_Design_of_Urban_Spaces_Using_Geometrical_and_Behavio.pdf): feedback between behavior, demand, and geometry.
- [ProbableTrain MapGenerator](https://github.com/ProbableTrain/MapGenerator): inspectable street-generation reference; its American-city aesthetic is not the Tokyo art target.
- [tmwhere, Procedural City Generation](https://www.tmwhere.com/city_generation.html): practical priority-queue road proposals, local intersection/snap constraints, density-guided branching, and travel-time pathfinding. Proposed adaptation: deterministic bounded worker queues, separate proposal acceptance from funded construction and opening, and blend seeded suitability with actual accessibility and demand. Its random building placement with collision displacement is not the proposed parcel-generation strategy: retain block subdivision and guaranteed frontage/access. This reference is conceptual guidance, not evidence that these changes are implemented or that its code has been reviewed for reuse.

## Acceptance examples

- A new lane is visibly built before its adjacent apartments open.
- A bridge connects two active networks before cross-river trips begin.
- A station creates demand around actual entrances and walking routes.
- No seed produces inaccessible occupied buildings.
- A full city continues generating everyday activity without crossing its bounds or caps.


### Implementation checkpoint: shallow frontage fallback (2026-10-10)

Progressive infill now appends 8 m deep frontage opportunities centered 12 m from the road after the existing grid and standard 12 m deep gap candidates. Existing connectivity, exact geometry, river, tower reservation, and demand checks remain required. The per-road cursor supports 66 candidates while evaluation limits remain 24 roads and 48 sites. Sites develop through ordinary construction, not instantaneous building insertion. The fallback is compiled into the browser WASM; root's mature-save probe completed 44 additional connected buildings over 20,000 ticks without extra roads. This expands viable infill, but does not establish the target metropolitan density. See `validation/initial-repo-shallow-wasm.json` and the current spec audit for evidence and cross-target limitations.


### Coordinated reservation timing evidence

A matched five-seed experiment reserves a valid coordinated block before the global usable-infill/building-project early return, while retaining normal construction scheduling. This prevents unrelated frontage development from consuming the future block corridor. Production-kit projected coverage increased by 1.86–8.02% across seeds 0, 87231, 7, 41, and 2026 at tick 242000 and development rate 16. This is not an approved final density target: floor-area and building-count changes are mixed, and browser-level neighborhood review remains required. See `validation/early-block-reservation-coverage-comparison.json` and the paired native reports. Integration must retain bounded admission cadence, land/connectivity checks, finite reservation capacity, and save/replay continuity.


Production checkpoint: early coordinated reservation is integrated. A newly admitted plan reserves space before ordinary infill can consume it but starts no perimeter road that tick; road work follows the existing later cadence. Native suite: 197 passed; web suite: 197 passed across 41 files. Root rebuilt-WASM seed87231 checkpoint restores canonically and matches 2000 additional ticks. These checks do not close the remaining dense-city visual acceptance gate.
