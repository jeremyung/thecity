# Water pass isolation — 2026-10-11

The prior tessellation experiment did not resolve the broad angular patches.
This follow-up separates two contributors without stacking visual changes.
Both temporary edits were reverted; runtime water remains identical to HEAD.

## Reproduction

Normal World → Import of `.tools/fidelity-review/fair-routes-87231.json`,
seed 87231, tick 160000, 1839 buildings, clear weather, paused. Anime,
50 mm, DOF off, lighting preview 18:00, WebGPU/high detail, 1099 × 1071.
Apply **Hold waterfront view** after import. Both building and neighborhood
queues were observed at zero through the production scheduler for each trial.
No manual drain or hidden browser state mutation was used.

[Baseline](screenshots/waterfront-preset/first.jpg).

1. Set only `RiverWater.mesh.receiveShadow=false`.
   [Capture](screenshots/water-isolation/no-received-shadows.jpg).
   The broad angular patches disappear. Reflected bridge structure and repeated
   fine bands remain. The patches originate in received shadowing in this view;
   the previous description implying malformed water triangles was unsupported.
   This does not prove every shadow is geometrically correct, but tessellation
   and reflection gain are inappropriate fixes for these cast-shadow shapes.
2. Restore received shadows, and replace only the reflection/base color mix
   with `base` (all other normal, displacement and material code unchanged).
   [Capture](screenshots/water-isolation/no-planar-reflection.jpg).
   Bridge reflections disappear; angular shadows and repeated fine bands remain.
   Planar reflection is not required to produce either visible pattern.

A transient graphics-start warning during the first hot reload cleared as the
renderer started; the mature scene loaded normally. These are appearance
experiments, not performance benchmarks or all-backend validation.

## Decision and next action

Retain received shadows and planar reflections. Removing either would lose
required physical cues rather than solve the water's remaining artistic gap.
The repeated fine bands require a separate surface-lighting diagnostic. The
coherent sinusoidal normal terms are a candidate, not a proven cause yet.
Temporarily isolate them at this same pose before changing their spectrum;
if confirmed, compare an aperiodic, footprint-filtered slope field that preserves
rain response and fine detail in motion. Do not add another dense mesh or tune
reflection gain to conceal cast shadows. Broader architecture, foliage and
composition remain larger unfinished concept-fidelity requirements.
