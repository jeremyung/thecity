# Repeatable waterfront review camera — 2026-10-11

The two rejected water experiments lacked registered camera comparisons. A
development-only **City details → Renderer details → Hold waterfront view**
action now selects a deterministic river view and holds it in Explore. This
is a review action, not a change to normal automatic scene direction.

The target is the arc-length midpoint of the river, at y=0. The nominal camera
offset is (110,145,180) metres. Existing world bounds and collision recovery
apply. Follow and automatic transitions are cleared, residual OrbitControls
damping is consumed before setting the pose, and the previous damping setting
is restored. Lens, style, time, simulation and scheduler are unchanged. A world
without a usable river returns false. Different worlds can produce different
poses; compare the same checkpoint and viewport.

## Verification

- Root camera/UI tests: 62 passed. Coverage includes different prior modes and
  update histories, arc-length midpoint, held updates, lens preservation,
  invalid/disposed state, and a tower obstructing the nominal pose. A root-added
  adversarial controls clamp verifies no extra update undoes collision recovery.
  [Test log](waterfront-camera-tests.txt).
- TypeScript and production build pass; existing large-bundle warning remains.
- Browser: current mature seed 87231, tick 160000, 1839 buildings, clear weather,
  Anime, 18:00 light preview, 50 mm, DOF off, WebGPU/high detail, 1099×1071
  capture. Normal import settled both geometry queues at zero.
- Capture the preset, move to Overview, reopen the action, apply again and close
  the panel. [First capture](screenshots/waterfront-preset/first.jpg) and
  [repeated capture](screenshots/waterfront-preset/repeated.jpg) are byte-identical:
  SHA-256 `763784e09c0e00619add0fbd89fc9cc08e2eab0880de7050a1782c7a7118f56a`.
  This proves this paused, same-session browser witness; it does not establish
  cross-device pixel identity or moving-scene determinism.

For this checkpoint, the calculated nominal target is
(-119.6865692971,0,0.2985394558), with camera position
(-9.6865692971,145,180.2985394558). These are calculated from the authoritative
snapshot and preset, not extracted from hidden browser state.

## Next visual comparison

Import `.tools/fidelity-review/fair-routes-87231.json` paused, select the lens and
style, set a lighting preview and apply the preset. Wait for ordinary rendering
to settle and record queues before capturing. Keep viewport and all settings
identical across changes. Record separate noon/dusk/night views and examine
motion after the static comparison. The water shader remains unchanged from
before the rejected experiments. This preset exposes broad surface banding and
undistorted reflections; neither the preset nor passing tests constitutes a
water fidelity improvement.

## Low-angle pair — 2026-10-12

The preceding goal turn made progress by proving that lit building reflections
exist at a lower orbit but are suppressed by the normal water material.
Added DEV-only Hold low waterfront view alongside the unchanged overhead
preset. Both call reviewWaterfront with an explicit preset, use the arc-length
river midpoint, retain lens, clear automatic/follow state and enter Explore.
Low nominal offset is (130,55,215), overhead remains (110,145,180). Existing
bounds and building recovery apply; recovery can change the nominal pose.

Luna implemented the bounded camera/UI change. Root inspected it and restored
both overhead and low obstruction coverage by parameterizing the existing
collision test rather than replacing it. Root:48 camera tests pass; production
build passes with the existing bundle-size warning.

Normal full1839 save import,seed87231,tick75925,clear,paused22:00preview,
Anime,HighWebGPU,50mm,DOFoff,1280×720. All geometry queues zero before review.
The new low preset shows the river, far-bank facades and transport crossings.
After switching to Overview and back via the preset, the paused screenshot
is byte-identical. No captured browser warnings/errors. A transient HMR startup
notice cleared without retry before importing the fixture.

Evidence:
- screenshots/waterfront-growth/low-preset-anime-night.png
- screenshots/waterfront-growth/low-preset-anime-night-repeat.png

This is repeatable test framing, not an artistic improvement or a new Watch
scene. Retain both angles for the next bounded water-material experiment.
No water shader, authoritative city, saved format or simulation timing changed.
