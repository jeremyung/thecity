# Animated media walls

Reproduce in the production city renderer using `?review=architecture-city&family=shopping-street&district=shibuya`, seed 41. Production allocation selects tenant 13 (triptych), tenant 16 (curved), and tenant 19 (flat). Select Night and Animated. WebGL fallback adds `&webgl`.

Twelve fictional advertiser programs share four procedural animation treatments, city time and a 512×2048 sign atlas. Tiles use continuous campaign UVs; no per-building timers or video decoders. Static mode preserves campaign, dimensions and geometry. Existing global display allocation, spacing, inactivity/construction and far-LOD rules apply.

Validation: full suite passed 792 tests before the final static-mode refinement; final affected signage/business suite passed 24 tests and the production build passed. Geometry checks cover curved normals/depth, continuous tiled UVs, material-cache bounds and identical static/animated envelopes. Browser smoke checks exercise WebGPU and WebGL plus alternate art style. CPU benchmark results are included; they do not establish a GPU frame-rate budget. The expanded shared atlas adds approximately 2.67 MiB RGBA8 with mipmaps.

These are procedural advertisements, not video playback, screen-cast dynamic lighting or finished glass reflections. Accurate nearby-glass reflections remain issue #23.
