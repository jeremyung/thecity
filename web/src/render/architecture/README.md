# Procedural apartment architecture

Fifteen recipes: five older mid-rises, five contemporary mid-rises and five contemporary towers. `buildApartmentBuilding` and `buildResidentialTower` accept existing Building records and return local-space geometry or null for unsupported records. CityRenderer owns one ApartmentResources palette and texture set, shares batch shaders, synchronizes room occupancy/night/weather uniforms and chooses detail with hysteresis. No generation service runs in the client.

## City options and compatibility

City → Apartment architecture persists Mixed eras, Contemporary, Older styles or Existing renderer. The last option is an immediate rollback. `resolveApartmentArchitectures` assigns by stable appearance seed, suppresses nearby repeats in ID order and does not change engine records. Later infill does not restyle older IDs. City caches the layout assignment; occupancy/progress-only snapshots do not rerun spatial selection.

Supported mid-rises: non-anchor apartments, width ≥5m/depth ≥8m, 4–11 floors. Towers: non-anchor apartments, width/depth ≥14m, 12–60 floors. All support foundation, structure, cladding/finishing, open, renovation and top-down demolition. `lifecycle.ts` supplies exact installed/framed floor counts for both geometry and cache keys. Demolition removes crowns and upper floors while preserving recipe and adding bounded site hoarding/debris. Household dressing is reserved for occupied open-phase appearance.

**Compatibility boundary:** original-footprint extensions, unsupported dimensions/height/anchors and other building uses retain main's established renderer. Retained-footprint geometry, addition seams and access ownership are unchanged. Switching across that eligibility boundary may change visual family; this is an explicit fallback, not an engine/schema migration. Existing extension tests remain part of acceptance. The module does not claim reference-photo parity or introduce a new older-high-rise catalog.

## Room/material ownership

Room panes use main's `art.window('home')`, `windowLived`, normalized pane UVs and per-vertex windowInfo. Balustrades remain transparent glass. The shared Kit preserves production window/cladding tagging on boxes and authored components; physical hidden curtain meshes are not duplicated behind the room shader. Occupied/vacant, unfinished, renovation and demolition room states are tested. Runtime schedules use the existing city uniforms rather than a second lighting system.

Architecture-owned ordinary dressing and batch materials carry an owner flag as well as custom surface bindings. That keeps style switches on the correct palette. Custom opaque materials carry architectureWear; windowInfo and ordinary material classes do not require that attribute. Texture identity is part of a custom surface's batch key. Geometry is owned per building/chunk; shared materials and maps are released by their renderer owner.

## Offline pipeline and CI

- `npm run build:architecture` uses Blender (BLENDER_BIN override) to rebuild the eight fixed-size components from `art-workshop/architecture-pilot/runtime-components/build.py`.
- `ARCH_COMPARISON_CONFIG=<run config> node scripts/generate-architecture-comparison.mjs` runs the existing hosted material workflow. Sources/ledger are kept separate from runtime assets; spend caps and unresolved outcomes prevent accidental retries. Secrets are local environment files, never committed.
- `python scripts/prepare-architecture-textures.py art-workshop/architecture-pilot/round3` uses Pillow/numpy for review masters, seam correction and compact runtime derivatives. Exact prompts, model/cost, crop and hashes are retained.
- `node scripts/build-architecture-assets.mjs --manifest-only` updates hashes/version metadata after texture-only changes, without Blender.
- `npm run check:architecture` is offline and runs in CI. It verifies source/generated geometry, bounds, material roles, PNG dimensions and hashes. Content-hashed texture URLs prevent stale browser/CDN results.

Twelve shared sRGB base-color maps estimate 6.0 MiB RGBA8 with mipmaps: two 512px originals and ten 256px derivatives. Runtime relief/roughness remain procedural. Normal/roughness review derivatives are heuristic, not calibrated scans and are not shipped. No texture or material is allocated per seed.

## Verification

`npm run benchmark:architecture` records CPU build percentiles and triangle counts for 60 specimens per family/detail level. It does not measure GPU time. Production evidence and known limits: `validation/apartment-production/README.md`. Future office/mixed-use/family plan: `docs/architecture-families.md`.

Development-only review URLs:
- `/?review=apartment-pilot` — dimensions, seeds, recipes and LOD.
- `/?review=architecture-city&family=towers` — actual renderer, five towers, day/dusk/night, rain, demolition, both styles.
- `/?review=architecture-city&family=mature` — pinned real mature snapshot, after `npm run setup:fixtures` and `npm run wasm`.
- Append `&webgl=1` for the WebGL backend.

Office, commercial and mixed-use façade generation reuses this owner and pipeline. See [business architecture](BUSINESS.md) for its fifteen-recipe catalog, storefront atlas, billboard controls, integration and simulation boundary.
