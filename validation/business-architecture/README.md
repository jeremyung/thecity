# Business architecture validation

Thirty recipes: ten offices, ten commercial buildings and ten mixed-use façades, with five older and five contemporary options in each family. They use the same city placement, shared material owner, versioned textures, lifecycle, distance detail and chunk scheduling introduced by the apartment integration. No engine/save-schema changes or generation-provider calls were needed.

## Checks

- Full suite: **777 passed, zero failed**.
- After final façade, service-roof and advertising refinements: focused business, signage, surface, batching, style and city signature checks passed; 55 focused checks passed; exact counts are in `tests.json`.
- Contract sweeps cover every recipe, all lifecycle stages/LODs, 5–90 m widths and 1–60 floors, plus separate mixed-use access, era fixtures and advertising eligibility.
- Production TypeScript/Vite build passed; existing large-bundle warning remains.
- Offline architecture asset validation still verifies eight authored components and twelve textures.
- Browser: the expanded catalog rendered on WebGPU; focused commercial, mixed-use and office views exercised the actual city renderer. Commercial night/rain signage and retail interiors also rendered on WebGL in anime style with the final glazing shader, reaching zero pending buildings/chunks with no console errors. Animated, static and no-advertising modes were exercised on WebGPU.
- Examples in the PNGs show held façade and street views. Local FPS observations are not controlled GPU benchmarks or cross-device certification.

## Coverage and cost

`geometry-benchmark.json` is reproducible with `npm run benchmark:business -- validation/business-architecture/geometry-benchmark.json`. It measures Node CPU geometry only across four passes over ten specimens per family/detail. Most district builds are inexpensive, but first/large office builds can exceed a frame. The existing source-build frame guard and incremental city chunks remain necessary.

The pinned mature-world census selects business recipes for **169 of 449 buildings**, covering all thirty types. This is selector/census evidence, not a claim that all 449 buildings use these new families. Retained footprints, anchors and unsupported records preserve compatibility rendering.

There are no new downloaded texture files. Existing plaster, ceramic, concrete, stone and metal derivatives are reused. One shared 512×1024 storefront atlas supplies 24 identities and eight price/sale cards; occupied and vacant materials share it. Existing street-sign/poster atlases are reused for blade signs and the three animated advertising programs. Animation uses city uniforms with no per-building video decoding or canvas redraws.

## Limits and review

Mixed-use bands are visual and choose different room schedules. Separate tenant jobs, resident capacities, access destinations and occupancy are **not simulated independently**. The existing building record remains authoritative. Night lighting and façade detail improve the target Tokyo streetscape, but this remains procedural stylized architecture rather than photographic reference parity. Retail interior variants include shelves, counters and boutique displays; more business-specific interiors and authored landmarks remain possible follow-ups.

Run `npm run dev`, then open `/?review=architecture-city&family=business`; select a family, building, era, display mode, stage, light and style. Add `&webgl=1` to exercise fallback. The fixture never writes saved worlds.

See [business contract and reference direction](../../web/src/render/architecture/BUSINESS.md).

## Reference-driven realism expansion

Reusable lattice screens wrap two elevations and upper volumes step back; vertical fins cast real façade shadows; older corporate slabs use shallower floorplates; department stores use pilasters/cornices; roof services use fans, enclosures and seeded equipment variants. Older shops have uneven widths, folded/striped canopies, price cards and dense merchandise. Contemporary flagships keep broad coherent glazing and restrained signs. Media buildings reserve taller screen zones under the existing city allocation budget.

`realism/` captures the expanded set. Earlier screenshots in this directory document the initial fifteen-recipe release. References and the visual-versus-simulated mixed-use boundary remain documented in the business contract. Visual comparison demonstrates clearer architectural differentiation; photographic reference parity is not claimed.
