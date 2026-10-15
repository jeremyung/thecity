# Train follow visibility — 2026-10-11

Previous turn made progress on synchronized door presentation and produced a
new failure witness: following the ordinary mature train framed a foreground
tower instead of the train. Collision-free camera movement did not ensure a
clear sightline to the subject.

## Bounded correction

A pure shot selector checks the train center and points 20 m toward each end
against the existing rotated building envelopes. It tries at most 24 candidate
angles/elevations, retaining a clear preferred view. It filters distant envelopes
once per selection. Camera coordinates retain the existing bounds. If no complete
view exists it returns the least-obstructed candidate and an explicit false flag;
it does not keep raising the camera indefinitely.

Train follow aims at rail/body height, retains the existing 10 m look-ahead, and
caches the selected offset until 12 m of travel, a heading change or an obstacle
revision. Rechecks prefer the last selected offset to avoid repeated low/high
searching. Existing eased collision-safe camera travel still owns transitions.
No simulation, asset visibility, style geometry or save state is changed.

## Evidence

- 49 tests pass across camera integration and pure visibility checks. Coverage
  includes blocked sightlines, alternate angles, a surrounding low wall requiring
  elevation, impossible enclosure, stable settled follow and existing reversal.
- The projection test now explicitly aims its mock camera at the target, as real
  OrbitControls does; the lightweight mock previously left orientation unchanged.
- Production build passes with the existing bundle warning; diff whitespace check
  passes. PR3 correction remains an ancestor; no commits or pushes made.
- Browser: seed 87231, tick 168000, 1839 buildings, paused dawn 06:00, day 7,
  50 mm lens, DOF off, High detail, WebGPU, 1280×720. Normal import completed with
  zero queued buildings and neighborhood batches before Follow train.
- [Before](screenshots/train-doors/follow-occluded.png): foreground tower hides
  the subject. [After](screenshots/train-follow/miniature-station.png): the full
  consist and station are readable through a higher corridor angle.

- [Matched Anime station](screenshots/train-follow/anime-station.png) keeps the same
  geometry and framing with DOF off. Resuming normal 1× operation advances through
  terminal events; at live 07:20 the train is visible in transit. The final paused
  [07:46 in-transit capture](screenshots/train-follow/anime-in-transit.png) also
  shows the whole consist. These are sampled observations, not a continuous motion
  recording or proof of every frame. Browser error log is empty.

## Limits and rejection conditions

This establishes one mature view improvement, not complete artistic acceptance.
Sightlines currently cover building envelopes; station canopies, bridges, trees
and terrain beyond the existing flat floor are not additional visibility proxies.
Transient obstruction during eased camera travel remains possible. The selector
is bounded but has no named-device profiling result yet. It applies to trains,
not every person, bus, indoor destination or scene anchor. Keep those distinct.

Reject or revise if motion testing shows angle oscillation, camera collisions,
train framing loss or a material measured cost regression. Further broad rail
asset work should wait until the subject remains watchable along its route.
