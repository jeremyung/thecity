# Nearby real street lights — 2026-10-12

Status: working-tree prototype, not full lighting acceptance.

The enlarged additive ground pools were rejected in street-light-coverage.md.
This alternative adds four fixed, downward-facing SpotLights at actual ordinary
street fixtures. Unlike a pavement decal, these lights participate in material
lighting on curbs, walls and actors. They do not cast shadows, so occlusion and
light leakage remain explicit acceptance risks. No light is created per road.

`NearbyStreetLights` keeps four shader-visible lights for its whole lifetime.
It selects within90m of the camera focus every0.5s, favors retained fixtures by
20% to avoid tie churn, and fades old slots to zero before relocating. Full fade
takes0.5s at normal frame intervals. Night scales their output; reset/removal
extinguishes a source immediately. Source keys and physical positions are
deduplicated and the source list is capped at1000. Coordinates come from the
same instanced lamp-head layout,8cm below its center, aimed inward/downward.
Sources enter only when the corresponding staged road/fixture batch commits.
The previous eight shop lights remain; no shadow maps or textures were added.

## Verification

Root reviewed Luna's eight tests, fixed its discovered duplicate-position case,
and ran32 passing checks across nearby-street-lights, street-lighting and
road-batches. These cover the fixed light count, source cap/filtering, duplicate
positions, zero daytime output, fade-before-relocation, reset, source positions
relative to actual lamp geometry and surrounding road behavior. Build passes
with the existing bundle-size warning. Tests do not establish art or GPU cost.

Normal startup progressed to21buildings before pausing. Exported via ordinary
UI as `.tools/fidelity-review/nearby-lights-city21.json`: seed87231,tick1246,
clear/intensity0,day0 approximately07:14. The held founder camera uses50mm,
DOFoff,High,WebGPU,1280×720. Preview22:00 is lighting-only; underlying schedules
remain morning. [Anime](screenshots/nearby-street-lights/anime.png) and
[Miniature](screenshots/nearby-street-lights/miniature.png) share camera and state.

The candidate visibly lights the sidewalk beneath fixtures and responds to
nearby surfaces. The scene still lacks concept-level richness. No matched
lights-off comparison or actor crossing sequence was captured, so benefit is
provisional. Do not infer moving-actor lighting acceptance from the shader type.

[Diagnostics](screenshots/nearby-street-lights/diagnostics.png) show all geometry
queues empty,120frames,p9510.0ms,CPU p954.8ms,max13.5ms in this small paused scene.
This is not a before/after benchmark or evidence for mature-city cost. Startup
displayed its timeout notice; two native input calls timed out. A later DOM
Pause click succeeded, the city rendered, and the console inspection returned
no errors/warnings. Causation and startup cost remain unresolved.

## Next gate

Before retaining as a production improvement, compare a fixed street with the
lights on/off and moving actors, inspect occlusion at walls and beneath decks,
and profile the723building checkpoint in both styles/backends. Check actual
selection transitions during camera travel. Revert integration if visual gain
is marginal or added shading/compilation cost is disproportionate. Do not
extend to elevated roads or add further lights before that decision.

## Matched larger-city comparison — 2026-10-12

Added a development-only Renderer details button, `Nearby light contribution:
on/off`, to compare the contribution without resetting the world, camera or
selection. It changes intensity only: **the four SpotLights and their shader
paths remain present**, so off is not a performance baseline without the feature.

Normal Import restored seed87231/tick27925,723buildings,clear/intensity0, paused
day1 09:55. Preview22:00, Miniature, High, WebGPU,50mm,DOFoff,1280×720. Explore
held the post-import camera. Captured [on](screenshots/nearby-street-lights/city723-on.png)
and [off](screenshots/nearby-street-lights/city723-off.png), changing only the
development contribution toggle between them. No camera gestures or world
steps intervened; presentation animation is not a frozen image comparison.

The center street's curbs and lower facade receive a visible warm wash on the
on image. This is a localized improvement, not a transformation of the night
city. No obvious elevated-deck wash is visible in this pose; that does not
prove occlusion correctness elsewhere or moving-actor behavior.

[Settled diagnostics](screenshots/nearby-street-lights/city723-diagnostics.png):
all geometry queues0,120frames,frame p9515.4ms,CPU p9512.7ms,max18.5ms. A later
on sample was15.7ms; off reported15.0ms but includes the same light shader paths
and cannot establish marginal GPU cost. No named-hardware benchmark was run.
Build and UI/selector checks pass; a contribution-toggle regression verifies
off leaves positions and light count unchanged. The mature Miniature visual
gate now has direct A/B evidence. Fallback, Anime mature A/B, selection motion,
actor crossings, occlusion and a true no-light-cost comparison remain open.

## Fresh startup style and orbit observations — 2026-10-12

Recovered the pending UI/selector test process: 27 tests passed in two files.
The previous retained tab was absent from browser inventory. Opened tab21 via
normal localhost startup, paused the settlement, selected Explore and preview22:00.
Viewport1280×720, DOFoff. Saved
[Miniature](screenshots/nearby-street-lights/founding-miniature.png),
[settled Anime](screenshots/nearby-street-lights/founding-anime.png), and
[Anime orbit](screenshots/nearby-street-lights/founding-orbit.png).
The initial Anime capture was taken during preparation and replaced only after
the UI confirmed Anime selected. No warnings/errors were returned at that point.

The style comparison holds the paused world and camera; the final orbit used
drag(640,350)→(830,425). Both styles render local illumination. Night style
distinction remains weak. The Anime orbit reveals angular pavement illumination
near the shop: attribution to the new street lights versus existing shop lights
or stylized material response is unresolved. Do not add more lights or tune
exposure to hide this. A held-camera contribution comparison at that angle is
the next useful isolation check. No continuous-motion recording was collected.

Before checkpoint export, the tab unexpectedly became about:blank. Seed/tick,
exact camera and renderer settings were not recovered, so these are appearance
observations, not reproducible acceptance evidence or a performance benchmark.
No production source changed in this follow-up. The prototype remains provisional.

## Mature Anime comparison and waterfront scope — 2026-10-12

Previous goal turn progressed integrated CPU/browser verification. Re-read the
locked Last Train concept. Held tab22 seed87231tick27925,723buildings,clear,
underlying09:55paused,22:00preview,50mmDOFoffHighWebGPU1280×720.
[Anime on](screenshots/nearby-street-lights/city723-anime-on.png) and
[Anime off](screenshots/nearby-street-lights/city723-anime-off.png) change only
the contribution toggle at the held post-import Explore camera. Little visual
benefit is apparent at this city-wide framing. This does not contradict the
closer curb/facade response previously seen, but fails to establish city-wide
lighting improvement. Do not increase the light count to chase the concept.

All queues0; on120frames p9515.3ms,CPU p9511.9ms. Anime preparation reported
109/109samples,1308ms submit,16999ms await. This long style preparation needs
a matched no-feature baseline before attributing cost to the prototype. Off
retains shaders and is not that baseline. Restored contribution on, then used
Hold waterfront view (deterministic preset).
[Waterfront](screenshots/nearby-street-lights/city723-waterfront-night.png) shows
local street washes but a nearly black river and large infrastructure dominating
the composition. It contains little of the concept's warm, dense riverfront.
The comparison now closes the missing mature Anime still-image observation;
actor crossings, selection motion, occlusion, fallback and marginal cost remain
open. Keep the feature provisional and do not expand it.
