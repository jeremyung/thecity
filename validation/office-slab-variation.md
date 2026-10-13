# Office silhouette variation — 2026-10-12

Previous goal turn made progress by rejecting and reverting an isolated water
trial with captured evidence. Primary stream now architectural fidelity.

## Deficiency and intervention
Every eligible 4–11-floor office used the same two-floor podium, narrowed shaft
and two-floor crown. This repeated stepped profile is visible in the full-city
low waterfront baseline. Existing core-side window correction remains intact.

A seeded subset (unsigned appearance seed modulo3 equals0) now uses its full
existing body envelope for floors0 through floors-2, with only the final floor
inset. The top width is85–90% and depth86–92%, with small contained offsets.
Remaining offices retain the previous profile. Apartments, anchors, towers,
authoritative floor counts, lots and simulation rules are unchanged.
No new material, texture or rendering pass. Building-massing family identity
remains setback-office; the existing facade/roof builder consumes both variants.

## Verification
Luna implemented only building-massing.ts and its tests. Root inspected the diff
and independently ran massing, midrise geometry and core-facade suites:17passed.
Tests cover deterministic variation, floor coverage without gaps/overlap,
support, envelopes, narrow rendered parcels and shared-style geometry.
Production build passes with the existing bundle warning.

Normal import of fully-realized.json:seed87231,tick75925,1839buildings,
clear,paused,HighWebGPU,50mmDOFoff,1280×720,Hold low waterfront view.
All geometry queues zero before framing. Compare the previous
screenshots/waterfront-growth/low-preset-anime-night.png against
office-slab-low-night.png in that directory. The broad straight-sided office
left of center adds a distinct vertical profile among the stepped offices.
Also captured office-slab-low-noon.png and office-slab-miniature-noon.png.
Miniature switch completed at the same camera/time. Captured logs empty.
Transient startup notice cleared without retry before import.

Retain provisionally as a modest silhouette improvement, not concept acceptance.
Miniature capture has conspicuous fine edge noise that requires separate AA/
resolution review; neither motion nor fallback acceptance is established here.
More facade richness, street-level activity and material differentiation remain
necessary. Added facade surface may increase geometry despite fewer mass groups;
no measured GPU or memory improvement is claimed. Close construction transitions
and exact incremental cost remain next checks before broad acceptance.

## Independent cost and construction geometry — 2026-10-12

Previous turn implemented and visually compared the candidate. This turn adds
production-kit cost evidence and construction floor-plate coverage.

Luna authored scripts/audit-office-slab-cost.mjs; root inspected and reran it,
pinning baseline revision8e9f5e196659c29c9c6a5b9dcd6bed43fd1de2f7 rather
than mutable HEAD. It verifies that removing only the slab branch reconstructs
the baseline helper, overrides only that helper in esbuild, and keeps all other
current kit code identical. It restores the normal full1839 WASM save, records
input/canonical-save hashes and disposes generated geometry/materials.

Across126 eligible office kits:365064→374016triangles (+8952,2.45%);
25425456→26052096geometry-array bytes (+626640,2.46%);1455meshes in both.
58kits change. Largest absolute per-building delta828triangles/57960bytes.
This is unbatched authored CPU geometry accounting, not total browser/GPU
memory, frame time or production draw-call count. See office-slab-cost.json.

Root added a geometry regression covering4/8/11floor slab offices in both
styles: concrete plates keep the full lower-body footprint until the single
inset top plate; every tested structure stays in its parcel. Root massing and
midrise suites17pass. Earlier core-facade coverage remains unchanged.

Source inspection identifies an outstanding continuity defect shared by old and
new midrises: renderMassingConstruction installs generic panel/window groups,
while open buildings use renderUrbanFacade. Tower construction already uses
its authored facade during late cladding. The slab structural outline is
consistent, but these tests do not establish midrise finishing-to-open facade
continuity. Next bounded change should reuse authored midrise facade geometry
during late construction, preserving progressive floor admission and a clear
construction site, then verify normal saved-job playback. Do not add more
massing variants before this continuity gap is addressed.
