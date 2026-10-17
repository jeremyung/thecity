# Dense Tokyo commercial districts

Review: `/?review=architecture-city&family=shopping-street&district=shibuya`. The normal city controls also expose the persisted Commercial district preference. Seven choices: standard, coherent Tokyo mix, Shibuya, Roppongi, Akihabara, Shinjuku and Ameyoko.

The reference mapping and primary sources are in `web/src/render/architecture/BUSINESS.md`. The official Center-Gai photo was visually inspected; neighborhood guides informed the other interpretations. Output brands are fictional. One new 1024-square procedural atlas shares 48 horizontal identities and 16 portrait variants across three activity/style pairs. No reference photographs or remote image assets are embedded.

The reusable shopping-street generator produces 24 variable-sized parcels in opposing rows, .65 m gaps and explicit road access. Profiles add seeded facade grammars, aligned shop titles, upper-floor labels, projecting two-sided blade signs, flush directories, canopies and service plant. Existing production allocation governs animated/static/off large displays; Ameyoko excludes large rooftop/facade ads. Office-with-shops layouts retain a separate entrance. LOD reduces sign detail; construction/demolition retain their existing stage behavior.

Validation: final full suite 142 files / 787 tests passed. TypeScript/Vite build passed with the existing large-bundle warning. Architecture asset validation verified 8 approved components and 12 textures. New coverage checks preference defaults/persistence, coherent cell selection, eligibility/fallbacks, deterministic non-overlapping street layout, road access, all five profiles at three LODs across lifecycle stages, material ownership, finite geometry, parcel bounds, atlas cells and separate upper-floor access.

Visual checks: WebGPU Shibuya daytime, Roppongi night and low market variation; WebGL Akihabara animated signage at night/rain in anime style. No console errors; no queued buildings/chunks after warmup. Screenshots are actual renderer output. Initial shader compilation can temporarily stall browser controls. No cross-device FPS guarantee is implied.

`npm run benchmark:districts` measures two passes across each complete 24-building street at three LODs using production sign allocation. The slowest profile's p95 CPU build was 3.41 ms close / 2.39 ms district / 1.31 ms far. Maximum per-building triangles: 7152 / 5652 / 2860. Shared atlas drawing is mocked in the Node benchmark; first browser font/atlas creation and GPU cost are not represented.

Limits: profile selection adapts existing eligible city buildings; it does not change engine parcel subdivision, density, capacity or jobs. The dense layout generator is a standalone review/generation fixture. Automatic narrow pedestrian-lane generation, richer crowds, varied ground surfaces and more street furniture remain future engine/art work. This is a stylized modular interpretation, not a photographic recreation of an entire Tokyo neighborhood.
