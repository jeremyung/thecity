# Checkpoint notes moved from specs/08-delivery.md

Dated candidate, checkpoint and status notes, moved verbatim on 2026-10-16 so the
spec describes current design only. Newest decisions win over older notes.

## Current acceptance milestones — 2026-10-15

This sequence governs the next work and supersedes older work-selection sequencing below. The original delivery slices remain scope requirements, not proof of current completion. The complete [goal text](../validation/living-city-goal.md) retains the architecture and full living-city objective.

PR19 is merged as an engineering checkpoint. No milestone below is declared visually accepted by that merge. Work on one primary milestone; later systems may support it when necessary.

| Order | Milestone / status | Required visible result and exit gate |
| --- | --- | --- |
| 1 | Camera, shaders and rendering — **active, not accepted** | Reference-led optics, focus, material response, lighting/shadows, atmosphere/water and stable moving images. Evaluate both styles, DOF off/on, day/night/weather and measured frame cost; local scene screen then three seeds/stages and user review. |
| 2 | Convincing reference neighborhood — **pending, not accepted** | A normally generated scene whose streets, buildings, materials, frontage and planting work together toward the locked concepts. Improve credible proportions, surface construction, facade/roof character and purposeful ground transitions. Local matched visual screen, then three-seed/stage/style validation and user review. |
| 3 | Consistent city character — **pending** | Quality extends across residential/commercial, station and waterfront scenes without repetitive kits, arbitrary clutter or displaced gaps. Preserve coherent layout, access and infrastructure. |
| 4 | Cinematic presentation across the city — **pending** | Lighting, landscape, water, skyline/horizon and camera approach both concepts; stable in motion and day/night, distinct styles, no fog/blur masking. Geometry/material and presentation gains are assessed separately. |
| 5 | Complete living experience / release — **pending** | Purposeful activity, coherent growth/renewal, camera/audio and production-scheduler, multi-hour, save/replay and performance gates pass, with user acceptance against concepts. All original release scope remains. |

### Work selection and stopping rules

The primary rendering pass explicitly includes texture quality: screen-space resolution, texel density, UV scale/repetition, filtering/mips, normal/roughness response and facade/roof/road readability. Report whether each visible defect needs texture, shader, geometry or lighting work. Cities: Skylines is a comparative material/cohesion benchmark, not a replacement for the locked art direction. Targeted texture replacement/generation requires visible benefit and measured browser memory/download cost.

User steering prioritizes camera, shaders and rendering now. Select the largest visible mismatch within that primary workstream; architecture remains required but does not gate this presentation pass. Record intentionally changed camera/lens controls and retain a fixed-camera comparison when evaluating optics. Assess streets, architecture, materials and frontage as a complete neighborhood. Correctness work remains required but bounded to demonstrated blockers; it must not indefinitely defer visual progress. For every intervention, name its visible flaw, evidenced structural cause, bounded reusable change, expected scene-level gain and stop condition. First compare the affected ordinary neighborhood plus nearby weak views. If there is no meaningful scene gain, stop before broad validation. A promising result then needs matched interior/frontage/station/waterfront/overview views across three seeds, developing/mature stages and both styles, with numerical camera/lens/light/weather/resolution/quality recorded and DOF-off evidence. Reuse state where possible; explain growth-history changes when generation changes. Include normal camera motion and full acceptance checks before delivery.

Counts, parcel coverage and passing tests are supporting diagnostics. They cannot substitute for a better scene. Keep implemented, technically validated and visually accepted status separate. After two unsuccessful variants, reconsider the intervention rather than extending a fitting or embellishment loop. Defer unrelated refactors, new simulation systems and texture batches unless they unblock the named visual outcome.

The isolated reserved-access correction remains unaccepted. Its second fallback restores some mass in the seed5 loss view but has not established an ordinary-neighborhood aesthetic gain. Preserve its code/evidence and outstanding correctness work; do not start a third fitting variant as the next aesthetic milestone. Next primary investigation: camera/lens behavior and the shader/rendering pipeline against the concepts, using unchanged saved worlds. The daylight comparison and subsequent local screens are closed; inspect the checkpoint and material-study outcomes before selecting the next substantial scene-level intervention. See `validation/reference-aesthetic-direction.md` for source evidence and scope.

## Immediate milestone after the master-plan review (2026-10-11)

Use the seeded master plan for new worlds. The next acceptance milestone is a beautiful, watchable, measured **800 m city**, first in Last Train, First Light and then verified in Little Tokyo using the same world, camera and time. This orders the work; it does not shrink the original release scope.

- Exercise normal startup, development and mature-city loading. Geometry must become visible through the production scheduler without manual queue draining. Record time to a complete scene, worker step latency, render frame timing and memory on named hardware and browser; software WebGL captures establish appearance only.
- Review matched founding, developing and mature scenes through dawn, noon, dusk and night. Preserve camera/lens behavior, style distinction with DOF disabled, simulated water, full-scene anti-aliasing, purposeful activity and spatial audio. Beauty is an acceptance gate, not a trade for density.
- Profile the master-plan candidate scans at maturity. Introduce bounded/incremental work where measurements justify it, with saved progress and same-target replay. Keep visual quality and worker backpressure gates intact.
- Correct raised-platform passenger placement and physical flag-lot access before accepting the affected scenes. Review street detail, district character and building variation against the approved concepts.
- Remove obsolete planners as a separate tested cleanup, retaining or replacing generic travel, transit, business, current-save, determinism and bounded-state regression coverage. A failing test needs diagnosis, not deletion merely to make the suite green.
- Defer generic map frames, OSM/PLATEAU imports and the 2.4 km domain until this milestone passes. Keep them on the roadmap. After two unsuccessful variants of an intervention, record the evidence and reconsider the approach before another small tweak.

Backward save compatibility is not required (user decision, 2026-10-11). Do not build migration infrastructure for obsolete saves. Reject incompatible formats explicitly and offer a fresh world; do not silently reinterpret them. Current-format save validation and live-versus-restored continuation remain mandatory.

## Current work selection — 2026-10-12

The [updated goal](../validation/living-city-goal.md) and
[multidimensional review](../validation/archive/2026-10/fidelity-review-current.md#multidimensional-gap-assessment--2026-10-12)
set urban fabric and neighborhood composition as the next primary workstream.
This supersedes activity-first sequencing in earlier checkpoint notes without
removing outstanding behavioral, audio, correctness or performance requirements.

Start with measured footprint coverage, street share, block/parcel sizes, setbacks,
frontage and height distribution using existing fabric targets. Distinguish gross
and net developable coverage and overlapping corridor areas. Qualitative overhead
maps without a scale/legend do not establish surveyed terrain or precise targets.
Roads dominating a view do not by themselves justify reducing road widths.

Improve one normally generated local street/crossing/waterfront neighborhood with
reusable rules. Compare fixed world/camera/time/effect settings in both styles,
with DOF off and on; review ordinary motion and at least three seeds. Architecture,
frontage, landscape and material improvements must support the structural result.
Do not substitute hand-staged scenery, cosmetic crowds, extra towers, texture
batches or stronger blur for coherent density and readable composition. Retain all
release gates below, including founding/developing/mature daily cycles, audio and
normal loading/growth performance. Reassess after two unsuccessful interventions.

### Evidence reconciliation — 2026-10-13

The [updated gap priorities](../validation/archive/2026-10/fidelity-review-current.md#updated-goal-and-gap-priorities--2026-10-13)
retain successful compact fabric and the bounded apartment, entrance, canopy and
glazing improvements with their evidence limits. The gateway-stair correction now
has three-seed plan-view evidence; full physical and motion acceptance remains open.
Validate the in-progress shared station construction phases before accepting affected
infrastructure, then choose the next access/frontage intervention from matched
neighborhood evidence. See the [consolidated learnings](../validation/archive/2026-10/fidelity-review-current.md#consolidated-learnings-and-goal-reconciliation--2026-10-13).
Specimen A/B, lighting previews, sampled motion and focused tests do not establish
the complete release gates below.

Fabric acceptance (2026-10-11): at maturity, rendered-footprint crops of compact and station districts meet the [fabric targets](02-world-growth.md#fabric-targets) for at least three seeds, with no overlapping lots, no inaccessible occupied building, and connected junctions. Run `node scripts/audit/audit-masterplan-fabric.mjs` on a mature snapshot to report gross coverage, footprint median/p90, floor median and road share per crop.
