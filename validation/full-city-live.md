# Full-density live interaction check — 2026-10-12

The previous turn verified existing PR feedback but did not advance the living-city
implementation. This pass returns to runtime evidence rather than repeating that
status. No production source changes were made.

## Setup and procedure

Existing in-app browser tab 22, localhost:5173, 1280×720, WebGPU High detail,
50 mm lens, DOF off, clear weather. City seed 87231 began at tick 75925 with
1,839 buildings (the fully realized fixture), day 3 at 09:55, paused in the
shallow manual waterfront view documented in shallow-orbit.md.

Used Return to live, then Resume simulation. Inspected ordinary City details
and Renderer details; no queue draining or hidden state changes. Followed the
train, switched Anime to Miniature while running, waited for the UI to confirm
Miniature applied, then paused at day 3 14:02 and stopped following. Final tick
and exact camera coordinates were not exported; screenshots use visible clock
labels, not reconstructed tick values.

## Observations

- Buildings remained at 1,839. Journeys underway changed from 399 to 324, 113,
  95, and eventually 11. Train terminal, passenger counts, boarding/alighting,
  bus-stop and arrival events changed during the run. This verifies live updates
  and follow interaction; it is not a complete passenger/path correctness audit.
- All four displayed rendering queues were zero at sampled checkpoints.
  Works in progress was zero at these checkpoints. This run therefore does not
  validate active construction, renewal or infrastructure staging.
- Anime shallow-view samples: frame p95 13.8 / 12.9 / 11.3 ms, CPU p95
  11.7 / 10.5 / 9.3 ms; longest frames 46.2 / 44.1 / 14.6 ms. Each is a rolling
  120-frame window, not a whole-run percentile. Draws 583–595; about 10.3–10.5M
  triangles. These are browser diagnostics, not an isolated optimization comparison.
- Settled Miniature after pause: frame p95 12.7 ms, CPU p95 11.2 ms, longest
  frame 16.2 ms, 600 draws, 8,613,616 triangles, 38 textures, 341 shapes.
- The style-switch UI reports 106/106 shader samples, prepare 1 ms, submit
  2,916 ms (largest 39 ms), await 5,113 ms, apply 0 ms, applied. These stages
  are reported separately; no claim of instantaneous switching. The old view
  stayed visible with a Preparing Miniature indicator until completion.
- Captured warning/error log query returned an empty list during the switch.

The browser tools provide screenshots and interaction state, not a motion-recording
API here. No recording was made, so do not treat these samples as proof of smooth
continuous animation, absence of transient clipping, or shadow stability. Sound,
WebGL fallback, narrow viewports, loading latency and memory were not measured.
Hardware is the existing local Mac session; no fresh hardware inventory was taken.

## Visual judgment and next action

The shallow view exposes a larger composition problem: repeated apartment masses,
a dominating elevated-road band, and broad empty hills behind a finite neighborhood.
Train-follow framing keeps the train visible in the sampled views, but the station
canopy and surrounding repeated buildings remain visually simple. The rendering
is still substantially below the locked concepts. Do not spend the next pass on
more service-window or roof ornament, or repeat rejected water-gain experiments.

Next bounded workstream: compare the full-density skyline/height distribution and
its camera framing with the locked concepts and the generation specification.
First distinguish missing tall-building allocation from camera placement and
background treatment. Do not enlarge the map or add a fake populated simulation
outside its boundary to hide the gap. Choose one evidenced change after that audit.

## Captures

- screenshots/full-city-live/diagnostics-start.png: live shallow Anime view,
  clock 10:58; panel is scrolled so numeric diagnostics are not visible.
- screenshots/full-city-live/train-follow.png: Anime, 12:04.
- screenshots/full-city-live/train-miniature.png: still preparing Miniature,
  12:49; this is explicitly not a completed style comparison.
- screenshots/full-city-live/train-miniature-settled.png: Miniature applied,
  13:38. Different follow pose/time, not a matched style comparison.

Goal remains active. No commit, push or PR change.
