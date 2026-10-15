# Supported roof services — 2026-10-12

The broader Watch view exposed repeated bare roof fields and cylinders. Source
inspection found tanks starting above the roof surface without support geometry.
A proposed shop-interior intervention was rejected: glazing is opaque and the
solid building walls would hide props. Existing shop frontages already have relief.

## Bounded change

`roof-services.ts` replaces only the tank/service/unit/riser block of flat roofs.
It uses existing shared materials and Kit batching; no textures or per-frame work.
Tanks have four supporting legs and a pad. Roofs at least4.6m in both dimensions
also get supported HVAC with fan/grille relief and a2.1m access headhouse with a
human-scale door. Smaller roofs retain only a tank; below3.2m equipment is omitted.
Risers stay outside the headhouse. Whole arrangements mirror by appearance seed,
retaining relative clearances. Aerial/laundry and the waterproof field are unchanged.
The main service cluster is bounded to45 primitives, below top+3m, within roof x/z.
Tower crowns use their existing separate kit and are not changed by this pass.

Root review corrected a floating HVAC gap, buried risers and an undersized door
before the first integrated capture. Root retained the4.6m full-cluster threshold
and added whole-layout mirroring after the first visual study.

## Evidence and limits

Matched office Anime study: seed0/tick9250,260 buildings,50mm,High,WebGPU,DOFoff,
live15:15, unchanged checkpoint weather,1280x720. Normal paused import queues
reached zero; Frame review building1814 holds deterministic pose.
`screenshots/roof-services/office-anime-baseline.png` copies the prior daylight-fill
candidate (lighting unchanged). `office-anime-candidate.png` shows integrated
services before the final whole-layout mirroring. Root reviewed both: the access
structures and supported tank silhouettes make roofs more legible; tiny fans are
subtle. The photographed neighborhood baseline is contextual, not a matched pair.

The final mirror variation still requires a fresh browser check, including Miniature
and motion/night. No concept acceptance or performance benchmark is claimed.
The normal ready UI before framing reported571 draws and4,300,351 triangles;
this is diagnostic only, not a matched cost measurement.

Root ran25 tests across roof-services, midrise-kit and lowrise-kit plus TypeScript,
all passed after final production code changes. The subsequent roof-services run passed all6 tests, including the additional
whole-layout mirror regression.
All changes remain uncommitted. Next gate: final both-style visual check and reject
or simplify if extra detail does not justify its geometry at normal viewing scale.

## Final variation and sampled orbit review

Final source captures `office-anime-final.png` and `office-miniature-final.png`
verify seeded mirroring at the same fixed1814 pose, seed0/tick9250,50mm,High,
DOFoff,live15:15,1280x720,WebGPU. All geometry queues reached zero through normal
loading. Root inspected both: services remain legible and neighboring roof layouts
vary without visibly unsupported equipment in these views.

Saved `miniature-orbit.webm` and `motion-review.html`. Recording began from the
held1814 pose; two drags730,380→570,395 and570,395→700,365 occurred while AX still
confirmed recording20s remaining. Simulation stayed paused. Video backing size
is2048x1152. Root inspected decoded0.5s and1.5s frames; their different angles
confirm recorded motion. `orbit-1.5s.png` preserves the second sample. Roof props
remain attached and separated in those samples; this is not all-frame temporal
acceptance or moving-city evidence. Duration metadata still reports Infinity,
so no exact clip-length claim.

An earlier recording attempt ended before its intended gestures and was replaced,
not retained as motion evidence. Its orbit window reported a143.1ms longest frame
with queues zero; cause unisolated. Later settled windows do not erase that spike.
No claim of smooth camera performance from this pass. Production build passes
with the existing large-chunk warning (`/tmp/city-roof-services-build.log`).

Keep the bounded roof candidate; no further roof-detail expansion without broader
scene evidence. Night/Anime motion and active-growth verification remain open.
The next perceptual priority is the whole living neighborhood: the current camera
samples still expose repetitive large facades and broad empty road/ground areas,
far from the dense layered streets and water/transit activity of the concepts.

## User-requested retry

Repeated the same held1814 framing and two orbit gestures in the existing warmed
WebGPU session, Miniature, seed0/tick9250 paused,50mm,High,15:15,1280x720.
Immediately observed120-frame UI windows:
- DOFoff, no recording: frame p9510.8ms, longest11.1ms.
- DOFoff, canvas recording active19s remaining: p9516.3ms, longest19.9ms.
- DOFon, no recording: p9511.6ms, longest14.0ms.
All four geometry queues remained zero. Retained `miniature-orbit-retry.webm` and
`retry-dof-on-metrics.png`. Recording completed normally; retry video has not
received a separate all-frame visual review.

The143.1ms spike did not recur in these observed windows. These are rolling
windows, not instrumentation covering every frame of the entire recording; drag
timing is not deterministic and first-use state differs. Do not infer recording
is the cause, claim a fixed hitch, or use settled values as growth performance.
No source changes were warranted by this retry. Leave DOFon for normal viewing.
