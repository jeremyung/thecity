# Office, commercial and mixed-use architecture

The business families extend the apartment pipeline with thirty runtime recipes. `business-recipes.ts` owns selection and supported envelopes; `business.ts` owns geometry; `business-signs.ts` owns a bounded storefront atlas; `business-components.ts` owns reusable facade, awning, envelope and roof-service parts; `business-study.ts` supplies reproducible renderer fixtures. `buildBusinessBuilding` is exported from the architecture module and accepts the existing `Building`, shared `ArtMaterials`, recipe, distance detail and signage context. Runtime generation has no model calls, Blender dependency, video downloads or per-building animation loops.

## Catalog

| Family / era | Five recipes |
| --- | --- |
| Older offices | Tiled tenant, ALC ribbon, concrete structural, narrow vertical, corporate slab |
| Contemporary offices | Deep frame, curtain-wall tower, stepped terrace, vertical fins, glazed retail podium/tower |
| Older commercial | Neighborhood shopfronts, corner cafe, market arcade, open-front stalls, department store |
| Contemporary commercial | Vertical retail, glass showroom, screened flagship, terraced retail, media corner |
| Older mixed use | Shop/residences, vertical tenants, market/workshops, concrete shop/offices, postwar shops/flats |
| Contemporary mixed use | Corner work/live, office/retail podium, residential/retail tower, screened work/live, terraced workplace |

Footprints and floor counts remain authoritative. The city selects compatible recipes from kind, seed, height, width and the existing era preference; nearby repetition is suppressed in stable ID order. Most apartments remain in the residential catalog; a seeded minority receive mixed-use frontages. Anchors, retained-footprint extensions, unknown construction stages and unsupported envelopes retain the existing renderer. No simulation records or save schema change.

## Components and appearance

- Parametric window bays, mullions, structural frames, horizontal spandrels, podium setbacks and stepped terraces.
- Older ground-floor shops use seeded uneven tenant widths; contemporary flagships/showrooms have one coherent broad frontage. Ground-floor shops divide into non-overlapping tenant slots; mixed-use slots reserve separate upper-floor access. Entrance side, tenant mix and small displays use independent seed streams. Cafes draw from appropriate food/coffee tenants.
- Twenty-four fictional storefront identities in one 512×1024 atlas shared by occupied/vacant material variants. Existing blade signs, roof boards and poster atlas are reused.
- Wrapped lattice screens, projecting vertical fins, department-store cornices, shallow slab floorplates, covered arcades, sloped fabric awnings, wraparound corner signs, office directories, floor tenant plaques, cafe tables, menu boards, merchandise racks, eight shared price/sale placards, small displays and planters; close-only details disappear with distance.
- Roof fans, louvered equipment enclosures, seeded older water tanks / contemporary solar arrays and rear service doors stay within the parcel. Floor height stays at the city's 3.25 m clearance convention.
- Existing approved plaster, ceramic, concrete, limestone and metal textures are reused through `ApartmentResources`; no new provider generation was needed. Shop interiors vary between shader shelves/stock, counters and boutique displays rather than hidden room meshes. The weathering field remains nonperiodic. Office and shop glazing use stronger exterior reflectance and restrained transmission tints while residential glazing retains its existing treatment.

## Signs and animation

`commercialDisplays` persists independently of the era preference: `animated`, `static`, or `off`. It applies to new families and compatibility-renderer advertising. Storefront identities remain in all modes. Static converts selected screens to printed ads; off removes advertising and projecting blade panels. Large façade displays use the existing maximum-six / 30 m spacing policy around the station; commercial tenants of at least three floors can now participate. New contemporary screened showrooms, finned offices, curtain walls and terraced buildings keep restrained branding and do not consume advertising slots; media/tenant/podium designs can host them. Printed roof boards are restricted to older new-family buildings. Older office eligibility remains at six floors.

Three shared screen programs combine fictional atlas artwork, slow drifting highlights and subtle scanlines. They use the renderer's existing time/shop-light uniforms. There are no video files, canvas redraw loops or timers per sign. Animated screens, roof boards and blade signs are omitted from far LOD, unfinished buildings, demolition and inactive businesses. Vacancy uses dark signs and closed shutters. Source UVs, room activity and weather attributes survive city batching.

## Mixed-use boundary

Use bands distinguish retail, office and residential room-light schedules in the façade. They do **not** create independently simulated shops, jobs or residential capacity. A mixed-use façade still follows its existing authoritative building record. Separate tenant occupancy, destinations, jobs, capacity accounting and economic growth need engine/schema work before claiming simulated mixed use.

## Validation and extension

`npm run benchmark:business` measures CPU geometry and produces a pinned mature-city recipe census. `/?review=architecture-city&family=business` shows all thirty recipes through the real city renderer. Each family has exactly five older and five contemporary study specimens; the era control filters the review accordingly. `office`, `commercial`, `mixed-use`, and `mature` select focused fixtures; `&webgl=1` forces fallback. The building selector provides held façade views. Tests cover all recipes, phases and LODs, narrow parcels, activity encoding, deterministic selection, display policy, finite geometry and atlas ownership, plus 1–60 floors, 5–90 m widths, reserved upper-floor entrances and advertising eligibility. See `validation/business-architecture/README.md` for evidence.

Add recipes to the typed catalog, constrain their use/size eligibility, implement their geometry grammar, then expand the study and contract tests. Keep materials/atlases shared and add generated textures only when the approved surface library lacks a needed finish.

Reference direction: [Nikken Sekkei: YANMAR TOKYO](https://www.nikken.co.jp/ja/projects/office/yanmar_tokyo.html) for differentiated commercial/office frontage; [Japan Post: JP Tower](https://www.jp-re.japanpost.jp/properties/jptower.html) for podium/tower composition; [Amano Design Office](https://www.amanod.com/works2) for commercial façade differentiation. These informed the family grammar; the generator does not reproduce named landmarks or claim photo-matched replicas.

## Corporate skyscrapers

Five additional office grammars supplement the 30 general business recipes: split-volume, rounded, chamfered, setback, and sculpted-crown towers. `corporate-tower-recipes.ts` declares the catalog; `corporate-skyscraper.ts` builds polygonal floorplates, curtain walls, podiums and mechanical crowns. Width, depth, seed and floor count remain engine inputs. The selector uses these for office parcels at least 24 × 20 m and 24 floors, up to the existing 60-floor renderer limit. Contemporary mode selects this catalog; mixed mode allows it for four out of five eligible seeds; older/legacy modes retain their selection. Simulation height limits remain unchanged.

Review: `?review=architecture-city&family=skyscrapers`, using production selection and batching. Solar-control glass is a bounded shared material family with per-pane occupancy, night schedules and both style variants. Corporate glass retains its specialized batch material; ordinary windows retain the existing single batch. No new bitmap textures, image service, per-building material or runtime model is required. Corporate towers do not receive advertising billboards.

References: supplied split tower, rounded corporate tower and skyline photographs; [KPF Roppongi Hills](https://www.kpf.com/project/roppongi-hills) for broad office floorplates; [Nikken Tokyo Midtown Hibiya](https://www.nikken.co.jp/ja/projects/mixed_use/tokyo_midtown_hibiya.html) for podium/tower composition. Models interpret those features rather than reproduce landmarks. They remain stylized: richer reflected surroundings, measured facade details, landscaped podiums and surrounding streets are needed for photographic parity.

### Glass reflections

Miniature corporate glazing uses a physical dielectric material (coated tint, Fresnel specular response, low roughness and subtle world-space pane bow). Interior mapping supplies the room radiance separately; the flat analytical reflection is no longer emitted over the physical reflection. Anime retains its stylized analytical reflection. Four shared coating/style pairs cover both retail and office activity, which remains encoded per vertex.

`GlassEnvironment` owns one 128-pixel PMREM probe for corporate glass. It captures the live sky, a ground plane and up to 96 nearby building-massing proxies in a single instanced draw. Its anchor follows the nearest city area, refresh checks are limited to every 2.5 seconds, and unchanged scenes reuse the target. It runs only when corporate towers are selected in Miniature. No per-building cube maps or detailed six-pass city capture are introduced. Architecture materials borrow the probe texture; the renderer owns disposal. Sky-only lighting for other surfaces is unchanged.

Reflections approximate local massing rather than exact facade details, and a shared probe has positional error across a large district. Captures add a bounded GPU cost; cross-device frame rates remain unverified. The isolated tower review contains much empty ground, which legitimately darkens downward-facing reflections. Its Street control now offers a near-horizontal tower view for comparing glass response.

## Dense Tokyo shopping districts

`Commercial district` is a persisted renderer preference, independent of building era and advertising mode. `standard` preserves the base catalog. Five reference-informed profiles (`shibuya`, `roppongi`, `akihabara`, `shinjuku`, `ameyoko`) modify eligible commercial and small tenant-office frontages; `tokyo-mix` assigns coherent 160 m neighbourhood cells from the world seed. Large corporate offices, residential parcels, anchors, retained-footprint buildings and legacy mode retain their existing handling. Profile selection does not alter engine use, height, capacity, parcel dimensions or saved simulation data.

| Profile | Procedural emphasis | Reference |
|---|---|---|
| Shibuya | Fashion/music tenants, media/tenant/frame façades, canopies, portrait signs | [Center-Gai](https://www.gotokyo.org/en/spot/88/index.html) |
| Roppongi | Dining, jazz, bars and lounges, stacked directories, darker modern frontage | [Roppongi guide](https://www.gotokyo.org/en/destinations/southern-tokyo/roppongi/index.html) |
| Akihabara | Electronics/games/parts tenants, larger sign bands, department/media variations | [Akihabara guide](https://www.gotokyo.org/en/destinations/central-tokyo/akihabara/index.html) |
| Shinjuku | Karaoke, theatre, arcades and dining, layered entertainment signs | [Shinjuku guide](https://www.gotokyo.org/en/destinations/western-tokyo/shinjuku/index.html) |
| Ameyoko | Low market blocks, narrow shop bays, awnings and merchandise racks; no large ads | [Shopping-street guide](https://www.gotokyo.org/en/story/guide/neighborhood-shopping-streets/index.html) |

These are interpretations of particular retail streets, not complete geographic reconstructions or statements that every building in a neighborhood has that appearance. The official Center-Gai storefront photo was inspected for narrow frontages and layered signage. All tenant names and branding in the output are fictional.

`commercial-district.ts` declares selection/grammar; `district-frontage.ts` composes signs and services; `district-signs.ts` owns one 1024×1024 atlas with 48 horizontal tenant identities and 16 portrait variants. Three bounded activity/style pairs share it (closed, retail, late nightlife). Ground titles follow actual shop bays; directories avoid allocated media panels. Large displays still use production allocation limits and animated/static/off controls. Dense small tenant identities remain functional signage when advertising is off. No external images or per-building canvases/timers are used. Estimated RGBA8 atlas allocation is 4 MiB, approximately 5.33 MiB with mipmaps.

`shoppingStreetStudy(seed,district)` generates a packed 24-parcel, two-sided street with .65 m lot gaps, variable width/depth/heights, opposing frontage rotations and road access. Open `?review=architecture-city&family=shopping-street&district=shibuya` and switch profiles. This reusable layout is a review/generation fixture, not a change to simulation parcel subdivision: choosing the preference in a saved city adapts its existing compatible buildings. Richer pedestrian crowds, street furniture and automatic simulation generation of narrow pedestrian retail lanes remain separate work.

Run `npm run benchmark:districts` for reproducible CPU geometry measurements across all five profiles and three LODs. Results do not certify cross-device frame rate.
