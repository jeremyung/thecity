# Physical CoC integration plan

## Finding

`web/src/camera/lens.ts` already computes a thin-lens blur diameter in output pixels, but the live render pass does not consume it. `CityRenderer` passes the original scene `viewZ` node to Three's `dof()` and supplies positive axial focus depth, a style-specific focus-width threshold, and a style-specific bokeh scale. That depth input is correct and should remain an actual signed view-space Z value.

The installed `three/examples/jsm/tsl/display/DepthOfFieldNode.js` (the implementation re-exported by `three/addons/tsl/display/DepthOfFieldNode.js`) has a useful bounded gather, but no public per-pixel CoC input. In `setup()`, it computes `signedDist = -viewZ - focusDistance`, classifies near/far from its sign, and turns `smoothstep(0, focalLength, abs(signedDist))` into the two field masks. Its 64-tap then 16-tap passes sample a fixed Vogel kernel with offsets `invSize * bokehScale * CoC`; the CoC field is therefore normalized from 0 to 1 and `bokehScale` is the maximum gather radius in pixels. The near/far masks also drive the composite. The class's render targets and materials are private implementation details, so passing a fake `viewZ` value or trying to alter those internals is not a stable extension point.

## Recommended data contract

Keep the existing scene-depth path intact. Add an explicitly named per-pixel thin-lens field, calculated from positive axial depth `z = -viewZ` in scene metres and the current positive focus depth `s_f` in scene metres. Convert physical dimensions to metres before doing the lens calculation:

- `f = focalLengthMm / 1000`
- `D = f / fNumber`
- `v(z) = f*z / (z - f)` and `v(s_f) = f*s_f / (s_f - f)`
- sensor blur diameter in metres: `c_sensor = D * abs(v(z) - v(s_f)) / v(z)`
- signed output diameter: `c_px = sign(z - s_f) * (c_sensor * 1000 / sensorHeightMm) * outputHeightPx`

Use `z - s_f < 0` for near-field (negative) and `z - s_f > 0` for far-field (positive), matching the node's current sign convention. Preserve the sign for classification; clamp only the magnitude. Reject or return zero for invalid/non-finite depths and depths at or in front of the focal length. The CPU helper should expose signed diameter separately from any normalized gather value so the units remain auditable.

Use `opticalModelScale` as a separately named, positive viewer parameter for miniature presentation. Apply it only to the scene distances used by the optical calculation (`z * opticalModelScale`, `s_f * opticalModelScale`); do not modify camera matrices, view Z, simulation coordinates, or the focus-distance uniform. At scale 1, the calculation is the physical full-city lens and is expected to be subtle. A miniature preset may opt into a smaller scale to make the city behave like a tabletop model; when both object and focus distances remain beyond the focal length, reducing this scale increases their modeled defocus. Keep style's existing art direction distinct: it may affect the chosen model scale and maximum blur budget, but must not relabel physical millimetres or rewrite depth semantics.


For each gather pass, let `maxGatherRadiusPx` be a bounded per-pass radius budget. Derive `cocMagnitude = clamp(abs(c_px) * 0.5 / maxGatherRadiusPx, 0, 1)` because the thin-lens result is a diameter while each sample kernel's offset has a maximum radial extent. Three's `_invSize` is set to reciprocal full-resolution dimensions in `setSize()` and reused by the half-resolution gathers, so `bokehScale * cocMagnitude` expresses the nominal sample offset in full-output pixel units. Route `cocMagnitude` to near or far based on the signed value. Keep the near/far field textures and existing composite behavior; only the CoC generation changes. This avoids a diameter/radius factor-of-two error and bounds each gather pass independently of the optical model. Do not describe that bound as the total blur radius: the sequential 64- and 16-tap gathers expand effective support, and the near-field Gaussian preblur adds further spread.

## Integration boundary

There is no clean current public hook to inject that field into Three's `DepthOfFieldNode`: its constructor accepts color, true `viewZ`, focus depth, a linear world-space transition distance, and bokeh scale. Its CoC-generation block is inside `setup()`; the later gather and composite consume private render targets/materials. Do not encode CoC into the depth node, overload the `focalLength` argument, or mutate private node fields.

The smallest auditable production path is a local, app-owned `PhysicalDepthOfFieldNode` based on Three's existing bounded node, with one deliberate change: replace the CoC mask-generation block with signed thin-lens CoC normalized to a bounded gather radius. Preserve the existing 64/16 sample kernels, half-resolution near/far gathers, near-field edge preblur, and composite. Keep the copied/derived implementation isolated in one file and note the exact upstream Three revision beside it, so a future Three upgrade can be diffed. Avoid broad postprocessing changes or a second unbounded blur pass. If avoiding a local fork is a hard requirement, the alternative is a small upstream contribution that adds an optional normalized signed-CoC node while leaving the current distance ramp as the default; the installed API does not currently provide that extension point.

## Cost and limits

The change does not add taps or full-resolution gather targets. It replaces a small scalar CoC expression in the existing full-resolution CoC pass. The current node sets `_invSize` from the full-resolution color texture, allocates full-size two-channel CoC and composite targets plus half-size blur targets, and performs the 64- and 16-tap near/far gathers; preserving those passes keeps the cost profile stable. Physical blur can request a large radius for close foreground objects, so clamp each gather's input before sampling and retain existing half-resolution bounds. The sequential passes and near-field Gaussian spread mean the final blur footprint can exceed one gather's radius cap. Do not run CPU CoC evaluation per pixel; compute the formula in TSL from the true `viewZ` node and a handful of uniforms.

## Validation

- Unit-test signed CoC: an object in front of focus is near/negative, an object behind is far/positive, at-focus is zero, and a wider aperture (smaller f-number) or smaller miniature optical scale increases magnitude. Check 35/50/85 mm presets, finite/invalid boundaries, sensor-height pixel conversion, and the diameter-to-radius normalization.
- Shader-level test/inspection: the depth texture passed into the pass remains the unmodified `scenePass.getViewZNode()`; near/far masks use the sign, and normalized CoC remains in `[0,1]`. Verify Three's negative view-Z convention and near/far classification in both WebGPU and explicit WebGL fallback.
- Image comparison at fixed world, camera, time, viewport, and quality: foreground / focus subject / background; physical scale 1; miniature scale; 35 vs 50 vs 85 mm at the same camera pose. Confirm in-focus subject remains crisp, foreground and background blur on the correct sides, changing focal length does not move camera/target, and lens changes do not affect simulation hashes.
- Verify moving edges remain antialiased and stable, no bright/dark halos appear at near-field silhouettes, capped blur cannot leak beyond the bounded radius, and pause/resume or resize does not retain stale CoC targets. Compare both renderer backends and a reduced-resolution/mobile viewport.


## Implementation checkpoint

The CPU lens contract now provides signed CoC diameter, positive `opticalModelScale`, and explicit diameter-to-gather-radius normalization with nine passing lens tests. `web/src/render/postprocessing/SignedCoCDepthOfFieldNode.js` isolates a Three 0.183.2-derived bounded gather with an explicit signed normalized CoC input; its upstream source hash and MIT license are preserved. The 64/16-tap passes, sizes, composite, and disposal remain upstream behavior. This local node is not wired into the live city yet. Next: implement the equivalent TSL optical expression from unmodified scene view-Z, connect lens/viewport/style uniforms, then validate focus and silhouettes in both rendering backends. Syntax/type tests do not establish shader compilation or visual correctness.


### Live integration checkpoint

CityRenderer now derives signed thin-lens CoC from true view-Z and axial focus, converts sensor diameter to output pixels, and normalizes diameter/2 to a bounded 4px Anime or 8px Miniature gather radius. Optical scales .04/.01 are provisional art settings. Output height follows drawing-buffer resize; optics follow the director lens every frame, including automatic lens changes. Disabling DOF zeros the CoC without a zero-radius division. Build passed (`physical-dof-build.txt`) before the final director-sync correction. Initial default-backend browser render with DOF enabled showed the focal shop and blurred foreground/background, with no captured console errors. This is a smoke check only: explicit backend verification, matched lens/style comparisons, edge halos, resize, and fallback checks remain outstanding.


### Explicit WebGL smoke check

The UI reported WebGLBackend at `?webgl`, with depth of field enabled. Paused day 0 at 07:18, Explore camera, 50mm lens: Anime and Miniature both rendered and style switching completed with no captured console errors. Captures: `captures/physical-dof-webgl-diagnostics.png`, `physical-dof-webgl-anime.png`, `physical-dof-webgl-miniature.png`. This scene demonstrates fallback compatibility only, not matched cross-backend or aperture accuracy. Miniature foliage shows dark speckling relative to Anime; a focused material investigation is assigned. CPU/FPS numbers in the diagnostic screenshot are instantaneous, not benchmark results. The build after director lens synchronization also passed.


### Optical module and camera regression check

The production TSL CoC expression is isolated in `web/src/render/postprocessing/physical-coc.ts`, with an explicit typed node contract and shared sensor-height constant. Its denominator remains finite at invalid optical depths, and disabled output is zero. `npm run build` passed (`physical-coc-module-build.txt`); lens and camera suites passed 32/32. These cover lens framing/manual priority and CPU optics but not GPU numeric equivalence. A scalar execution test of the actual TSL expression against the independent CPU thin-lens model is assigned. Miniature foliage received a provisional night-aware shadow fill; visual acceptance remains pending.


### Independent scalar verification and foliage rejection

Root reran `tests/physical-coc.test.ts`, `tests/lens.test.ts`, and `tests/foliage-materials.test.ts`: 13 tests passed (`physical-coc-root-tests.txt`). The three scalar-expression cases cover production TSL arithmetic across lens presets, model scales, depth signs, output sizes, enable state and bounds against the independent thin-lens CPU oracle; this does not certify GPU compilation or pixel accuracy. Browser review of the provisional foliage fill showed pale gray crowns, so the first fill is not visually accepted. Suspected omission of instance tint in the emissive path is assigned for inspection. Capture: `captures/foliage-fill-first-review.png`.


### Corrected foliage tint browser check

The fill now includes the same instance-color varying used by diffuse foliage. Root inspected the updated Miniature WebGL founder scene: crowns retain green and blossom tint rather than the prior pale gray; no captured console errors. Evidence: `captures/foliage-fill-tinted-webgl.png`. This accepts the tint correction in that view only, not all leaf silhouette noise, night appearance, or matched cross-backend quality. Full web-suite session 37899 remains live with no completed summary at this checkpoint and must be polled, not restarted.


### Night foliage review

At lighting preview22:00 in the same paused Miniature WebGL founder scene, foliage stayed dark relative to the shop lighting with no obvious self-lit glow. No captured console errors. Evidence: `captures/foliage-fill-night-webgl.png`. The correction is acceptable for these dawn/night smoke views; mature crowns, motion, and WebGPU parity remain unverified.


### Current WebGPU confirmation

A fresh default-backend tab initially exceeded startup warning time without a captured shader error. During subsequent source hot reload, the latest page reached loop-started and visibly reported WebGPUBackend with Miniature and DOF enabled, empty rendering queues, and no captured errors. Evidence: `captures/foliage-dof-webgpu-diagnostics.png`. This confirms current shader compatibility; it does not explain or resolve the earlier startup delay, nor prove performance. Canvas `data-graphics-phase` now reports device-initialization, device-error, pipeline-setup, or loop-started for diagnosis (loop-started is not proof of a rendered frame).
