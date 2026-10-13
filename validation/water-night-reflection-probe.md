# Night water reflection isolation — 2026-10-12

Previous goal turn progressed matched Anime lighting evidence. Current question:
is the nearly black river caused by an empty reflection pass or final surface
shading? Prior radiance/distortion trials were rejected as pale mirror/banding;
do not repeat gain tuning without isolation.

Added a DEV-only Renderer details `Water view: normal/raw reflection` control.
It swaps to a lazily allocated MeshBasicNodeMaterial using the same reflector
RGB and original vertex displacement. Same reflector, geometry, world and camera;
no new reflection pass. It bypasses diffuse/specular surface lighting, base tint
and shore wash, so it isolates available reflection content rather than measuring
one individual shading term. Tone mapping and the app postprocess still apply.
Normal material is restored by toggling again. Both materials are disposed by
RiverWater; the UI control is absent in production. Original normal shader unchanged.

Normal Import growth-723.json:seed87231tick27925,723buildings,clear,paused09:55,
22:00light preview,Anime,HighWebGPU,50mmDOFoff1280×720. Hold waterfront view
preset sets repeatable camera. Geometry queues reached0 normally.
- [Normal](screenshots/water-night-probe/normal.png)
- [Raw reflection](screenshots/water-night-probe/raw.png)

Raw view contains bright cloud/sky shapes, both bridge undersides and small
lamp points. Thus the reflection pass is not empty in this scene; final shading
substantially darkens it. Raw view also looks like a flat mirror. This is diagnostic
evidence, not a candidate art treatment. Sparse lit riverfront remains a scene
content limitation. No captured console errors/warnings. Normal mode restored
and verified in UI before handoff. Initial startup warning resolved without retry.

Production build passes with existing bundle warning;19UI/material-lifecycle
checks pass. Probe test verifies identical original material restored, lazily
reused diagnostic material, and both disposed exactly once. No WebGL, motion,
other style, or performance acceptance is claimed. A future experiment must
preserve reflected radiance while addressing rough-surface reflection and must
avoid the previously rejected coherent bands/pale mirror; retain this fixed
camera and test noon/dusk/night before promoting any change.
