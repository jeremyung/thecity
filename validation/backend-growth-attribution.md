# Growth stall attribution — 2026-10-12

## Evidence and setup

The previous turn made progress by preserving both-style camera recordings and
sampled frames. This investigation follows the separate unresolved ordinary-growth
stall, avoiding further unverified DOF tuning.

Apple M2 Max / Codex in-app browser / WebGPU, High, Miniature, 50mm, DOF off,
noon preview, Explore after Frame review building1814. Each run imported the normal
seed0/tick9250/260-building checkpoint while paused, then resumed at simulation1×,
growth16×. No recorder active, no manual geometry queue drain. Import load samples
at tick9250 are retained but excluded from the growth numbers below. HMR startup
briefly showed the graphics watchdog and recovered without switching backend.

Temporary wrappers measured synchronous backend pipeline creation, attribute
creation/update, and begin/finish render passes. First trace:
`backend-growth-trace.json`, 36 threshold-selected growth samples after9250.
At tick9300:220.2ms render,4.7ms attribute creation,0.6ms pipeline creation. At
tick9600:205.9ms render,0.7ms attribute creation,0.5ms pipeline creation.
These operations did not explain the main stall in these samples.

A second run also wrapped Three NodeManager.getForRender. Raw console history is
`backend-node-growth-trace.json` and can contain earlier trace entries; select
samples with tick>9250 and a `parts.nodeBuild` field for this attribution (19
samples). At tick9480:245.5ms render,228.0ms node preparation; tick9450:235.3ms
render,218.8ms node preparation; tick10120:190.5ms render,174.5ms node preparation.
Individual calls reached41.6ms. Counts include cache hits, not just builds.

This locates the dominant synchronous CPU cost inside node-state lookup/build.
It is not GPU execution timing, an all-frame percentile, or proof of the cause
of every earlier camera or growth stall. Wrapping/timing/console output adds
measurement overhead. The 120ms recording spike is not independently attributed.

Both instrumentation patches are retained. Production city.ts was restored
exactly from its pretrace copy; no wrappers remain. TypeScript passed for the first
instrumented variant; no new full-suite claim. `git diff --check` passed afterward.

## Independently inspected lifecycle hypothesis

Luna performed a bounded read-only audit; root inspected the cited implementation.
In installed Three0.183.2, NodeManager.getForRender builds a state on cache miss,
and NodeManager.delete evicts it when usedTimes becomes zero. RenderObject keys
include material and geometry layout, rather than the geometry's identity.
BuildingChunks.commit removes the old batch and calls CityRenderer.retire before
the new batch renders. Retire disposes immediately unless style compilation is
pending. Geometry disposal tears down old render objects. Shared materials alone
therefore may not preserve cached builders across replacement.

This eviction explanation remains a hypothesis: traces do not yet correlate the
same cache key's last release with its next rebuild. Do not change material keys
or introduce persistent sentinel objects based on it alone.

## Next bounded experiment

Trace zero-reference cache deletion and subsequent same-key builds around one
ordinary growth commit. If confirmed, test deferred disposal until after replacement
has rendered, preserving atomic visible replacement, picking and all geometry.
Maintain a strict bound and flush on reset/dispose; do not retain old city resources
indefinitely. Reject the approach if same-key rebuild counts and measured stalls do
not drop, if new references do not acquire state before old disposal, or if resource
lifetime/visual correctness regresses. Revalidate ordinary growth and camera motion
without instrumentation before claiming performance acceptance.


## Correlation rejected eviction; persistent lighting candidate implemented

`node-cache-correlation.json` recorded80 misses with80 distinct keys and no matching
zero-reference deletion in the growth window. This does not support the proposed
same-key eviction explanation; deferred retirement was **not** implemented.
`node-key-components.json` identifies repeatedly recreated instanced meshes with
counts118–122, stable pole material/program keys and changing object IDs. Installed
Three RenderObject.getMaterialCacheKey appends each InstancedMesh UUID, explaining
why new identities defeat node-state reuse even with shared materials.

Root and Luna independently inspected the road path: each completed-road rebuild
called buildStreetLighting, creating fresh pole, arm, head and glow-pool instances.
Luna implemented prepareStreetLighting/PersistentStreetLighting; root reviewed the
transform formulas and integrated publication into updateRoads. Candidate data is
prepared off-scene. At atomic road commit, four persistent1000-capacity meshes copy
packed transforms/counts and publish corresponding nearby-light sources. Geometry,
night uniforms, pavement-pool placement and authored detail are preserved. Reset
sets count zero; shutdown retires owned resources with the existing compilation
safety mechanism. No third-party code or shader-key overrides were introduced.

Root added initial zero counts/dynamic buffer usage, explicit root detachment on
dispose and a production road-commit regression: candidate lighting cannot mutate
visible transforms before roads publish, and instance identities survive afterward.
Updated the old road-disposal test to require owned lamp materials to survive road
replacement.40 focused tests pass (street lighting, nearby lights, road batches,
style warmup); production build passed before the final test-only addition.
Temporary correlation/key wrappers are removed; patches retained.

Uninstrumented ordinary growth from the same seed0/tick9250 checkpoint reached
285 buildings with2 neighborhood batches pending. One120-frame UI window reported
93fps, frame p9517.1ms, CPU p9516ms and longest26.5ms; no captured console errors.
`screenshots/persistent-street-lighting/growth-metrics.png` preserves the window.
This is encouraging but is not an all-run maximum or matched before/after benchmark.
The browser viewport changed to847×764, limiting comparison with1280×720 captures.
A post-growth night screenshot checks retained lamps; it is not a same-tick visual
diff. Full node-build trace after this change, both-style growth, reset/style lifecycle,
night motion and named-viewport performance acceptance remain pending.


## Persistent lighting: matched growth trace — 2026-10-12

Revalidated the candidate through normal scheduling in the Codex in-app browser,
WebGPUBackend, on a MacBook Pro Mac14,5 / Apple M2 Max / 32 GB RAM. Browser engine
version and GPU execution timings were not collected. Viewport 1280×720, High
quality, Miniature, DOF off, 50 mm lens, held review building 1814, noon lighting
preview. Imported seed 0 / tick 9250 / 260 buildings while paused; waited for all
four visible geometry queues to settle, then resumed at simulation 1× / growth
16×. No queues were manually drained. Weather was inherited from the checkpoint.

`persistent-light-growth-trace.json` records all 4,725 sampled animation frames
with 9250 < tick < 10300, rather than only slow frames:

| CPU measure | p95 | Maximum | Frames over 30 ms |
| --- | ---: | ---: | ---: |
| Full animate body | 15.1 ms | 49.2 ms | 27 |
| Render submission | 9.4 ms | 43.3 ms | 1 |
| Node-state lookup/build | 0 ms | 30.0 ms | 0 |

Only one new node-state cache miss was recorded (MeshStandardNodeMaterial,
30.0 ms). The previous diagnostic recorded repeated 190–245 ms render submissions
with 174–228 ms spent in node-state work. Those repeated stalls did not recur in
this interval. Earlier traces sampled slow frames only, so percentile comparisons
would be invalid. This is CPU submission evidence, not GPU timing or overall
frame-interval acceptance. Remaining 49.2 ms work spikes still need attention.

Root reran 40 tests across road batches, street lighting, nearby lights and style
warmup; all pass. TypeScript passes after removing instrumentation. The diagnostic
patch is archived in `persistent-light-growth-instrumentation.patch`; production
code contains no trace wrappers. Existing lifecycle audit found no concrete defect
in cancellation, reset, style traversal or deferred shutdown disposal. Both-style
live growth, long-run memory, night motion and broad performance remain unaccepted.
The immediate shader-churn hypothesis is supported; further generic cache work is
not justified by this result. Resume perceptual camera/composition work next.
