# Compact midrise envelope study — 2026-10-11

**Development candidate only; not promoted to the default renderer.** The live geometry study is available with `?midrise=compact` on the Vite development server, including the architecture review. Production builds ignore that URL flag. Both styles use the same candidate geometry and authoritative world. Normal building dimensions and all simulation/save data remain unchanged.

The study tests a specific rendering gap: wider apartment and office parcels use an arbitrary 84% body width/depth, leaving proportional setbacks even where a compact service edge would suffice. Only non-anchor apartments/offices with 4–11 floors and parcel dimensions at least 8 m are eligible. Body width becomes max(existing width, parcel width−1.6m); depth becomes max(existing depth, parcel depth−1.8m). Narrow parcels keep their current body rather than shrinking to the new target. This preserves about 0.8 m side and 0.9 m front/rear clearance on wider lots, with the existing 0.64 m clearance retained at 8 m. Anchors, low-rise families and towers retain their existing envelopes.

The construction crane's proportional jib was identified as the limiting projection: its outer end reaches 0.59×body width. Its new parcel-aware limit keeps the jib within the lot when the candidate widens the body. Existing facade and balcony clearance logic is retained. No trees, fake buildings or occupied-lot moves cover empty space.

## Measured result

Same seed 87231, tick 242000, 850 buildings. Save and snapshot building arrays are identical. 139 buildings meet the basic midrise eligibility. The coverage audit clips actual kit triangles above local y=1 m and measures their projected union on a 0.5 m grid. It includes roofs/eaves/props; it is not legal building footprint, usable floor area or net developable-land coverage.

| Measurement | Default | Candidate |
| --- | ---: | ---: |
| Projected kit union |122,736.25m²|124,077.25m²|
| Full 800×800 m world coverage |19.1775%|19.3871%|
| Projected union / nominal parcel union |76.6763%|77.5140%|
| Authored building triangles |2,135,638|2,152,958|
| Counted retained CPU geometry arrays |219,495,028bytes|221,420,044bytes|

The candidate adds 1,341 m² (+1.09%) projected coverage, 17,320 triangles (+0.81%) and 1,925,016 counted bytes (+0.88%). Nominal parcels are unchanged. These small citywide gains rule out midrise body widening as a solution to the large undeveloped interiors. It may improve street-wall continuity, but that remains a visual hypothesis. Counted arrays exclude textures, browser heap overhead, transient peaks and GPU memory; no frame-rate claim follows.

## Reproduction

From repository root:

```sh
node scripts/audit/audit-rendered-coverage.mjs --input validation/early-reservation-wasm-87231-save.json --output /tmp/default-coverage.json
node scripts/audit/audit-rendered-coverage.mjs --input validation/early-reservation-wasm-87231-save.json --output /tmp/compact-coverage.json --compact-midrise
node scripts/audit/audit-mature-building-geometry.mjs validation/early-reservation-wasm-87231-snapshot.json /tmp/default-geometry.json
node scripts/audit/audit-mature-building-geometry.mjs validation/early-reservation-wasm-87231-snapshot.json /tmp/compact-geometry.json --compact-midrise
```

The four sibling JSON reports retain input and transitive source hashes. They measure generated kit geometry, not browser screenshots.

## Verification and promotion gate

Root's focused verification passes 34 tests across midrise massing/kits, low-rise/tower regressions and coverage math. The candidate sweep covers 360 kits: two uses, four parcel sizes 8×8/14×12/24×19/40×32, three floor counts, three appearance seeds and five construction/open/renovation cases. Bounds remain finite and inside the parcel; both styles produce identical vertex positions in the checked specimen. The entrance-glazing regression runs both default and candidate envelopes. TypeScript and production Vite build pass with the existing large-bundle warning. The emitted city renderer contains `compactMidriseReview=false`, confirming that the URL study is disabled in production.

The full unrestricted Vitest run finished with 264 passed and two timeouts (one low-rise geometry check and one short WASM growth test). With two workers and unchanged time limits it finished with 265 passed and one timeout in the 300,000-tick WASM endurance test; that endurance test had passed in the first run. No assertion mismatch was reported. The isolated endurance rerun also exceeded its 60-second limit (62.15 seconds test duration, 62.27 seconds total). These runs are not a clean full-suite pass; timing variability and the endurance timeout remain validation limitations. The renderer study does not change the WASM binary used by that test.

Runtime visual review is still pending: the in-app browser responds to controls, but both screenshot APIs returned “Unable to capture screenshot” while the Mac was locked. Do not claim visual acceptance or promote this candidate on the strength of geometry tests. Compare matched camera/light/scene views at noon, dusk and night in Anime and Miniature; inspect entrance clearance, narrow side courts, crane scale and roof repetition. Review both a close street and the mature city after geometry queues drain, then measure browser timing/residency. Reject or revise if the denser massing reads as generic inflated boxes.

The main density work remains upstream parcel/block layout and connected infill. Artistic variety, water, lighting and full-scene quality remain independent unfinished requirements.
