# Rendering and Artistic Styles

Render one shared three-dimensional world with selectable material and shader treatments. Last Train, First Light is the initial implementation. Little Tokyo follows using the same geometry and state.

## Foundation

Flat-roof service candidate (2026-10-12): tanks have physical supports; wider roofs
receive bounded HVAC and a human-scale access headhouse using existing materials.
An Anime still study supports improved silhouettes, while final seeded variation,
both-style motion and night acceptance remain open. See
[roof service evidence](../validation/roof-services.md).

Daylight fill candidate (2026-10-12): matched, fixed-camera DOF-off comparisons in
both styles support increasing hemisphere fill from0.5 to1.0 at full daylight.
The visible sky does not illuminate the scene. Night endpoint remains1.05;
intermediate dawn/dusk and motion acceptance remain open. See
[daylight evidence](../validation/daylight-fill.md). This is a readability
improvement, not acceptance of overall lighting or concept fidelity.


Vehicle checkpoint (2026-10-12): road traffic and the city bus use bodies traced from generated side elevations (`scripts/build-vehicle-kits.py` → `public/art/vehicles`, `web/src/render/vehicle-manifest.json`). Eleven bodies share one material, a 2048×1024 atlas and a 1024×512 paint/lamp/glass mask (13.3 MiB decoded with mips) plus a 512×1024 canvas livery sheet. Each engine length class has weighted body variants (compact: hatch, crossover, minivan; kei: tall wagon, hatch, pickup; taxi: modern, classic sedan), and each vehicle gets a stable weighted paint, a few percent of shade and width variation, and one of thirteen invented operator liveries where it is a fleet vehicle. Bodies narrow toward each end in plan; fronts take corner lamps and a flat grille below the belt line. Wheels roll and steer in the vertex stage; lamps light at night with brake response. Traffic drives on the left: the engine's lane and kerb offsets (`KERB_LANE`, `BUS_STOP_KERB`) are negative along (-dz, dx), which is the driver's right in the renderer's right-handed y-up frame; bus doorways and the kerb-side elevation are on local +x. The engine reports the body class as the movement kind (1 compact, 4 kei, 5 taxi, 6 van, 7 truck, 8 mail van) and queues road vehicles at half their combined lengths plus 0.5 m, never below 4.5 m. Lane queues run every tick; a car enters a lane only a queue gap behind the last car in; driveways queue at both ends of the access path, and a car still waiting at its door is not emitted. Seed-87231 save over 2,000 ticks: 109 sub-2 m road-vehicle pair samples (31 under 1 m, all momentary driveway crossings), longest stationary vehicle 120 ticks. Hardware WebGL (ANGLE/OpenGL ES, RTX PRO 6000; `scripts/benchmark-vehicles.mjs`): 1,000 moving vehicles add about 0.8 ms per frame including updates and shadows, 16 draws and 0.36 M triangles. Open: WebGPU frame time (headless Chromium exposes no adapter here) and fascias, which remain approximations from the side elevation.

A four-SpotLight nearby-fixture prototype now follows committed ordinary-road
lamp heads and fades selection changes. It has a fixed light/source budget;
occlusion, actor crossings, mature-city cost and fallback acceptance remain
open. See [prototype evidence](../validation/nearby-street-lights.md). It is not
a replacement for the complete night-scene acceptance gate.

Ground streetlight coverage correction (2026-10-12): shared fixtures now use
actual road widths, terrain-level local bases and sidewalk-level arterial bases.
Legacy duplicate ordinary-road fixtures were removed. An enlarged soft-pool
experiment was rejected after close playback review; restrained existing pools
remain an approximation, not dynamic actor lighting. See
[coverage validation](../validation/street-light-coverage.md); elevated-road
illumination and full day-cycle/motion acceptance remain open.

Flat roofs should distinguish waterproof roof fields from parapet coping and
service equipment. The current candidate restores this separation without new
textures; [validation](../validation/roof-surface.md) records the limited daytime
evidence and outstanding matched-camera/night review.

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
see [current handoff](../validation/HANDOFF.md) and
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


Style-switch checkpoint (2026-10-10): warmup submits one material/layout representative at a time and yields a browser task between samples. Superseded requests stop before the next sample; the live material swap occurs after successful preparation. Founder WebGL observations reduce the largest uninterrupted submission from a multi-second all-layout call to under 100 ms per sample, but total warmup remains about ten seconds and mature/WebGPU/performance acceptance is open. See the audit for measurements and caveats.

Street-lighting checkpoint (2026-10-10): ordinary open connected streets share a deterministic instanced lamp kit across styles, with at most 1,000 placements. Warm lamp emission and soft additive pavement pools follow the common night uniform. These pools are an approximation, not dynamic lights on pedestrians or vehicles. Bridge lighting and full night-scene visual acceptance remain open; see the latest validation audit.

Geometry streaming coalesces spatial batch rebuilds while arrivals are pending. Existing batches stay visible; busy chunks become eligible after 12 render updates. Preparation and merging now advance under vertex/index quotas with one active chunk, then atomically replace the completed batch. Preserve authored triangles, materials, and picking spans. Completed source meshes are released; immutable building descriptors regenerate them when their chunk changes, with at most one regeneration factory call per update. Owned sign materials and textures transfer to the batch and retire only after outstanding style compilation releases its references. Source contour copies remain resident. Renderer diagnostics expose remaining building and batch queues for visual verification.

Memory/performance checkpoint: the 850-building fixture retains 2,135,638 triangles with 219,495,028 bytes of counted CPU geometry arrays after source release (150,410,212 fewer source-array bytes). These are Node structural/array measurements, not browser heap or GPU memory. A single regenerated building can still exceed a frame budget: an earlier descriptor benchmark recorded replacement-drain p95 31.3 ms per update; a later root rerun recorded p95 3.092 ms and max 4.493 ms. Host load was not isolated, so this spread is variability evidence, not a measured tenfold optimization. Kit construction remains synchronous. Contour extraction now advances at a shared 2,000-work-unit quota per BuildingChunks update (bounds vertices, triangles, unmatched edges and final output floats); registration defers extraction entirely. Complete contours are cached per source and published with the complete batch. This candidate preserves exact legacy edge coordinates in focused tests and two production tall-building comparisons, but browser frame timing and publication latency remain acceptance gates. Count quotas do not prove bounded frame time; continue profiling and stage these operations without dropping focal detail. See [the geometry audit](../validation/mature-building-geometry-budget.md) and [regeneration benchmark](../validation/staged-building-chunks-benchmark.md).

Style-resource checkpoint: shader warmup now uses `compileAsync(samples, camera, liveScene)` to borrow the actual scene lights. It creates no temporary light clones or shadow targets and leaves live light ownership unchanged. Mature WebGPU Anime→Miniature→Anime→Miniature verification held at277 textures and504 geometries after warmup, with zero queued geometry and no captured console errors. This supersedes the earlier clone-and-dispose workaround. The short counter plateau does not prove the multi-hour memory gate. The warmed repeat still took about3.3 seconds of submission/await work, with a25ms largest submission; no performance improvement is claimed. Visual transitions and latency remain unfinished.

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

### Dense-city reference update

The user explicitly rejected the sparse procedural appearance. Use
`concepts/05-dense-last-train-first-light.png` and
`concepts/06-dense-little-tokyo.png` alongside the original growth boards and
the photographs under `concepts/photo-references/`. These are artistic targets,
not evidence that the browser implementation has reached them.

Mature scenes require near-continuous mixed-height urban fabric, compact plots,
recognizable tower clusters, narrow service spaces, and rail/street corridors
that organize the composition. Preserve small historic shops within that fabric.
Taller isolated boxes on lawns do not satisfy the requirement. Detail must work
at skyline, neighborhood, and street scale with depth of field disabled.

Use a hybrid production approach: author a convincing benchmark district and
reusable architectural/streetscape modules, then assemble those modules with
the simulation's constrained parcel and growth rules. Improve actual geometry,
surface textures, lighting and composition before adding stylistic filters.
Separate style treatments must operate on the same spatially convincing world.
Generated concept paintings are references, not substitutes for live scenery.

Lowrise surface checkpoint (2026-10-10): non-anchor home windows now use layered dark reveals, separated sash frames and sills, preserving footprint and shared material batching. Anime matte surfaces use world-anchored pigment with derivative-based attenuation of fine strokes; horizontal paving receives a restrained wash. Root passes 26 focused geometry/material tests and inspects both styles in forced WebGL. The controlled views still have plain facades and weak style distinction; this is not art acceptance or motion-stability proof.

### Party walls, lanes and elevated layers

Master-plan partition cells follow clipped block geometry; rectangular building parcels are fitted inside those cells. Some rejected or unusable cells remain unbuilt. Bodies run to the shared lot lines, keeping a fixed side margin for eaves, sills and foundation lips: 0.9 m for low-rise homes and shops, and 1.3 m for apartments and offices. Neighbors therefore stand about 0.9–1.3 m apart, near Tokyo's 0.5 m-per-side norm, and form a near-continuous street wall. Towers and anchors keep their proportional plaza inset; depth keeps a front setback and rear yard. Realized building sites render their retained partition cells as urban ground. Merely releasing a parcel does not pave its block; unbuilt cells retain natural ground. Vacancy and renewal retain the realized ground. Local streets render 6.5 m wide with painted edge lines and no raised sidewalk or center line. Lanes are 4 m. Only arterials have sidewalks and center markings. Party-wall sides should move to quiet, windowless material (not yet implemented). Elevated rail runs on a viaduct roadbed centered 6.4 m above the ground, with piers at least every 24 m, and trains follow the deck height. Expressway deck elevations come from authoritative `road_decks`: the east–west route is at 11 m and the north–south route at 20 m, on single-column piers with sound barriers. Upper piers skip lower transverse road/rail clearance envelopes. Neither elevated layer occludes construction picking or street lighting below.

At 3,000 buildings, distant low-rise fabric must use simplified merged geometry: body box, roof color and window band. Full kits are reserved for focal and near chunks, so the triangle budget stays within the existing performance policy.

Expressway furniture checkpoint (2026-10-11): completed elevated roads now use
shared-style lane/shoulder markings, a median and deck-mounted lamps rather than
the ground-road fallback. The former flat crossing now uses independent 11 m and 20 m corridors
with plan-derived elevations and separate graph nodes. This is an overpass,
not an interchange or a traffic implementation. See [the fidelity workstream](../validation/living-city-fidelity.md).

### Compact architectural families — next implementation

The 799-building overview exposes a massing problem beyond density: most non-tower buildings share a rectangular shell, many typology differences are applied only to facades, and apartment setbacks usually affect just one top floor. Color variation alone cannot resolve this repetition.

Introduce a deterministic family selector using existing building kind, floor count, parcel proportions, and appearance seed. Narrow low-rise homes and shops should support rowhouse/machiya forms with quieter party-wall sides, rear service volumes, and varied gable/parapet profiles. Midrise apartments and offices should have distinct massing: walk-up circulation cores, recessed balconies or loggias, and multi-floor setbacks where appropriate. Retain authored storefront detail and the established anchor identity. Keep geometry within the existing parcel envelope, preserve authoritative floors and access, and apply the same family in both styles and construction stages. Put family/layout decisions in a separate pure module rather than expanding the existing shell branch indefinitely.

Validate silhouettes in neighborhood and overhead views with depth of field off, plus street-scale entry/access readability. Both low-rise and midrise families are implemented in the shared kit, but neither has passed the complete city-context artistic gate.

Finishing-stage checkpoint (2026-10-10): supported low-rise buildings use their finished seeded facade and roof from 86% construction progress, retaining worksite barriers and suppressing business/window emission until opening. The partial shell is replaced, not layered underneath. Root verifies 11 focused tests and 99% Anime/Miniature browser views; this remains a discrete finishing phase, not continuous component-by-component assembly.

Low-rise checkpoint (2026-10-10): `lowrise-massing.ts` defines seeded frontage and rear service masses for supported non-anchor one-to-three-floor homes, shops and cafes. The kit uses parcel-aware roofs, centered street entries and separated facade depth layers in both styles, with construction and renovation support. Root passes 10 focused tests covering layout, actual transformed parcel containment, entry/depth relationships and unsupported detached roof trim, plus the production build. Front Anime/Miniature captures show the repaired facade layers without earlier striping; the rear Miniature capture confirms removal of a floating gable trim strip. See `validation/captures/lowrise-miniature-rear-repaired.png`. Materials still look plain and style distinction is weak in this controlled scene. Neighborhood/overhead integration, construction visual review and artistic acceptance remain open; geometry checks do not establish beauty or density.

Implementation in progress (2026-10-10): `building-massing.ts` now defines pure seeded layouts for non-anchor 4–11-floor apartments and offices. Apartments combine a full-height circulation core with a two-floor setback; offices combine a podium, offset shaft, and inset crown. Root verification covers 1,024 combinations of kind, floor count, seed, and envelope for supported tiers, envelope containment, and unchanged maximum floor count. The shared building kit now consumes these layouts for open, construction, and renovation stages. Root verified 11 focused geometry/layout/tower tests and the production build. Construction tiers no longer acquire floating scaffold before their supporting floors; renovation scaffold is restricted to ground-level masses. Browser specimen review confirms distinct silhouettes in Anime and Miniature, but narrow-parcel equipment and entry/window conflicts have since been corrected and verified across 162 narrow builds. Balcony depth adapts to available clearance. The full 145-test suite passes; mature-neighborhood review and artistic acceptance remain open. The architecture review office is now eight floors so it exercises this family, while the separate 28-floor tower remains. Matched pre-integration capture: `validation/captures/midrise-eight-floor-before.png`.

### PLATEAU morphology benchmark

Build a separate synthetic benchmark district from calibrated ordinary street fabric, compact service spaces and station clusters, using the existing procedural kits first. Compare identical overhead, neighborhood and street cameras with depth of field disabled, in both styles at noon, dusk and night. Measure rendered footprint union/frontage and inspect repeated silhouettes, gaps and unexplained lawn; preserve parks, banks, access and breathing space where justified. Source roof geometry may inform silhouette families, but photographic textures are not the default. Art acceptance still requires concept-board review with water, anti-aliasing, lighting and purposeful activity intact. Reference extraction and benchmark thresholds remain proposed work in the [PLATEAU pilot](../validation/plateau-research.md).

### Offline asset workshop

Apply the reference-conditioned, global-to-local workflow discussed with the
user as an offline art-production process. Start with a station-front street
composition, extract reusable architectural families, and inspect their actual
3D geometry from front, rear, elevated, and street-level views before expanding
the library. The development architecture review scene is a diagnostic tool;
it does not replace a district produced by the authoritative simulation.

Every imported or authored family must declare physical scale, ground pivot,
parcel envelope, actual occupied footprint, entrance anchors, material roles,
construction representation, and detail levels. Preserve window emission and
glass as separate material roles so lighting and style switching remain live.
Avoid baked sunsets, shadows, or reflections in base-color textures. Generated
meshes require inspection for distorted rear elevations, fused objects, floating
parts, inconsistent scale, and excessive geometry before runtime use.

Review one coherent set of a narrow shop, walk-up apartment, office, and station
tower together under both styles. Match their facade rhythm, scale, surface
detail and palette without making them identical. Then validate their placement,
entrances, construction and replacement in a real seeded neighborhood. Retain
provenance in the asset manifest. A concept image or offline beauty render does
not prove runtime material, performance, or artistic acceptance.

### Small generated texture library

Use image generation during development to accelerate reusable surface art. OpenRouter is an optional production-service adapter, not a runtime dependency; no provider credentials or generation requests belong in the browser. The application ships reviewed static assets. The user approved the Tencent Hy Image 3.5 Preview pilot: at most four 1K candidates and $2 total. The offline script `scripts/generate-texture-pilot.mjs` reads the ignored local key, pins the Tencent provider, checks published endpoint pricing, persists a request ledger before each call, and does not retry automatically. An unknown outcome or unexpected cost stops the batch. Generated candidates stay in `art-workshop/tencent-pilot/` until visual acceptance; approval of this pilot does not authorize further batches.

Pilot one convincing station-front street with at most six reusable material tiles (plaster, concrete, painted metal, roof, timber and paving) plus one storefront-decoration atlas. Prefer 512–1024 px tiles; allow a 2048 px atlas only where its screen coverage warrants it. This is a starting library, not a requirement to generate all assets before reviewing the first one. Reuse the existing ivory-plaster candidate before making redundant variants. Its current review-only status does not authorize promotion without live-city visual and resource checks.

Keep geometry, window emission, reflections and readable Japanese text procedural or authored. Generate flat unlit base colour with a declared physical tile size; check seams, perspective, repeated motifs and baked shadows. An attractive scene painting is unsuitable as a wall texture. Normal and roughness maps require separate inspection; do not assume generated maps are physically valid. Apply coherent variants to common geometry/UVs across styles, with deterministic tint variation instead of a unique image per building.

The pilot's incremental texture allocation target is 32 MiB decoded including complete mip chains and both resident style variants, separate from the application's total GPU budget. Count base colour, normal and roughness textures separately; compressed download bytes are not decoded allocation. An RGBA8 1024-square texture with full mips is about 5.33 MiB, so six such tiles plus an atlas would exceed this target. Start smaller or share maps; choose GPU compression only after both backends and fallback behavior are tested. Preserve batching through shared atlases/material classes, bounded loading and disposal. A file audit is an estimate, not measured browser GPU usage.

Accept each candidate only after comparison in a real street at dawn, noon, dusk and night in both styles, including camera motion and distant sampling. Record dimensions, bytes, provenance and acceptance in the asset manifest. Scale the library only after it visibly improves the scene within the browser budget.

Use Three.js WebGPURenderer and TSL node materials. The renderer supports WebGPU and a WebGL2 fallback, but effects must be validated separately on both backends. Feature-detect capabilities and maintain explicit quality profiles.

Create geometry in physical world units. Use modular roofs, facades, windows, shopfronts, road furniture, vegetation, and construction stages. Seed cosmetic variation from appearance IDs. Author enough geometric depth for miniature rendering; stylized materials may simplify it.

Reuse glTF assets, mesh instances, shared materials, texture atlases, and distance-based detail. Group repeated geometry by mesh, material, and spatial chunk. Visibility culling must operate at useful chunk granularity rather than treating the entire city as one giant instance batch.

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

## Style contract

A style receives surface category, base colors, normals, depth, world coordinates, lighting, wetness, and shared water state. It owns surface response, edge treatment, atmosphere, color grading, focus defaults, and required render passes.

| Treatment | Last Train, First Light | Little Tokyo |
| --- | --- | --- |
| Surface | Painted texture, restrained light bands | Rough tactile materials, fine physical detail |
| Edges | Selective thin contours | Geometry and lighting |
| Sky | Illustrated clouds and atmospheric gradients | Photographic sky and haze |
| Night | Warm emissive windows and luminous signs | Small realistic light sources and subtle bloom |
| Focus | Gentle optional cinematic focus | Stronger adjustable miniature tilt-shift |
| Water | Simplified colored reflections | More physical surface response |

Painterly and ink styles are future implementations of this contract. Brush marks attach to surfaces, outlines vary predictably with distance, and hatching follows stable orientation. Avoid frame-random noise that boils during camera movement.

## Style switching

Prepare the new pipeline without discarding the current one, precompile likely material variants, then switch at a frame boundary. Preserve camera, clock, selection, geometry, and world IDs. Reset temporal history when the pipeline changes. Dispose unused resources after the switch. Transition effects must respect reduced motion and memory ceilings; a short fade is sufficient.

## Water

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

Use finite river geometry and a flow field following its course. Layer flowing normals and gentle displacement with a bounded low-resolution ripple grid near observation areas. Rain and wakes inject impulses. When no fine grid is active, analytic waves preserve continuity. Local ripple detail is visual and does not affect city economics.

Combine shoreline coloration, foam where appropriate, sky reflection, and a reduced-resolution planar reflection for a principal water surface. Update reflections at a lower cadence or when the camera changes sufficiently. Avoid recursively rendering water in reflection passes. More complex water bodies may use a cheaper reflection approximation.

Flooding, erosion, hydraulic infrastructure, and full-domain fluid dynamics are outside initial scope. The water remains animated and locally simulated in every artistic style.

## Lighting and weather

Night readability checkpoint (2026-10-11): shared hemisphere fill now blends
from 1.05 at night to the existing 0.5 in daylight, with neutral nighttime ground
bounce. Matched WebGPU comparisons in both styles improve facade/road legibility;
the daytime control retains its prior appearance. See
[comparison evidence](../validation/screenshots/night-fill/README.md). This adds
no local lights or passes and does not satisfy the full visual release gate.

One shared sun and moon trajectory follows authoritative city time. Use a bounded shadow region around the observed area with distant lighting approximations. Window and sign emission depend on building use and activity. Avoid thousands of shadow-casting point lights; combine emissive surfaces with a small pool of nearby lights.

Rain changes wetness, ripples, pedestrian accessories, visibility, and sound. Wetness rises and dries gradually. Clouds, trees, and particles follow presentation time derived from the simulation. Lighting preview changes only lighting and labels that fact in the UI.

## Anti-aliasing

FSAA is required across styles. Start with four-sample MSAA where supported, including the actual offscreen scene target when post-processing is used. Canvas antialias settings alone do not validate the complete pipeline.

Test wires, rails, silhouettes, foliage, water highlights, and moving outlines. If temporal reconstruction is added, supply correct motion vectors and reset history on cuts, style changes, and resolution changes. Inspect ghosting around trains and pedestrians. Choose one primary AA approach per quality profile rather than blindly stacking every technique. Post-process edges may require a final edge treatment; decide by image comparisons.

## Performance policy

Cap device pixel ratio and adjust internal resolution with hysteresis. Reduce reflection resolution, shadow coverage, particles, and depth-of-field cost before sacrificing the core scene. Maintain smooth transitions between mesh detail levels.

Track draw calls, triangles, buffer sizes, texture memory, pipeline compilation, and pass timing where available. Reuse GPU resources through construction and redevelopment; the first building's removal must not leak its materials forever.

## References

- [Renderer backends and MSAA options](https://threejs.org/docs/pages/WebGPURenderer.html)
- [TSL post-processing and temporal inputs](https://threejs.org/manual/pages/webgpu-postprocessing.html)


### Expressway connection geometry (implemented 2026-10-11)

The authoritative master plan supplies cubic controls for elevated road sections.
The renderer samples those controls by arc distance and sweeps indexed deck,
median, barrier and paint geometry along the shared endpoint frames. Lights and
piers follow the same curve. Construction reveals a clipped arc length. Curves
do not change when later road sections appear. Coincident but disconnected nodes
and different deck levels remain separate.

This supersedes the temporary renderer-derived two-metre smoothing. The planner
now allows broader handles and excludes lots from a conservative sampled curve
corridor before final admission. Current scope remains constant-height routes;
ramps, vertical transitions, expressway traffic and engineered merges are still
required to approach the new photographic references. Future expressway vehicle
motion must sample these controls rather than the original straight chords.

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
and artistic acceptance remain open. See `validation/fidelity-review-current.md`
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

### Storefront notice atlas checkpoint

Existing shop glazing now includes a bounded authored 512² atlas of paper menus
and notices, shared across buildings and both styles. It adds no emission and
does not state simulated opening hours. Two triangles per notice; full street
and residency review remain required. See `validation/storefront-notices.md`.

### Paved ground and vegetation

Natural tree roots must not emerge through uninterrupted developed paving.
Clearance follows the rendered filled polygons, preserving unpaved gaps and
concave notches; canopy overhang from outside the paved area is allowed.
The current block-polygon clearance is verified on one developing checkpoint
in both styles; progressive ground realization and full motion/backend review
remain open. See [ground surface evidence](../validation/ground-surface-artifacts.md).


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

Tower finishing checkpoint: installed floors now use the shared authored open
facade rather than generic temporary panels. Roof equipment waits for full
cladding and operational emission waits for opening. Normal Anime before/after
playback confirms retained facade geometry; full motion and both-style browser
acceptance remain open. See the renewal evidence above.


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
remain open. See validation/fidelity-review-current.md for evidence and limits.


### Rail access boundary candidate — 2026-10-13

The gateway staircase uses the inward platform end so the full run remains inside
the finite world. Engine rail access paths share those stair endpoints and approach
around the lower tread, rather than crossing beneath it. Renderer treads support
either signed run. Retain finite bounds, body clearance, queue support and usable
ground connections through rotation and construction. Focused mesh/native tests
and fresh WASM replay support the candidate; normal browser motion and multi-seed
infrastructure acceptance remain open. See `validation/fidelity-review-current.md`.


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
