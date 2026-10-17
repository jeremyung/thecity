# Fully curved media landmark

Review: `?review=architecture-city&family=curved-media&district=shibuya`. The isolated fixture uses the production recipe and sparse landmark selector. The initial camera frames the whole building. Dusk and Animated reveal the continuous panorama around both corners; Show controls restores lighting, display, lifecycle and seed controls.

The fourth landmark grammar rounds the actual podium and upper floorplates, glazing, roof perimeter and crown. A recessed upper section leaves a curved podium terrace. The display uses arc-length panorama UVs and outward analytic normals. Glazing retains pane attributes and uses the shared physical-glass family plus the existing provisional environment capture; detailed local reflections remain #23. The shared presentation clock animates content even when simulation is paused.

Validation: 18 focused curved-landmark, other-landmark and signage tests passed, along with production build and final typecheck. Tests cover phase/LOD/display controls, ownership, parcel bounds, continuous UVs and unit glazing normals. WebGPU review showed no reported console errors; paired screenshots show artwork moving at the same camera. This curved extension has not received a separate WebGL visual check. Existing shaders/materials are shared with the corporate and media systems.

`npm run benchmark:media-landmarks -- --curved` reproduces the included CPU geometry measurements. Close detail is approximately 18.7k triangles for this 12-floor specimen; far detail is approximately 2.7k. Construction is heavier than the simpler media forms, so the existing sparse placement cap and LOD policy remain important. These measurements are not a GPU frame-rate guarantee. No new bitmap textures or per-building animation timers.
