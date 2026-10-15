# Waterfront realization audit — 2026-10-12

The preceding bank-geometry turn made progress. This pass tests whether sparse
waterfront captures represent missing planned development or incomplete realization.

`scripts/audit-waterfront-growth.mjs` restores the current723building checkpoint
and advances normal WASM growth in24batches of2000ticks, without edits to state,
plan, demand or placement. `waterfront-growth.json` records25snapshots and the
script exports `.tools/waterfront-growth/fully-realized.json` for normal browser import.
Counts use building centers within100m of the world-X river edge, not occupied
footprint area or a claim of complete district density.

| Tick | Buildings | Realized/planned lots | West waterfront buildings | East |
|---|---:|---:|---:|---:|
|27925|723|750/1839|0|54|
|39925|1023|1033/1839|11|160|
|51925|1293|1326/1839|56|216|
|75925|1839|1839/1839|193|216|

At75925all564planned roads are realized. This single-seed audit demonstrates
that the apparent empty west bank is not permanently absent from this plan.
It does not prove appropriate cross-bank pacing across seeds. Road selection
scores immediate frontage lots, nearby active buildings, kind and station
proximity; bridges needpopulation600. Those rules could defer expansion into
an undeveloped bank, but causal attribution needs a candidate-score audit before
changing priorities.

The displayed Maturity stage is not plan completion: lib.rs uses active-sites,
realized developable-parcel infill and population thresholds. In particular it
can label a dense developed subset Maturity while most of the plan is unrealized.
Do not use the label alone to select final-density visual fixtures. The723fixture
remains useful for development and loading checks, not full-density acceptance.

## Browser evidence

Normal import of generated1839save accepted. Seed87231,tick75925,clear,
simulationpaused,Miniature,HighWebGPU,50mm,DOFoff,1280×720. Existing Hold
waterfront view sets the same camera as the723comparison. All geometry queues0
through production scheduling before framing/capture. No manual queue draining.

- [Full noon](screenshots/waterfront-growth/full-miniature-noon.png)
- [Full night22:00](screenshots/waterfront-growth/full-miniature-night.png)
- [Developing723noon](screenshots/riverbank/miniature-noon.png)

The missing bank fills with actual buildings. Water stays nearly black atnight;
large bare facades, sparse street activity and dominant transport decks remain
far from the locked concepts. These are evidence corrections, not a new visual
feature or artistic acceptance. No captured console errors/warnings. Diagnostics
before fixed framing reportedframep9518.4ms,CPUp9510.8ms,max21.5ms; not a
matched performance benchmark, loading measurement or complete performance gate.

Next primary assessment should use this full-density fixture for final-city
architecture/light/water judgment while preserving the723development gate.
Do not alter density/growth rules just to fill the earlier camera. A bridge-score
and pacing intervention requires a separately defined cross-seed baseline.
No production behavior changed. Goal active. Tab22leftpausedMiniature22preview,
full waterfront. No commit/push.

## Full-density reflection and camera isolation — 2026-10-12

Normal import of fully-realized.json: seed87231, tick75925,1839buildings,
clear, paused, Anime, High WebGPU,50mm,DOFoff,22:00preview. All four geometry
queues reached zero through ordinary scheduling. Viewport1280×720, existing
Hold waterfront view. The PR11 recheck yielded new external evidence (unchanged
head now conflicting with main); living-city work resumes here.

Captures in screenshots/waterfront-growth/:
- current-anime-night-matched.png: normal water at the fixed preset.
- current-anime-night-raw.png: same camera, raw reflection.
- current-anime-night-shallow-raw.png: lower orbit, raw reflection.
- current-anime-night-shallow-normal.png: unchanged lower orbit, normal material.

The high-angle probe sees mostly sky and bridge undersides even at full buildout.
Lowering the orbit reveals recognizable lit building reflections in raw mode;
restoring the normal material suppresses these into faint window specks.
Missing luminous scene content is therefore not a sufficient explanation.
Camera angle changes available content, and surface shading further attenuates
it. Source inspection agrees: reflector RGB is mixed into standard-material
colorNode, so reflected radiance receives surface lighting again. Earlier rejected
radiance trials already identified this mechanism; the new evidence supplies
a content-bearing angle for evaluating it.

A Luna source audit proposed a short promenade-lantern diagnostic. Defer it:
the lower-angle probe already proves lit geometry is available; extra fixtures
would conflate content and shading. No production code changed.

Lower orbit used a normal drag from (800,400) to (800,350) after the preset.
Camera coordinates were not exported; this pair is internally matched but is
not an exact reusable fixture. Establish an explicit lower review preset before
a candidate and retain the overhead gate. Future radiance work must preserve
irregular waves and reject pale mirrors, coherent bands and motion shimmer.
Prior failed gain/mip trials remain rejected; do not repeat them blindly.
Require noon/dusk/night, both styles and motion before promoting a candidate.

Captured browser warnings/errors empty. Normal water restored. These stills
are diagnostic evidence, not motion, hardware or artistic acceptance.
