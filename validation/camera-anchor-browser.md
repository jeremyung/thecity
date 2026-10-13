# Camera optimization browser follow-up — 2026-10-12

Previous turn made progress: exact skyline anchor parity and lower CPU cost.
Current ordinary browser review does **not** establish resolved growth stalls.
M2 Max, in-app browser, WebGPU, High detail, 1280×720, 50 mm, DOF off.
Imported seed87231/tick22026 via normal UI, Miniature, Overview, 8× simulation,
16× development. Startup warning cleared without Retry. No console warnings or
errors returned at the check after pausing growth.

At638buildings: loading rolling120-frame p95 62.7ms, CPU p95 31.6ms,
longest229.2ms. At pause723buildings: loading p95 81.0ms, CPU p95 33.5ms,
longest242.5ms. Different sampled windows/populations preclude a controlled
before/after browser speedup claim. Tree clearance and unprofiled frame stalls
remain. No quality settings or geometry reduced.

Exported final checkpoint to `.tools/movement-support/browser/growth-723.json`:
seed87231,tick27925,clear/intensity0,723buildings,410journeys,9projects,
day1 09:55. No camera gestures between these settled Overview captures:

- [Miniature](screenshots/movement-presentation/settled-723-miniature.png)
- [Anime](screenshots/movement-presentation/settled-723-anime.png)

All four geometry queues were zero before each capture. Miniature rolling p95
28.1ms/max30.4ms; later Anime p95 16.7ms/max18.3ms. These separate samples do
not establish which style is faster. Anime warmup reported323shader samples,
1118ms submission/15383ms await; style-switch latency remains an open issue.

Visual inspection finds the same building/corridor/bank layout in this pair;
material response, contours and atmosphere account for the evident difference.
This resolves the earlier unverified riverbank mismatch as an unproven geometry
defect, not a general proof of identical geometry in all style transitions.
Camera transform was not exported; held Overview and no camera/time inputs are
the comparison record. No continuous-motion recording was captured.

Then selected Hold waterfront view through renderer details:
[Anime waterfront](screenshots/movement-presentation/waterfront-723-anime.png).
Read the locked04concept directly. Main perceptual gaps are smooth water/cloud
reflections, bare planted edges with uniformly repeated stones, and transport
corridors dominating the foreground. Dense massing is now visible but does not
make this scene match the concept. The original depth stripes are not visible
here, which does not prove the original camera repaired.

Work selection: retain measured camera CPU improvement; leave performance gates
open. Return to the previously rejected planting experiment's explicit next
step: a dedicated, readable low-shrub specimen, evaluated in-browser before
city integration. Do not repeat scaled tree-card scattering. Read
bank-planting-experiment.md, preserve clearance/shared geometry/bounded meshes,
and reject the specimen if it does not materially improve the bank at the
actual viewing scale. Test both rendering styles and minification.
No code changes, commit, push, or goal status change this turn.
