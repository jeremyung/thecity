# Train terminal reversal — 2026-10-11

Previous goal turn made concrete progress: corridor wiring, platform-clearance
geometry and browser evidence. While inspecting dwell behavior this turn, the
renderer exposed a more fundamental defect: it used interpolated travel heading
to orient all coaches. The engine reverses heading at a terminal, so the complete
47 m consist could rotate through the station instead of changing driving ends.

## Change

`CityTrains.setWorld` caches the current straight route's fixed orientation from
its authoritative nodes. Coach body poses stay aligned to that route regardless
of travel direction. Cab lights are separate bounded instances: warm lamps move
to the driving end, red lamps to the trailing end. The renderer passes the raw
movement heading for lamp direction, while retaining position interpolation.
Route caches clear on world replacement/reset. Existing saves/engine behavior
are unchanged. This does not add doors or change dwell duration/boarding.

## Verification

- Production build passes with the existing large-chunk warning.
- Five focused rail suites: **18 tests pass**.
- Regression compares every body instance transform before, through and after
  a 180-degree heading change; bodies remain identical at the same position.
- Lamp geometry centers and rotations swap ends, and actor IDs remain attached.
- A replacement world reusing the train ID with an invalid route does not inherit
  the prior orientation.
- Initial lamp assertion failed because it inspected the instance origin while
  assuming a geometry-centered origin. Corrected it to transform the actual
  geometry bounding-box center; no runtime workaround was made to satisfy it.

## Limits and next step

This is the fixed orientation for today's straight shuttle route, not a general
route or articulation solver. Future curved routes need per-coach and per-bogie
samples, including physical orientation independent of travel direction.

Animated doors still need actual openings in the shell, platform-side selection,
and timing from authoritative dwell/boarding state. A decorative moving panel on
an opaque body would not satisfy that requirement. The engine currently boards
within its dwell interval; a proper approach must also prevent departure before
visible doors close. Do not start by animating panels without this contract.


## Browser smoke

Normal mature import of seed 87231 / tick 160000 / 1,839 buildings settled both
queues at zero on WebGPU High detail. Miniature, DOF off, 50 mm lens, 1280×720,
16:00 lighting preview, unchanged checkpoint weather. View station then manual
Explore orbit; exact numeric camera transform not exported. Screenshot:
`screenshots/train-reversal/terminal-smoke.jpg`. Browser warning/error query was
empty. This confirms integration renders; it does not establish reversal-motion
acceptance or named-hardware performance. The reversal correctness claim is
supported by the geometry-transform regression, not this still image.
