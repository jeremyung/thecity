# Checkpoint notes moved from specs/05-rendering.md

Dated candidate, checkpoint and status notes, moved verbatim on 2026-10-16 so the
spec describes current design only. Newest decisions win over older notes.

Flat-roof service candidate (2026-10-12): tanks have physical supports; wider roofs
receive bounded HVAC and a human-scale access headhouse using existing materials.
An Anime still study supports improved silhouettes, while final seeded variation,
both-style motion and night acceptance remain open. See
[roof service evidence](../validation/roof-services.md).

Daylight fill history (2026-10-12): the earlier0.5→1.0 hemisphere trial improved
shaded-wall readability in a limited comparison; see
[daylight evidence](../validation/daylight-fill.md). Current source uses0.78 at full
daylight for Anime and0.6084 for Miniature, together with a prefiltered sky
environment at intensity0.26. Night hemisphere is1.05 for Anime and0.819 for
Miniature. The earlier scalar is historical, not the current lighting contract.
A controlled daylight diagnosis is in progress; no new lighting treatment has
been accepted. Overall concept fidelity and daily-cycle/motion gates remain open.

Vehicle checkpoint (2026-10-12): road traffic and the city bus use bodies traced from generated side elevations (`scripts/assets/build-vehicle-kits.py` → `public/art/vehicles`, `web/src/render/vehicle-manifest.json`). Eleven bodies share one material, a 2048×1024 atlas and a 1024×512 paint/lamp/glass mask (13.3 MiB decoded with mips) plus a 512×1024 canvas livery sheet. Each engine length class has weighted body variants (compact: hatch, crossover, minivan; kei: tall wagon, hatch, pickup; taxi: modern, classic sedan), and each vehicle gets a stable weighted paint, a few percent of shade and width variation, and one of thirteen invented operator liveries where it is a fleet vehicle. Bodies narrow toward each end in plan; fronts take corner lamps and a flat grille below the belt line. Wheels roll and steer in the vertex stage; lamps light at night with brake response. Traffic drives on the left: the engine's lane and kerb offsets (`KERB_LANE`, `BUS_STOP_KERB`) are negative along (-dz, dx), which is the driver's right in the renderer's right-handed y-up frame; bus doorways and the kerb-side elevation are on local +x. The engine reports the body class as the movement kind (1 compact, 4 kei, 5 taxi, 6 van, 7 truck, 8 mail van) and queues road vehicles at half their combined lengths plus 0.5 m, never below 4.5 m. Lane queues run every tick; a car enters a lane only a queue gap behind the last car in; driveways queue at both ends of the access path, and a car still waiting at its door is not emitted. Seed-87231 save over 2,000 ticks: 109 sub-2 m road-vehicle pair samples (31 under 1 m, all momentary driveway crossings), longest stationary vehicle 120 ticks. Hardware WebGL (ANGLE/OpenGL ES, RTX PRO 6000; `scripts/benchmark/benchmark-vehicles.mjs`): 1,000 moving vehicles add about 0.8 ms per frame including updates and shadows, 16 draws and 0.36 M triangles. Open: WebGPU frame time (headless Chromium exposes no adapter here) and fascias, which remain approximations from the side elevation.

Ground streetlight coverage correction (2026-10-12): shared fixtures now use
actual road widths, terrain-level local bases and sidewalk-level arterial bases.
Legacy duplicate ordinary-road fixtures were removed. An enlarged soft-pool
experiment was rejected after close playback review; restrained existing pools
remain an approximation, not dynamic actor lighting. See
[coverage validation](../validation/street-light-coverage.md); elevated-road
illumination and full day-cycle/motion acceptance remain open.

Ground-road continuity checkpoint (2026-10-12): local streets, lanes and
junction fill now share a .3 m asphalt top; open locals render edge lines
without construction cones. Actual mesh regressions and isolated dual-backend
captures support this correction. A shared width-aware rounded footprint now replaces mixed-width circular
pads in road and sidewalk geometry. Arterial bends add clipped outer sidewalk
connections using that shared footprint. Local/arterial paint now clears finite
incident ground carriageways. Full-city motion and broader street realism
remain unfinished. See
[validation](../validation/ground-surface-artifacts.md#ground-junction-datum-and-open-local-streets--2026-10-12).

Ground-road overlap candidate (2026-10-12): coplanar pavement now has explicit
ownership. Existing road footprints, junction remainders and growing surfaces
are partitioned without changing the .3m datum. The completed ownership index
commits with the visible road batch; progress updates fork it. The captured
edge-overlap triangle is clean in a static Miniature view and mesh coverage tests
pass. Motion, both styles and mature active-construction performance remain open;
see [current handoff](../validation/archive/2026-10/HANDOFF.md) and
[CPU profile](../validation/road-surface-partition-cost.json).

Visible-infrastructure checkpoint (2026-10-12): queued building geometry is
admitted only when its explicit access-road identity matches a committed road
batch. Identity includes endpoints, road kind and deck geometry; world reset
invalidates older commit tokens. Missing, closed or disconnected dependencies
remain queued. Unrelated buildings on unchanged roads continue normally. This
is partial implementation: actor movement, ground, access paths and other
dependent presentation still need synchronized infrastructure admission. Do not
accept a coherent living-city frame from the building gate alone. See
[the road staging report](../validation/road-staging.md).

Road-growth checkpoint (2026-10-12): completed infrastructure and active
construction use separate attached material batches. Construction progress
retains finished roads, river rails, junctions, lighting and station geometry;
topology/crossing, deck, platform, river and canopy dependencies invalidate it.
Measured CPU work declines on retained updates without dropping geometry, but
full topology rebuilds and browser growth still hitch. See
[the batch retention audit](../validation/road-batch-retention.md); this does not
satisfy the frame-time or concept-fidelity gates.

Train integration checkpoint (2026-10-14): generated side elevations supply a
four-car commuter consist (cab, trailer, motor, cab), each20m with0.6m gangways,
81.8m total. The three car roles share a2048×1024atlas and1024×512mask; livery is
baked into the atlas, with lamp and glass responses in the mask. Each physical
car samples its own position on the curved alignment. Reversal preserves body
orientation and car order while a facing attribute switches cab lamps. Wheels
follow the shared rail gauge; motor-car pantograph contact follows the wire datum.
Platform edges, terminal inset and straight approaches derive from the consist.

Style-switch checkpoint (2026-10-10): warmup submits one material/layout representative at a time and yields a browser task between samples. Superseded requests stop before the next sample; the live material swap occurs after successful preparation. Founder WebGL observations reduce the largest uninterrupted submission from a multi-second all-layout call to under 100 ms per sample, but total warmup remains about ten seconds and mature/WebGPU/performance acceptance is open. See the audit for measurements and caveats.

Street-lighting checkpoint (2026-10-10): ordinary open connected streets share a deterministic instanced lamp kit across styles, with at most 1,000 placements. Warm lamp emission and soft additive pavement pools follow the common night uniform. These pools are an approximation, not dynamic lights on pedestrians or vehicles. Bridge lighting and full night-scene visual acceptance remain open; see the latest validation audit.

Implementation checkpoint (2026-10-10): the outer scenic terrain is a finite seeded heightfield with a river valley, shared by both styles. It leaves the authoritative basin flat and does not affect growth, collision, or saves. Browser verification and remaining artistic gaps are recorded in [the audit](../validation/spec-audit.md#scenic-terrain-verification--2026-10-10).

Atmosphere checkpoint (2026-10-10): clear-air exponential fog uses density 0.00022 for Miniature and 0.00030 for Anime, plus 0.00025 times rain intensity. The lower aerial haze improves mature overview contrast; it is not a density fix. See the latest audit for captures and outstanding dusk/night/backend checks.

Horizon checkpoint (2026-10-13, supersedes the fog densities above): exponential fog is replaced by range haze (Anime 380–3200 m, Miniature 460–3700 m from camera, both pulled in by up to 45% in rain) coloured toward the horizon sky. The city stays clear while hills, mountains and a new visual-only distant skyline (`skyline.ts`: six seeded tower clusters at 1.8–2.35 km, peaks up to ~360 m, kept off the river vista, sparse lit windows after dusk) recede into the sky. The daytime key is lower (about 50° at 14:00) and warmer, with less hemisphere fill, for longer modelling shadows. Before/after captures (software WebGL, balanced quality, DOF off, seed 87231 at tick 80000): `validation/captures/horizon-atmosphere-*.jpg`. Not yet reviewed on WebGPU, at dawn/dusk transitions, in motion, or in Miniature; the near backdrop hills still read as a broad green blanket.

Water and landscape checkpoint (2026-10-13, supersedes the water checkpoint above): the river follows three.js Water2Mesh rather than tuning the earlier lit, fixed-share reflection. The surface is unlit: two scrolling layers of a procedurally generated, seamlessly tiling ripple normal map (`water-normals.ts`, no image asset) distort both the planar reflection and a refracted view of a new shallow riverbed (−1.2 m at the walls to −2.8 m mid-channel). Refraction is tinted by water thickness from the viewport depth (red absorbed first), Schlick Fresnel (F0 0.035 Anime, 0.025 Miniature) blends reflection over the body, a narrow sun lobe gives glints, and a noisy foam lip forms where thickness approaches zero. Bank trees are now ~10 m apart and stretched 1.7–2.6× vertically inside the unchanged railing envelope, giving the water dark masses to reflect. Outer terrain is painted as farmland parcels, warped woodland on high ground and suburban roof blocks near the basin instead of one mottled green. Evidence: `validation/captures/water-landscape-*.jpg` (same harness as the horizon checkpoint, noon/dusk/night). Open: reflection resolution still steps at grazing angles under the High preset's 0.35 scale; WebGPU, Miniature and motion review; the outer land is a flat painted surface with no 3D vegetation.

Ambient/sky checkpoint (2026-10-13): half-resolution GTAO (radius 3 m, normals reconstructed from depth, 65% strength) multiplies scene colour before depth of field and bloom. Hemisphere fill is tinted toward the live sky colours in both styles; standard (Miniature) surfaces also receive a 128 px PMREM environment filtered from the sky dome, refreshed only when its colours drift (≥1.5 s apart), with hemisphere fill reduced 22% to compensate. Drifting cloud shadows (~220 m features, about 40% cover, up to 42% darkening on sun-facing surfaces, off at night, dusk and in rain) are an albedo factor shared by art, land and backdrop materials, approximating removal of the direct term. Evidence: `validation/captures/ao-sky-clouds-*.jpg`. Open: AO cost on WebGPU hardware is unmeasured; toon (Anime) surfaces cannot use the environment map; cloud shadows also darken the ambient term on lit faces.

Water flow checkpoint (2026-10-13): ripple layers are advected by a per-vertex current (`flow` attribute: along each river segment, ~0.6 m/s mid-channel falling to 0.15 m/s at the walls) using Water2-style two-phase sampling over a 6 s cycle, cross-faded to avoid stretching. `validation/captures/water-flow-frames.jpg` shows six frames 1.5 s apart with no visible pulse. Open: no wakes or eddies behind bridge piers; bends change direction per segment rather than smoothly.

Road surface checkpoint (2026-10-13): open roads use a dedicated asphalt material (`ArtMaterials.asphalt`). Roads are merged without lane UVs, so detail is world-anchored: aggregate grain faded by screen footprint, 5.5×9 m repair patches, sparse faint hairline cracks and broad wear, all multiplied by cloud shadows. Rain darkens asphalt (to 80%) and collects in world-space puddles. While streets are wet (hysteresis 0.08/0.03), the pipeline is rebuilt with a view-normal target and half-resolution SSR. Reflectivity is derived inside the SSR pass from depth and normals: flat ground below ~1.6 m, weighted by the same puddle mask. SSR output is added to the scene (it is premultiplied with alpha 1, so alpha blending produced black mirrors). Evidence: `validation/captures/asphalt-*.jpg`. Open: no tyre tracks or lane-aligned wear without road UVs; SSR shows only on-screen content; the pipeline rebuild at rain onset may hitch.

Mist and light-shaft checkpoint (2026-10-13): `scene.fogNode` replaces `scene.fog`, combining the range haze with low-lying mist: it builds with distance (1 − e^(−0.0032·d)) but falls off with height (e^(−(y+1.5)/8)), peaking at 05:30–07:00 (0.5) and in rain (+0.3). Golden-hour light shafts (GodraysNode through the key light's shadow map, half resolution, 40 steps, low density so aerial views do not fill with uniform glow) are added only while the sun is low, with pipeline rebuild hysteresis like wet SSR. Evidence: `validation/captures/mist-shafts-*.jpg`. Open: shafts exist only inside the 460 m shadow frustum; the first frames after load render haze-grey until the new fog programs compile.

Sky and hill forest checkpoint (2026-10-13): `sky-scattering.ts` ports three.js SkyMesh's Preetham model to compute zenith and horizon radiance for the current sun; by day the painted dome's top and horizon colours lead toward them (up to 42%), while night, dusk mood and illustrated clouds stay painted. `backdrop-forest.ts` instances low-poly crowns (one draw, no shadows) wherever the backdrop rises above 30 m, matching the painted woodland, with a soft tree line. Fixes found in review: AO now fades out from 700 to 1300 m (depth-reconstructed normals banded across the mountain ring), and distant skyline towers keep footprints apart (overlaps z-fought into stripes at 2 km). Evidence: `validation/captures/sky-forest-*.jpg`. Open: crowns are uniform icosahedra; plain-level woods have no 3D trees; scattering colours are evaluated per frame on the CPU.

Water checkpoint (2026-10-10): broad normal variation now uses continuous two-axis drifting noise instead of river-spanning axial sine bands. Fine-wave screen-footprint filtering and the simulated rain grid remain. Dusk WebGPU review is recorded in the audit; broad reflected glare and the remaining style/backend/motion checks are still open.

Lowrise surface checkpoint (2026-10-10): non-anchor home windows now use layered dark reveals, separated sash frames and sills, preserving footprint and shared material batching. Anime matte surfaces use world-anchored pigment with derivative-based attenuation of fine strokes; horizontal paving receives a restrained wash. Root passes 26 focused geometry/material tests and inspects both styles in forced WebGL. The controlled views still have plain facades and weak style distinction; this is not art acceptance or motion-stability proof.

Expressway furniture checkpoint (2026-10-11): completed elevated roads now use
shared-style lane/shoulder markings, a median and deck-mounted lamps rather than
the ground-road fallback. The former flat crossing now uses independent 11 m and 20 m corridors
with plan-derived elevations and separate graph nodes. This is an overpass,
not an interchange or a traffic implementation. See [the fidelity workstream](../validation/living-city-fidelity.md).

Finishing-stage checkpoint (2026-10-10): supported low-rise buildings use their finished seeded facade and roof from 86% construction progress, retaining worksite barriers and suppressing business/window emission until opening. The partial shell is replaced, not layered underneath. Root verifies 11 focused tests and 99% Anime/Miniature browser views; this remains a discrete finishing phase, not continuous component-by-component assembly.

Parks and street-tree checkpoint (2026-10-14): `parks.ts` lines open, connected arterials with deterministic street trees on both sidewalks, about every 9 m. Trees keep clear of junctions, cross streets, lamps, building entries, bus stops and viaduct corridors, with at most 900 placements. Vacant protected lots render as pocket parks: gravel ground, a curb and mesh fence with a street-facing entry, a rear hedge, a swing and a slide or sandbox, benches, a lamp and a lit vending machine. The master plan's protected reserve renders as a shrine grove with a torii, a stone approach and a dense evergreen canopy. These trees share the instanced grove foliage and its near/far detail switch. Seeded grove trees stand aside inside parks and the reserve. Concept-board review of these scenes, and seasonal color, remain open. Plaza lots render with granite paving, zelkova planters ringed with benches, a lit clock and bicycle racks. Shrine lots render with a street-side torii, lanterns, a stone fence, a small hall and old corner trees. The busiest arterial junction within 150 m of the station gets scramble markings.

Seasons checkpoint (2026-10-14): a compressed city year has four seven-day seasons, starting in spring. Tree crowns change color by species: cherry blossom is pink only in spring, ginkgo street trees turn gold in autumn, broadleaf trees turn amber and red, winter mutes deciduous crowns toward bare branches, and evergreens stay dark. Colors blend over a season's first day. Foliage geometry does not change with the season.

Street wiring checkpoint (2026-10-15): `overhead-lines.ts` strings utility poles and wires along open, connected local streets and lanes, and along arterials outside 220 m of the station, where cables are underground. Master-plan roads split at every junction, so collinear segments are first chained into whole streets and poles are spaced about every 30 m along each chain, on one seeded side. Local streets and lanes are shared surfaces whose facades meet the carriageway edge: their poles stand 0.35 m inside it and every conductor hangs on the street side, clear of balconies. Arterial poles stand on the sidewalk. Poles keep clear of lamps, street trees, every leg of building entry paths, bus stops, open-space lots, and cross streets including ones still being built. Rail and expressway keep-outs follow the engine-owned curves (rail alignment samples and expressway deck Béziers, sampled every 4 m), and stations use the same clearance as foliage (`overlapsStation`) for every engine platform. Poles under an expressway deck are cut to 2.5 m below its own deck height (at most 8.5 m). Each pole carries two high-voltage conductors, a low-voltage line and a telecom bundle with parabolic sag; about a quarter carry a transformer. Crossing spans join street ends over the junction but may not cut across a block, and no span passes under the viaduct, a bridge or a platform. Poles and fittings are instanced (at most 1,200). Wires are camera-facing ribbons batched in 420 m cells that stay at least one pixel wide, fade as they get thinner and fade out between 330 and 450 m; neither poles nor wires light up at night. The rail catenary stays with the rail corridor (`rail-corridor.ts`). The old pole pair on large corner lots is removed. Seed-87231 review city: 758 poles; on hardware WebGL the layer adds 9–13 draws, about 0.2 M triangles and 0–0.2 ms per frame, and rebuilds in about 23 ms ([benchmark](../validation/overhead-lines-benchmark.md)). Open: house service drops, WebGPU and mobile frame cost, a hardware shimmer review with the post pipeline, and the rebuild hitch on a growing city.

Building surfaces checkpoint (2026-10-14): `building-surfaces.ts` gives walls a Tokyo cladding and every window a room. Each building's walls take a seeded finish by use and height: lap cement siding or plaster on detached homes, facing tile on most apartment blocks, and tile, ALC panel or formed concrete on offices; tile-faced apartment towers and concrete office towers follow the same rule. Cladding is a shader pattern along the facade (siding laps and aligned panel joints, running-bond tile with per-tile tone, sealed ALC joints, form-panel seams with tie holes), with no texture memory; its lines and per-element variation fade to their mean before a repeat spans fewer than about four pixels, and Anime shows it at 55% contrast so the painted surface leads. Window panes use interior mapping: a box room behind the glass, wider than the pane and set below the sill, with tatami or wood floors and furniture at home, carpet and partitions at work. Homes hang sheer, drawn or closed curtains; offices and shops lower blinds. Each room's light switches on when its schedule's occupancy (homes after dusk, offices through the working day, shops in trading hours) passes the room's own threshold, so windows light one by one; vacant and unfinished buildings keep bare, dark rooms. Lamps are warm or cool white at home and cool in offices; the evening home peak is about half the rooms. Panes far enough away collapse to their room's average so parallax cannot sparkle, and a Fresnel sky reflection sits over the room. All clad walls share one batch per neighbourhood (the kind is a vertex attribute), and all panes share one batch (the schedule is packed with the occupancy flag). The change also replaces the fixed lit-window choice and the towers' glowing overlay boxes, and fixes side-elevation windows whose frame boxes had hidden their glass. On hardware WebGL the frame time matches `main` with up to eight fewer draws ([benchmark](../validation/building-surfaces-benchmark.md)); matched before/after captures are in `validation/captures/building-surfaces/`. Open: weathering and grime, WebGPU and mobile cost, and a hardware review of the cladding fade in motion.

Signage checkpoint (2026-10-14): `signage.ts` adds one generated atlas of invented tenant names and billboards; no real brands are used. Open offices of 3–15 floors and shop-houses carry blade signs; mid-rise offices carry up to four stacked tenant panels, mounted on the set-back shaft. Commercial buildings of 4–11 floors may carry a lit rooftop billboard frame. The chance is 15% generally, 40% in the station fringe and 70% in the station core. Up to six station-area towers and offices within 220 m, at least 30 m apart, carry a podium video wall or printed wall ad, alternating by rank. Video walls are generated from city time. Storefront signs vary across seven shop labels and three cafe labels. Signage is built into each building's geometry, so it batches with the building and follows business turnover and renovation. Night review and art acceptance remain open.

Low-rise checkpoint (2026-10-10): `lowrise-massing.ts` defines seeded frontage and rear service masses for supported non-anchor one-to-three-floor homes, shops and cafes. The kit uses parcel-aware roofs, centered street entries and separated facade depth layers in both styles, with construction and renovation support. Root passes 10 focused tests covering layout, actual transformed parcel containment, entry/depth relationships and unsupported detached roof trim, plus the production build. Front Anime/Miniature captures show the repaired facade layers without earlier striping; the rear Miniature capture confirms removal of a floating gable trim strip. See `validation/captures/lowrise-miniature-rear-repaired.png`. Materials still look plain and style distinction is weak in this controlled scene. Neighborhood/overhead integration, construction visual review and artistic acceptance remain open; geometry checks do not establish beauty or density.

Apartment slab candidate (2026-10-15): proportion-qualified non-terrace apartments of4–6floors now have a continuous dwelling wing beside a full-height service/core bay, with continuous front balconies, occupied windows and shared roof services. Applicability requires depth>=8m and at least6m dwelling width after a1.8–2.4m core; actual scope is43–46 mature buildings in each of the three saved worlds. Authoritative use, floors, entries and parcel bounds remain unchanged. Targeted layout/lifecycle tests pass. Six-world geometry audit finds no overflows or merge errors, unchanged material-mesh counts, and2.8–3.6% fewer triangles across the audited4–11floor apartment set. Local matched views show a limited improvement in roofline/frontage coherence; the80-image three-seed/stage capture completed with matching controls and selected-pair visual review. The bounded production growth/load check completed with all sampled actors rendered, no recorded errors, and eight day/night previews inspected. It exposed loading stalls (candidate max457ms, baseline375ms); construction/renewal transitions and full performance acceptance remain unverified. This is provisional, not complete neighborhood or performance acceptance. See `validation/street-building-material-study.md` for evidence and limits.

Implementation in progress (2026-10-10): `building-massing.ts` now defines pure seeded layouts for non-anchor 4–11-floor apartments and offices. Apartments combine a full-height circulation core with a two-floor setback; offices combine a podium, offset shaft, and inset crown. Root verification covers 1,024 combinations of kind, floor count, seed, and envelope for supported tiers, envelope containment, and unchanged maximum floor count. The shared building kit now consumes these layouts for open, construction, and renovation stages. Root verified 11 focused geometry/layout/tower tests and the production build. Construction tiers no longer acquire floating scaffold before their supporting floors; renovation scaffold is restricted to ground-level masses. Browser specimen review confirms distinct silhouettes in Anime and Miniature, but narrow-parcel equipment and entry/window conflicts have since been corrected and verified across 162 narrow builds. Balcony depth adapts to available clearance. The full 145-test suite passes; mature-neighborhood review and artistic acceptance remain open. The architecture review office is now eight floors so it exercises this family, while the separate 28-floor tower remains. Matched pre-integration capture: `validation/captures/midrise-eight-floor-before.png`.

Walk-up facade checkpoint (2026-10-12): the exposed circulation-core flank now
carries narrow service windows; the attached apartment face stays unperforated.
Existing materials and common geometry preserve style switching. See
[geometry and visual evidence](../validation/core-facade.md); broader architectural
fidelity remains open.

Tall-home frontage checkpoint (2026-10-13): generic homes of four or more
floors now receive the shared centered entrance and a reserved ground-window
opening. Focused geometry regressions cover open and renovating presentation in
both styles; browser appearance remains under review. Authoritative use and access
are unchanged. See the current fidelity review for scope and limitations.

Engineered-bank checkpoint (2026-10-12): continuous wall, coping and promenade
sweeps follow the shared X-offset river boundary. This replaces repeated polygonal
shoulders and overlapping segment boxes; existing railings remain. Matched static
views support silhouette correction, while closer bends, motion and fallback
remain under review. See [bank evidence](../validation/riverbank.md).

Fine-detail checkpoint (2026-10-11): independent advected 2D noise slopes replace
coherent sine normals that produced river-wide bands. Screen-footprint attenuation,
roughness transfer and simulated rain remain. Matched dusk views and fallback
smoke support this narrow correction; full motion, rain, nighttime and measured
shader cost remain open. See [validation](../validation/water-fine-normals.md).

Night readability checkpoint (2026-10-11): shared hemisphere fill now blends
from 1.05 at night to the existing 0.5 in daylight, with neutral nighttime ground
bounce. Matched WebGPU comparisons in both styles improve facade/road legibility;
the daytime control retains its prior appearance. See
[comparison evidence](../validation/screenshots/night-fill/README.md). This adds
no local lights or passes and does not satisfy the full visual release gate.

Ceramic-roof checkpoint (2026-10-11): low-rise and anchor gables share bounded
rounded tile rolls and course lips, merged into existing materials. Controlled
views and 27 architecture tests support the local change; full-city motion,
loading cost and night fidelity remain acceptance work. See
[roof validation](../validation/ceramic-roof.md).

Apartment family candidate (2026-10-12): a seeded minority of4–11-floor apartments
now uses a broad front terrace and solid front balcony bands, alongside existing
side-core walkups. Shared massing carries the same authoritative floors and parcel
through construction and renovation. Focused geometry checks and one matched
neighborhood comparison support further review; multi-seed motion/lighting checks
and artistic acceptance remain open. See `validation/archive/2026-10/fidelity-review-current.md`
and `validation/station-architecture-candidate.json` for evidence and limitations.

### Commuter rail fidelity checkpoint — 2026-10-11

The first rail family uses a compact fictional three-car stainless commuter kit,
15 m coaches with rounded shoulders, paired doors, green/orange bands, cab lights,
roof equipment and bogies. Geometry is shared through bounded instances (eight
trainsets maximum). A shared rail profile aligns wheels with the 1.067 m inside
rail gauge, coach floor with the engine's 8 m platform surface, and platform edge
0.1 m from the main body. This is an authored approximation, not an exact train model.

Separate streetcar, ground-level commuter and elevated infrastructure kits remain
required. Prioritize coherent catenary, ballast/track slab, corridor edges and platform
detail, then coach articulation and door/dwell behavior. Do not imply that selecting
a different shader supplies these missing geometry or behavior features.
See `validation/commuter-rail.md` for verification and remaining fidelity limits.


The elevated family now includes a bounded straight-corridor catenary and service
edge kit, with a shared pantograph contact height and clearance reservations from
both authoritative terminal frames. The contact wire is straight; curved-track
support, variable heights and complete fencing remain outstanding. See
`validation/rail-corridor.md` for cost and limited browser acceptance.


Terminal reversal now keeps the physical consist oriented along its current
straight route and switches head/tail lamps between cabs. Travel-heading changes
do not rotate the entire vehicle through the platform. General curved-route
articulation remains required; see `validation/train-reversal.md`.

Platform-side doors now use real sidewall openings and shared sliding leaves, driven
by authoritative terminal and opening-fraction fields. The opposite side remains
closed; movement clears stale open poses. This works in both material styles.
Overhead canopy occlusion and snapshot interpolation still limit visual acceptance;
see `validation/train-doors.md`. Individual passengers do not yet walk through
specific door apertures.

Shadow binding checkpoint (2026-10-11): shared storefront atlas node materials
explicitly use the default TSL materialColor expression so unchanged receivers
refresh bindings after shadow texture resizing. Keep the focused observer test
and actual WebGPU style/quality transition checks when upgrading Three.js.
Do not globally dispose city materials to repair binding lifetime: the tested
workaround introduced a roughly 4.8-second hitch. See
[ground/shadow evidence](../validation/ground-surface-artifacts.md#shadow-binding-diagnosis-and-narrow-material-fix--2026-10-11).

Shop-sign batching checkpoint (2026-10-12): ordinary shop/cafe/anchor signs share
one opaque512×528 padded atlas with cell UVs. Source groups own geometry;
ArtMaterials owns the shared texture/material. Focused checks verify in-bounds
cells, merged label UVs and disposal. Small-city style captures and limitations
are recorded in [the atlas report](../validation/shop-sign-atlas.md); mature
GPU savings and full visual acceptance are not established by these tests.

Tower construction checkpoint (2026-10-12): apartments/offices of twelve or more
floors share their finished podium/shaft/crown mass definitions with staged
construction and demolition. Normal-import structure-stage captures in both
styles verify recessed frame geometry; facade completion and recorded motion
remain unaccepted. See [renewal evidence](../validation/height-redevelopment.md).

Construction batching candidate (2026-10-12): non-open buildings use isolated
batches while completed buildings retain spatial merging. Both share the authored
geometry, material and quota paths. Cross-pool transitions must preserve exactly
one visible building and all static neighbors. See the height-redevelopment
validation report; browser motion, first-renewal handoff latency and draw-call
cost remain acceptance work. This does not change the art or geometry requirements.

Reopening handoff candidate (2026-10-12): final open geometry first publishes in
its isolated construction batch; a budgeted background source then returns it to
the static neighborhood batch. Retire isolated geometry only after the exact
replacement is committed. The mature synthetic audit improves first visibility
from 463 to 1 update, while background work remains bounded and drains. Browser
motion acceptance is still required; see validation/height-redevelopment.md.

Committed-source retirement candidate (2026-10-12): construction entry may degenerate only the retiring building's indexed triangles and contour segments after its isolated replacement is ready. Preflight uses existing per-update quotas; oversized sources retain normal rebuild behavior. Picking and committed membership must reflect the removed source; owned resources retire with the batch. Synthetic improvement is documented in validation/height-redevelopment.md; browser/backend acceptance remains pending.

Office silhouette candidate (2026-10-12): a seeded subset of eligible midrise
offices keeps a straight lower body with one inset top floor; the remaining
offices retain podium/shaft/crown silhouettes. Same authoritative lot/floors
and shared style geometry. See validation/office-slab-variation.md for limited
visual and geometry evidence and remaining cost/motion checks.

### Street-light instance lifetime checkpoint — 2026-10-12

Road preparation produces detached lamp transforms and source records. Atomic road
publication copies them into four persistent instanced meshes (capacity1000), so
normal growth preserves shader-node cache identities. Reset clears active counts;
disposal releases owned geometry/materials under existing compilation safeguards.
This preserves lamp and glow detail. Focused publication/lifetime tests pass;
full both-style performance and visual acceptance remain outstanding. Evidence:
`validation/backend-growth-attribution.md`.


Persistent-lighting verification (2026-10-12): one matched 4,725-frame Miniature
growth interval recorded one new shader-state build, versus repeated builds in the
prior diagnostic. Render CPU maximum was 43.3 ms; complete animation CPU maximum
49.2 ms. This supports retaining persistent instance identities, but does not
establish GPU, both-style, long-run or smooth-growth acceptance. See
[measurement scope](../validation/backend-growth-attribution.md#persistent-lighting-matched-growth-trace--2026-10-12).

### Illuminated glazing candidate — 2026-10-13

Illuminated panes now have an explicit material class, separate from lamps/signs.
Daylight uses blue-gray glazing plus the existing analytic sky response; night
and activity uniforms reveal warm interiors. Source kits and neighborhood batches
share the response, and unfinished low-rise windows use unlit glass. No new texture
assets or geometry are introduced. Focused regressions/build and bounded browser
checks support the candidate; full daily-cycle, motion and artistic acceptance
remain open. See validation/archive/2026-10/fidelity-review-current.md for evidence and limits.

### Rail access boundary candidate — 2026-10-13

The gateway staircase uses the inward platform end so the full run remains inside
the finite world. Engine rail access paths share those stair endpoints and approach
around the lower tread, rather than crossing beneath it. Renderer treads support
either signed run. Retain finite bounds, body clearance, queue support and usable
ground connections through rotation and construction. Focused mesh/native tests
and fresh WASM replay support the candidate; normal browser motion and multi-seed
infrastructure acceptance remain open. See `validation/archive/2026-10/fidelity-review-current.md`.

### Station construction phase candidate — 2026-10-13

Engine platform views supply one construction phase shared by geometry, batch
identity and presentation revision: site below48%, supports48–73%, deck/access
73–85%, canopy85–100%, complete at100%. The85% threshold permits the normal
8-work-step station to show its canopy at87.5% before completion; a92% threshold
skipped that phase. Finished lamps, furnishings and signs appear only at completion.
Visible movement requires the completed platform identity to be committed. Preserve
service readiness as an independent simulation condition. No save migration is
introduced: the phase is derived from the current station project.

Focused geometry and scheduler tests, ordinary native/WASM growth and current-save
replay support the candidate. Browser support/canopy snapshots are limited static
evidence; continuous construction transitions and full release acceptance remain
open. See validation/station-construction-current.json and the current fidelity log.


Station foliage clearance candidate (2026-10-13): reserve the platform/canopy and
both signed stair footprints in tree clearance, expanded by the existing tree-canopy
margin. Include station geometry in clearance cache identity, excluding phase-only
changes. Ordinary growth must not erect platform meshes through retained foliage.
Focused tests and a matched seed87231 canopy-stage view support the correction;
post-fix live-motion and broader visual acceptance remain open.


Station approach candidate (2026-10-13): the engine platform contract now carries
local ground_path and ground_width. Both pedestrian approach waypoints and renderer
paving derive from these points. The1.6m strip uses a7.5m side offset to clear piers,
with a joined outline and ground-level stair landing. Paving appears with phase2
(deck/access); earlier phases retain site/support geometry only. Movement identity
and tree-clearance cache include the approach contract. This is a bounded candidate:
full road-endpoint support, mature passenger traversal, exact corner/land ownership
audits and wider visual acceptance remain required. No save migration is provided.

### Station road-end support — 2026-10-13

Ground streets terminating at a station node include a0.4m end apron so pedestrians
can turn at the authoritative node with their full body width on paving. Ordinary
ends and boundary gateways keep their prior extent. The production footprint and
road mesh constructor are shared with support audits; the apron remains inside
the existing road-surface partition to avoid coplanar overlaps. This changes
rendered support, not route lengths or simulation timing. See
`validation/station-road-landing-height.json` and the current fidelity log.

Riverbank planting candidate (2026-10-13): small trees are seeded inside the existing
unpaved bank strip, with crown bounds checked against water and the promenade
railing. Existing development and transport clearance remains authoritative for
visibility; no road width or routing changes. Three-seed retention and focused
mesh-envelope tests pass; motion/construction and full visual acceptance remain
open. See [planting evidence](../validation/riverbank-planting.md).


Office side-elevation candidate (2026-10-13): side walls with sufficient parcel
clearance now carry the office ribbon-glazing treatment used at the front, making
use visible from more neighborhood viewpoints. Both-style held stills and focused
geometry/cladding tests support further review; motion and developing-state visual
acceptance remain open. The three-seed mesh audit also records pre-existing narrow
parcel overhangs; no full containment claim. See [candidate evidence](../validation/office-side-facades.md).

### Persistent park and utility instances — 2026-10-15

Park input changes retain compatible tree-cell and utility-pole instance objects/buffers. Matrices, colors, visible counts and bounds update in place; capacities grow geometrically, absent batches retire, and adopted pole geometry is not disposed with the old group. Shared tree geometry remains terrain-owned. This preserves placement and detail while avoiding repeated WebGPU node-program construction. A bounded seed5/development16 check reduced pipeline creations597→14 and sampled loading frame p9582–343ms→57–76ms; full render-call maximum remains154ms and world updates48ms. These single-run diagnostics do not establish release performance or resource stability.36 targeted tests and TypeScript pass; both styles visually screened after growth. Evidence and limits: `validation/street-building-material-study.md`.


Daylight key candidate (2026-10-15, provisional): the direct key receives an additional1.19×daylight×(1−dusk)×(1−clamp(rain,0,1)) intensity and blends toward#fff1db by that factor. Ambient fill, environment, AO and exposure remain unchanged. Selected matched13:00 views show improved sunlit facade/roof readability; selected settled dawn/dusk/night pairs now preserve the prior appearance; the three-seed/stage matrix and static rain checks now pass control verification; continuous temporal acceptance remains unverified. See `validation/street-building-material-study.md`, daylight key local result. This is not acceptance of the lighting or neighborhood milestone.

## Optional tilted focus — candidate, 2026-10-15

The camera can intersect each visible view ray with a tilted focus plane, then feed the resulting focus distance to the signed thin-lens CoC gather. This preserves depth-dependent focus across roofs and streets rather than applying a flat image stripe. It is an artistic optics model with a shared scene-distance scale of `min(0.01, 1.2 / dampedFocusDepth)` while tilt-shift is enabled. This preserves the close-view treatment and holds the modeled subject at 1.2 m for distant overviews; ordinary DOF retains its style-specific scales. The tilted effect uses the same controls in both styles.

Desktop Details and mobile Visual settings expose focus position, band angle, band width (aperture multiplier), plane tilt and maximum blur radius. Enabling tilt-shift enables DOF. Disabling it restores ordinary DOF; the explicit Camera blur master toggle bypasses blur entirely without discarding settings. Slider edits while blur is off preserve the unblurred view. Defaults are off and these new settings currently last for the app session. The plane update writes a reusable vector only when enabled, without frame-loop allocation.

The second blur gather now averages its center and sixteen samples instead of taking per-channel maxima. Matched local day/night comparisons show less highlight expansion; defocused night lights are consequently dimmer. This is provisionally retained, with gather-specific broad and temporal validation outstanding. It does not change the signed near/far coverage or establish global energy conservation.

Acceptance requires a visible miniature effect, readable selected subject, clean silhouette transitions, stable camera motion, usable controls and bounded browser cost. The first static Miniature witness is promising; the initial Anime response was too weak and now uses the shared tilted optical scale. Full visual acceptance is pending. Current runs and evidence belong in `validation/archive/2026-10/HANDOFF.md` and `validation/street-building-material-study.md`.


Horizontal pigment candidate (2026-10-15): Anime horizontal matte surfaces use restrained neutral, derivative-filtered grain instead of the wall pigment wash; vertical treatment and Miniature are unchanged. A matched developing-neighborhood screen shows locally cleaner roof fields. TypeScript and 12 material tests pass; All 60 broad captures across three seeds and two stages pass scene-control checks, with selected matched pairs retaining the local gain. Sixteen matched renderer-weather captures also pass control checks; selected rainy-dusk/night views remain readable. A four-run camera-motion check also passes finite optics and matched-start checks; rAF p95 is 33.4–34.5 ms with a maximum of 158.3 ms. This is not GPU timing or proof of smooth continuous motion. Full visual, temporal and production-performance acceptance, including simulation-weather behavior, remain open. See the horizontal painted surface section in `validation/archive/2026-10/HANDOFF.md`. This is provisional, not milestone completion.

Low-rise rear elevation candidate (2026-10-16): shared rear room openings above attached service roofs, shallow window hoods/sills and roof-connected drainage complete the existing home/shop-house kits. Upper shop-house rooms use the same residential lighting schedule as their front and side rooms. Existing masses, floors, primary entrances and roof shapes remain unchanged. Selected matched blur-off views show more inhabited rear elevations; all60 broad captures across three seeds and two stages pass source and scene-control checks, with selected pairs visually inspected. Affected mature-seed5 kits add291,156 authored triangles (+12.0%), with unchanged authored mesh count; this is not a frame-cost measurement. Sixteen dusk/night renderer-weather captures pass source and scene-control checks; the selected rainy-night Miniature pair preserves the localized rear-room gain. Existing Anime roof/parapet shadow artifacts remain. Four camera-motion runs pass finite-optics and matched-start checks, with rAF p95 33.7–34.1 ms and a 166.6 ms maximum; these are not GPU timings or proof of smooth motion. Temporal visual acceptance, simulation-weather behavior, production performance and full visual acceptance remain open. See `validation/archive/2026-10/HANDOFF.md` for the current run.


Canopy interior candidate (2026-10-16, provisional): five compact leaf masses give the shared foliage a readable interior beneath the existing cutout-card silhouette. The shared opacity node distinguishes opaque core coverage from textured leaves; tree placement, extent and clearance remain unchanged. Geometry grows by240 triangles per crown (near624, far408 total); it does not add instances, materials or draw calls. Six targeted tests and TypeScript pass. Selected matched waterfront pairs in both styles and developing Anime overview show denser leafy crowns; rounded shadows are heavier. Broad coverage, motion/LOD stability, day/night/weather and production cost remain unaccepted. Local evidence ledger: ignored `canopy-volume-visual-review.json`; run state in `validation/archive/2026-10/HANDOFF.md`.

### Automatic Watch subject variety (candidate, 2026-10-16)

Watch must not cycle indefinitely between the most populated neighborhoods. Retain representatives of available subject families within the32-anchor budget; activity contributes bounded interest rather than unbounded population weight. A tour visits each retained subject before repeating, with unseen subject families presented early. Preserve existing subject framing, safe path planning, manual takeover and world-replacement resets. New/removed subjects are reconciled with the bounded visit set on snapshots. Selection-only fixture audits and camera tests validate fairness; actual composition, transitions and both-style visual acceptance remain separate gates. Current implementation is under browser review and does not establish full camera milestone acceptance.

Founder framing candidate (2026-10-16): use parcel-scale sightline proxies separately from the expanded camera-travel envelopes. A storefront visibility probe must not be rejected solely because it starts inside a neighbor's camera safety apron. Blocked founder views search bounded entrance-facing positions; already-clear original compositions remain, and nearby structural changes trigger reframing. Travel and final-position collision checks retain their existing expansion. Parcel sight proxies are approximate and may omit small projecting props; they are not exact rendered geometry. The mature seed0 café is readable in both styles with blur off, with a fixed-camera control confirming a framing-only gain. Broader seed/stage and motion acceptance remain pending; this does not complete the camera/rendering milestone.
