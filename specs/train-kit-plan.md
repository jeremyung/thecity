# Train kit — implementation plan (all phases)

Status: Phase 1 complete. Phases 0 (partly), 2–6 pending.
Branch: `render/train-kit`.
Related: [rendering spec](05-rendering.md), [world growth — rail and transit](02-world-growth.md#rail-and-transit), [pilot review](../art-workshop/train-pilot/REVIEW.md).

## 1. Goal and definition of done

Replace the three instanced boxes that stand in for trains with a kit traced
from generated orthographic elevations, at the same quality bar as the road
vehicles:

> A generated commuter train runs on the existing rail corridor as a traced,
> multi-car consist with a real cab face, lit windows, lamps and destination
> sign, rolling bogies, and a consistent livery pointing the correct way at
> both terminals — in both rendering styles and both backends, matching the
> engine's length and the consist manifest, accepted against the concept boards.
> Cars and buses behave exactly as before.

Non-goals: multi-station service, schedules, passenger logic, new rail classes,
or map growth. Those belong to the rail/transit work in the world-growth spec.
Engine changes are limited to what the renderer needs.

## 2. Locked decisions

These came from the vehicle-kit prior art and the Phase 1 pilot; later phases
should not relitigate them without new evidence.

| # | Decision | Source |
| --- | --- | --- |
| D1 | **Class-aware from the start, one class shipped.** The manifest and runtime key by train class, but Phases 0–6 deliver only the commuter EMU. Additional classes are new source views plus a manifest entry plus engine eligibility, never a rewrite. Symmetric consist `cab + intermediate… + cab`; the renderer orients cars from `direction`, so the tail is a mirrored cab. | user review; current 3-coach layout |
| D2 | The kit unit is a **car body** plus a traced **front end cap**. The front is a first-class source view, not approximated from the side. | pilot cleared the cab front |
| D3 | Matting uses a **separable row+column background model** from the car-free top band. The vehicle builder's per-row test fails on a 20 m car with a visible floor and horizon. | pilot finding |
| D4 | Roof equipment and cab presence are assigned **per consist role**, not inferred from a single generated image (the pilot's "trailer" came back with a pantograph). | pilot finding |
| D5 | Livery is **baked into the elevation** (`tint:false`, like the bus). Promote to per-instance decals only if multiple operators are wanted. | one operator |
| D6 | The **consist manifest is the single source of truth** for car lengths; the engine's rail length must agree, as `vehicleLength` does for cars. | `TRAIN_HALF_LENGTH` |
| D7 | Share code rather than fork: extract the Python image ops from `build-vehicle-kits.py`, and generalize the TS material; keep the car kit as the regression test. | plan |
| D8 | Local HunyuanImage-2.1 full is the default generator; `Qwen-Image-2.1` (installed) is the edit fallback, `qwen/qwen-image-3` the hosted flat-elevation fallback. | pilot decision |

## 2.1 Train classes and variety

"Multiple train types" mixes three independent axes. Keeping them separate is
what makes variety cheap; conflating them is what makes it a rewrite.

- **Livery / operator** — paint and decals on a shared body. No new geometry.
  Highest visual variety per unit of work; the car kit already ships thirteen
  invented operators this way.
- **Class** — silhouette, dimensions, consist, front cap. Needs new source
  views and a manifest entry.
- **Mode / service** — corridor, route, stopping pattern, speed, platform.
  Simulation and world growth, not art.

Plausible types, against what the prototype can actually show today (one
elevated corridor, one station, one gateway, no line or class concept):

| Type | Infrastructure it needs | Asset cost | Priority |
| --- | --- | --- | --- |
| Commuter EMU (JR/private) | the existing elevated line | pilot done | 1 — the default |
| Operator / livery variants | same line | canvas or tint only | 1 — bundle with the class |
| Rapid / limited express | same corridor, fewer stops, maybe 6 cars | body + livery; same front family | 2 — when multi-station service lands |
| Subway / metro stock | a tunnel or covered station | another commuter-like class | 3 — only once a covered line exists |
| Shinkansen | dedicated high-speed corridor and a segregated station | large nose, ~25 m cars, new platform geometry | 4 — its own world milestone |
| Tram / light rail | street track and overhead wire | narrow articulated car | 4 — a different mode entirely |
| Freight | shared corridor plus a siding | container flats, no passengers | 5 — low priority |

The engine currently exposes none of this. `TrainView` has no class, and the
renderer cannot tell a Shinkansen from a local. So the class abstraction has to
be introduced deliberately:

- The manifest is keyed by class id (`classes: { commuter: … }`), and each class
  owns its dimensions, doors-per-side, nose profile, consist and atlas entry.
- Add a serde-defaulted `class` (and optionally `operator`) to `Train` /
  `TrainView`, defaulting to the commuter class so existing saves and fixtures
  still load. The renderer reads it by id the way `buses.ts` reads
  `world.buses.find(b => b.id === id)` — `city.ts` already has `worldState` in
  `updateActors`.
- Within a class, livery is per-instance paint/decal on the shared body.
- Do **not** attempt Shinkansen, subway or tram art before the world can render
  their corridors and the simulation can schedule them; a nose with no track is
  the worst kind of scope creep.

## 3. Phase overview

| Phase | Outcome | Depends on | Exit gate |
| --- | --- | --- | --- |
| 0 | Branch, baseline capture, hygiene | — | before-train capture exists |
| 1 | Generation pilot and keep decision | 0 | **done** — `REVIEW.md` |
| 2 | Runtime assets: atlas, mask, manifest | 1 | builder runs; debug overlay reviewed |
| 3 | Geometry and material modules | 2 | unit tests green |
| 4 | Fleet, city integration, engine agreement | 3 | train runs in the city; buses/cars unchanged |
| 5 | Review scene, captures, benchmark | 4 | both styles captured; frame cost measured |
| 6 | Tests, docs, evidence, PR | 2–5 | full check suite green; art accepted |

Work lands as reviewable commits per phase; Phases 2–6 may ship as one PR or be
split (2–3 offline, 4–6 runtime) if review prefers.

## 4. Phase detail

### Phase 0 — Branch and baseline (partly done)

- [x] Create `render/train-kit` from `main`.
- [x] Add `.claude/` to `.gitignore` so worktrees cannot be swept into a commit.
- [ ] Capture the current box train in `asset-review` (`buildStationFront`
      already builds a platform, rails and sleepers), both styles, before the
      swap. Store under `validation/captures/train-kit/` or the pilot dir.
- [ ] Record the baseline consist constants that must be preserved:
      `TRAIN_HALF_LENGTH = 23.5` (`engine/src/lib.rs`), coach offsets
      `(coach-1)*16` and `RAIL_DECK` (`web/src/render/city.ts`).

**Exit:** a checked-in "before" image and a note of the constants.

### Phase 1 — Generation pilot (done)

- [x] `art-workshop/train-pilot.json` (views, dimensions, suffixes, criteria).
- [x] `scripts/generate-train-pilot-local.py` (per-view sizes, local ledger).
- [x] `scripts/review-train-pilot.py` (separable matting, metrics, contact
      sheet, at-scale sheet).
- [x] Four views generated locally at $0; body proportion error −0.8% to −8.9%
      on sides, −1.6% on the front; livery/window bands consistent.
- [x] Keep decision in `art-workshop/train-pilot/REVIEW.md`.

**Exit:** met. Fallback path (edit model) documented but not needed yet.

### Phase 2 — Kit builder

Offline; no runtime change.

- [ ] **Extract shared image ops.** Move `foreground`, `flood`, `dilate`,
      `erode`, `shift`, `wheels`, `refine_wheel`, `column_outline`, `simplify`,
      `masks`, `box_blur`, `masked_blur`, `bleed` from
      `scripts/build-vehicle-kits.py` into e.g.
      `scripts/vehicle_kit_image.py`, imported by the vehicle builder. Re-run
      the car build and confirm byte-identical outputs before proceeding.
- [ ] **Add the separable background model** to that module (`background_model`
      as prototyped in `scripts/review-train-pilot.py`), and switch the vehicle
      builder to it only if the car outputs remain identical; otherwise expose
      it as an option.
- [ ] `art-workshop/train-kit.json` input, **keyed by class** so later classes
      are additive: pixels-per-metre (start ~72 so a 20 m car fits the 2048
      atlas), atlas size, and per class, per consist: view source paths, metric
      length/width/height, tumblehome, belt, doors-per-side, window/door/roof
      bands, bogie centres and wheel radius, clearance, end rounding, lamp and
      headlight positions, destination-sign box, livery mode, per-role roof
      equipment.
- [ ] `scripts/build-train-kits.py`:
      - matte each view with the separable model;
      - trace the side outline (reuse `column_outline` + `simplify`);
      - detect **bogies** (two axles per end) rather than two single wheels;
      - trace the **front silhouette** in (x, y) and the windshield, head/tail
        lamp, marker and destination-sign masks;
      - normalize paint to grey for tinting (or skip for baked livery);
      - bleed edges and pack a multi-view atlas;
      - write the consist manifest in metres (z forward, y up, ground y=0,
        kerb side +x) matching `vehicle-manifest.json` conventions, under a
        `classes` map keyed by class id.
- [ ] Outputs: `public/art/trains/train-atlas.png`, `train-mask.png`,
      `web/src/render/train-manifest.json`,
      `validation/captures/train-kit-debug.png`.
- [ ] Respect budgets: run `npm run audit:textures`; keep the atlas within the
      2048 side cap and the decoded-mip total within policy.

**Exit:** builder is deterministic and re-runnable; debug overlay shows correct
outline, bogies, paint/lamp/glass masks, and the front cap for review.

### Phase 3 — Runtime kit and material

- [ ] `web/src/render/train-kit.ts`:
      - reuse the `Builder`, ring clipping, tumblehome and end-rounding maths
        from `vehicle-kit.ts`;
      - add end caps from the front outline, bogie wheel groups, couplers,
        roof equipment (AC boxes, pantograph), and the destination-sign box;
      - keep the attribute contract (`vpart`, `lateral`, `wheel`, `tinted`),
        extending `vpart` for end cap / bogie / sign if needed;
      - export `trainClass(id)`, `trainSpec(className)`, `consistSpec(className)`,
        `buildTrainGeometry(className, role)` and a consist length helper, so the
        runtime selects by class rather than importing one hardcoded body.
- [ ] Material: generalize `vehicle-materials.ts` into a traced-kit factory
      (body role + manifest argument) or add `train-materials.ts` reusing
      `streamedTexture`, `wheelPosition`, the lamp/glass logic and
      `signTexture`. Required behaviour:
      - interior window band emissive at night;
      - lit doorways while dwelling (as `buses.ts` does);
      - head/tail/marker lamps keyed to the car's direction;
      - destination display via `signTexture`;
      - both styles from the same atlas.
- [ ] Tests: `tests/train-kit.test.ts` (outward winding, triangle budget,
      manifest size, bogie count, end caps, consist length, livery UV).

**Exit:** geometry and material unit tests pass; `tsc --noEmit` clean.

### Phase 4 — Fleet, city integration, engine agreement

- [ ] `web/src/render/trains.ts`: `TrainFleet` instanced per consist role, with
      the ownership/disposal discipline of `VehicleFleet`; consist placement by
      `direction` (cab leading/trailing); bogie roll; contact shadow; stable
      per-consist look (livery/baked paint).
- [ ] Replace the box train in `city.ts`:
      - remove `this.trains`, `this.trainGlass`, `this.trainStripe`, and the
        train use of `this.wheels` (`~114–117`, `~201–207`, `~595`, `~599–600`);
      - drive placement from the manifest, deleting the hardcoded
        `(coach-1)*16` loop;
      - keep movement kind 2, `actorIds` picking, follow, and the audio
        snapshot's `trains` array intact.
- [ ] Add the train to `asset-review.ts` at `buildStationFront`, and to
      `buildTraffic` if useful for review.
- [ ] **Engine class identity (D1, D6).** Add a serde-defaulted `class` (and
      `operator` if useful) to `Train` / `TrainView`, defaulting to the commuter
      class so existing saves and fixtures load. Read it in `city.ts` by id, as
      `buses.ts` reads `world.buses`, and pass it to `TrainFleet`. Add the class
      to `movement_buffer` only if per-car poses are needed; otherwise the id
      lookup is enough. Keep the renderer's consist length and
      `TRAIN_HALF_LENGTH` in agreement (assert in a test). Do not change service
      behaviour. Run `npm run test:engine -- --release` and `npm run wasm`.
- [ ] **Livery variants within the class.** Reuse the per-instance paint and
      livery-panel machinery so one class reads as several operators; only add a
      second class when its silhouette genuinely differs.

**Exit:** the traced train runs in the real city in both styles; cars and buses
are visually and behaviourally unchanged.

### Phase 5 — Review scene, captures, benchmark

- [ ] `scripts/capture-train-review.mjs` (lineup on the review station, both
      styles, day/night) and `scripts/capture-train-city.mjs` (train in the
      living city, following it).
- [ ] `scripts/benchmark-trains.mjs`: 0/1/3 consists, median/p95, draw calls and
      triangles, hardware WebGL (ANGLE/OpenGL ES, as `benchmark-vehicles.mjs`).
      State that headless Chromium exposes no WebGPU adapter here.
- [ ] Capture the train at 128/64 px for typology and inspect AA, night
      emission, curve/bridge running, and reverse at the far terminal.

**Exit:** before/after captures and a frame-cost number suitable for the spec
checkpoint.

### Phase 6 — Tests, docs, evidence, PR

- [ ] `tests/trains.test.ts` (deterministic look, consist placement and
      reversal, disposal) and extend `tests/vehicle-texture-stream.test.ts`
      only if the loader was changed. Keep bus/transit tests green.
- [ ] Update `specs/05-rendering.md` with a Train checkpoint modelled on the
      Vehicle checkpoint (geometry, atlas/mask sizes, draw/tri/frame numbers,
      open items).
- [ ] Add a `train-kit` runtime entry to `assets-manifest.json` (and move the
      Phase 1 pilot images from development candidate to their final role).
- [ ] Update this plan with outcomes and any changed decisions.
- [ ] Verify: `npm test`, `npm run test:engine -- --release`, `npm run wasm`,
      `npm run build`, `npm run audit:textures`, `git diff --check`.
- [ ] Art gate: compare with `concepts/04-last-train-first-light.png`,
      `concepts/05-dense-last-train-first-light.png` and
      `concepts/photo-references/rail-corridor-density.png`,
      `station-crossing-bus.png`, both styles, AA on. The primary agent owns
      art acceptance and must reject generic results even when tests pass.

**Exit:** definition of done satisfied; PR description written.

## 5. File inventory

New:

- `art-workshop/train-kit.json`
- `scripts/build-train-kits.py`, shared image-ops module
- `public/art/trains/train-atlas.png`, `train-mask.png`
- `web/src/render/train-manifest.json`
- `web/src/render/train-kit.ts`, `train-materials.ts` (or generalized material)
- `web/src/render/trains.ts`
- `tests/train-kit.test.ts`, `tests/trains.test.ts`
- `scripts/benchmark-trains.mjs`, `capture-train-review.mjs`,
  `capture-train-city.mjs`

Modified:

- `scripts/build-vehicle-kits.py` (imports the extracted ops; outputs unchanged)
- `web/src/render/vehicle-materials.ts` (generalized) and/or `vehicles.ts`
- `web/src/render/city.ts`, `web/src/render/asset-review.ts`
- `web/src/render/types.ts` if `World.trains` typing changes
- `engine/src/lib.rs` only if Phase 4 needs it (+ rebuilt WASM)
- `assets-manifest.json`, `specs/05-rendering.md`, `.gitignore`

## 6. Verification commands

```sh
npm test
npm run test:engine -- --release   # only if the engine changed
npm run wasm                       # only if the engine changed
npm run build
npm run audit:textures
git diff --check
# offline kit rebuild
python3 scripts/build-train-kits.py
python3 scripts/review-train-pilot.py
```

## 7. Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Front/side or role mismatch across generated views | Per-role assignment (D4); edit model fallback (D8); accept only if the nose reads at ~128 px |
| Coach length vs 2048 atlas cap | Lower ppm (~72); split per-view files if needed; measure decoded MiB |
| Consist length disagrees with the engine | Manifest is the single source of truth (D6); assert in tests |
| Car regressions from shared Python ops | Extract, do not fork; confirm byte-identical car outputs before proceeding |
| Bogie wheel shader reuse | Extend `wheel` semantics; no per-axle steering on rail |
| `city.ts` train/picking/audio breakage | Remove only the box meshes; keep kind 2, `actorIds`, and the audio snapshot |
| WebGPU unvalidated | No adapter in headless; validate on hardware and state the gap |
| Visual quality | Compare against concept boards in both styles before acceptance |

## 8. Open decisions

1. **Class scope for the first PR.** Commuter only, or commuter plus a few
   operator liveries? Recommendation: both — livery variants are nearly free
   and give the visible variety the question is really about.
2. **Add the engine `class` field now?** Recommendation: yes, serde-defaulted to
   the commuter class. It is a small change that avoids a second renderer
   refactor when the next class arrives.
3. **Consist length and composition.** Keep `TRAIN_HALF_LENGTH = 23.5` and three
   cars, or move to a manifest-defined consist length? Affects the engine
   agreement.
4. **When to attempt a second physical class.** Recommendation: only with the
   infrastructure — rapid/limited express with multi-station service, subway
   with a covered line, Shinkansen with its own corridor and station. Each is a
   world milestone, not an asset branch.
5. **Interior lighting fidelity.** Emissive window band only, or lit interior
   visible through glass? Start with the band.
6. **Pantograph.** Static raised arm on the motor car, or omit until a catenary
   exists? The corridor renders no overhead wire today.
7. **Save compatibility.** If engine fields are added, confirm the
   serde-default policy against the current "saves must validate and resume"
   requirement.

## 9. Definition of done

The Phase 6 exit gate plus this checklist:

- [ ] Traced consist with a real cab face, end caps, bogies and roof equipment.
- [ ] Class selected by id from the engine; a second class is additive, and one
      class can carry several operator liveries.
- [ ] Night windows, head/tail lamps, lit doorways, destination sign.
- [ ] Correct orientation at both terminals; consistent livery.
- [ ] Both styles and both backends; AA clean on rails and silhouettes.
- [ ] Length agrees with the engine; cars and buses unchanged.
- [ ] Tests, build, texture audit and evidence committed; art accepted.
