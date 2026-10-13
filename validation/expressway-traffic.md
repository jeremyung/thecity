# Elevated through-traffic checkpoint — 2026-10-11

Regional cars use the same master-plan cubic controls as the decks. The engine
caches arc-length samples, persists trips and departure state, caps the fleet at
48, and admits traffic only after a complete boundary-to-boundary chain opens.
Trips retire at the opposite boundary. Ground gateway connectivity is not a
requirement for regional through-traffic. No local exits, ramps, merges or lane
changes are implemented in this slice.

Worker protocol 3 retains the six-float movement stride and generalizes explicit
surface height to vehicles. The renderer avoids adding ground-bridge lift to an
already elevated car. Inspection identifies regional trips and actual endpoint
node IDs. Save validation rejects malformed and colliding trip identities.

## Verification

- Full native release suite: 417 passed, 0 failed, 53 ignored.
- Focused frontend contracts: 48 passed across five files, including rebuilt
  WASM natural growth, curve-relative lane offsets, headings, surface heights,
  and exact save/replay continuation.
- WASM and production builds passed.
- Full frontend run: 300 passed, one outdated active-traveler count assertion
  failed. Updated it to include regional cars; the complete affected WASM file
  subsequently passed all five tests. No timeout limits were changed.
- Imported `.tools/fidelity-review/traffic-87231.json` through the normal UI.
  A regional car was visibly on an elevated deck; clicking it correctly showed
  “Regional through-traffic” and its boundary journey.

## Open acceptance issues

Following the observed car at accelerated time exposed a retirement lifecycle
problem: after the car leaves, the inspector becomes a generic person and the
camera can remain outside the city. This was corrected in the follow-up below. Close-up
motion, both-deck visual coverage, traffic density, queue-settled performance and
matching the reference's beauty remain open. Tests are not artistic acceptance.

The supplied waterway-ramp photograph remains the visual target: long sweeping
curves, slender supports, purposeful branching, painted separation zones, and
buildings fitted around the corridor. The current two crossing through-routes
are only a foundation for that target.

## Follow retirement correction

Movement-only subjects now invalidate their follow pose when omitted from the
complete movement publication, retaining structural indoor/onboard fallbacks.
During the four-second grace period the camera holds its last desired pose;
it no longer repeatedly adds look-ahead to its target. A missing subject then
returns to Watch through the normal camera transition. An `unknown` inspection
clears the panel rather than constructing a generic person.

Root verification: camera/UI 58 tests pass, production build passes. Tests
exercise an elevated movement-only car, no target drift during grace, return
from Explore-follow to Watch, and reappearance before grace expires.

Browser: imported the saved regional-traffic checkpoint normally, selected a
visible upper-deck car, followed it at 8×, and observed its retirement. The
subject panel cleared and Watch returned to the built city. No manual queue
draining or camera reset was used to achieve that return. Normal scheduler
queues were subsequently 0 buildings / 0 neighborhood batches.

Evidence: `screenshots/expressway-traffic/follow-retired-watch.jpg`, seed 87231,
tick 168478, Miniature, 50 mm, DOF off, lighting preview 09:00, overcast intensity
0.22800122, WebGPU high detail. Camera is the automatic Watch pose following
retirement; exact camera coordinates were not exported. This is a behavioral
witness, not a matched concept comparison or a performance acceptance profile.
