# Ground streetlight coverage — 2026-10-12

**Current decision:** retain width/base/duplicate-fixture corrections; the
enlarged-pool experiment below was rejected after the playback review. Production
now uses the original radii formula min(1.8 m,17% of actual width) and 4 m
longitudinal radius. The body below records the experiment, not shipping status.

The held night review in `roof-surface.md` showed bright fixtures with little
readable pavement illumination. This pass first audited the existing lighting
implementation rather than adding dynamic lights or another glow system.

## Concrete corrections

- Ground light-pool code assumed local roads were 9 m wide (actual 6.5 m),
  allowing the outer footprint to extend 1.17 m beyond asphalt. Both placement
  and pools now use `roadWidth`.
- Local poles used the .475 m arterial sidewalk base despite standing outside
  a shared-surface local street. Their base now meets terrain at zero; arterial
  poles retain their .475 m sidewalk base.
- `roadKit` also emitted an older set of ordinary-road lamps every 44 m,
  independently of the shared instanced kit. That legacy path is now bridge-only.
  Bridge and expressway fixtures remain unchanged.
- The existing soft pool grows to a transverse radius of min(4 m, 40% of actual
  road width), with a 6 m longitudinal radius. Its outer edge stays .08 m inside
  asphalt; existing 10 m fixture endpoint clearance contains the full patch.
  Existing night uniform, color, .2 opacity ceiling and falloff are unchanged.

The shared kit remains capped at 1,000 placements and four instanced meshes.
No extra textures, lights, shadow maps or simulation state were added. Wider
transparent footprints increase overdraw; no GPU performance improvement is
claimed. These pools approximate pavement illumination only: they do not cast
light or shadows onto people, vehicles or facades.

## Verification

Root independently confirmed the width and local-base findings from Luna's
read-only audit, reviewed its test edits, and ran 24 passing checks across
street-lighting and road-batches. The prior pool test hard-coded the incorrect
9 m width; its first run failed after the candidate as expected. Updated tests
use the real road width and inspect transformed pool vertices against lateral
and endpoint boundaries, including a diagonal arterial, and local/arterial pole
base heights. `npm run build` passes with the existing large-bundle warning.

Normal Import loaded seed87231/tick27925, 723 buildings, clear weather, paused
day1 09:55. Lighting preview22:00, WebGPU, High, 50 mm, DOF off,1280×720.
An initial HMR startup timeout notice resolved before import without Retry.
Overview was allowed to settle, then Explore and one upward scroll page at
(620,380) produced [district candidate](screenshots/street-light-coverage/night-anime.png).
Two more upward pages produced the close pair:
[Anime](screenshots/street-light-coverage/close-anime.png) and
[Miniature](screenshots/street-light-coverage/close-miniature.png).

Warm illumination is now visible in the ground streets between buildings.
The large elevated expressways still lack local pavement illumination and
dominate the composition. The close pair shares the same held camera/time;
the older roof-review baseline was not exported as an exact camera pose and
is not a pixel-matched comparison. Daylight fade, fallback, continuous motion
and actual night schedules remain unaccepted. Retain the placement corrections
and bounded coverage candidate; do not call the complete night scene accepted.

Next: review daylight fade and a close ground-street view with moving actors,
then address the elevated-road lighting gap using the actual curved deck
geometry. Do not increase global exposure or add lights per fixture.

## Daylight and growth review; enlargement rejected

The next turn checked noon at the held close camera in
[Miniature](screenshots/street-light-coverage/noon-miniature.png) and
[Anime](screenshots/street-light-coverage/noon-anime.png). Warm pavement patches
were absent in both daylight samples. The simulation then resumed at1× while
keeping22:00 lighting preview (development pace retained16× from the save).
The visible building count rose from723 to766; field notes reported a local
street opening and new houses. An orbit drag from(650,360)to(735,405) exposed
the [near-overhead playback view](screenshots/street-light-coverage/playback-orbit.png).
This is a sampled view after motion, not a continuous recording or proof of
shimmer-free animation. The final tick was not exported before source HMR.

That view changes the art decision: the enlarged pools read as repeated warm
oval decals, with little corresponding illumination of curbs, people or walls.
Visibility alone is not fidelity. Reverted the radius enlargement, while
retaining use of authoritative road widths, grounded local poles, and removal
of duplicate fixtures. Tests retain transformed-vertex containment checks,
with dimensions reflecting the restored footprint. No new texture or light
system was added.

Next approach should test a small fixed pool of nearby real street lights
selected from the same fixture placements, fading selection changes and
preserving bounded shader cost. Existing distant pavement approximation may
remain restrained. Do not continue expanding/brightening additive ellipses or
copy the rejected treatment onto expressways. Compare actual curb/vehicle
response and frame cost before promoting another lighting change.
