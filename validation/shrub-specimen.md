# Low shrub specimen — 2026-10-12

The previous turn advanced evidence with a settled style comparison and current
waterfront screenshot. This isolated experiment follows the earlier rejected
bank-planting report; it is not imported by production code.

Open `/validation/shrub-specimen.html` with Vite running. Both panels use current
ArtMaterials, identical seeded geometry (512), 32 instances, 4× MSAA, no DOF,
50 mm/24 mm sensor height, 580×480 pixels each. Camera direction is
(.20,.65,.733), looking at the origin, at selectable distances25/100/250m.
Synthetic clear lighting, hemisphere2/directional2.5, no shadows/water; this
isolates silhouette and material, not final lighting or browser performance.
Opaque clusters contain38icosahedra,760triangles per shrub, one instanced draw
per panel. No textures added. Two style renderers are diagnostic overhead only.

Observed25m,100m,250m in WebGPU:

- [Near](screenshots/shrub-specimen/opaque-near.png): readable full canopy but
  distinctly rock-like faceting and missing leaf-scale edges. **Reject for city
  integration.** This does not meet the concept's natural foliage character.
- [Far](screenshots/shrub-specimen/opaque-far.png): maintains visible mass without
  dissolving into stems. This supports an opaque canopy interior, not this asset.

Root reviewed Luna's source audit of bank space: revetment ends at offset
halfWidth+3.5m, railing at+5.4m, walkway begins+5.5m. Proposed specimen envelope
for integration: center+4.45m, radial width<=1.2m, along-bank length<=2m,
height .55–.75m. The current generic specimen has not been fitted to that
envelope and uses a simplified1m rail, whereas production posts reach1.46m.
Do not treat this as clearance validation. Full footprint clearance must account
for roads, buildings, bridges/expressway crossings and the promenade.

Next bounded variant: fewer opaque interior masses plus fine leaf silhouettes,
with a shared near/far silhouette. Evaluate beside actual-size railing and bank
cross-section before integration. The current760triangle budget is excessive
for its poor close result; proposed <=320instances and ~192triangles per shrub
are starting targets, not permission to sacrifice quality. If the second
variant also fails, reassess asset approach before further scattering or tuning.

Experimental HTML/TS retained as a reproducible rejected reference. No code
tests or production acceptance claimed; no production integration, commit,
push, or goal status change. Tab16 retains the250m specimen; tab15 retains the
paused city waterfront.

## Hybrid follow-up and reassessment

`?hybrid=1` selects the second isolated candidate: five opaque20-triangle cores
plus48masked leaf cards (196triangles total), using the existing shared foliage
mask and two instanced draws. Same geometry in both styles. The specimen rail
was corrected to production heights .64/1.28m, with1.46m posts. The opaque mode
now also uses that rail; original screenshots retain the earlier1m reference.

Reviewed [25m](screenshots/shrub-specimen/hybrid-near.png) and
[250m](screenshots/shrub-specimen/hybrid-far.png) in WebGPU. Leaf edges are finer,
but the near canopy reads as pale speckles; far view is still isolated dots.
Reject this asset for integration as well. This is not a controlled opacity
comparison: the hybrid footprint/height is smaller than the original candidate,
and the rail changed. Do not attribute the difference solely to leaf masking.
No city geometry or renderer code has been changed.

Two unsuccessful specimens trigger reassessment. Stop procedural single-bush
tuning. The concept's bank vegetation reads as connected irregular plant masses
with deliberate breaks, whereas both specimens use repeated isolated bushes.
The next proposal should establish a short authored planted-verge patch with
continuous low cover, height variation and exposed stone/soil, fitting the bank
cross-section. Evaluate composition/silhouette first; leaf detail is secondary.
Do not blindly increase scatter count, enlarge trees, or run another parameter
sweep. The full living-city performance, fidelity and motion gates remain open.

## Composition-scale reassessment — 2026-10-12

`?verge=1` reuses the hybrid geometry in six connected patches,12canopy units
each, with authored along-bank offsets and deliberate gaps. The diagnostic now
includes production-width1.9m planting slot,5m promenade, revetment and a flat
water stand-in. It is not the simulated city water. Show planting checkbox
enables a genuinely matched bare/planted comparison without camera changes.

At25m, grouping reads as a planted edge, but pale leaf speckling remains. At100m
the [planted](screenshots/shrub-specimen/verge-planted-100.png) versus
[bare](screenshots/shrub-specimen/verge-bare-100.png) comparison shows only a
small visual contribution in both styles. It does not warrant integrating the
rejected asset or its placement machinery. Geometry remains outside production.

Decision: stop this planting workstream. The narrow slot and low canopy limit
projected mass at overview distance; repeated shrub tuning is not addressing the
larger concept gap. Bank profile, material character and relationship to water
need a structural art review before more foliage. Do not widen banks without
checking authoritative river/road/building clearances. These diagnostic files
are retained for evidence, not shipped assets. Other known defects (raised road
junction disks, water flatness, growth stalls) remain higher-value candidates
than another shrub variant. No performance/production tests claimed this turn.
