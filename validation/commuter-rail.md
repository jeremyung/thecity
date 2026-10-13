# Commuter rail — 2026-10-11

Subsequent [reversal correction](train-reversal.md) keeps physical coach orientation fixed at terminals.
The [corridor wiring checkpoint](rail-corridor.md) implements the initial
straight catenary and service edge; the observations below describe the earlier kit.

## Deficiency and bounded change

The earlier renderer used rectangular coaches, separate floating window/stripe
boxes and wheel blocks. Its rail centers were 2.7 m apart for a 3.1 m vehicle,
and the platform began 1.05 m beyond the body. The supplied photographs instead
show coherent vehicle, wheel, track and platform proportions plus distinctive
roof equipment and door/window rhythm.

The first family now has rounded stainless bodies, paired sliding-door panels,
green/orange bands, cab glazing, head/tail lights, underfloor cabinets, bogies,
roof equipment and a middle-coach pantograph. It is a compact fictional 15 m
coach, not a reproduction of a particular operator's vehicle. Three roles share
instanced geometry, bounded to eight trainsets. Materials remain owned by the
existing style cache; no generated textures or runtime AI calls were added.

The shared profile uses a 1.067 m inside gauge, .07 m rail heads, 6.935 m railhead
elevation, and a 1.065 m floor offset to meet the existing engine's 8 m platform.
Roadbed center changes from 7 m to 6.4 m. Platform outer edge remains 7.8 m;
inner edge moves to 1.65 m (0.1 m from the main coach body). Door overlays reduce
that clearance slightly. Existing walking surfaces and queue coordinates remain
unchanged. Sleepers are now 2.2 m wide at .75 m spacing instead of 5 m at 2.5 m.
No engine or save contract was changed by this rail intervention.

## Verification

- `npm run build`: pass, existing large-chunk warning remains.
- Train-kit, station-kit and road-elevation suites: **11 tests pass**. Includes
  finite geometry/envelopes, wheel alignment, role differences, instance poses
  in both directions, actor IDs/reset, floor elevation, and ray-tested platform
  support including the new inner edge in rotated terminal frames.
- `git diff --check`: pass.
- Authored geometry audit: lead 2,128 triangles / 9 material batches;
  middle 2,092 / 8; tail 2,128 / 9. Total 6,348 triangles and 26 batches,
  at most 50,784 train triangles at eight instances before extra render passes.
  This increases draw batches over the old four-mesh kit. Closer sleepers also
  increase track geometry. No named-hardware performance acceptance is claimed.

## Browser evidence

Normal import of `.tools/fidelity-review/fair-routes-87231.json`, seed 87231,
tick 160000, 1,839 buildings. WebGPU, 1280×720, High detail, DOF disabled,
Neighborhood 50 mm. No rain visible; weather unchanged from the checkpoint.
Building and chunk queues settled at zero. Used View station, then manually
orbited/zoomed in Explore; the exact numerical camera pose was not exported.

- `screenshots/commuter-rail/miniature-morning.jpg`: 08:00 preview, wider view.
- `screenshots/commuter-rail/miniature-detail.jpg`: 16:00, closer oblique view.
- `screenshots/commuter-rail/anime-detail.jpg`: same paused pose/time as Miniature.
- `screenshots/commuter-rail/departure.jpg`: live 1× departure from that view,
  still at 16:00 lighting preview; exact subsequent tick not recorded.

Observed stronger rounded coach silhouettes, roof equipment and platform fit.
The scene remains much plainer than the photographic references. These are
candidate captures, not a matched old/new benchmark or final artistic acceptance.
Browser warning/error query returned no entries after the live check. WebGL and
full dawn/night/motion acceptance were not repeated for this intervention.

## Remaining work, in order

1. Coherent overhead contact wire/supports meeting the pantograph; track-bed
   material, drainage, fences/parapets and station context. Pantograph is
   currently visibly unconnected to any overhead wire.
2. Each coach and bogie following the route independently, consistent couplers,
   smoother departures/braking, animated doors tied to actual dwell/boarding.
   Current three-car pose still shares one heading, and terminal reversal swaps
   role placement. Do not present that as articulated rail behavior.
3. Separate short colorful streetcar kit, embedded/segregated track, small stops
   and actual street-level service. Ground commuter corridors need their own
   ballast, retaining-wall and crossing treatment. Neither exists yet.
4. Matched dusk/night and motion reviews, WebGL check and measured hardware cost.

Preserve the same world/geometry between Anime and Miniature. Do not use DOF,
shader changes, or extra miniature details to conceal missing infrastructure.
