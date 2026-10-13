# Walk-up core side elevation — 2026-10-12

## Deficiency and bounded correction

The full-density75925tick waterfront baseline shows large blank apartment end
walls. Code inspection traces these to the full-depth walk-up circulation core:
its front/rear had narrow windows, while its exposed X-facing side had none.
Main apartment windows correctly omit the core-attached side.

Added a restrained rhythm of narrow upper service windows only to the outer
core flank, with dark backing, existing glass and a shallow trim mullion. No new
emission, texture, material type, footprint or authoritative simulation change.
The hidden attached side remains solid. Count is one bay per roughly5mdepth,
per floor; each bay adds3boxes/36triangles. This is a service-elevation approximation,
not detailed circulation/interior modeling. Uses the same kit in both styles.

## Root verification

Root implemented the kit change. Luna supplied actual geometry tests; root fixed
an incorrect mirrored-inner-side test coordinate for negative-side cores before
rerunning. Focused core+midrise suites9pass, production buildpass with existing
bundle warning. Tests cover opposite seed parities, box-corner geometry, absence
on the mirrored attached face, parcel bounds and identical vertex data across
styles. Existing midrise tests cover the broader mass family. These checks do
not establish close-range aesthetic acceptance or city-wide performance.

Full1839save imported normally:seed87231,tick75925,clear,paused09:55,HighWebGPU,
50mmDOFoff1280×720,noonpreview,Hold waterfront view. All geometry queues0 before
capture. Startup graphics delay warning cleared without retry. Compare:

- [Before Miniature](screenshots/waterfront-growth/full-miniature-noon.png)
- [After Miniature](screenshots/core-facade/miniature-noon.png)

The large upper-left apartment wall and lower-left end wall now have readable
vertical window rhythms. Retain the correction without claiming the buildings
match the concepts. Materials, street-level activity, water and broader lighting
remain deficient. This pass does not warrant expanding into unrelated facade
ornament. Motion/close views/fallback and incremental GPU cost remain unmeasured.
No commit/push. Goal active.

Anime switch completed with same heldcity/camera; [noon capture](screenshots/core-facade/anime-noon.png)
shows corresponding service windows. No captured console warnings/errors. Tab22
leftpausedAnime/noon/full1839waterfront for subsequent review.
