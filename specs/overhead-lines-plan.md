# Overhead lines — implementation plan

Status: rebased onto `codex/city-runtime-review` (2026-10-15). Street wiring (Phases 1, 2 and 5) is implemented on `render/overhead-lines`. The rail work in Phases 0 and 3 is superseded by that branch. Phase 4 (service drops) is deferred.
Related: [rendering spec](05-rendering.md), [visual release gate](08-delivery.md#visual-quality-is-a-release-gate).

## Outcome after the rebase (2026-10-15)

The first version targeted `main` and drew its own straight-segment rail catenary. It also fixed train seating and the pantograph. `codex/city-runtime-review` already does all of that along the engine's curved rail alignments, so the rebase keeps only the street wiring:

- **Dropped.** The rail masts and catenary, the train and platform height constants and the pantograph change are superseded by `rail-corridor.ts`, `rail-profile.ts` (`RAIL_HEAD_Y`, `PANTOGRAPH_CONTACT_HEIGHT`) and the engine's `rail_platforms`.
- **Curved keep-outs.** Pole and span keep-outs now follow the curves the renderer draws:
  - rail alignment samples
  - expressway deck Béziers, sampled every 4 m
- **Expressway decks.** Each deck carries its own height and profile width, and poles beneath a deck are cut to 2.5 m below it.
- **Stations.** Every engine platform uses the foliage clearance (`overlapsStation`).
- **Entry paths.** Clearances cover every leg of an entry path, not just the door-to-curb pair.
- **Rebuilds.** The layer rebuilds with the park layer, whose key already covers decks, platforms and alignments.
- **Evidence.** Captures stay local to match the branch's no-binary-evidence rule; `scripts/capture-overhead-lines.mjs` reproduces them.
- **Benchmark.** 758 poles, +9–13 draws, 0–0.2 ms per frame and a 23 ms rebuild ([report](../validation/overhead-lines-benchmark.md)).

The original plan follows for reference. Its rail sections (Phase 0's train heights and Phase 3) describe the superseded straight-segment work.

## 1. Goal and definition of done

Tokyo streets read as Tokyo largely because of what hangs over them: concrete
utility poles every few tens of metres, sagging wire bundles that cross at
junctions, pole transformers, and drop lines to houses. Elevated railways carry
masts and a contact wire. The city currently has neither. The only wires are a
pair of poles with one taut wire on anchor corner lots of at least 16 × 16 m,
kept inside the parcel (`kit.ts:1391`). The rail viaduct has rails and sleepers
but no overhead line (`city.ts:584`), even though the motor car carries a
pantograph (`train-kit.ts:142`).

Done means:

- Open, connected local streets and lanes carry deterministic utility poles
  and wire spans. Arterials follow the policy settled in §8.
- The rail viaduct carries masts, a cantilever and a contact wire, set at the
  height the pantograph actually reaches.
- Poles never collide with lamps, street trees, building entries, bus stops,
  park, plaza or shrine entries, or the scramble crossing.
- Both styles render the lines without visible shimmer while the camera moves.
- The added draw calls and triangles are measured and recorded.
- Captures at street level and at overview distance pass the visual review.

## 2. Locked decisions

- **Renderer only.** No engine or save changes. Placement is a pure function
  of `World`, like `streetLightPlacements()` and `streetTreePlacements()`.
- **One placement module**, `web/src/render/overhead-lines.ts`, which exports
  `utilityPolePlacements(world)`, `wireSpans(placements)`,
  `railMastPlacements(world)` and `buildOverheadLines(world, art)`.
- **Rebuild with the park layer.** The `parkKey` in `city.ts:315` already
  combines road signatures, open spaces, building entries and bus stops, which
  are exactly the inputs pole clearance needs. No new invalidation key.
- **Instanced poles, merged wires.** One `InstancedMesh` per pole part (shaft,
  crossarm, transformer, rail mast). Wires go into one merged ribbon geometry
  per 160 m spatial cell, so frustum culling still works. Materials come from
  `art.get(...)`, so both styles share the geometry.
- **Replace the anchor-lot special case.** Remove the parcel pole pair in
  `kit.ts:1391` once street poles exist, so the two systems never overlap.

## 3. Phases

| Phase | Scope | Exit |
|---|---|---|
| 0 | Baseline and measurements | Captures saved; pantograph and rail-top heights recorded |
| 1 | Street pole placement and tests | Deterministic placements pass the clearance tests |
| 2 | Pole and wire geometry | Street-level and overview captures in both styles |
| 3 | Rail overhead line | Train pantograph meets the contact wire at every station approach |
| 4 | Service drops (optional, §8) | Drops follow building turnover without rebuild stalls |
| 5 | Cleanup, benchmark, docs, PR | Benchmark recorded; spec checkpoint; asset manifest updated |

## 4. Phase detail

### Phase 0 — Baseline and measurements

- Capture seed 87231 at tick 242000 in Anime and Miniature, with depth of
  field off. Take a local street, a lane, an arterial, the station approach
  and the overview. Extend `scripts/capture-parks.mjs` or add
  `scripts/capture-overhead-lines.mjs` with the same camera list, so the
  before and after views match.
- Measure in world space where the pantograph head sits.
  - `TrainFleet.add` is called with lift `RAIL_DECK` (6 m, `city.ts:658`).
  - The rail deck box is drawn at `y = 1 + RAIL_DECK` and the rails at
    `y + .45` (`city.ts:566`, `:585`).
  - Confirm that the wheels sit on the rails. If they don't, fix that first,
    because the contact-wire height follows from it.
- Record the base draw calls and triangles with the existing benchmark
  harness.

### Phase 1 — Street pole placement (`overhead-lines.ts`)

- **Spacing.** Walk each open, connected `local` and `lane` road (arterials
  per §8), sorted by id like `streetLightPlacements()`. Aim for 30 m between
  poles, within 26–36 m. Keep 6 m clear of junction nodes, so the corner pole
  stands on the approach rather than in the turn. Poles stay on one side of
  the street, and the side is seeded by road id. They switch sides only where
  a span would cross a junction, as on real streets.
- **Offset.** On local streets and lanes, put the pole 0.35 m inside the
  carriageway edge. These are shared surfaces with no sidewalk, and Tokyo
  poles stand at the edge of the road. On arterials, put it on the sidewalk,
  outside the street-tree line.
- **Clearance.** Use `SpatialCells` the same way `streetTreePlacements()`
  does: 2 m to lamps, 1.5 m to street trees, the door-to-curb segment of
  each `building_access` plus 1 m, 4 m to bus stops, and open-space lots
  plus 1 m. A blocked candidate slides up to ±4 m along the road. If no
  position fits, drop the pole, and let the span join the poles on either
  side when the result is no longer than 45 m.
- **Junctions.** Where two pole runs end within 25 m of each other across a
  junction node, add a crossing span. This produces the familiar wire
  web at intersections without a pole in the junction itself.
- **Bridges, expressway and rail.** No poles on bridge decks. Under the
  expressway, keep poles out of the pier line, as the street trees do. Keep
  poles out of the rail viaduct corridor.
- **Cap.** At most 1,200 poles, matching the lamp and tree caps.
- **Variety.** Each pole carries seeded attributes: height of 10–12 m, two
  or three crossarms, and a transformer can on about one pole in four. Every
  third pole gets a street-number plate band.

Tests (`tests/overhead-lines.test.ts`):

- The same world always gives the same placements.
- Spacing stays within bounds.
- Every clearance holds for lamps, trees, doors, stops and open spaces.
- No pole lands on a bridge, the expressway pier line or the rail corridor.
- The cap holds.
- Junction crossings join only poles that are within reach.
- Lanes and local streets produce poles; survey and unconnected roads do not.

### Phase 2 — Geometry and materials

- **Pole parts.**
  - Tapered octagonal concrete shaft (`#a9a89f`), 0.17 m radius at the base
    and 0.11 m at the top.
  - Steel crossarms and porcelain insulators.
  - Grey transformer cylinders, and a step-bolt band from 2.5 m up.
  - All parts are instanced and receive shadows. Only the shaft casts shadows.
- **Wires.**
  - Each span has three or four conductors: high voltage on the top crossarm,
    low voltage below, and a telecom bundle lowest.
  - The sag is parabolic, 0.25–0.5 m at mid-span and scaled by span length.
  - Each conductor is a ribbon of 8–12 segments that turns to face the camera
    in the vertex stage.
- **Anti-aliasing**, the main risk.
  - Clamp the ribbon's width in the vertex stage to at least about one pixel.
  - Fade its opacity as the true width drops below one pixel, so distant wires
    thin out instead of shimmering.
  - Fade wires out completely past about 450 m. Poles stay visible.
- **Styles.**
  - Anime: wires near-black with a cool sky rim.
  - Miniature: slightly thicker, matte, darker cords that read as a scale
    model.
  - Neither style gives the lines night emission. Transformers are not lit.
- **Budget.** Aim for about 6 draw calls, plus one per wire cell in view, and
  under 150 k triangles for a mature city. Phase 5 measures the real figures.

### Phase 3 — Rail overhead line

- **Masts.** `railMastPlacements(world)` puts H-section steel masts every
  50 m along each open rail segment, on the outer edge of the viaduct deck.
  They stay clear of the station platform's sides and canopy
  (`city.ts:530–549`), and where the rail meets the platform the gap closes
  with a portal beam.
- **Cantilever and wires.** A cantilever with a register arm holds the
  messenger wire and the contact wire. The contact wire hangs at the
  pantograph head height measured in Phase 0. It zig-zags ±0.2 m between
  masts, as on real lines, with droppers every 5 m on the near level of
  detail only.
- **Pantograph match.** Raise the pantograph in `train-kit.ts` (the bar at
  `roofY + 2.0`) so it meets that height. Add a test that the pantograph top
  and the contact wire agree to within 2 cm.
- **Tests.** Mast spacing, platform clearance, and wire continuity across
  segment joins: the end of one segment meets the start of the next.

### Phase 4 — Service drops (optional)

- From each occupied low-rise building that has a `building_access`, run one
  drop line from the nearest pole within 18 m to a fixing point under the
  eave on the street face.
- Build the drops into the same per-cell wire geometry.
- Building turnover changes drops often, so update one cell per frame at
  most. Don't rebuild the whole layer.
- Skip the drop when the line would pass through another building's footprint.

### Phase 5 — Cleanup, benchmark, docs, PR

- Remove the anchor-lot poles in `kit.ts:1391–1408` and update any kit test
  that counts their parts.
- Add `scripts/benchmark-overhead-lines.mjs`, modelled on
  `scripts/benchmark-trains.mjs`, to measure draw calls, triangles and frame
  time against the Phase 0 baseline in hardware WebGL.
- Add a dated checkpoint in `specs/05-rendering.md`, an entry in
  `assets-manifest.json`, and a line in the README's current scope.
- Save captures under `validation/captures/overhead-lines/`. Include a
  moving-camera clip, or a frame strip 0.25 s apart, for the shimmer review.

## 5. File inventory

| File | Change |
|---|---|
| `web/src/render/overhead-lines.ts` | new: placement, spans, geometry |
| `web/src/render/city.ts` | build the layer in `rebuildParks()` alongside the street trees |
| `web/src/render/kit.ts` | remove anchor-lot poles |
| `web/src/render/train-kit.ts` | pantograph height |
| `web/src/render/parks.ts` | none expected: `streetTreePlacements()` is already exported, so pass its result from `rebuildParks()` |
| `tests/overhead-lines.test.ts` | new |
| `scripts/capture-overhead-lines.mjs`, `scripts/benchmark-overhead-lines.mjs` | new |
| `specs/05-rendering.md`, `assets-manifest.json`, `README.md` | docs |

## 6. Verification commands

```sh
npm test tests/overhead-lines.test.ts tests/parks.test.ts tests/street-lighting.test.ts tests/train-kit.test.ts
npm test
npm run build
node scripts/capture-overhead-lines.mjs anime && node scripts/capture-overhead-lines.mjs miniature
node scripts/benchmark-overhead-lines.mjs
git diff --check
```

## 7. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Thin wires shimmer or crawl in motion, which fails the anti-aliasing gate | Width clamp, sub-pixel opacity fade, distance cutoff; review the moving-camera strip, not only still frames |
| Lines clutter the overview and hide the street fabric | Wires fade with distance; check the overview against `concepts/04-…` and `concepts/01-…` before tuning density |
| Poles collide with furniture added later | All clearance checks sit in one `SpatialCells` pass with tests; new furniture registers there |
| Rebuild cost when roads change | Rebuild with the park layer, which already rebuilds only on relevant changes; record the rebuild time in Phase 5 |
| Pantograph and contact wire disagree | Measure in Phase 0, and test the agreement in Phase 3 |
| Camera collides with or clips poles at street level | Check the camera clearance used for buildings (`tests/camera-clearance.test.ts`); add poles as obstacles if the street camera passes through them |

## 8. Open decisions

1. **Arterials: poles, or buried cables?** Many Tokyo arterials and station
   areas have their cables underground. Recommendation: no poles on
   arterials within 220 m of the station, which is the same radius as the
   station-area displays. Poles elsewhere stand on the sidewalk.
2. **Service drops (Phase 4): in this PR or a follow-up?** Recommendation:
   a follow-up, because it couples wires to building turnover.
3. **Rail catenary style.** Simple catenary with droppers, or a single
   contact wire? Recommendation: simple catenary, with the droppers on the
   near level of detail only.
