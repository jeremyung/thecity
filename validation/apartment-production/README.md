# Apartment production integration validation

This release integrates 15 deterministic apartment recipes (five older midrises, five contemporary midrises, five towers), eight authored components and twelve runtime textures into the existing city renderer. It includes material ownership/disposal, room lighting schedules, construction and demolition, neighborhood anti-repetition, distance LOD, versioned texture URLs and a persisted legacy-renderer rollback.

## Verification

- Full Vitest suite: 765 passed, zero failed before the final frame-budget guard.
- Final guard and road regression suite: 28 passed, zero failed (includes the new scheduling regression).
- Additional focused architecture, room, component and compatibility suite: 120 passed.
- Production TypeScript/Vite build passed; existing large-chunk warning remains.
- `npm run check:architecture`: eight components and twelve textures verified.
- WASM build and all 23 pinned fixture downloads verified.
- `git diff --check` passed.

WebGPU mature-city day/night and WebGL five-tower day/night/rain were visually checked locally. WebGL anime/demolition screenshot is included. The tower fixture reached zero pending buildings/chunks. The frozen mature fixture retained three infrastructure-gated buildings in both modular and legacy modes; it had no pending chunks.

## Performance and support boundaries

The frozen mature world has 449 buildings; 62 use modular architecture, including all ten midrise recipes and one tower. Other kinds, unsupported footprints, anchors and retained-footprint additions preserve the established renderer. See `mature-census.json` for exact census and CPU geometry timings.

The local mature WebGPU scene showed approximately 54–60 FPS and the five-tower WebGL night/rain view 41 FPS. These are observations on this machine, not cross-device certification or controlled benchmarks. The mature snapshot freezes simulation and actor workload. Detailed CPU-only measurements are in `geometry-benchmark.json`: individual builds can exceed 16 ms, so the scheduler now defers a second expensive rebuild once its frame budget is spent. Worker-based geometry remains a potential improvement.

The new texture set is roughly 1.8 MB compressed and approximately 6 MiB decoded with mipmaps. A pre-existing oversized development texture remains outside this scoped architecture asset check. This work improves architectural variety and integration; it does not claim photographic parity with the reference imagery.

## Reproduction

Run `npm run setup:fixtures`, `npm run wasm`, `npm test`, `npm run check:architecture`, `npm run build`, and optionally `npm run benchmark:architecture`. Start `npm run dev` and visit `/?review=architecture-city&family=mature` or `/?review=architecture-city&family=towers&webgl=1`. The study changes no saved worlds.

See [architecture contract](../../web/src/render/architecture/README.md) and [office, mixed-use and other building families](../../docs/architecture-families.md).
