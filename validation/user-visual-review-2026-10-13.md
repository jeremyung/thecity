# User visual review and revised priorities — 2026-10-13

This review supersedes the preceding next-work rankings. The user rejects the
current visual result across the city, not only the waterfront. Existing meshes,
passing geometry tests and compact-crop coverage do not establish visual fidelity.
Do not keep choosing building details or actor counts because they are easy to
validate. The full living-city objective and specifications remain unchanged.

## User-observed gaps

1. Urban fabric is too open, regular and straight: transport occupies too much of
   the frame, buildings stand apart on grass, and street/river/rail geometry lacks
   the irregularity and sinuous composition of the references. Diagnose city-wide.
2. Japanese street identity is weak: poles/wires, layered vertical signage,
   awnings, vending machines, bicycles, plants, AC units and guardrails do not
   create convincing occupied frontage.
3. Vegetation lacks visual coverage and softness: trees, layered bank growth,
   blossom and roof planting do not register sufficiently in ordinary scenes.
4. Lighting lacks directional warmth and contrast; atmospheric separation and a
   convincing horizon are weak, exposing the world's edge. Horizon integration
   is the user's proposed inexpensive high-impact improvement, not yet a measured
   cost or accepted implementation.
5. Generic building silhouettes, repetitive windows, saturated palette accents
   and blotchy wall noise fall short of warm neutral Japanese architecture and
   layered balconies, stairs, laundry and roof services.
6. Water reads as a pale tiled plane with repeating reflection and hard banks;
   dark depth, glints, reflected city and planted edges remain weak.
7. Streets lack purposeful visible life and human/vehicle readability. Activity
   remains required, but greater counts cannot resolve points1–5 in a still frame.

## Source reconciliation (current local branch)

These are implementation facts, not rebuttals of the visual review:

- `web/src/render/kit.ts` has a conditional pair of service poles and one taut wire
  only on anchors with parcels at least16m by16m. Rail catenary is separate in
  `rail-corridor.ts`. Neither establishes a coherent local-street utility network.
- Vending-machine file count does not establish instance prevalence. Placement
  gates and visibility need measurement at actual frontages.
- Balcony, laundry, side-stair and roof-service code exists in the building kits.
  Its prevalence, silhouette and projected size remain the visual question.
- `foliage.ts` uses64/28 leaf clusters per near/far crown, each with three cards;
  those numbers are not city-wide tree counts. Crown appearance, distribution,
  ground layers and projected canopy coverage must be evaluated separately.
- `city.ts` builds a sky sphere and three distant mountain rings. Their existence
  does not establish a believable visible horizon or continuity at the city edge.
- FogExp2 constructor density0.0008 is overwritten in `updateLight`: clear haze
  is0.00022 Miniature or0.0003 Anime, plus rain×0.00025. Inspect actual distance,
  camera framing and weather before choosing an atmospheric correction.
- The earlier47.35–49.62% compact-crop gross coverage is real but insufficient:
  it does not prove tight frontage, low visible ground, district-wide variation,
  or pleasing screen-space road/building balance. Preserve those measurements;
  remove their use as a reason to dismiss the city-wide concern.

## Current decision and order

Primary workstream: city-wide spatial composition and environmental depth.
Create a fixed, representative review matrix from ordinary three-seed worlds:
compact interior, local-street frontage, station/transport, waterfront and overview.
Use developing and mature checkpoints, fixed numerical camera/lens/quality,
DOF off, and noon plus low-sun controls. Include the uncomfortable views; do not
select only crops that already look dense or use sunset to hide geometry.

Measure street/deck projected area, visible residual ground, frontage continuity,
building gaps and height rhythms alongside corridor union and footprint coverage.
Check existing sky/mountain visibility and the finite-world boundary in those
same views. Label measured values, observed defects and causal hypotheses separately.
Do not infer exact real-world dimensions from unscaled maps.

Then choose ONE reusable, scene-scale correction. Horizon/edge integration is the
first rendering hypothesis to test because it may improve the whole composition
without changing simulation topology; establish why existing background geometry
fails before adding another backdrop. If spatial metrics show street/parcel rules
are the dominant failure, change those rules instead. Narrow lanes, irregular
blocks and sinuous corridors must retain legal junctions, parcel access, clearance,
and causal construction. Do not globally shrink roads or merely hide their edges.

Next priorities after that comparison: architecture/palette/material hierarchy
and frontage continuity; integrated vegetation and recognizable street furnishings;
water/bank response; purposeful activity and audiovisual continuity. Lighting and
camera support each comparison. These are ordered obligations, not simultaneous
speculative workstreams. Full day-cycle, motion, performance and artistic review
remain required.

The office fitting candidate remains local, with matched both-style views,
557open/3342stage envelope checks and sampled construction evidence. Rear assembly
visual inspection is still open; do not falsely label it complete. Preserve it
without extending its variants or letting that small check block the broader
reassessment. No production source was changed in response to this review yet.

Acceptance: a clear improvement at ordinary neighborhood AND wider city viewing
distances in both styles, with unchanged comparison conditions and readable depth
without DOF. Preserve normal scheduling, current-save determinism and believable
infrastructure. Reject improvements confined to one flattering view or accompanied
by new gaps/collisions. After two failed attempts, reassess. User artistic acceptance
is still unmet.
