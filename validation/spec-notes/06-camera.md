# Checkpoint notes moved from specs/06-camera.md

Dated candidate, checkpoint and status notes, moved verbatim on 2026-10-16 so the
spec describes current design only. Newest decisions win over older notes.

Developer station review (2026-10-12): View station uses a deterministic held
Explore pose centered on the station platform, fitted to a55m neighborhood radius
with the selected lens and checked against building occlusion. It no longer uses
the close entity-follow path. Missing or infeasible stations leave the camera
unchanged. This diagnostic view does not replace ordinary Watch/Follow direction
or establish artistic acceptance. See validation/archive/2026-10/fidelity-review-current.md.

Implementation checkpoint (2026-10-10): the pure model lives in `web/src/camera/lens.ts`; five focused tests pass independently (`validation/lens-foundation-tests.txt`). The live viewer now exposes 35/50/85 mm focal-length selection under City details, defaults to 35 mm, and persists the choice. Selection changes FOV without moving the camera or target and takes manual Explore control. The live pass now computes signed thin-lens CoC from axial depth, current lens aperture and drawing-buffer height, with explicit optical model scales (.04 Anime, .01 Miniature) and bounded per-pass gather radii (4/8 pixels). These style values are provisional. Nine CPU lens tests pass; default-backend and explicit WebGL smoke renders work. A tilted focus plane is not implemented, and full matched lens/resize/motion acceptance remains open.

Focus-depth checkpoint (2026-10-10): the live pass now uses positive axial camera-space target depth, matching Three’s depth input, with exponential damping at 8 inverse seconds. First-frame and world-reset focus snaps avoid carrying old focus across a replaced city. Ordinary OrbitControls targets usually lie on-axis, so this is a consistency and transition improvement, not a claim of a large static-image correction. Physical aperture now participates in the signed CoC calculation described above; tilted-plane rendering and full visual acceptance remain unfinished.

Clearance implementation checkpoint (2026-10-10): camera target/distance/height bounds currently do not establish collision safety. Ordinary automatic pose transitions can cross occupied building volumes; the existing elevated skyline path alone does not satisfy the all-transition requirement. A pure rotated-box segment-clearance module is being developed before director integration. Integration must account for rendered roof equipment, lift before crossing an obstructed footprint, preserve continuous camera movement, and retain immediate manual priority. Manual final poses also require clearance; an automatic-only repair must not be reported as complete camera safety.

Manual-clearance checkpoint (2026-10-10): the director now rejects manual swept paths through conservative rotated building envelopes/terrain, restores the last clear pose, and retains Explore control. Snapshot-time recovery handles a camera already enclosed by a loaded/new structure, with a small clearance epsilon. Root camera/clearance/lens tests pass43/43. Automatic scene transitions still need collision-safe routing, and browser motion quality remains unverified; this is not complete camera-safety acceptance.

Automatic-clearance update (2026-10-10, supersedes the implementation gaps above): ordinary automatic moves now plan straight or rise/cross/descend paths against conservative building envelopes. Watch, Follow, and skyline motion check the actual post-OrbitControls swept segment, including settled frames and waypoint snaps. Rejected motion restores position, target, and orientation; manual Explore keeps its own clearance handling. Temporary automatic orbit limits allow elevated paths. Root camera/planner/clearance/lens tests pass 57/57 (`validation/camera-final-guard-root-tests.txt`), including an adversarial controls clamp after route completion. Reduced-motion cuts validate their destination rather than the intervening segment. Browser motion quality, camera recovery in all live growth situations, and framing fidelity remain acceptance work.

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
