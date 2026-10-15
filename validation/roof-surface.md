# Roof surface composition — 2026-10-12

## Deficiency and intervention

The current 723-building city has substantial massing but repeated pale flat
roofs, plain facades and a much weaker waterfront composition than concepts
04 and 01. Both locked sheets were inspected. Additional texture generation
was rejected as the next step: the prior concrete pilot added grain without
solving architectural composition.

Source inspection found that `renderFlatRoof` covered its entire dark roof
slab with a pale trim slab. The bounded candidate uses the existing roof
material for that surface and puts trim on the parapet coping instead.
Existing service structures, tanks and pipework remain. This adds four boxes
(48 triangles) per affected roof, uses existing materials, and adds no textures
or simulation state. Other roof families are unchanged.

## Browser evidence

Normal Import restored `.tools/movement-support/browser/growth-723.json` while
paused: seed 87231, tick 27925, 723 buildings, day 1 09:55, clear weather.
WebGPU, High detail, 1280×720, 50 mm, DOF off. No scheduler bypass.

- Initial [Anime baseline](screenshots/roof-surface/before-anime.png) and
  [Miniature baseline](screenshots/roof-surface/before-miniature.png).
- Candidate [Miniature](screenshots/roof-surface/candidate-close-miniature.png)
  and [Anime](screenshots/roof-surface/candidate-close-anime.png).

The candidate pair holds the same camera and city time. It was reached through
Overview, Explore, then one upward scroll page at (620,380). The initial
baseline stopped the 16-second overview transition earlier: **baseline and
candidate poses are not matched**. These images establish visible roof material
separation in the actual city, not a controlled whole-scene fidelity comparison.
The roofs now have distinct dark fields and pale parapets, with equipment more
legible against the field. Retain as a small candidate correction, not artistic
acceptance or a substitute for the larger facade/composition work.

Renderer details reported zero building, neighborhood, sidewalk and road queues
after the Anime switch. The settled 120-frame sample reported frame p95 16.9 ms,
CPU p95 15.2 ms, maximum 20.8 ms; this is not a hardware benchmark or a before/after
performance claim. Style switching still took seconds. No motion, fallback,
night or growth acceptance follows from this paused daytime review.

## Verification and next decision

`npm run build` passes (existing large-bundle warning). Four existing suites,
29 tests, pass: building-massing, building-chunks, extended-building and
building-outlines. These cover surrounding geometry/batching behavior, not
visual quality. No new tests merely mirror this material substitution.

Before stacking rooftop details, obtain an exact held-camera comparison and
check dusk/night readability; revert the candidate if the dark fields flatten
the silhouette or lose equipment readability. Broader priorities remain
architectural variety/street character, waterfront structure and lighting,
and measured growth hitches. Do not restart isolated road or shrub tuning.

## Dusk and night follow-up — 2026-10-12

The next turn held the existing candidate camera and simulation checkpoint,
using the visible lighting slider to preview 18:00 and 22:00. All other settings
remain as above. Captured [Anime dusk](screenshots/roof-surface/dusk-anime.png),
[Miniature dusk](screenshots/roof-surface/dusk-miniature.png),
[Miniature night](screenshots/roof-surface/night-miniature.png), and
[Anime night](screenshots/roof-surface/night-anime.png). The clock explicitly
reports LIGHT PREVIEW; these are not simulated evening schedules. No console
warnings/errors were returned at the final inspection.

Pale parapets and service masses remain distinguishable from dark roof fields
at both hours. Retain the small correction; no further rooftop ornament is
justified by this view. This closes the candidate's sampled dusk/night
readability question, not the unmatched baseline comparison, motion, fallback,
or full artistic acceptance.

The larger scene is still substantially below both locked targets. At 22:00,
streetlights mostly read as isolated points with little apparent illumination
on the road surface; window grids supply most of the visual activity, and the
two styles are too similar. The river is a dark empty band in this view.
Next lighting investigation should tie restrained surface illumination to
actual lamp placement and preserve plausible light direction, finite browser
cost and style-specific treatment. Avoid another global exposure increase or
generic bloom pass: neither would establish believable local light sources.
