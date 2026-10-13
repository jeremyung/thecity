# Elevated commuter corridor — 2026-10-11

The preceding commuter-kit turn made concrete progress: a bounded vehicle kit,
corrected gauge/platform fit, tests and browser evidence. This turn continues the
same workstream rather than reopening water or planting experiments.

## Change and criterion

The pantograph previously touched empty air and the broad roadbed lacked rail
infrastructure. Added a dedicated corridor helper: straight contact wire,
segmented sagging messenger wire, droppers, cantilever supports, insulators,
a darker bed below sleepers, one service trough and edge curb. Contact-wire
underside shares the authored pantograph shoe height (11.8775 m above ground).
Infrastructure uses the current straight rail axis; it is not a curved-track
solver. Only open rail receives this furniture.

Supports default to one side. The engine's terminal frames reserve platform
space, so poles switch sides at the oppositely facing terminal and the curb/
trough stop through its envelope. This avoids poles and curb geometry inside
either boarding platform. The geometry remains within the existing deck width.

Acceptance for this bounded increment: continuous contact height, visible rail
infrastructure, clearance from coaches and both terminal platforms, shared
geometry across styles, and bounded static generation. Reject if it adds bulk
without readable rail character or creates platform obstructions.

## Validation

Production build passes (existing large-bundle warning). Four focused suites
pass **16 tests**, including five corridor tests independently reviewed by the
primary agent. Tests exercise rotated/reversed segments, finite bounds, contact
wire endpoints, actual pantograph maximum height, coach clearance, both terminal
walking envelopes in either storage direction, invalid inputs and geometry cost.

800 m corridor without platform cutouts: **3,648 triangles / five material
batches** before render passes. No texture added. Platform cutouts add a small
bounded number of boxes. This is a geometry count, not hardware performance
acceptance. Road rebuilding already includes platform signatures.

Normal mature import: seed 87231, tick 160000, 1,839 buildings, checkpoint
`.tools/fidelity-review/fair-routes-87231.json`. WebGPU, High detail, 1280×720,
DOF off, 50 mm lens, 16:00 lighting preview, checkpoint weather unchanged.
UI showed zero queued buildings and neighborhood batches. View station followed
by manual Explore orbit/zoom; exact camera transform was not exported.

`screenshots/rail-corridor/anime-afternoon.jpg` and `miniature-afternoon.jpg`
are the same paused pose/time with only style changed. They show supports,
wire, service edge and coach/platform fit. They are not exactly matched to the
preceding train-kit camera. Thin wires remain subtle at neighborhood distance;
close vehicle surfaces and the plain station roof remain far below photo fidelity.

A brief 1× live run moved the train out of the station view;
`screenshots/rail-corridor/after-departure.jpg` records the unobstructed track.
The exact later tick was not recorded, and this is not a full motion/aliasing
review. Browser warning/error query returned no entries after the check.

## Scope still incomplete

This is a simplified visual catenary, without electrical/tension simulation,
turnouts, variable heights, wire staggering, curved alignment or portal bridges.
The one-sided curb is not a complete safety-fence kit. Ballast currently has a
flat material without individual stones. Streetcars, ground-rail corridors,
independent coach articulation and dwell-linked doors remain unfinished. Full
night, WebGL, motion and named-hardware acceptance remain outstanding. Do not
confuse this local geometry improvement with final concept fidelity.
