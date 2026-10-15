# Neighborhood material response trial — 2026-10-13

## Decision

Reject the roughness-only candidate. At the held station-neighborhood view,
Miniature shows no useful scene-scale improvement; do not continue tuning these
scalars. Preserve the existing palette, daylight fill and roof/coping separation.
The rejected patch is `neighborhood-roughness-rejected.patch`.

## Trial

In the ordinary building kit only, wall/roof roughness changed from 0.85 to 0.68,
concrete to 0.96 and pale concrete to 0.8. Colors, geometry, exposure, sun and
hemisphere lighting were unchanged. All roughness values remained above the Anime
painted-surface threshold. The trial split material classes rather than adding
textures or authoritative simulation state.

## Evidence

The paused city export is `.tools/material-review-save.json`, seed 87231,
tick 86278, clear weather. Both baseline styles use the deterministic **View
station** control, first station 38, 50 mm, 1280×720, High WebGPU, DOF off, noon
lighting preview. After HMR, normal import and scheduler completion restored the
same world and framing. See the `roughness-*` images and metadata in
`screenshots/neighborhood-materials/`.

Production framing calculation gives camera (9.7895854,191.9653012,56.5157275)
and target (159.3511,8,-82.81525). This is an offline calculation of the production
method with no-op control updates, not browser readback. The screenshots visually
match architectural framing. The pre-export display showed 465 journeys and the
restored display 461; do not claim actor-frame equivalence. Rendering counts were
observed at different frames and are not a controlled performance benchmark.

TypeScript passed during the trial. Focused tests: 13 passed, 2 failed. The two
midrise tests select/compare materials by the previous roughness identity, so the
trial invalidated their lookups. Do not change those assertions to retain an
experiment with no demonstrated visible benefit. Re-run after restoring source.

## Next decision

One material-response attempt is rejected. Return to the locked concepts and
scene-scale architectural composition: repeated full-width facade rhythms and
roof silhouettes remain, while scalar specular response contributes little at this
view. Do not interpret rejection as proof that all material work is futile. A new
experiment needs a different visible mechanism and explicit predicted benefit,
not another set of roughness or ambient-fill values.

After restoring the exact pre-trial source, all 15 focused tests pass. Anime
candidate inspection also shows no useful neighborhood-scale benefit. No roughness
trial changes remain in production.
