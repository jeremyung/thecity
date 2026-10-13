# Irregular water detail — 2026-10-11

## Diagnosis and bounded change

After isolating shadows and reflection, removing only the two fine sinusoidal
normal terms eliminated the regular river-wide bands. Broad drifting normals,
reflection, received shadows, displacement and rain simulation remained active.
[Diagnostic capture](screenshots/water-normals/no-fine-normals.jpg).

Replace those sine terms with two offset 2D Perlin fields, advected continuously
by the existing time-dependent phase coordinates. Both projected phase
footprints attenuate fine detail; unresolved variation still raises roughness.
This is an approximate screen-footprint filter, not exact noise integration.
No geometry, texture allocation, simulation, save or worker contract changes.
It adds two 2D noise evaluations in place of two sines; GPU cost is unmeasured.

A read-only Luna review suggested reusing broad noise to warp the sine phases
without new noise samples. Primary chose independent fine noise for this first
candidate to eliminate coherent wave fronts rather than merely bend them.
That choice remains subject to measured shader cost; no speedup is claimed.

## Browser evidence

Normal import: `.tools/fidelity-review/fair-routes-87231.json`, seed 87231,
tick 160000, 1839 buildings, clear weather, paused, Hold waterfront view,
50 mm, DOF off, lighting preview 18:00, WebGPU/high detail, 1099 × 1071.
Both geometry queues reached zero through the normal scheduler.

- [Original matched Anime baseline](screenshots/waterfront-preset/first.jpg)
- [Irregular Anime detail](screenshots/water-normals/irregular-anime-dusk.jpg)
- [Same paused scene in Miniature](screenshots/water-normals/irregular-miniature-dusk.jpg)

Regular diagonal bands no longer span the river. Shadows and bridge/sky
reflections remain. Retain this narrow improvement. Fine detail is subtle in
this overview; water still looks too flat and cloud reflections too smooth to
claim concept fidelity. No general artistic acceptance is implied.

Resumed normal 1× playback with the held Miniature camera; two sampled live
frames show changed water illumination and traffic, with no captured warning
or error logs. This is a short continuity smoke check, not a frame-by-frame
motion/shimmer or multi-hour acceptance result:
[live A](screenshots/water-normals/live-miniature-a.jpg),
[live B](screenshots/water-normals/live-miniature-b.jpg).
After pausing again, [noon review](screenshots/water-normals/miniature-noon-after-playback.jpg)
retains the same framing but a later, unrecorded exact tick. Do not use it as a
matched before/after. Night, rain and camera-motion comparisons remain open.

Explicit `?webgl=1` founding-city smoke check, Miniature, 06:31 live clock,
50 mm, DOF off, Hold waterfront view: WebGLBackend ready, both queues zero,
no captured warning/error logs, visible water. [Fallback capture](screenshots/water-normals/webgl-founder-smoke.jpg).
This is shader/runtime compatibility evidence, not a matched mature backend
comparison or performance benchmark.

TypeScript and production build pass with the existing bundle-size warning.
`git diff --check` passes. No engine changes or broad suite result are claimed.

## Remaining priority

Close this bounded banding intervention. Do not keep tuning water indefinitely:
architectural variation, vegetation, street composition and elevated merges are
still substantial perceptual gaps. Water reflection distortion and richer light
need a separately evidenced comparison, with rain and motion acceptance before
calling the waterfront complete.
