# Phase-aware frame diagnostics — 2026-10-11

The prior display averaged the last 120 frame intervals across loading and
background gaps, refreshed only every 90 frames, and paired that average with a
single current CPU duration. At low frame rates a reading could remain stale
for tens of seconds. That cannot diagnose the recorded 3 fps sample reliably.

The renderer now uses a bounded 120-pair sampler. It resets on world replacement,
loading/ready changes and visibility transitions, and publishes on a one-second
wall-clock cadence when animation frames run. Loading includes building work,
neighborhood batches and pending shader compilation. Diagnostics expose phase,
sample count, average fps, nearest-rank frame p95, CPU p95 and longest frame.
The first frame after a reset is explicitly collecting, not reported as 0 fps.
Genuine slow visible frames remain in the sample; no long-frame filtering was
added. Quality selection, scene detail and simulation behavior are unchanged.

CPU denotes synchronous animation work and render submission, not GPU time.
Frame intervals include scheduling and presentation effects; CPU and frame
percentiles are separate distributions and must not be added together.

## Verification

32 focused timing/UI/quality tests pass; production build passes. Tests cover
bounded state, hidden-return gaps, phase resets, backwards clocks, preserved
600 ms visible frames, nearest-rank percentiles and the diagnostic labels.
An initial UI test targeted a closed SSR panel; it was corrected to render the
extracted diagnostic component directly rather than weakening its assertions.

Normal browser import of seed 87231, 1854 buildings, current mature checkpoint:
- Loading witness: 120 samples, 63 fps, frame p95 20.6 ms, CPU p95 18.8 ms,
  longest 26.9 ms, 0 buildings and 5 neighborhood batches queued.
- Settled witness: ready, 120 samples, 53 fps, frame p95 26.1 ms, CPU p95 20.8 ms,
  longest 28.9 ms, both queues zero; 1343 draws and 14001614 reported triangles.
- WebGPU high detail, Anime, noon lighting preview, 50 mm, DOF off, held Explore
  view. These are uncalibrated in-app-browser observations, not a named-hardware
  benchmark, matched before/after optimization, or a GPU timing measurement.

Capture: `screenshots/ceramic-roof/mature-frame-timing.jpg`. Values fluctuate
between the accessible-text observation and screenshot. The old 3 fps reading
is retained as historical evidence; this change does not prove its cause or
claim a speedup. Next profiling should separate geometry/update work, submission
and GPU costs before reducing fidelity.
