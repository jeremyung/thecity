# Rendering and Artistic Styles

Render one shared three-dimensional world with selectable material and shader treatments. Last Train, First Light is the initial implementation. Little Tokyo follows using the same geometry and state.

## Foundation

A four-SpotLight nearby-fixture prototype now follows committed ordinary-road
lamp heads and fades selection changes. It has a fixed light/source budget;
occlusion, actor crossings, mature-city cost and fallback acceptance remain
open. See [prototype evidence](../validation/nearby-street-lights.md). It is not
a replacement for the complete night-scene acceptance gate.

Flat roofs should distinguish waterproof roof fields from parapet coping and
service equipment. The current candidate restores this separation without new
textures; [validation](../validation/roof-surface.md) records the limited daytime
evidence and outstanding matched-camera/night review.

Door apertures are cut from the body geometry; separate leaves retain atlas UVs
and follow engine dwell phases on the platform side. Infrastructure admission
and presentation interpolation govern the visible fleet. Static vertex attributes
are packed to fit the observed eight-buffer WebGPU limit. Native, WASM and mesh
checks support the integration, and paired station captures render in both styles.
Actual platform-side boarding visibility and broader moving-scene acceptance
remain unproven. Original fleet benchmark numbers do not measure this integrated
door/interior/station workload. See
[integration evidence](../validation/pr-scale-performance-review.md).

Asset generation provenance remains in the train generator/art metadata. Other
train classes and trams remain gated on their own infrastructure and acceptance.

Geometry streaming coalesces spatial batch rebuilds while arrivals are pending. Existing batches stay visible; busy chunks become eligible after 12 render updates. Preparation and merging now advance under vertex/index quotas with one active chunk, then atomically replace the completed batch. Preserve authored triangles, materials, and picking spans. Completed source meshes are released; immutable building descriptors regenerate them when their chunk changes, with at most one regeneration factory call per update. Owned sign materials and textures transfer to the batch and retire only after outstanding style compilation releases its references. Source contour copies remain resident. Renderer diagnostics expose remaining building and batch queues for visual verification.

Memory/performance checkpoint: the 850-building fixture retains 2,135,638 triangles with 219,495,028 bytes of counted CPU geometry arrays after source release (150,410,212 fewer source-array bytes). These are Node structural/array measurements, not browser heap or GPU memory. A single regenerated building can still exceed a frame budget: an earlier descriptor benchmark recorded replacement-drain p95 31.3 ms per update; a later root rerun recorded p95 3.092 ms and max 4.493 ms. Host load was not isolated, so this spread is variability evidence, not a measured tenfold optimization. Kit construction remains synchronous. Contour extraction now advances at a shared 2,000-work-unit quota per BuildingChunks update (bounds vertices, triangles, unmatched edges and final output floats); registration defers extraction entirely. Complete contours are cached per source and published with the complete batch. This candidate preserves exact legacy edge coordinates in focused tests and two production tall-building comparisons, but browser frame timing and publication latency remain acceptance gates. Count quotas do not prove bounded frame time; continue profiling and stage these operations without dropping focal detail. See [the geometry audit](../validation/mature-building-geometry-budget.md) and [regeneration benchmark](../validation/staged-building-chunks-benchmark.md).

Style-resource checkpoint: shader warmup now uses `compileAsync(samples, camera, liveScene)` to borrow the actual scene lights. It creates no temporary light clones or shadow targets and leaves live light ownership unchanged. Mature WebGPU Anime→Miniature→Anime→Miniature verification held at277 textures and504 geometries after warmup, with zero queued geometry and no captured console errors. This supersedes the earlier clone-and-dispose workaround. The short counter plateau does not prove the multi-hour memory gate. The warmed repeat still took about3.3 seconds of submission/await work, with a25ms largest submission; no performance improvement is claimed. Visual transitions and latency remain unfinished.

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

### Party walls, lanes and elevated layers

Master-plan partition cells follow clipped block geometry; rectangular building parcels are fitted inside those cells. Some rejected or unusable cells remain unbuilt. Bodies run to the shared lot lines, keeping a fixed side margin for eaves, sills and foundation lips: 0.9 m for low-rise homes and shops, and 1.3 m for apartments and offices. Neighbors therefore stand about 0.9–1.3 m apart, near Tokyo's 0.5 m-per-side norm, and form a near-continuous street wall. Towers and anchors keep their proportional plaza inset; depth keeps a front setback and rear yard. Realized building sites render their retained partition cells as urban ground. Merely releasing a parcel does not pave its block; unbuilt cells retain natural ground. Vacancy and renewal retain the realized ground. Local streets render 6.5 m wide with painted edge lines and no raised sidewalk or center line. Lanes are 4 m. Only arterials have sidewalks and center markings. Party-wall sides should move to quiet, windowless material (not yet implemented). Elevated rail runs on a viaduct roadbed centered 6.4 m above the ground, with piers at least every 24 m, and trains follow the deck height. Expressway deck elevations come from authoritative `road_decks`: the east–west route is at 11 m and the north–south route at 20 m, on single-column piers with sound barriers. Upper piers skip lower transverse road/rail clearance envelopes. Neither elevated layer occludes construction picking or street lighting below.

At 3,000 buildings, distant low-rise fabric must use simplified merged geometry: body box, roof color and window band. Full kits are reserved for focal and near chunks, so the triangle budget stays within the existing performance policy.

### Compact architectural families — next implementation

The 799-building overview exposes a massing problem beyond density: most non-tower buildings share a rectangular shell, many typology differences are applied only to facades, and apartment setbacks usually affect just one top floor. Color variation alone cannot resolve this repetition.

Introduce a deterministic family selector using existing building kind, floor count, parcel proportions, and appearance seed. Narrow low-rise homes and shops should support rowhouse/machiya forms with quieter party-wall sides, rear service volumes, and varied gable/parapet profiles. Midrise apartments and offices should have distinct massing: walk-up circulation cores, recessed balconies or loggias, and multi-floor setbacks where appropriate. Retain authored storefront detail and the established anchor identity. Keep geometry within the existing parcel envelope, preserve authoritative floors and access, and apply the same family in both styles and construction stages. Put family/layout decisions in a separate pure module rather than expanding the existing shell branch indefinitely.

Validate silhouettes in neighborhood and overhead views with depth of field off, plus street-scale entry/access readability. Both low-rise and midrise families are implemented in the shared kit, but neither has passed the complete city-context artistic gate.

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

Use finite river geometry and a flow field following its course. Layer flowing normals and gentle displacement with a bounded low-resolution ripple grid near observation areas. Rain and wakes inject impulses. When no fine grid is active, analytic waves preserve continuity. Local ripple detail is visual and does not affect city economics.

Combine shoreline coloration, foam where appropriate, sky reflection, and a reduced-resolution planar reflection for a principal water surface. Update reflections at a lower cadence or when the camera changes sufficiently. Avoid recursively rendering water in reflection passes. More complex water bodies may use a cheaper reflection approximation.

Flooding, erosion, hydraulic infrastructure, and full-domain fluid dynamics are outside initial scope. The water remains animated and locally simulated in every artistic style.

## Lighting and weather

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

### Expressway connection geometry

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

Tower finishing checkpoint: installed floors now use the shared authored open
facade rather than generic temporary panels. Roof equipment waits for full
cladding and operational emission waits for opening. Normal Anime before/after
playback confirms retained facade geometry; full motion and both-style browser
acceptance remain open. See the renewal evidence above.

Dated implementation notes for this spec: [validation/spec-notes/05-rendering.md](../validation/spec-notes/05-rendering.md).
