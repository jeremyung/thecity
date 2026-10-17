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

Twelve shared campaign programs combine fictional atlas artwork, four motion treatments (colour spotlight, sweep, bloom and moving ribbons), pulse and subtle scanlines. They use the renderer's existing time/shop-light uniforms. There are no video files, canvas redraw loops or timers per sign. Animated screens, roof boards and blade signs are omitted from far LOD, unfinished buildings, demolition and inactive businesses. Vacancy uses dark signs and closed shutters. Source UVs, room activity and weather attributes survive city batching.

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

## Reflection readiness and variety assessment

Vision-window materials expose `userData.reflectionSurface = {version: 1, role: 'vision-glass'}` on both style variants. This is a CPU material contract for a future reflection mask, not an implemented G-buffer output. Corporate glazing retains its material through batching; ordinary panes use the shared window batch. Pane UVs and `windowInfo` carry room identity separately from reflected radiance. Opaque cores, mullions and spandrels are separate materials. Rounded corporate glass uses analytic outward normals across tessellation chords; chamfered corners retain hard normals. Chunk triangle spans retain building identity for future probe assignment. Balcony guards are not classified as vision panes and require an explicit separate role if the reflection renderer includes them.

Accurate local captures, parallax correction, vertical-glass SSR, guard glazing and performance validation are tracked in [issue #23](https://github.com/jeremyung/thecity/issues/23). The existing shared proxy capture remains provisional; this contract does not make reflections physically correct by itself.

The core catalog totals 50 designs: ten apartment midrises, five residential towers, thirty general business recipes and five corporate tower forms. Five district profiles modify eligible frontages; they are not additional core building designs. Width, depth, floor count, entrance and occupancy variation increase combinations, but corporate seeds currently offer limited independent massing variation. This is a useful initial catalog, not proof of a non-repeating large skyline. Next variety work should add bounded podium/shaft ratios, corner radii, setbacks, crowns and glazing finishes within each corporate grammar, followed by dense skyline repetition checks rather than counting seed combinations as unique designs.


## Expanded media walls

Eligible allocated Shibuya/Akihabara and media façades deterministically choose flat, shallow curved or synchronized triptych screens. Other hosts retain flat screens. Layout is bounded within the existing display envelope; curved glass has continuous mesh normals and the three tiles share a global UV range and one campaign material. All animation uses the existing city clock, with no videos, downloads, canvas redraw loops or per-sign timers. Static mode substitutes printed ads while retaining the selected layout and campaign; off/far/inactive/construction rules and the six-host/30 m spacing allocation remain intact. This is stylized procedural advertising, not decoded video playback or simulated screen light cast onto streets.

The shared blade/billboard atlas grows from 512×1024 to 512×2048 for twelve fictional advertisers (approximately 5.33 MiB RGBA8 with mipmaps, up from 2.67 MiB). Screen cache is bounded to twelve style pairs; split panels share materials and batch normally. Reflections of these displays remain part of issue #23.

## Crossing-facing media landmarks

Three additional architectural compositions interpret the supplied commercial-crossing reference: `wrap` has a building-scale curved LED face beneath an exposed glazed crown; `stack` layers up to four independently programmed media panels with a tenant-sign column; `frame` sets a large artwork screen inside a structural portal. Each includes a multi-storey glazed retail podium, a variable entrance, interior shelving/ceiling strips and rooftop services. These are three landmark grammars layered onto the core catalog, not three texture variants or exact reproductions of real buildings.

`media-landmark-selection.ts` selects at most three eligible hosts with 100 m spacing, within 150 m of stations or road junctions of degree three or more. It applies to contemporary shop/café/office parcels, 6–20 floors, at least 18 m wide and 14 m deep, in Shibuya/Shinjuku/Akihabara profiles (including coherent Tokyo-mix cells). Corporate towers, apartments, anchors, retained footprints, older/legacy styles and other district profiles do not acquire this override. Existing engine parcels, dimensions, use and height limits remain authoritative; this does not create new simulation parcels or guarantee a landmark in every saved city.

Landmark candidates participate in the existing six-host/30 m spacing allocation. Selected landmark hosts receive priority within the maximum three animated screens; additional selected hosts remain static ads. The architectural override is independent of business activity and construction state, while media is disabled for inactive/unfinished buildings, off mode and far LOD. Screen count is a host budget: a single stacked landmark can contain multiple panels. Four bounded shared artwork/style pairs provide moving orbital sculpture, ribbons, constellations and equalizer campaigns with distinct timings, using one shared presentation clock (including paused simulation) and the existing fictional-brand atlas. No new raster textures, per-building timers or video decoders. Static mode keeps artwork surfaces and geometry but uses a printed campaign. Shared vision-window materials retain the reflection contract. Screen emission does not yet cast area lighting or produce correct nearby-glass reflections; see #23.

Review all forms through production selection at `?review=architecture-city&family=media-landmarks&district=shibuya`. Select individual buildings, Night, and Animated/Static/Off. Add `&webgl` for fallback. `npm run benchmark:media-landmarks` measures geometry construction for all forms, LODs and display modes; this is not a GPU frame-rate certification.


### Fully curved media building

The fourth landmark grammar, `curved`, rounds the entire footprint: podium slabs and glazing, recessed upper floorplates, façade glazing, roof edge and mechanical crown. Its panorama follows both rounded front corners with continuous distance-based UVs and analytic outward normals. Vision glass uses the shared physical-glass family and provisional environment capture; accurate local reflections remain #23. The same sparse placement, three-landmark cap, animation budgets, dimensions and lifecycle controls apply. Existing catalog review retains its original three specimens; `?review=architecture-city&family=curved-media&district=shibuya` isolates the fully curved building at a useful initial camera angle. `npm run benchmark:media-landmarks -- --curved` measures its CPU geometry construction across LOD and display modes.

### Flagship realism components

`media-campaigns.ts` supplies four fictional product-led campaigns with two scenes each and one owned 1024×1024 runtime atlas (about 5.33 MiB with mipmaps). Four shared animated style pairs and one shared static poster pair reuse it. Timed transitions use the existing shared presentation clock; cabinet joints and a subtle highlight replace the dominant abstract equalizer on the curved flagship. Other landmark grammars retain their existing abstract artwork library.

`flagship-interiors.ts` provides transparent physical retail glazing, coherent shop/café/bookshop modules, an entrance-aligned escalator atrium, and restrained aluminium finish variation. Curved floorplate templates are cached per build and disposed after geometry assembly. The builder adds screen housings, deeper mullions, an open atrium floor, aligned entrance steps/canopy, storefront props, roof louvres and service equipment. Far LOD omits interiors and small details. The original curved-landmark validation images are a baseline; `validation/business-architecture/realism` records the current pass and geometry cost.

`MediaScreenLighting` owns exactly two reusable, non-shadowing spill spotlights, gated by active allocated media, distance, LOD, display mode and daylight. It uses campaign palettes and the shared presentation clock. This bounded approximation lights adjacent ground and surfaces; it does not replace physically accurate glass reflections or emissive area-light transport. Issue #23 remains open.

`media-intersection-study.ts` creates a paved commercial corner with eight neighboring buildings and production roads, sidewalks and scramble crossing, plus supported frozen pedestrian/vehicle review poses. Open `?review=architecture-city&family=media-intersection&district=shibuya` for matched day/night review. This is a presentation fixture; engine parcels, capacities, road planning and movement simulation remain authoritative in actual cities.

Flagship animated campaigns include stable cabinet brightness/tint calibration, dark module seams and a fine emitter mask. Unwrapped UV derivatives fade hardware detail before minification to avoid distant moiré. Printed posters remain unchanged. This adds no textures, geometry, timers or material variants.

Primary business, corporate and media-landmark doors stay on local X=0, matching the engine front-centre walking-access endpoint under every rotation. Mixed-use and district-office tenant bays reserve a central lobby gap; secondary tenant/service entrances may vary independently. Recipe version 9 invalidates cached geometry after this layout correction. Integration coverage checks actual glazed geometry and authoritative access paving across all 30 business recipes, five corporate forms and four media forms, two seeds and three rotations.
