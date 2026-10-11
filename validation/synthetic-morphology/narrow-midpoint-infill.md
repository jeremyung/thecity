# Narrow midpoint infill — 2026-10-11

Production now appends an8×12m candidate at each existing frontage-grid position,
after the previous grid, gap and shallow candidates. The existing minimum8m
width is preserved. No parcel, road, river, tower or capacity clearance is relaxed.
This is an enumeration fix, not a7m lot policy or a new district density target.

## Concrete witness

On mature seed87231 road2985, the production generator previously emitted four
candidates. Its16×16m midpoint and shifted10.10m-wide gap candidates overlapped
existing parcels under the exact2m separation rule. The8×12m midpoint at
(140.85898,−214.86212), with connected access node2952, passes unchanged geometry,
access, tower and local/global activity checks. Both7m and8m widths fit, so the
7m fixed-grid experiment did not justify reducing the minimum.

The focused regression verifies this shape now appears, sets the persisted scan
cursor to it, and calls ordinary `try_progressive_frontage_infill`. It admits a
parcel with correct frontage metadata, validates the world, starts construction
through ordinary80ticks, and preserves byte-identical save/resume. Cursor targeting
isolates the regression; the separate WASM continuation below does not override it.

## Bounds and compatibility

At most34 extra shapes are appended per road, deduplicated by position/size.
The per-road bound rises66→100. The per-evaluation budget remains48sites and
24roads. Existing candidate order is retained as a prefix; new candidates can
lengthen a full scan. Candidate-list preparation includes bounded deduplication
outside the site-visit counter, so unchanged counters do not prove unchanged
worker time. Browser profiling remains necessary.

No save schema changes. Existing cursor values remain valid; rebuilt engines
can evolve differently after loading the same saved city, as intended by this
new development opportunity. Same-version replay is verified. Existing building
and parcel capacities, construction sequencing and global caps are unchanged.

## Validation

- Full native release suite:369passed,0failed,45ignored (49.18s test time).
- Focused admission/save-resume/construction regression independently rerun.
- Existing shallow-infill WASM regression passes (1test,2.16s).
- WASM rebuilt successfully. `verify-narrow-frontage-wasm.mjs` continues the exact
  mature save for30,000ticks at rate16 through normal scheduling:850→871buildings,
  including18new8×12m buildings, all open by the final checkpoint.
- Restored at15,000ticks, then compared exact saves every200ticks for the remaining
  15,000ticks. All match. Source/WASM/final hashes and checkpoints are in
  `narrow-frontage-wasm.json`.
- The fixed-grid diagnostic (`compact-frontage-witness.json`) checks crop/full-world
  proposals and retains source state. It excludes the actual gap fallback list and
  is not an exhaustive opportunity census. The compact crop has one fitting lot
  per compact recipe, not a two-lot same-host witness.

No rendered comparison, browser timing improvement or overall Tokyo art acceptance
is claimed. This is a bounded real infill improvement; it does not close the
roughly20percentage-point compact-reference coverage gap or solve street layout.

Reproduce:

```
node scripts/test-engine.mjs --release compact_frontage_width_comparison_at_crop_host -- --nocapture
COMPACT_FRONTAGE_REPORT=/tmp/frontage.json node scripts/test-engine.mjs --release compact_frontage_existing_road -- --ignored
npm run wasm
node scripts/verify-narrow-frontage-wasm.mjs
```
