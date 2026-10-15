# Office side facade candidate — 2026-10-13

## Deficiency and change

Office fronts already have broad ribbon glazing, but the shared side-elevation
loop uses residential punched windows. In the deterministic station view those
side walls dominate several buildings, weakening the distinction between housing
and offices. The candidate carries the existing office ribbon proportions,
mullions and spandrels around both side elevations. Existing colors/materials,
entrances, setbacks, floor counts and authoritative state are unchanged.

Only sides with at least 0.24 m clearance receive the new treatment. Narrower
sides retain their prior geometry. Construction uses the existing admitted-floor
path, so bands appear only on clad floors. No new textures or material families.

## Verification so far

TypeScript and all 15 focused midrise geometry/cladding tests pass with the final
clearance guard. A new actual-mesh ray test checks
ribbon glazing across both sides in both styles, plus parcel containment.

The production-mesh comparison in `office-side-facades.json` checks all open
non-tower offices in three ordinary snapshots: 173 / 173 / 211 offices. Final
candidate adds 43,080 / 39,336 / 46,560 triangles in aggregate. This is authored
geometry cost, not GPU timing. The audit substitutes only tenant-sign canvas
materials for Node execution; sign geometry remains. Baseline/candidate source
and input hashes are recorded. The candidate patch preserves the exact change;
its reverse can reconstruct the pre-trial kit for rerunning the audit.

**Existing correctness finding:** 30 / 29 / 64 complete office meshes cross their
local parcel boxes in baseline and candidate. The final guard introduces or worsens
none of the checked overhangs. This is not complete parcel-containment acceptance:
the existing narrow-parcel/minimum-body-size issue remains open. Report includes
actual IDs and before/after bounds; investigate before declaring full access or
neighbor clearance.

## Browser evidence and remaining work

The initial Anime candidate is captured at seed87231/tick86278 saved world,
50 mm, High WebGPU, 1280×720, noon preview, DOF off, deterministic View station.
Normal geometry queues reached zero. Compare
`screenshots/office-side-facades/anime-candidate.jpg` with the unchanged-material
`screenshots/neighborhood-materials/roughness-baseline-anime.jpg`.
Ribbon sides are visibly distinct on offices at the left and lower-right of this
neighborhood. The initial image predates the final narrow-side clearance guard;
it is supporting evidence, not final-candidate acceptance.

Final both-style captures are recorded below; ordinary motion and developing-state
visual checks remain required. Broader three-seed appearance, actual daily cycle, hardware
performance and user artistic gates remain open. Keep this as a candidate until
those bounded checks; do not add further facade variants meanwhile.

## Final guarded candidate still review

Normal import reached zero queues, then deterministic View station restored the
same composition. Final `anime-final.jpg` and `miniature-final.jpg` now include the
clearance guard. Both show clearer office-specific horizontal glazing on the left
and lower-right facades compared with the original-material baselines. Colors and
lighting remain fixed. This is a visible bounded benefit, not whole-scene artistic
acceptance. Retain as a candidate for motion/developing review; do not add variants.

## Sampled ordinary motion — 2026-10-13

Recorded 20-second canvas clips with normal simulation at 1×, 50 mm, High WebGPU,
1280×720, noon lighting preview and DOF off. Started from the existing ordinary
seed87231 review world; live time advanced between clips, so these are not matched
actor-frame or exact-tick A/B comparisons. Deterministic View station restored the
starting composition, then a horizontal drag exercised the orbit. All geometry
queues were zero before recording; no scheduler overrides were used.

- `recordings/office-side-miniature.webm`: live activity and orbit.
- `recordings/office-side-anime.webm`: live held view; the attempted orbit missed
  the capture window. Preserved as activity evidence, not camera-motion evidence.
- `recordings/office-side-anime-orbit.webm`: repeated with the drag inside the
  recording window; the decoded samples confirm camera movement.

Reviewed six decoded frames per clip at 0.1/4/8/12/16/19 seconds, saved under
`screenshots/office-side-facades/` as `miniature-motion-sheet.png`,
`anime-motion-sheet.png` and `anime-orbit-sheet.png`. Office horizontal bands remain
readable across the sampled angles; no obvious facade detachments or disappearance
are visible in those samples. This supports the modest final-still benefit, but
does not establish every-frame stability or temporal aliasing acceptance.

One Miniature orbit rolling window reported an 82.3 ms longest frame; another Anime
window after recording reported 80.3 ms. These are unisolated observations, not
measured incremental facade cost or named-hardware acceptance. No works were active
in these mature views. Developing/live construction checks remain the next gate;
do not close that gate based on these recordings or existing static cladding tests.

## Ordinary live construction review and retention

Regenerated a current-WASM checkpoint with:
`node scripts/capture-midrise-cladding.mjs .tools/office-side-construction office 4 .65 .75`.
The capture helper now accepts an optional progress range, retaining its previous
default range. Seed0 naturally admits office2602, seven floors, at tick15800 and
progress0.72427076. Original live and restored engines match at every one of50ticks
to opening at15850. Metadata/hashes: `office-side-construction.json`. No building,
project or save-state values were injected.

Normally imported the same before-opening save for both browser recordings, framed
building2602 with the existing review control,50mm/High WebGPU/1280×720/DOFoff,
noon lighting preview. Simulation1×, development16×. Geometry queues reached zero
before recording. Both20second clips include ongoing surrounding development;
no forced queue drain. Files: `recordings/office-construction-{anime,miniature}.webm`.

Reviewed ten frames per clip at0.1/0.5/1/1.5/2/3/5/8/12/19seconds. Contact sheets
and before screenshots are in `screenshots/office-side-facades/construction-*`.
The visible side and front retain partial lower-floor glazing with exposed upper
structure, followed by the finished facade/roof. No obvious side-wall detachment
or loss appears in those samples. The rapid transition is sampled, not proof of
every intermediate frame or every parcel/family. Both styles share the starting
save and review framing; wall-clock sample times are not exact simulation-tick pairs.
A201.5ms rolling maximum was displayed following Miniature style preparation and
framing; it is not attributed to glazing or accepted performance.

Together with final matched mature stills, sampled both-style orbits,15focused
geometry/cladding tests and three-seed actual-mesh comparison, retain the bounded
office-side treatment. Stop facade variants. Whole-city artistic acceptance,
actual daily lighting, broader construction/parcel cases and hardware performance
remain open. Next diagnose the previously recorded parcel overhangs; retaining this
candidate does not assert that those baseline defects are resolved.
