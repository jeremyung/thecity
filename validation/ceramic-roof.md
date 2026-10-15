# Ceramic roof detail — 2026-10-11

## Deficiency and bounded intervention

The locked concepts show fine ceramic roof rhythm and rounded highlights.
The existing low-rise specimen roofs read as large metal panels: five coarse
courses and sparse straight ribs. The controlled baseline is
`screenshots/ceramic-roof/before-anime.jpg`.

A shared ceramic-detail helper now supplies rounded ridge-to-eave rolls and
restrained course lips to low-rise and anchor gables. Existing pitched panels,
eaves and building massing remain. Rolls use four arc subdivisions and just two
longitudinal cross-sections; detail is capped at 32 rolls per side, 12 courses,
and 850 triangles for the tested 20-by-12-metre roof. Geometry merges into the
existing roof material, with no textures, light objects or per-frame work added.

Primary review corrected buried detail, the surface-normal offset direction,
and the low-rise ridge-cap height. Detail uses the roof tone to avoid bright
metallic-looking grid lines. Two-sided winding, finite normals, panel clearance,
indexed merge attributes, roof bounds and geometry cost have focused coverage.

## Verification and judgment

- Root run: 27 tests pass across ceramic, low-rise, mid-rise and tower kits.
- Production build passes (existing bundle-size warning).
- Browser fixed Inspect low-rise street composition at 1280x720, open stage,
  controlled daytime lighting, no DOF. Architecture specimens have no simulation
  seed/tick/weather. Camera reset action is identical before and after.
- `after-anime.jpg` and `after-miniature.jpg` show finer, rounded tile rhythm
  using the same production building kits. The change is retained as a local
  improvement; it is not full architectural or concept acceptance.
- `night-miniature.jpg` records a limitation: this isolated viewer is too dark
  to judge most roof detail. It does not exercise the living-city light rig.

Abandon if the detail is buried, folds, breaks bounds, causes material/geometry
merge failures, or adds noise without visible benefit. The final controlled
views and tests do not exhibit those defects. Mature-city loading, geometry
cost and motion shimmer still require normal-scheduler verification.

The broader architecture remains plain relative to concepts: walls need
material depth, shopfronts need more character, and the city needs a matched
waterfront composition with richer vegetation, night light and reflections.
Avoid treating this roof correction as a substitute for those larger gaps.

## Mature-city verification and cost

A fresh current-engine seed 87231 was grown through 160000 ticks at development
rate 16, yielding 1854 buildings. The save was imported through World → Import.
Both building and neighborhood queues reached zero through ordinary rendering.
No private drain or synthetic geometry completion was used.

The independent Node audit compares identical building descriptors against the
pinned pre-roof kit at commit `8e9f5e196659c29c9c6a5b9dcd6bed43fd1de2f7` and
current kit, without changing working sources. Root reviewed and reran it after
pinning the baseline and disposing owned sign resources. Results reproduce:

| Authored building geometry | Before | After | Difference |
| --- | ---: | ---: | ---: |
| Triangles | 4513402 | 4692330 | +3.96% |
| Typed-array bytes | 314999772 | 322194812 | +2.28% |
| Generated meshes | 29670 | 29051 | -2.09% |

These are cumulative per-building source-array counts, not simultaneous browser
heap, GPU allocations or rendered draw calls. See `ceramic-roof-cost.json` and
`scripts/audit-ceramic-roof-cost.mjs`; the input hash is recorded in the report.

Matched mature views are `screenshots/ceramic-roof/mature-miniature.jpg` and
`mature-anime.jpg`: seed 87231, tick 160904, clear weather/intensity 0, noon
lighting preview, 50 mm, DOF off, same held Explore camera, WebGPU high detail.
Camera coordinates were not exported; the pair shares a held same-session view.
The roof detail remains visible without changing the city layout on style swap.

`mature-diagnostics.jpg` records zero queues, 1379 draws and 14008408 rendered
triangles. It also shows a **3 fps** sample and 11.9 ms reported CPU time. These
are uncalibrated browser observations, not a named-device performance profile;
no smooth-playback or performance acceptance is claimed. The Node cost delta
does not establish that this roof change caused the low frame-rate sample.
Motion shimmer, load timing, and real hardware profiling remain open.

Current reusable save: `.tools/fidelity-review/ceramic-mature-87231.json`.
Compared with the concepts, the full view still exposes larger deficiencies in
water/light, repetitive facades, greenery and atmosphere. Further tile detail
has lower priority than those full-scene issues and the performance measurement.
