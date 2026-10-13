# Current growth stage trace — 2026-10-12

Follow-up to movement-growth-browser.md, same M2 Max / in-app browser,
WebGPU, High detail, Miniature, Overview, 50 mm, DOF off. Imported
`.tools/movement-support/browser/growth-575.json` through normal UI, then ran
8× simulation / 16× growth. Temporary timing markers did not change scheduling.
Two bounded traces were run; patches are archived and removed from production.

`growth-stage-v2-raw.json` contains 199 console entries. Excluding startup and
the import tick (retain tick >22026), 79 world updates have median 46.1 ms,
maximum 61.6 ms. Median stage costs: transit/access 22.7 ms, trees 21.8 ms,
ground 2.1 ms, building inputs .4 ms, road inputs .1 ms, sidewalk inputs .1 ms.
The frame trace quota filled during startup, so it provides **no growth-frame
stage evidence**. Do not attribute the observed 259 ms frame solely to these
world calls. Instrumentation and console logging may affect timing.

The subsequent `growth-transit-v2-raw.json` measures 40 world updates after
tick 22026 with individual calls separated:

| Stage | Median ms | Maximum ms |
|---|---:|---:|
| Camera director world update | 20.5 | 22.5 |
| Bus world update | 0 | .1 |
| Train world update | 0 | .1 |
| Building access paving | .3 | 12.4 |
| Rear extensions | 0 | .1 |

Zero values mean below clock/report resolution, not zero work. The first trace's
transit/access bucket also includes infrastructure request; the second isolates
the director, establishing it as a significant recurring cost. Next inspect
anchor construction and obstacle handling within CameraDirector.world with a
repeatable CPU harness. Preserve anchor choice/activity, developed extent,
snapshot collision recovery, and follow behavior. No speculative cache based
only on entity counts; same-count geometry and activity changes matter.

Luna's read-only riverbank audit, independently checked against applyStyle:
style switching swaps materials/contours/lens treatment; it does not call
buildTerrain. Railings are part of asynchronously committed road geometry.
Different material/haze contrast or a pending batch completing between the
previous captures remain plausible; there is no demonstrated style-induced
terrain mutation. A settled matched pair is still required.

No production change retained this turn. Removing tracing triggered the usual
Vite reload, so the tab no longer retains the profiled mature city. The saved
checkpoint remains available. Full goal remains active and incomplete.
