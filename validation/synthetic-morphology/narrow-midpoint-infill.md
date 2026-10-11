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

## Render review follow-up — 2026-10-11

The optional output-directory argument to `verify-narrow-frontage-wasm.mjs`
exports the final save and snapshot for browser review:

```
node scripts/verify-narrow-frontage-wasm.mjs .tools/narrow-frontage-review
node scripts/audit-rendered-coverage.mjs --snapshot .tools/narrow-frontage-review/final-snapshot.json --output validation/synthetic-morphology/narrow-final-coverage.json
```

The recorded report measures the kit **before the entrance correction below**.
Its bundle hash matches `../compact-midrise-review/baseline-coverage.json`: projected
coverage increases from 122,736.25 to 124,446.75 m² (+1,710.50 m², about 1.39%).
This compares the full continuation, including renewal and all 21 additional
buildings; it does not isolate the 18 narrow buildings' contribution. This is
a CPU projection at 0.5 m sampling, not a rendered screenshot or performance result.

Inspection found five new two-/three-floor apartment/office instances lacked
visible doors in the generic urban facade. That path now reuses the midrise
entrance and reserves its window opening; parcel bounds also constrain balcony
projection. The 8 m gable trim had a 4.5 cm lateral overrun, now bounded to the
parcel width. The regression uses all five actual archetype/seed combinations
and checks door glazing, parcel containment and matching geometry in both styles.

A headless WebGPU attempt loaded the final 871-building world, but yielded no
frame metrics and timed out during screenshot capture. No new visual acceptance
is claimed; the last usable screenshots still show the earlier 850-building city.

After the entrance/containment correction, `narrow-entrances-coverage.json` records
124,413.25 m² with the same final snapshot (33.50 m² less projected kit geometry
than before the correction). This includes changes to all affected existing
low urban facades, not just newly added buildings. Root independently passes
19 lowrise/midrise tests and the production build/typecheck; the existing large
bundle warning remains.
