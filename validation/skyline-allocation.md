# Skyline allocation audit and rejected early-cluster trial — 2026-10-12

The previous turn made progress by running the full-density city and identifying
skyline/composition as the next gap. This pass distinguishes missing towers from
camera framing, then tests and rejects a bounded allocator change.

## Baseline evidence

Seed 87231, original full-density fixture tick 75925: 1,839 open buildings,
only one tower (18 floors, office at 189.536 / -43.655, 371.19 m²). Root independently
restored the fixture in current WASM and confirmed the histogram:
1:40, 2:665, 3:618, 4:132, 5:168, 6:116, 7:27, 8:35, 9:7, 10:14, 12:16, 18:1.

The ignored native diagnostic `skyline_allocation_diagnostic` regenerates plans
using Engine geography. Its optional fixture join verifies seed, plan digest and
lot count, then maps plan lot → persisted parcel → building. Root inspected and
reran it successfully. `skyline-allocation.json` contains the evidence.

| Seed | Core lots | Non-flag core lots ≥280 m² |
|---|---:|---:|
|0|105|39|
|5|80|38|
|77|101|28|
|87231|59|19|

All 19 qualifying lots in the 87231 fixture have buildings; the station is online.
This proves geometric opportunity at maturity, not that every site was served
when first admitted. The original allocator uses a 20% roll at building admission,
requires station service and a qualifying site, and caps existing towers at eight.
It has no cluster target. Existing `renewal()` renovates vacant buildings after a
quiet period but does not raise their height. Height renewal and lot assembly
are already explicit, unimplemented requirements in spec 02.

The baseline station skyline capture shows more of the mid-rise district than
the shallow waterfront shot, but cannot reveal a tower cluster that does not exist.
See screenshots/skyline-allocation/station-miniature.png (Miniature, 50 mm,
DOF off, High WebGPU, 1280×720, clear, day 3 14:02, paused; live fixture advanced
from 75925 in the preceding interaction check, final exact tick not exported).

## Trial and rejection

Trial favored the first three qualifying served tower sites, then retained the
20% chance and eight-tower cap. It used the same single growth RNG draw, normalized
for floor/use selection, and preserved normal demand, access and construction
admission. It did not manufacture buildings in the renderer.

`scripts/audit-skyline-growth.mjs` runs fresh normal engine growth at development
rate 16 (an existing UI control), snapshots every 2,000 ticks through 76,000,
exports a normal save and compares the original with a restored engine for a
further 200 ticks. Baseline WASM was copied before the trial. All five runs passed
that limited replay comparison. Full records: skyline-growth.json.

| Run | Buildings at 76,000 | Towers | Early concern |
|---|---:|---|---|
|87231 baseline|1839|18 floors|One tower first observed at 10,000 ticks|
|87231 trial|1839|24,26,32,18|Three by 4,000 ticks, only 125 buildings|
|0 trial|1730|20,19,31,32|Four by 2,000 ticks, only 79 buildings|
|5 trial|1899|26,31,17,21,20,16,18,22|Three by 2,000 ticks, only 77 buildings|
|77 trial|1826|22,28,23,21,23,17,18,31|Four by 2,000 ticks, only 74 buildings|

Seed 5 was still developing at the endpoint (1,945 of 2,062 lots released),
so it is not a full-density acceptance fixture. All other listed endpoints
filled their plan. First-observed ticks are sampled upper bounds, not event times.

The trial city was loaded through normal Import into the browser; all rendering
queues reached zero without intervention. Miniature and Anime both render the
new cluster. Automatic skyline direction overlaps the three adjacent tall towers;
a manual orbit separates them. That is a separate camera-composition gap.
The fixed waterfront preset does not include these tower sites and is not a useful
acceptance view for this change. A temporary 1280×720 viewport override was reset.

**Rejected and reverted:** the better mature skyline comes with towers appearing
too early in a small settlement. This violates believable growth. Removed the
trial helper, minimum quota and four trial-only tests, restoring the original
inline 20% allocator. Retained the diagnostic, reproducible growth script and
experimental evidence. Do not reintroduce the quota or increase tower odds as a
shortcut. No production skyline improvement is claimed.

Candidate-only validation before rejection: 11 native masterplan tests passed,
1 long-run test ignored; 51 tower-kit/camera tests passed; WASM and production
build passed (existing warnings). These results are not proof of the full goal
and are recorded as trial evidence, not tests of a retained allocator change.

## Next bounded workstream

Implement gradual, causal height redevelopment on existing served station sites,
as required by spec 02. Start with a bounded scan and a single vacancy-safe
redevelopment project; preserve the founding anchor and character buildings,
parcel/access geometry, project budgets, history, construction visibility and
save/replay. Establish vacancy/demand/age and neighborhood-intensity baselines
before choosing eligibility thresholds. Do not move occupied residents without
real relocation capacity or silently replace a building at completion.

Acceptance needs fresh growth plus an aged mature city: no early forced towers,
visible staged redevelopment, coherent routing/occupancy, bounded site changes,
current-format replay while construction is in progress, and a readable mature
station cluster in both styles. Lot assembly is still in the overall scope but
should not be bundled into this first height-renewal slice.

No commit or push. Goal remains active.

## Restoration verification

Rebuilt WASM after reverting the trial. A fresh restored-code seed87231 run to
tick76000 produces a save byte-for-byte identical to the pre-trial baseline,
including its one18-floor tower; the200-tick replay check passes. Browser reloaded
the restored engine and the baseline save through normal Import. No trial city
is presented as retained work.
