# Office, commercial and mixed-use architecture

The business families extend the apartment pipeline with fifteen runtime recipes. `business-recipes.ts` owns selection and supported envelopes; `business.ts` owns geometry; `business-signs.ts` owns a bounded storefront atlas; `business-study.ts` supplies reproducible renderer fixtures. `buildBusinessBuilding` is exported from the architecture module and accepts the existing `Building`, shared `ArtMaterials`, recipe, distance detail and signage context. Runtime generation has no model calls, Blender dependency, video downloads or per-building animation loops.

## Catalog

| Office | Commercial | Mixed-use façade |
| --- | --- | --- |
| Tiled tenant office | Neighborhood shopfronts | Shop with homes above |
| ALC ribbon office | Corner cafe | Vertical tenant stack |
| Deep framed office | Covered market arcade | Corner work/live |
| Curtain-wall office tower | Vertical retail tenants | Retail and office podium |
| Stepped terrace office | Glass showroom | Retail and residential tower |

Footprints and floor counts remain authoritative. The city selects compatible recipes from kind, seed, height, width and the existing era preference; nearby repetition is suppressed in stable ID order. Most apartments remain in the residential catalog; a seeded minority receive mixed-use frontages. Anchors, retained-footprint extensions, unknown construction stages and unsupported envelopes retain the existing renderer. No simulation records or save schema change.

## Components and appearance

- Parametric window bays, mullions, structural frames, horizontal spandrels, podium setbacks and stepped terraces.
- Ground-floor shops divided into tenant slots; mixed-use slots reserve separate upper-floor access. Entrance side, tenant mix and small displays use independent seed streams. Cafes draw from appropriate food/coffee tenants.
- Twenty-four fictional storefront identities in one 512×1024 atlas shared by occupied/vacant material variants. Existing blade signs, roof boards and poster atlas are reused.
- Covered arcades, colored awnings, wraparound corner signs, office directories, floor tenant plaques, cafe tables, menu boards, small displays and planters; close-only details disappear with distance.
- Roof mechanical boxes, parapets and rear service doors stay within the parcel. Floor height stays at the city's 3.25 m clearance convention.
- Existing approved plaster, ceramic, concrete, limestone and metal textures are reused through `ApartmentResources`; no new provider generation was needed. Shop interiors add shader shelves/stock rather than hidden room meshes. The weathering field remains nonperiodic.

## Signs and animation

`commercialDisplays` persists independently of the era preference: `animated`, `static`, or `off`. It applies to new families and compatibility-renderer advertising. Storefront identities remain in all modes. Static converts selected screens to printed ads; off removes advertising and projecting blade panels. Large façade displays use the existing maximum-six / 30 m spacing policy around the station; commercial tenants of at least three floors can now participate. Older office eligibility remains at six floors.

Three shared screen programs combine fictional atlas artwork, slow drifting highlights and subtle scanlines. They use the renderer's existing time/shop-light uniforms. There are no video files, canvas redraw loops or timers per sign. Animated screens, roof boards and blade signs are omitted from far LOD, unfinished buildings, demolition and inactive businesses. Vacancy uses dark signs and closed shutters. Source UVs, room activity and weather attributes survive city batching.

## Mixed-use boundary

Use bands distinguish retail, office and residential room-light schedules in the façade. They do **not** create independently simulated shops, jobs or residential capacity. A mixed-use façade still follows its existing authoritative building record. Separate tenant occupancy, destinations, jobs, capacity accounting and economic growth need engine/schema work before claiming simulated mixed use.

## Validation and extension

`npm run benchmark:business` measures CPU geometry and produces a pinned mature-city recipe census. `/?review=architecture-city&family=business` shows all recipes through the real city renderer. `office`, `commercial`, `mixed-use`, and `mature` select focused fixtures; `&webgl=1` forces fallback. The building selector provides held façade views. Tests cover all recipes, phases and LODs, narrow parcels, activity encoding, deterministic selection, display policy, finite geometry and atlas ownership. See `validation/business-architecture/README.md` for evidence.

Add recipes to the typed catalog, constrain their use/size eligibility, implement their geometry grammar, then expand the study and contract tests. Keep materials/atlases shared and add generated textures only when the approved surface library lacks a needed finish.

Reference direction: [Nikken Sekkei: YANMAR TOKYO](https://www.nikken.co.jp/ja/projects/office/yanmar_tokyo.html) for differentiated commercial/office frontage; [Japan Post: JP Tower](https://www.jp-re.japanpost.jp/properties/jptower.html) for podium/tower composition; [Amano Design Office](https://www.amanod.com/works2) for commercial façade differentiation. These informed the family grammar; the generator does not reproduce named landmarks or claim photo-matched replicas.
