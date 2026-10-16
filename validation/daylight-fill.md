# Daylight facade fill — 2026-10-12

> Historical experiment. As verified on2026-10-15, current source uses
> `1.05 - daylight * .27`, with Miniature hemisphere intensity multiplied by0.78
> and a sky environment at `0.1 + daylight * .16`. The0.5→1.0 trial below is
> not the current renderer setting. Consult HANDOFF.md for active comparisons.

## Decision and scope

Retain a provisional one-parameter lighting adjustment: hemisphere intensity
changes from `1.05 - daylight * .55` to `1.05 - daylight * .05`. Fully daytime
fill rises from0.5 to1.0. At daylight0 the value stays1.05; intermediate dawn/dusk
values also change and still need matched review. Sun, exposure, material colors,
fog and shadow settings are unchanged. No additional lights, textures or passes.

Deficiency: shaded facades in the camera study read almost black, losing window
and wall detail. The visible sky mesh does not supply environment illumination.
Acceptance for this bounded trial: improve shaded-wall readability while retaining
sunlit/shaded orientation and cast-shadow structure in both rendering styles.
Reject if the matched images become flat or lose architectural separation.

## Matched browser evidence

A fresh in-app browser tab recovered image capture after tab29 returned zero-width
screenshots. These are new matched pairs at1280x720, not comparisons against the
older847x764 Watch images. Normal paused import of seed0/tick9250 checkpoint
`after-opening.save.json`;260 buildings,86 journeys,7 works; review building1814,
50mm, High, WebGPU, DOFoff, live15:15, unchanged checkpoint weather. Frame review
building gives the deterministic held pose. All four scheduler queues reached zero
before capture. Candidate required reimport after HMR; same framing action used.

Files in `screenshots/daylight-fill/`:
- `anime-baseline.png` / `anime-candidate.png`
- `miniature-baseline.png` / `miniature-candidate.png`

Root visual review: shaded walls, horizontal facade bands and window frames are
more legible in both candidates. Bright coping and roof surfaces remain distinct;
cast shadows still describe the street and roof geometry. Anime surface treatment
and Miniature remain visually different with DOFoff. The improvement is modest;
large repetitive facades, sparse street activity and weak material richness remain.
This is not locked-concept acceptance or a hardware performance benchmark.

TypeScript passes. A pure lighting scalar introduces no geometry or simulation
behavior requiring new unit tests. Dawn/dusk, moving views, founding/mature scenes
and full daily-cycle acceptance remain open. Keep the change provisional until
those checks and user artistic review; do not compensate for remaining material
or activity deficiencies by repeatedly raising fill.

## Daily lighting spot checks

Retained-candidate captures now include Anime and Miniature at06:00,18:00 and22:00
lighting preview, with the same paused seed0/tick9250, deterministic building1814
framing,50mm,High,WebGPU,DOFoff,1280x720. Files are
`{anime,miniature}-{dawn,dusk,night}-candidate.png` in the same screenshot folder.
These are candidate-only checks, not baseline/candidate pairs at those hours.
Normal import queues reached zero before framing; no source changes this turn.

Root inspected all six images. At dawn/dusk, walls and horizontal framing remain
readable, lit edges and shadowed roof fields remain distinct. At22:00, roof fields
darken while storefront pools and occupied windows remain legible. No broad
washout is apparent in this particular view. This does not prove full daily-cycle
continuity, camera motion or night equivalence. At daylight0 the fill endpoint is
mathematically unchanged; the screen captures are not a pixel-equivalence test.

Close this bounded fill tuning pass with the scalar retained provisionally. Do not
keep increasing ambient brightness: current imagery still lacks the locked04
concept's varied facade detail, lively frontages and connected water/transit/street
composition. Next inspect a broader living neighborhood and prioritize the largest
remaining gap there. This office close-up is useful for lighting comparison but is
not an artistic acceptance composition. Full motion, mature/founding and user
artistic gates remain open.
