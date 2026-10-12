# Train kit plan — generated side elevations to running rail stock

Status: proposed (branch `render/train-kit`). This mirrors the vehicle-kit work
delivered on `render/vehicle-kit` (commit `1612c48`) and recorded in
[rendering spec](05-rendering.md#foundation), for the trains.

## Why

Road traffic and the city bus are traced from generated orthographic side
elevations and share one material, one atlas and one mask. Trains are still
three instanced boxes with static window strips (`city.ts`), a hardcoded
`(coach-1)*16` consist, no cab face, no lighting, no wheel motion and no
operator identity. The camera is told to follow trains and hold station scenes
([camera spec](06-camera.md)), so the train front is a first-class view — unlike
cars, where the front was explicitly left approximate. The city's first
milestone is *Last Train, First Light*; the train is the namesake subject.

## Current car pipeline (the template being copied)

Offline asset production:

| Step | File | Produces |
| --- | --- | --- |
| Approved generation config | `art-workshop/vehicle-pilot.json`, `vehicle-bus.json`, `vehicle-variants*.json`, `vehicle-liveries.json` | prompts, metric dimensions, models, cap, criteria |
| Hosted generation | `scripts/generate-vehicle-pilot.mjs` | images + durable `ledger.json`, no auto-retry, cost from usage |
| Local generation | `scripts/generate-vehicle-pilot-local.py` | ComfyUI HunyuanImage-2.1 images + `local-ledger.json` |
| Build input | `art-workshop/vehicle-kit.json` | source paths, ppm, atlas size, per-body dimensions, lamp bands, wheels, roofCut/tint/paintRegion |
| Kit builder | `scripts/build-vehicle-kits.py` | `public/art/vehicles/vehicle-atlas.png`, `vehicle-mask.png`, `web/src/render/vehicle-manifest.json`, debug overlay |
| Review decision | `art-workshop/vehicle-pilot/REVIEW.md` | measured proportion error, cost, keep/fallback decision |

Runtime:

- `web/src/render/vehicle-kit.ts` — extrudes the traced outline (side panels
  split at the belt with tumblehome, perimeter strip, wheels, roof sign, livery
  panel). Attributes `vpart`, `lateral`, `wheel`, `tinted`.
- `web/src/render/vehicle-materials.ts` — one `vehicleMaterial` for all bodies
  (paint via mask R, lamps/glass via mask G/B, grille trim, night emission,
  wheel roll/steer in `positionNode`, wetness) plus a canvas `liveryMaterial`
  and the `streamedTexture` async loader with placeholder and disposal.
- `web/src/render/vehicles.ts` — `VehicleFleet` instanced per body,
  `VehicleMotion` (odometer/steer/brake), deterministic `vehicleLook`.
- `web/src/render/buses.ts` — special case on the same kit with lit doorways
  and a destination display.
- `web/src/render/city.ts`, `asset-review.ts` — integration and dev viewer.
- Engine reports body class as movement kind (1,4,5,6,7,8); `KERB_LANE`, queue
  spacing by `vehicleLength`.
- Evidence: `benchmark-vehicles.mjs`, `capture-vehicle-review.mjs`,
  `capture-vehicle-city.mjs`; tests `vehicle-kit`, `vehicles`,
  `vehicle-texture-stream`, `buses`; `assets-manifest.json`; texture audit.

## How trains differ (the parts that must not be copied blindly)

1. **The front is the subject.** The car kit only reads a side elevation and
   approximates fascia; the spec lists that as open. A train needs a generated
   **front (nose)** view. The consist must have a real cab face with
   windshield, headlights, marker lights, coupler and destination sign.
2. **Consist, not a single body.** Cars are one instanced body. A train is a set
   (`cab + intermediate… + cab`) placed along the route. The current renderer
   hardcodes three coaches at −16/0/+16 m in `city.ts`, duplicating the engine's
   `TRAIN_HALF_LENGTH = 23.5`. The consist geometry belongs in the kit manifest
   and the engine and renderer must agree on total length, as `vehicleLength`
   does for cars.
3. **Bogies, not axles.** A coach rides on two bogies near its ends; the car
   wheel detector expects exactly two single discs. The builder needs bogie
   support and the wheel shader needs to rotate about a bogie/axle without
   steering (trains do not steer per-axle).
4. **Scale.** A 20 m coach at the car's 110 px/m is 2200 px, over the 2048 px
   atlas cap. Train pixels-per-metre must be lower (start ~72), and the atlas
   packs side + front + mask. Run `npm run audit:textures`.
5. **Livery is integral.** A city has one rail operator; the front livery and
   rollsign are part of the vehicle identity. Bake the livery into the elevation
   (like the bus `tint:false` path) rather than adding per-instance decals.
   Promote to `liveryMaterial` only if multiple operators are wanted.
6. **Lighting roles.** Continuous interior window bands, lit doorways at
   stations, head/tail/marker lamps, and the destination sign. The bus already
   shows lit doorways and `signTexture`; reuse that pattern.
7. **Two styles.** Everything must work in Anime (toon) and Miniature
   (standard) from the same geometry and atlas, per the style contract.

## Non-goals

- Multi-station service, schedules, or passenger simulation changes belong to
  the rail/transit work in [world growth](02-world-growth.md#rail-and-transit),
  not this asset branch. Keep engine changes to what the renderer needs.
- Do not expand the city maps or add new rail classes beyond what the pilot
  proves. Start with one commuter class and a symmetric consist.
- No provider credentials in the browser; no new runtime dependency.

## Plan

### Branch and baseline

- Work on `render/train-kit` (created), rebased on `main`.
- Add the train to `ArchitectureAssetReview` (`buildStationFront` already builds
  a platform, rails and sleepers) and capture the current box train before the
  swap, so before/after is measurable in both styles.

### Phase 1 — generation pilot (offline, no runtime)

- Add `art-workshop/train-pilot.json` modeled on `vehicle-pilot.json` /
  `vehicle-bus.json`: purpose, `capUSD`, hosted models with published
  estimates, a common strict-orthographic prompt, per-view prompts and metric
  dimensions, and acceptance criteria. Add a **three-view** block and
  front-view prompt rules (exact profile, level camera, no perspective, plain
  backdrop) plus a warning that a single view must not imply the others.
- Subjects for the first consist: `commuter-cab-front`, `commuter-cab-side`,
  `commuter-intermediate-side` (and a `commuter-motor` only if roof equipment
  and pantograph are wanted now). Rear = front mirrored unless the tail differs.
- Add `scripts/generate-train-pilot.mjs` (clone of the hosted generator, with
  `TRAIN_CONFIG`) and `scripts/generate-train-pilot-local.py` (clone of the
  local generator). Keep `art-workshop/train-pilot/ledger.json`,
  `local-ledger.json`, `contact-sheet.jpg`, `REVIEW.md`.
- Follow the settled car decision: local HunyuanImage-2.1 full with the
  orthographic suffix as default (free seed sweeps), `qwen/qwen-image-3` as the
  cleaner fallback, and try the installed `Qwen-Image-2.1` edit model to keep
  the front consistent with the approved side. Record every request and cost,
  cap at $2, no automatic retries.
- Gate: `REVIEW.md` records per-view proportion error, front/side consistency,
  cost and a keep/regenerate decision before any runtime work.

### Phase 2 — kit builder

- Add `art-workshop/train-kit.json`: pixels-per-metre, atlas size, and per
  consist: view source paths, metric length/width/height, tumblehome, belt,
  window/door/roof equipment bands, bogie centres and wheel radius, clearance,
  end rounding, lamp/headlight positions, livery mode.
- Add `scripts/build-train-kits.py`. First **extract the shared image
  operations** (`foreground`, `flood`, `dilate/erode`, `wheels`, `outline`,
  `simplify`, `masks`, `bleed`, `box_blur`) from `build-vehicle-kits.py` into a
  small module imported by both builders, so the car path is untouched and
  regression-tested, rather than forking 400 lines. New work in the train
  builder: front-view matting, front silhouette in (x, y), windshield/headlight/
  marker/destination masks, bogie detection, multi-view atlas packing, consist
  manifest.
- Outputs: `public/art/trains/train-atlas.png`, `train-mask.png`,
  `web/src/render/train-manifest.json`, `validation/captures/train-kit-debug.png`.
- Manifest per consist: `{length, width, height, tumblehome, cars:[…]}`, and per
  car role: side UV/outline/belt/wheels/paint/roofY, front UV/outline/lamps/
  glass/sign, bogies. Keep the metres convention (z forward, y up, ground y=0,
  kerb side +x) used by `vehicle-kit.ts`.

### Phase 3 — runtime kit, material and fleet

- Add `web/src/render/train-kit.ts`. Reuse the vehicle extrusion maths; add end
  caps from the front outline, bogie wheel groups, roof equipment, couplers and
  a destination sign. Keep `left`/`right` symmetry and the `vpart`/`lateral`/
  `wheel`/`tinted` attribute contract so the shader work is shared.
- Generalize `vehicle-materials.ts` into a traced-kit material factory (role and
  manifest argument) or add `train-materials.ts` that reuses `streamedTexture`,
  `wheelPosition`, the lamp/glass logic and `signTexture`. Interior window
  emission at night, lit doorways while dwelling, head/tail/marker lamps with
  direction, destination display.
- Add `web/src/render/trains.ts`: `TrainFleet` instanced per consist role with
  the same ownership/disposal discipline as `VehicleFleet`, consist placement
  driven by `direction` (cab leading and trailing), bogie roll, and one contact
  shadow per car or consist.
- Replace the box train in `city.ts`: remove `trains`, `trainGlass`,
  `trainStripe` and the train use of `wheels`; keep movement kind 2, picking and
  follow working via `actorIds`, and keep the audio snapshot's `trains` array.
  Remove the hardcoded −16/0/+16 loop; derive it from the manifest.
- Integrate into `asset-review.ts` and capture both styles, day and night.

### Phase 4 — engine touch (minimal)

- Default: no simulation change. The renderer builds the consist from the
  manifest and uses the existing `direction`/`progress`.
- If a consist/class or operator identity must persist, add serde-defaulted
  fields to `Train`/`TrainView` and mirror the total length in the manifest so
  `TRAIN_HALF_LENGTH` and the renderer agree (the cars' `vehicleLength`
  precedent). Do not change service behavior.
- Rebuild WASM (`npm run wasm`) after any engine edit; update engine tests.

### Phase 5 — evidence, tests, docs and art acceptance

- Tests: `tests/train-kit.test.ts` (outward winding, triangle budget, manifest
  size, bogie count, end caps, livery UV), `tests/trains.test.ts` (deterministic
  look, consist placement and reversal, disposal), extend the stream test only
  if the loader is shared. Keep `bus-wasm` and transit tests green.
- Benchmarks/captures: `scripts/benchmark-trains.mjs` (0/1/3 consists, median/
  p95/draws/triangles, hardware WebGL), `scripts/capture-train-review.mjs`,
  `scripts/capture-train-city.mjs`. Headless Chromium has no WebGPU adapter
  here; state that limit as the car work does.
- Docs/manifest: add a Train checkpoint to
  [rendering spec](05-rendering.md#foundation), a `train-kit` entry to
  `assets-manifest.json`, and update this plan with outcomes.
- Verify: `npm test`, `npm run test:engine -- --release`, `npm run wasm`,
  `npm run build`, `npm run audit:textures`, `git diff --check`.
- Art gate: compare with `concepts/04-last-train-first-light.png`,
  `concepts/05-dense-last-train-first-light.png` and
  `concepts/photo-references/rail-corridor-density.png`,
  `station-crossing-bus.png` in both styles with AA on. The primary agent owns
  art acceptance and must reject generic results even when tests pass.

## Risks and open questions

| Risk | Mitigation |
| --- | --- |
| Front/side inconsistency from text-to-image | Image-edit model conditioned on the approved side; accept only if the nose reads at ~128 px on screen |
| Coach length vs 2048 atlas cap | Lower pixels-per-metre; split per-view files if needed; measure decoded MiB |
| Consist length disagreement with engine | Single manifest source of truth, like `vehicleLength`; assert in tests |
| Bogie wheel shader reuse | Extend `wheel` attribute semantics; no steering on rail |
| Livery baked vs decal | Bake for one operator; promote to `liveryMaterial` only for multiple |
| WebGPU validation | No adapter in headless here; validate on hardware and state the gap |
| Car regressions from shared Python ops | Extract, don't fork; keep the car kit as the regression test |

## Definition of done

A generated commuter train runs on the existing rail corridor as a traced,
multi-car consist with a real cab face, lit windows and lamps, rolling bogies
and a destination sign; it matches both styles, both the engine's length and the
consist manifest, passes the checks above, and is accepted against the concept
boards. Cars and buses behave exactly as before.
