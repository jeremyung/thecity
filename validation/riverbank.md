# Continuous engineered riverbank — 2026-10-12

Previous goal turn progressed a bounded reflection trial and rejected it. Its
matched views identified the repeated ico shoreline as a separate geometry defect.
This pass replaces that fringe and independently rotated segment boxes with
continuous wall, coping and promenade sweeps. No water shader change.

## Implementation and root review

`riverbank.ts` follows the same world-X-offset boundary as land/water, using
identical stations at bends rather than perpendicular offsets. Flat outward
normals and UVs preserve shared material batching. Only global endpoints are
capped; interior stations have no overlapping end faces. Luna implemented the
module/tests; root requested correction of initially missing global caps, reviewed
winding, added a welded-edge manifold regression and integrated the terrain call.

Wall: boundary offsets0–1.4m,y−1.3–.04m; coping0–1.6m,y.04–.18m;
promenade5.5–10.5m,y−.02–.10m. Existing5m promenade width, railings, trees,
benches, water boundary and simulation reservations are retained. Coping and wall
use an engineered treatment; this does not implement natural sloped stone banks
visible in parts of the concept board. Geometry per component is8 triangles per
river segment plus4global-cap triangles. No new textures/material types/passes.

## Verification

Root:10focused tests pass (riverbank4,water lifecycle1,ground surfaces5).
Tests check both sides, sharp bends, actual triangle winding, no water intrusion,
finite bounds, caps only at global ends, and welded-edge counts of2 throughout.
Production build passes with existing large-bundle warning. No Rust/WASM change.

Browser:normal import growth-723.json,seed87231,tick27925,723buildings,clear,
paused09:55,HighWebGPU,50mmDOFoff,1280×720. All geometry queues settled to0
through production scheduling. Existing Hold waterfront view restored framing.

- [Before,Anime noon](screenshots/water-rough-reflection/restored-noon.png)
- [After,Anime noon](screenshots/riverbank/anime-noon.png)
- [Anime dusk18:00](screenshots/riverbank/anime-dusk.png)
- [Anime night22:00](screenshots/riverbank/anime-night.png)
- [Miniature noon](screenshots/riverbank/miniature-noon.png)

The repetitive tooth silhouette is removed and the continuous bank edge is
readable. Retain this narrow geometry correction. Night water remains nearly
black, daytime reflection flat, riverfront sparse, transport decks dominant.
No concept-fidelity acceptance is claimed. No captured console errors/warnings.
Motion, closer bend views, fallback and other seeds remain unverified visually;
geometry regressions support continuity but do not replace those checks.
No performance claim from these captures. No commit/push. Tab22 remains paused
Miniature/noon/waterfront for next review. Goal active.
