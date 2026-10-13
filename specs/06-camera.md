# Camera and Scene Direction

The camera reveals growth while preserving the close, miniature scale that makes ordinary activity enjoyable. Manual exploration always takes priority.

Developer station review (2026-10-12): View station uses a deterministic held
Explore pose centered on the station platform, fitted to a55m neighborhood radius
with the selected lens and checked against building occlusion. It no longer uses
the close entity-follow path. Missing or infeasible stations leave the camera
unchanged. This diagnostic view does not replace ordinary Watch/Follow direction
or establish artistic acceptance. See validation/fidelity-review-current.md.

## Camera modes

Observe holds or slowly moves through a scene. Explore gives direct control. Follow tracks a selected subject with a stable offset. Overview frames the developed footprint. Lighting preview retains the existing mode while changing only lighting.

Manual input cancels automatic motion immediately and smoothly. Do not reclaim control after a timeout; provide Watch to resume. Orbit and zoom stay inside safe camera bounds, above terrain and outside buildings. Keyboard and touch behavior have equivalent outcomes.

## Finite scene anchors

The director maintains a bounded set of anchors: founding shop, river crossing, station, active construction, neighborhood street, waterfront, and broad skyline. Derive anchors from real world entities and pedestrian-accessible locations, not invented narrative events.

Store a persistent identity, geographic extent, available camera poses, visual importance, activity summary, last visit, and validity. Refresh anchors when construction opens or a district changes. Keep at most 32 active candidates initially.

## Showing expansion

Track the connected developed footprint and a smoothed extent. Do not widen the camera for a distant project marker or one empty road segment.

When sustained footprint growth exceeds about 10–15% since the last framing update, schedule a broader reveal. Smoothly pull back or rise over roughly 20–40 seconds. Use hysteresis and a minimum interval between reveals to avoid repeated reframing.

Limit zoom-out by readability: if street activity becomes too small, hold the scale and introduce another scene. A finite world overview is occasional orientation, not the permanent endpoint of the camera.

Typical progression:

1. Start close to the founding shop and first construction site.
2. Ease back to reveal a newly opened lane and homes.
3. Follow a delivery along that lane into a new neighborhood.
4. Hold a station scene as its first service arrives.
5. Briefly reveal two connected centers and the river crossing.
6. Return to an intimate street scene at dusk.

This is an example of event-driven direction, not a fixed story every seed repeats.

## Scene selection

Rank valid anchors by visible activity, meaningful change, composition, time-of-day suitability, and time since last visit. Penalize repetition, busy camera travel, and scenes already observed. Cosmetic selection randomness is independent of authoritative RNG.

Hold a scene for at least 30 seconds initially; quieter scenes may last 60–120 seconds. Avoid cuts simply because an event occurs. Prefer a subtle reframing when the new subject is nearby. For distant scenes, use a short dissolve or intentional slow journey. Do not fly through buildings to reach a target.

At night, favor stations, illuminated streets, and waterfront reflections. Dawn favors emerging activity and quieter roads. Rain favors shelter, crossings, and reflective surfaces. These are composition preferences, not changes to the world.

## Follow behavior

Follow a train with look-ahead along the track and restrained camera acceleration. Follow a resident at neighborhood scale rather than face level. When the subject enters a building, settle on the destination, show the activity on request, and release follow gracefully. If a target disappears, hold the last valid scene and explain its current state without snapping to the origin.

## Quality and comfort

No continuous camera roll, oscillating focus, dramatic acceleration, or rapid cuts. Focus follows a stable subject with damping. Reduced motion uses static holds and optional user-initiated scene changes. Audio listener movement is smoothed independently during cuts to avoid loud abrupt transitions.

## Acceptance criteria

- A growing district remains framed without continuous tiny zoom adjustments.
- New neighborhoods become visible without making the original one disappear from history.
- A full city still offers readable close scenes.
- User interaction interrupts any automatic move on the next frame.
- Camera mode and style never alter simulation hashes.


## Photographic lens model

Camera position, target, and lens are distinct viewer state. Dolly changes camera distance and perspective relationships; focal length changes angular framing. A style switch preserves all three. Lens choices must not affect authoritative simulation state or RNG.

Developer visual review includes a held waterfront preset: arc-length river
midpoint target, fixed camera offset, normal bounds/collision recovery, and
Explore hold with no automatic drift. It preserves the selected lens and world
state. This explicit diagnostic cut is not part of normal Watch transitions.
See [repeatability evidence](../validation/waterfront-review-camera.md).

The initial physical lens library uses a 24 mm-high sensor with 35 mm environmental, 50 mm neighborhood, and 85 mm detail presets. Vertical field of view is derived from sensor height and focal length; it is not an unrelated artistic constant. Aperture is expressed as f-number. Focus distances are in scene metres and optical dimensions in millimetres. The thin-lens circle of confusion is converted to output pixels and capped to a bounded kernel budget.

Implementation checkpoint (2026-10-10): the pure model lives in `web/src/camera/lens.ts`; five focused tests pass independently (`validation/lens-foundation-tests.txt`). The live viewer now exposes 35/50/85 mm focal-length selection under City details, defaults to 35 mm, and persists the choice. Selection changes FOV without moving the camera or target and takes manual Explore control. The live pass now computes signed thin-lens CoC from axial depth, current lens aperture and drawing-buffer height, with explicit optical model scales (.04 Anime, .01 Miniature) and bounded per-pass gather radii (4/8 pixels). These style values are provisional. Nine CPU lens tests pass; default-backend and explicit WebGL smoke renders work. A tilted focus plane is not implemented, and full matched lens/resize/motion acceptance remains open.

A physical lens viewing a full-size city hundreds of metres away produces little defocus. The miniature treatment therefore needs an explicit optical model scale or a separately named artistic tilt-focus treatment, rather than silently misrepresenting metres as millimetres. A tilted focus plane is distinct from ordinary depth blur. Before promotion, compare a fixed city/camera/time across lens settings, show foreground/subject/background focus behavior, preserve manual camera priority, and verify stable anti-aliased edges during motion in both render backends.

Focus-depth checkpoint (2026-10-10): the live pass now uses positive axial camera-space target depth, matching Three’s depth input, with exponential damping at 8 inverse seconds. First-frame and world-reset focus snaps avoid carrying old focus across a replaced city. Ordinary OrbitControls targets usually lie on-axis, so this is a consistency and transition improvement, not a claim of a large static-image correction. Physical aperture now participates in the signed CoC calculation described above; tilted-plane rendering and full visual acceptance remain unfinished.


Clearance implementation checkpoint (2026-10-10): camera target/distance/height bounds currently do not establish collision safety. Ordinary automatic pose transitions can cross occupied building volumes; the existing elevated skyline path alone does not satisfy the all-transition requirement. A pure rotated-box segment-clearance module is being developed before director integration. Integration must account for rendered roof equipment, lift before crossing an obstructed footprint, preserve continuous camera movement, and retain immediate manual priority. Manual final poses also require clearance; an automatic-only repair must not be reported as complete camera safety.


Manual-clearance checkpoint (2026-10-10): the director now rejects manual swept paths through conservative rotated building envelopes/terrain, restores the last clear pose, and retains Explore control. Snapshot-time recovery handles a camera already enclosed by a loaded/new structure, with a small clearance epsilon. Root camera/clearance/lens tests pass43/43. Automatic scene transitions still need collision-safe routing, and browser motion quality remains unverified; this is not complete camera-safety acceptance.

Automatic-clearance update (2026-10-10, supersedes the implementation gaps above): ordinary automatic moves now plan straight or rise/cross/descend paths against conservative building envelopes. Watch, Follow, and skyline motion check the actual post-OrbitControls swept segment, including settled frames and waypoint snaps. Rejected motion restores position, target, and orientation; manual Explore keeps its own clearance handling. Temporary automatic orbit limits allow elevated paths. Root camera/planner/clearance/lens tests pass 57/57 (`validation/camera-final-guard-root-tests.txt`), including an adversarial controls clamp after route completion. Reduced-motion cuts validate their destination rather than the intervening segment. Browser motion quality, camera recovery in all live growth situations, and framing fidelity remain acceptance work.

Follow lifetime contract: a missing movement-only subject holds its last desired
pose for a four-second grace period, then returns to Watch. Reappearance cancels
the grace period. Structural indoor/onboard positions remain valid fallbacks;
expired inspection results clear the subject panel.

### Train follow visibility checkpoint — 2026-10-11

Train follow now checks building-envelope sightlines to the center and both ends
of the consist, chooses from bounded angle/elevation alternatives, and caches
its shot offset between spatial or obstruction changes. Collision-safe travel
remains separate from subject visibility. The mature station witness improves;
canopy occlusion and route-wide motion acceptance remain open. See
[validation](../validation/train-follow-visibility.md).

Manual skyline range checkpoint (2026-10-12): maximum orbit polar angle is1.44rad
so the50mm lens can include the horizon from a ground-target view. Independent
height/building recovery remains enabled. This changes available manual framing,
not automatic Watch composition. See [interaction evidence](../validation/shallow-orbit.md).

Developer waterfront review now has overhead and low-angle variants sharing
the same arc-length river midpoint. Nominal offsets are (110,145,180) and
(130,55,215) metres; normal clearance may raise either pose. These are held
diagnostic views, not automatic Watch compositions. Low-angle repeatability
is recorded in validation/waterfront-review-camera.md.

Photographic review checkpoint (2026-10-12): developer Frame review building now
uses a deterministic held Explore destination with current-lens framing and
collision validation; it no longer invokes the animated user Follow action.
Repeated same-world/same-lens requests must reproduce position and target
regardless of previous camera history. Unknown/unsafe requests must be inert.
This diagnostic cut is not automatic scene direction. Matched DOF toggles show
existing depth blur but do not establish photographic fidelity; a larger blur
cap trial was reverted because its follow-interrupted camera invalidated the
comparison. See validation/camera-dof-baseline.md. Tilted focus, atmospheric
separation and both-style motion acceptance remain open.

Lens-composite candidate (2026-10-12): working Miniature gather cap is16px;
Anime remains4px. Coverage is now based on output-pixel CoC radius rather than
radius divided by that cap, preventing a larger cap from weakening unsaturated
blur. Four-pixel full coverage is provisional. Exact held-camera repetition and
one Miniature static comparison are verified; Anime, lens/resize and motion
acceptance are outstanding. This supersedes the8px working cap above only as an
unaccepted candidate; see validation/camera-dof-baseline.md.


Follow focus checkpoint (2026-10-12): optical focus now tracks the actual followed
subject independently from composition look-ahead. Building and train centers or
published movement positions supply axial depth; missing-subject grace retains
the last point. Manual interruption returns focus to the orbit target. Watch
subject selection/visibility and tilted-plane focus remain unfinished. See
[verification and limits](../validation/camera-dof-baseline.md#follow-subject-focus--2026-10-12).


Visibility experiment (2026-10-12): a street-centroid projection with bounded
sightline search was rejected after browser review because its high-angle fallback
lost the photographic composition. It is not in production. Geometric clearance
alone is insufficient for Watch acceptance; selecting another meaningful subject
is preferable to an unappealing overhead fallback. See
[rejected experiment](../validation/watch-visibility-experiment.md).


Revised Watch candidate (2026-10-12): ordinary districts now consider up to four
nearby open street subjects, with a bounded viewpoint search capped near 33 degrees
elevation. A developing-city browser view is clearer and retains layered depth;
the candidate is present but motion, both-style and performance acceptance remain
open. Closed selected streets reframe, and manual control retains priority.
See [study](../validation/watch-visibility-experiment.md#lower-angle-alternate-street-candidate--2026-10-12).
