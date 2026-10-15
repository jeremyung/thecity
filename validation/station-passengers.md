# Elevated station passenger placement — 2026-10-11

## Baseline from the production engine

The master-plan city at seed 87231 was restored at tick 160000 and advanced
with ordinary `step(80)` calls. At tick 170480 two residents were waiting at
the station. No residents, paths, or queues were injected. The resulting
checkpoint was `.tools/fidelity-review/station-waiting-87231.json` (local evidence,
now incompatible). The updated audit runs against the current checkpoint after rebuilding WASM:

```sh
node scripts/audit-station-passengers.mjs .tools/fidelity-review/station-fixed-87231.json validation/station-passengers-current.json
```

The original diagnostic **exited 1**: both waiting passengers were outside the platform.
It reads actual WASM movement output, transforms it into the rendered platform's
local coordinates, and checks the current slab bounds. It does not reproduce
the engine's queue formula to generate its evidence. The fixture explicitly
checks the slab call in the renderer and must be updated if that geometry changes.
The report records the source revision. It is a failing baseline, not a passing
regression or visual acceptance claim.

| Resident | Platform-local X | Platform-local Z | Inside slab |
| --- | ---: | ---: | --- |
| 835 | -7.80 m | 12.00 m | No |
| 1089 | -9.10 m | 12.00 m | No |

The slab occupies X 2.6–7.8 m and Z -1–48 m. The current engine normal is
`(-uz, ux)`, while the renderer's positive local X maps to `(uz, -ux)`.
Thus all 96 possible station queue slots are on the opposite side of the
track. Fixing the sign alone is insufficient: the current rows span X
5.2–14.3 m, exceeding the slab width and intersecting furniture.

## Related defects confirmed in source

- `movement_buffer()` emits waiting rail passengers, bus queues, and ordinary
  walkers as the same kind 0 in `[id,x,z,heading,kind]`. There is no height or
  traversal state in this contract. A proximity-only lift would also raise
  pedestrians walking beneath the viaduct.
- The platform top is 8 m high: `RAIL_DECK = 6` plus the slab's 2 m top.
  Pedestrian meshes only apply road-bridge lift. A hard-coded 6 m offset would
  still put their feet inside the platform.
- Station entry and exit paths are direct two-point ground-plane connectors.
  The apparent entry structure is a set of elevated boxes; it has no usable
  staircase from street level. Correct queue coordinates alone do not repair
  walking continuity.
- Follow-camera targeting resets Y to zero. Umbrellas independently use fixed
  ground heights. Both must consume the same presented elevation as the person.
- The gateway also has a waiting queue but no corresponding platform kit.
  Its boundary presentation needs an explicit decision before claiming rail
  passenger realism across the entire route.

## Bounded implementation next

Keep the worker authoritative and keep actor kind separate from vertical
placement. Define a versioned movement elevation contract, with an explicit
ground-surface policy; do not overload kind values or infer passenger state from
proximity. Use one station layout for queue slots, walking connectors, furniture
clearance, and renderer geometry. Fit the existing 96-person cap within that
layout, with room for human body widths and the tactile edge strip.

Implement actual entry/exit traversal and stair geometry together. Preserve
resident IDs, queue reservations, boarding capacity, and current-save replay.
Apply the resulting pose consistently to body/head meshes, picking, follow,
and any rain accessories. Keep presentation style independent of geometry.

Acceptance requires all queue slots to clear the slab edge and furniture under
rotated/reversed rail axes, plus live walking → waiting → boarding → alighting
→ walking continuity. Test simultaneous street pedestrians beneath the elevated
station, bus waiters, pending alighters, save/restore mid-access, and both styles.
Capture the same live station view before and after, including motion. Do not
accept only an isolated raised passenger or an isolated station asset.

This investigation changed the next action from a renderer height patch to a
coordinated station-layout and movement-contract correction.

## Implementation checkpoint

The engine now exports both terminal frames and stair dimensions through
`rail_platforms`. Two columns of reserved queue positions fit the platform and
clear the benches. Entry/exit paths use the terminal's local basis, carry saved
surface heights, and go through a street-level stair entrance. Pending train
alighters use the same exit route; bus alighters at the same node retain ground
placement. Both styles share the new platform/stair kit.

Worker protocol 2 uses six floats per actor, preserving actor kind and adding
an explicit surface height. Bodies, heads, picking geometry, camera following,
and umbrella placement consume the height. Current master-plan saves require
rail access elevations and reject missing/malformed data. A digest salt rejects
obsolete master-plan layouts. Historical legacy-planner test fixtures retain
their existing validation path; no migration was added.

The fresh seed-87231 city supplies the reviewed mature queue at tick170720
(1,936 buildings). [Current WASM evidence](station-passengers-current.json)
finds one waiter inside the slab at surface Y=8, with zero containment or height
failures. This is a later tick than the old baseline because the new approach
walk takes longer. An original/restored pair matched for a further 800 ticks.
An in-place text mutation removing a required rail height array was rejected
for missing surface elevations. Preserve the raw save text when doing this:
JavaScript JSON number round-tripping can round the u64 plan digest and cause
an unrelated digest rejection before the intended validation.

Validation:

- Native release: **405 passed, 0 failed, 53 ignored**.
- Focused frontend geometry/camera/umbrella/worker checks: **71 passed**.
- WASM rebuilt; TypeScript and production build pass (existing bundle warning).
- Full frontend suite: **294 passed, 3 timed out** at their existing limits
  (`lowrise-kit`, `tower-kit`, and the previously observed WASM movement test).
  The two geometry files pass separately with one worker: **17/17**, unchanged
  time limits. This is not an all-green default-suite claim.

Live import reached zero building and neighborhood queues through normal
scheduling. Captures use seed 87231, tick170720, authoritative 08:43, 50 mm,
DOF disabled, held Explore camera, WebGPU, 1098×1070 viewport. The
[Anime district view](screenshots/station-placement/anime-context.jpg) records
context, not a close-up proof of foot contact. Automatic station following can
be occluded by adjacent buildings; manual exploration was needed.

Remaining acceptance: close-up motion across arrival/waiting/boarding/alighting,
busy-queue crowd intersections, gateway boundary presentation, and matched
daily lighting. Walking height currently interpolates along the stair incline;
individual treads are ~0.176 m high and do not have a foot-placement animation.
The full station realism gate and broader art/performance gates remain open.

The [matched Miniature capture](screenshots/station-placement/miniature-context.jpg)
preserves the held camera and paused tick. [Renderer diagnostics](screenshots/station-placement/diagnostics.jpg)
show both geometry queues at zero after switching. Browser warning/error logs were
empty. These are live context checks, not hardware benchmarks or full motion
acceptance. The developer-only View station action selects the actual station
through the existing follow camera; it does not guarantee an unobstructed view.
