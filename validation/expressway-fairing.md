# Broader elevated corridors — 2026-10-11

The latest user photograph calls for broad continuous curves fitted around the
city. Closed seams alone did not meet this: the ten-seed corpus included a
sampled 11.460 m centerline radius, barely wider than the deck half-width.

## Change and rejected alternative

A numerical prototype compared the existing tangent bisector, a wider weighted
secant tangent, and lateral route fairing. Weighted tangents alone worsened
several bends (seed 41 fell to 6.945 m); reject that approach. Three fixed Jacobi
passes smooth lateral offsets on independent elevated nodes, preserving their
longitudinal coordinates and outer interior anchors. Each update blends its
current offset with the distance-weighted interpolation of its neighbors.
Boundary tails then continue the new interior bearing. Street nodes do not move.
The existing cubic controls, conservative lot clearance and saved traffic use
the resulting authoritative route. Work is fixed and small at plan creation.

The [prototype numbers](expressway-fairing-experiment.json) are JavaScript
double-precision experiments, not native acceptance. The rebuilt WASM
[ten-seed corpus](expressway-corpus.json) now has a minimum sampled radius of
102.363 m. This is a sampled result for those seeds, not an engineering standard
or proof across every possible world. Ramps, graded merges and route-level
composition remain open. Reject changes that create folds, break clearance or
grade separation, or cost density without visible benefit.

## Verification and cost

- [Native release suite](expressway-fairing-native.txt): 422 passed, 0 failed,
  53 ignored. Compilation warnings remain. The crossing regression now checks
  intersections of sampled curves rather than requiring coincident nodes;
  independent decks must still cross without sharing graph nodes.
- [Native/WASM witness](expressway-fairing-native-wasm.json): seed 87231,
  tick 160000, all 16 controls equal, original/restored continuation 800 ticks.
- [Focused frontend](expressway-fairing-web.txt): 10 tests pass, including
  traffic, surface placement and actual swept mesh joins. This is not a new
  all-frontend-suite result.
- Mature seed 87231 has 1839 buildings, versus 1854 before fairing: 15 fewer
  lots (0.81%). Current five-seed parcel coverage is .422/.414/.405/.427/.437
  for 0/7/41/2026/87231. These are parcel envelopes, not rendered footprints.

## Browser review

Normal UI import of `.tools/fidelity-review/fair-routes-87231.json` reaches zero
building and neighborhood queues without manual draining. Seed 87231, tick
160000, clear weather, noon light preview, 50 mm, DOF off, WebGPU/high detail.
Overview was switched to Explore to hold a camera for the style comparison:
[Anime](screenshots/road-curves/route-fairing-anime.jpg),
[Miniature](screenshots/road-curves/route-fairing-miniature.jpg), and
[diagnostics](screenshots/road-curves/route-fairing-diagnostics.jpg).
The earlier before image has a different camera and tick; it is context only,
not a matched visual benchmark. No speedup is claimed from the displayed FPS.

Retain the geometry improvement: the displayed corridors have continuous broad
bearings while keeping separate crossing levels. The photograph's branching
ramps and richer structures are still absent. Miniature remains too dark in
this framing, and neither full concept fidelity nor close-up motion is accepted.
The plan digest changes; prior checkpoints reject, with no migration required.
