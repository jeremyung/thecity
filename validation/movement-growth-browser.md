# Ordinary growth after movement integration and tree cache — 2026-10-12

Current branch `codex/living-city-fidelity`; PR #3 head f386f45 is an ancestor.
Prior turn made progress by independently checking tree-cache tests and building
the application. This turn checks the running result; no performance acceptance.

Host: Apple M2 Max, Codex in-app browser, WebGPUBackend, 1280×720,
High detail, 50 mm, DOF disabled. Imported the current-format raw save
`.tools/movement-support/presentation-cost-save.json` through the normal Import
UI: seed 87231, tick 14400, 385 buildings. No private render state or manually
drained queues. The initial graphics startup warning cleared before Retry could
be clicked; no errors/warnings were returned from the subsequent console check.

Paused Overview at 20:24 reached zero geometry queues: rolling 120-frame p95
14.6 ms, CPU p95 12.2 ms, longest 20.0 ms. Ran simulation at 8×, growth 16×.
At 500 buildings, loading-phase rolling p95 was 66.6 ms, CPU p95 29.5 ms,
longest 259.1 ms, with four neighborhood, one sidewalk and one road job queued.
Paused at 575 buildings: loading p95 71.1 ms, longest 159.1 ms in that later
window. These are sampled rolling windows, not a complete trace or an A/B
comparison with the pre-cache browser. Growth stalls remain unaccepted.

Exported the paused city through the normal UI and preserved it at
`.tools/movement-support/browser/growth-575.json`: seed 87231, tick 22026,
clear weather, intensity 0, 575 buildings. UI clock 04:01, day 0.
Set lighting preview to 06:00 and retained Overview framing without further
camera gestures. Camera coordinates were not exported. Captures:

- [Anime dawn](screenshots/movement-presentation/growth-575-anime-dawn.png)
- [Miniature dawn](screenshots/movement-presentation/growth-575-miniature-dawn.png)

Both capture the same paused simulation and preview hour. The riverbank edge
reads differently after style switching; do not treat them as proof of identical
visible geometry until pending-batch/material effects are distinguished. No
queue snapshot was retained immediately before the Anime capture. Final
Miniature diagnostics show all four queues at zero, ready-phase p95 14.8 ms,
CPU p95 13.3 ms, longest 19.9 ms. Style preparation reports 278 shader samples,
2610 ms submission, 6072 ms await. These timing fields are not added together
as an asserted user-perceived switch duration.

Visual gaps: densely packed massing is present, but repeated window grids,
muted surfaces, sparse street activity at this hour and dominant elevated
corridors remain far from concept acceptance. This view does not reproduce the
reported ground stripes. No continuous-motion recording or sound verification
was completed, and no original-camera repair claim follows from these frames.

Next: localize remaining growth stalls with a bounded main-thread stage trace
on this current checkpoint, including work outside the render callback; do not
infer the cause from the render CPU metric alone. Avoid further speculative tree
optimization: its diagnostic aggregate gain did not remove long browser frames.
Also resolve the riverbank style comparison before using it as fidelity evidence.
Tab 15 is retained paused in Miniature with diagnostics open and dawn preview.
