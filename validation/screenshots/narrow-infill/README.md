# Mature narrow-infill visual review — 2026-10-11

`mature-871-software-webgl.png` is a real browser rendering of the exported
871-building continuation, after the narrow urban entrance fix. Source hash,
geometry queues, camera and capture conditions are in `capture.json`.

## Method and limits

Chromium headless uses `--use-angle=swiftshader --enable-unsafe-swiftshader` and
`?webgl=1`. The simulation was paused through its developer client and loaded from
`.tools/narrow-frontage-review/final-save.json`. The renderer animation loop was
stopped. The pipeline was temporarily detached while the existing render update
prepared all building/chunk queues (1,832 updates, both queues empty), then restored
for a normal final render. Geometry, materials and world dimensions were not
substituted. Depth of field is disabled to expose actual spatial structure.

Lighting was set to noon through the developer preview API; the UI still displays
the saved live clock (14:00) and its unsynchronized pause indicator. Do not read
those labels as capture lighting or proof of ongoing simulation. This static
preparation bypasses normal frame pacing and is not a loading/performance test.
It also does not resolve the hardware WebGPU capture failure. The camera differs
from earlier 850-building shots, so no matched visual delta is claimed.

## Root visual finding

The snapshot is drawable and the finished massing is readable. It still does not
meet the Tokyo references: long thin development strips, broad unoccupied green
block interiors, repeated detached spacing and a straight canal dominate the
composition. Extra narrow infill is too modest to establish continuous urban
fabric. This view does not resolve individual door details; entrance correctness
remains geometry-test evidence pending a close capture.

An independent source/fixture audit checked the particularly small crown on
four-floor offices: only two of 871 buildings use it. Altering that silhouette
would not address the dominant citywide gap, so no crown change was promoted.
The next density intervention should address connected use of block interiors
and street-front continuity, with a matched camera comparison, rather than
inflate a small architectural subtype or hide gaps with depth of field.
