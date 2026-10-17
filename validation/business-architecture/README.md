# Business architecture validation

Fifteen new recipes: five offices, five commercial buildings and five mixed-use façades. They use the same city placement, shared material owner, versioned textures, lifecycle, distance detail and chunk scheduling introduced by the apartment integration. No engine/save-schema changes or generation-provider calls were needed.

## Checks

- Full suite: **774 passed, zero failed**.
- After subsequent retail shader/shared-atlas refinements: 73 focused surface, batching, style, signage, architecture and preference tests passed.
- Further branding and parcel-envelope changes passed their focused business/signage suites. The final business/signage release suite passed 14 tests, zero failures; counts are recorded in `tests.json`.
- Production TypeScript/Vite build passed; existing large-bundle warning remains.
- Offline architecture asset validation still verifies eight authored components and twelve textures.
- Browser: all fifteen recipes rendered on WebGPU; focused commercial, mixed-use and office views exercised the actual city renderer. Commercial night signage and retail interiors also rendered on WebGL in anime style, reaching zero pending buildings/chunks with no console errors. Animated, static and no-advertising modes were exercised on WebGPU.
- Examples in the PNGs show held façade and street views. Local FPS observations are not controlled GPU benchmarks or cross-device certification.

## Coverage and cost

`geometry-benchmark.json` is reproducible with `npm run benchmark:business -- validation/business-architecture/geometry-benchmark.json`. It measures Node CPU geometry only across four passes over five specimens per family/detail. Most district builds are inexpensive, but first/large office builds can exceed a frame. The existing source-build frame guard and incremental city chunks remain necessary.

The pinned mature-world census selects business recipes for **169 of 449 buildings**, covering all fifteen types. This is selector/census evidence, not a claim that all 449 buildings use these new families. Retained footprints, anchors and unsupported records preserve compatibility rendering.

There are no new downloaded texture files. Existing plaster, ceramic, concrete, stone and metal derivatives are reused. One new 512×1024 storefront atlas supplies 24 identities; occupied and vacant materials share it. Existing street-sign/poster atlases are reused for blade signs and the three animated advertising programs. Animation uses city uniforms with no per-building video decoding or canvas redraws.

## Limits and review

Mixed-use bands are visual and choose different room schedules. Separate tenant jobs, resident capacities, access destinations and occupancy are **not simulated independently**. The existing building record remains authoritative. Night lighting and façade detail improve the target Tokyo streetscape, but this remains procedural stylized architecture rather than photographic reference parity. Some shop interior stock is intentionally shared; more business-specific interiors and authored landmarks remain possible follow-ups.

Run `npm run dev`, then open `/?review=architecture-city&family=business`; select a family, building, era, display mode, stage, light and style. Add `&webgl=1` to exercise fallback. The fixture never writes saved worlds.

See [business contract and reference direction](../../web/src/render/architecture/BUSINESS.md).
