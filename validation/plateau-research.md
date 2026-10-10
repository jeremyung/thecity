# PLATEAU research for the Tokyo living simulation

Research date: 2026-10-10 (America/Los_Angeles). Documentation-only research; no datasets downloaded, services purchased, runtime changes, engine tests, commits, or pushes. **Recommendation: calibrate procedural urban form first; consider a few simplified imported reference assets only after a convincing shared benchmark district exists.**

## Scope and evidence

This remains a beautiful autonomous browser observation experience: finite seeded geography, organic connected growth, Rust/WASM worker ownership, Three WebGPU/TSL, selectable Last Train, First Light and Little Tokyo, purposeful journeys, dynamic water, full-scene anti-aliasing, daily lighting, and spatial sound. PLATEAU supplies evidence and optional source geometry; it does not replace these requirements with a management game or a literal digital twin.

Read the specification index and all eight linked specifications, `.impeccable.md`, recent `validation/spec-audit.md` checkpoints, and relevant code: `engine/src/frontage.rs`, `engine/src/district_density.rs`, `web/src/render/{building-massing,lowrise-massing,kit,city}.ts`. The code already has compact low-rise frontage/service volumes and apartment/office massing families. Therefore “add varied roofs” alone is insufficient: distribution, street placement, gaps between parcels, infrastructure access, and ground treatment must work together.

The latest audit records native parity for incremental road-face topology, with face walking and production scheduling outstanding. Those graph results do not prove rendered density. Earlier matched project-policy experiments are mixed; nominal envelope coverage differs from rendered footprint union. Preserve ongoing resumable road-face work and its frozen reference. This research makes no promotion recommendation for that implementation.

Labels below distinguish **Fact** (primary source), **Observation** (repository evidence), and **Proposal/inference** (application recommendation). No numerical Tokyo morphology statistics were extracted in this research.

## Current sources and Tokyo availability

**Fact:** The requested [MLIT English overview](https://www.mlit.go.jp/en/toshi/daisei/plateau_en_2.html) now includes Vision 2026 news alongside a 2022-themed roadmap, including study of CityGML 3.0. Treat that roadmap as historical direction, not proof of a completed migration or today's exact dataset coverage.

**Fact:** Tokyo's [official development-status page](https://www.toshiseibi.metro.tokyo.lg.jp/machizukuri/smarttokyo/3d_digital_map/3d_digital_progress) is indexed with an open-data statement covering FY2022–FY2024 ward/Tama development (with Komae and part of Edogawa exclusions) and Izu islands. The fetched page body was incomplete around this statement; use this as regional availability evidence, then verify the selected resource's actual boundary and vintage. The [MLIT open-data directory](https://www.mlit.go.jp/plateau/open-data/) lists Tokyo's 23 wards and a Takeshiba sample.

The [older Tokyo23ku catalog](https://www.geospatial.jp/ckan/dataset/plateau-tokyo23ku) returned HTTP 403 to this research client. Its older indexed descriptions are not used to establish current LOD2 extents. Do not assume its historic central-area detail applies to every ward or the latest release. Follow the [official dataset explorer](https://docs.plateauview.mlit.go.jp/datasets/explorer/) and [CityGML distribution documentation](https://docs.plateauview.mlit.go.jp/datasets/citygml/) to discover ward/year resources; the explorer is dynamic and this text fetch did not expose individual Tokyo records.

**Fact:** [MLIT FAQ](https://www.mlit.go.jp/plateau/faq/) identifies CityGML as the standard distribution format, with some alternate formats available by city. Models reflect acquisition dates, rather than live construction; LOD1 height is abstracted. Consequently, a “2024 publication” must not be relabeled a 2024 survey without metadata.

**Fact:** The [official CityGML API documentation](https://docs.plateauview.mlit.go.jp/datasets/citygml/) describes municipality/year metadata including file sizes when available, file-URL search by mesh or bounding rectangle, and feature attribute lookup. This enables bounded source discovery before downloading. The service is experimental without a service-level guarantee; pin an explicit year and archive provenance rather than relying on `latest` for reproducible calibration. A rectangle search returns source files covering that area, not necessarily an already clipped 250 m dataset. These APIs were documented but not executed in this research.

**Proposed sample candidates, not verified resource selections:** a Shinjuku or Shibuya station-edge crop, a compact Taito/Sumida residential-commercial crop, and a Koto or Takeshiba waterfront crop. Select by morphology and adequate metadata, not landmark fame. Prefer one consistent survey vintage; log differences if cross-vintage comparison is unavoidable. Exact crop coordinates, feature counts, LOD availability, resource sizes and license remain a pilot prerequisite.

## Semantics and useful measurements

**Fact:** [MLIT building feature/LOD rules](https://www.mlit.go.jp/plateaudocument02/tocC/tocC_02/tocC_02_02/) distinguish buildings and building parts and allow use/floor attributes. LOD0 is planar geometry; LOD1 is a solid; LOD2 supplies thematic boundary surfaces. [LOD2 product rules](https://www.mlit.go.jp/plateaudocument/toc4/toc4_02/toc4_02_01/toc4_02_01_03/%E5%BB%BA%E7%AF%89%E7%89%A9%E3%83%A2%E3%83%87%E3%83%AB_lod2%E3%81%AE%E6%A6%82%E8%A6%81_/) separate roof, wall, ground and installations, with LOD2 sublevels. These are data-model detail levels, not the renderer's distance-based mesh levels.

**Fact:** [MLIT's LOD learning article](https://www.mlit.go.jp/plateau/learning/tpc01-2/) explains parallel geometry representations and roof/wall semantics. Its area-availability statements are dated educational context, not today's coverage guarantee.

**Proposal:** Offline extract the following, retaining source IDs and missing-value flags:

| Evidence | Derived measurement | Procedural use and caveat |
| --- | --- | --- |
| LOD0 footprint / LOD1 base geometry | Area, perimeter, oriented aspect ratio, compactness, orientation, nearest-building gap | Joint parcel/massing distributions; footprints are not cadastral parcels. Distinguish roof-edge projection from ground footprint. |
| LOD1 solid / measured height / floors if present | Height percentiles, height-to-width ratio, floor-count distribution | Mix small homes, midrise fabric and tower groups. Height does not reliably imply floor count; report inferred floor area separately. |
| LOD2 roof surfaces and building parts | Roof slope/flatness, multi-volume silhouette, podium/shaft relation | Calibrate gable/shed/parapet and setbacks. Unknown roof type stays unknown; generic detail or texture is not proof of tiny fixtures. |
| Buildings + street surfaces/edges | Block size, building footprint union, frontage/gap distribution, accessible depth | Tune frontage subdivision and future interior-access recipes; do not infer an authoritative routing graph from a mesh. |
| Building use and floors, where populated | Mixed-use composition and floor-area proxy | District archetypes; neither occupancy nor employment nor current business activity is supplied by shape. |
| Land use, terrain, water, bridge/rail geometry where available | Open-space categories, slope, bank section, corridor width and elevation | Geography and streetscape references; preserve finite seeded landscapes. |

Use a metric projected CRS suitable for Tokyo, record EPSG/axis order and vertical reference from the source, then recenter into local meter coordinates with explicit X/Z and elevation mapping. Preserve holes, parts and geometry validity. Deduplicate parent/part IDs and choose one LOD per measurement; summing all parallel LODs would double count. Clip building **unions** to exact crop boundaries. Report gross coverage over all crop area separately from net developable coverage; exclude water/protected land/infrastructure only with defensible polygons. Report sample completeness, not just percentiles.

**Inference:** Dense Tokyo character should come from correlated morphology: compact small-building groups, mixed heights along streets, occasional larger merged sites, service courts, and station-associated clusters. Derive conditional distributions by district and street context rather than randomly choosing each footprint, height and roof independently. Station-distance correlations are a hypothesis to test against verified station locations and pedestrian connectivity, not a causal conclusion from PLATEAU alone.

## Terrain, water, transport and activity

**Fact:** The official [visualization conversion specification](https://docs.plateauview.mlit.go.jp/spec/visualization/) maps building detail to 3D Tiles, many planar road/rail/land-use features to MVT, and bridges to 3D Tiles. It documents a 2023/standard-3.x conversion scheme; supported feature classes do not prove their presence in a particular Tokyo resource.

**Proposal:** Use terrain to compare basin slopes and bridge/bank sections, water geometry to compare constrained waterfront composition, and transportation geometry to compare station approaches, rail barriers and street widths. Keep flood-risk volumes distinct from actual water surfaces. Resolve grade separation explicitly: a bridge or elevated railway crossing is not an at-grade graph junction. Validate any derived graph offline before it influences simulation access.

PLATEAU static geometry does not supply this application's train timetable, household history, deliveries, traffic signals, footsteps or soundscape. Working destinations and networks must continue to generate purposeful movement and events. Station-based density requires an open working station and reachable walking catchment; a scenic station mesh must not trigger development. Water remains the existing animated shaded system, not a frozen imported surface. Do not infer land-use lawns from every unbuilt gap: the present terrain creates groves/meadows and clears development footprints, so mature unused land needs separately authored paved/service/vacant categories without erasing protected banks and parks.

## Prioritized adopt / adapt / defer

| Priority | Decision | Action | Evidence and completion gate |
| --- | --- | --- | --- |
| P0 | Adopt | A provenance-rich morphology reference pack and matched crop metrics | LOD semantics above; complete three bounded samples with union/gross/net denominators and missingness. |
| P0 | Adapt | District targets and joint footprint/height/frontage distributions | Existing density spec/controller; calibrate without bypassing demand, clearance, capacity or construction. Mixed-seed art/performance gate required. |
| P0 | Adapt | Urban ground treatment and near-continuous street fabric | Existing massing kit and meadow observation; classify gaps, retain viable entries/service spaces and protected open land. |
| P1 | Adapt | Station cluster archetypes, older pockets and podium/roof variation | Sample correlations plus LOD2 surfaces; verify working station access and bounded finite skyline. |
| P1 | Adopt | Offline conversion, simplification, metrics and source/license manifests | Official converter and distribution docs; ship compact definitions, not XML parsing in browser. |
| P2 | Adapt | At most a few simplified, texture-free distinctive assets | Only if procedural kits cannot express useful silhouettes; shared materials, bounds, picking and construction representation required. |
| P2 | Adapt | Terrain/bank/rail reference sections | Verify layer availability and coordinate alignment; keep simulation geography seeded. |
| P3 | Defer | Live city-wide 3D Tiles streaming or full Tokyo import | Adds static-world integration and residency complexity; does not solve organic growth or repetitive site placement. |
| P3 | Defer | LOD3/4 interiors, phototexture atlas, disaster/dashboard functions, real-time twin | Poor fit to watchability and resource priorities. Revisit only with a concrete visual or activity requirement. |

## Offline tooling and browser integration

**Fact:** [PLATEAU GIS Converter](https://github.com/Project-PLATEAU/PLATEAU-GIS-Converter) is a MIT-licensed Rust-based GUI/CLI converter supporting GeoPackage, GeoJSON, glTF/OBJ, MVT and 3D Tiles. Its README points to the upstream MIERUNE repository for newer releases. Pin and record a tested version; do not assume all output formats retain every semantic field. This research installed nothing.

**Proposal:** Inspect/crop and measure in QGIS using converted GeoPackage; retain a small semantic sidecar for fields lost in glTF. Use a local preprocessing script for validity checks, footprint union, roof classification and deterministic quantile tables. Converter processing belongs offline, outside the authoritative worker. Offline storage and RAM can exceed browser limits, so inspect resource size before retrieval and choose small separately distributed meshes/files. Abort if only a large ward bundle is available; use the official viewer for qualitative reference while locating a bounded resource. No large download is necessary to start visual calibration.

**Fact:** [NASA-AMMOS 3DTilesRendererJS](https://github.com/NASA-AMMOS/3DTilesRendererJS) provides a Three.js tile renderer under Apache-2.0. **Proposal:** reserve it for a separate reference viewer if necessary; compatibility with this application's WebGPU/TSL materials, shadows, picking and style changes must be demonstrated. A Three.js integration alone is not evidence of that compatibility. No Cesium runtime or paid hosting is required for the proposed pack.

For the simulation, prefer versioned static JSON definitions plus a few local GLBs only when useful. Strip aerial textures and irrelevant attributes from presentation geometry, preserve attribution/source IDs in a manifest, recenter, simplify distant shapes, merge compatible materials and spatial batches, and stage decode/upload. Reuse the existing construction/renovation lifecycle and both renderers. Runtime authoritative geometry stays in Rust; presentation variation cannot expand collision bounds or change seeded economic outcomes. A fixed calibration pack affects engine versioned definitions, not camera-driven loading decisions.

## Rights and attribution

**Fact:** [MLIT site policy](https://www.mlit.go.jp/plateau/site-policy/) applies PDL1.0 by default where no separate rights statement exists, requires source credit and disclosure of edits, and disallows implying MLIT authored processed results. It identifies municipal ownership of city-model data, CC BY compatibility, and separately notes public-survey legal constraints. Read each resource's rights statement before redistribution; data and tool licenses are separate.

**Proposal:** Add a manifest entry per source crop/derived asset: municipality/rightsholder, dataset/resource URL and ID, acquisition and publication year, exact license/link, source hash, accessed date, CRS, selected LOD, transformation/simplification steps and output IDs. Preserve software license notices for incorporated tooling. An attribution template, to complete after verification: “Derived from [dataset title/year], [municipality], Project PLATEAU, [resource link], [license link]. Cropped, simplified and restyled by this project.” Provide accessible credits and include the manifest in exported reference packs. Do not present synthetic cities as official PLATEAU models. If a resource expressly uses ODbL or other distinct terms, evaluate that license rather than assuming CC BY for all files. No asset has been cleared for shipping by this research.

## Small pilot: three crops, one synthetic district

This is proposed work, not completed sampling or an approved Tokyo numeric target.

1. **Source gate:** identify three 250 × 250 m crops representing station edge, compact ordinary neighborhood and waterfront. Limit source retrieval to 50 MiB total compressed, with no whole-ward download; if distribution granularity prevents that, stop retrieval and record the limitation. At least one crop must have verified LOD2. Record exact boundaries/vintage/license and all missing attributes. Use all buildings intersecting each crop; do not cherry-pick attractive buildings.
2. **Reference pack:** produce `validation/plateau-pilot/provenance.json`, `morphology.json`, and three diagrams showing footprint union, heights and open-space categories. Summarize area/aspect/gap/height p10/p50/p90, frontage ratio and roof-family shares. Keep reference-derived definitions under 100 KiB compressed. No measured numbers should be filled from visual guesses.
3. **Shared art benchmark:** assemble a 250 m synthetic station-edge district from existing procedural kits and legitimate access rules, with at least 100 buildings spanning low-rise, midrise and clustered taller forms. Start with authored composition before attempting automatic target policy. Use a separate fixture; do not silently change normal seed trajectories. Optional imported geometry: at most three silhouettes, total pack ≤2 MiB compressed, ≤100k additional visible triangles, ≤16 MiB counted GPU allocations, and ≤10 added visible draw calls. Zero imported assets is preferred if kits achieve the result.
4. **Art acceptance:** compare identical cameras with depth of field off at overhead, neighborhood and street scales in both styles, at noon, dusk and night. Measure rendered ground-footprint union and usable frontage; aim within 10 percentage points of the chosen reference crop's gross coverage/frontage and within 20% of its median footprint area and p90 height, while retaining intentional stylization. These tolerances are pilot proposals. Reject if repeated detached shells, broad unexplained lawn, unreadable access or isolated random towers dominate. All protected land, visible construction, purposeful travelers, water, sound and stable anti-aliased edges remain required. Metrics cannot substitute for concept-board review.
5. **Performance acceptance:** use recorded reference desktop/mobile hardware and fixed resolution/camera/effects; capture a 60-second warmed moving view and repeated style switches. Maintain spec targets of near 60/30 fps, main-thread application CPU typically <4 ms/frame, simulation-step p95 <5 ms at 20 Hz, WASM <128 MiB, CPU <300 MiB, estimated GPU <256 MiB and initial compressed scene <15 MiB. Additionally permit ≤10% p95 frame-interval regression against the same fixture baseline; report CPU/GPU/overall timing separately. Stage asset work; no new task ≥50 ms attributable to pilot loading after warmup. Record initial upload stalls separately and verify counters settle after ten style/camera cycles. Short tests do not establish the existing multi-hour memory gate.
6. **Growth acceptance:** after art calibration, compare seeds 0 and 87231 plus at least one new seed from founding to maturity with the same binary and tick checkpoints. Verify connectivity, occupancy, construction, clearances and save/resume. Do not equate native counts with WASM trajectories; the spec already records target divergence. Only then consider enabling district-policy changes.

Current measured renderer pressure matters: `specs/05-rendering.md` records ~2.14 million triangles and ~219.5 MB counted CPU geometry arrays for an 850-building fixture, before claiming a full browser heap measurement. This leaves little room under the 300 MiB CPU proposal. Morphology tables have much better cost/benefit than indiscriminately adding textured meshes. Reduce duplicate geometry/storage and distant optional detail while preserving focal beauty, water and anti-aliasing.

## Precise suggested spec/task changes

No shared specification or audit file was edited, to avoid collision with active implementation agents. Suggested narrow patches follow; they preserve all current requirements.

| Destination / section | Suggested addition | Task and dependency |
| --- | --- | --- |
| `specs/README.md`, Decision status | Link this report as external morphology research, explicitly not implementation evidence. | Documentation link after root review. |
| `specs/02-world-growth.md`, Density feedback and urban form | Require source-vintage-tagged joint footprint/height/frontage calibration; separate rendered union, nominal envelope proxy and net developable land; condition station archetypes on working pedestrian access. | `PLATEAU-01`: three-crop measurements; then `PLATEAU-04`: matched seeded growth calibration. Do not change current experimental weights until reference extraction and access work are accepted. |
| `specs/05-rendering.md`, Dense-city reference update / Compact architectural families | Add a benchmark using calibrated ordinary fabric, service spaces and station clusters; measure silhouette repetition and unexplained lawn across both styles. PLATEAU LOD2 informs form, not automatic phototexture adoption. | `PLATEAU-02`: synthetic shared district, noon/dusk/night views; depends on `01`, reuses existing kits. |
| `specs/04-architecture.md`, Extending the world | Add offline-only source processing and versioned compact morphology definitions; require local origin, stable IDs, bounds, manifests and staged optional asset uploads. Source geometry never mutates authoritative world from a renderer. | `PLATEAU-03`: reference pack/schema and optional ≤3-asset experiment after `02` proves need. |
| `specs/08-delivery.md`, Validation / Proposed budgets | Add the pilot's matched camera/art and measured residency gates; mark its numerical tolerances provisional and keep the existing full visual release gate. | `PLATEAU-05`: hardware-recorded profile and cross-style review; blocks promotion of `03/04`. |
| `specs/07-audio.md`, Assets and rights | If data-derived assets ship, extend credits with municipality, resource, license and edits; retain separately licensed sound/music and activity-driven audio. | Fold into `03`; PLATEAU provides no audio rights or event timing. |

## Remaining uncertainties

Selected Tokyo resources, source file sizes and exact current LOD2 coverage could not be verified through the blocked catalog/dynamic explorer; these are explicit source-gate work. No downloaded data supports quantitative density/roof claims yet. Attribute completeness, station locations, road-layer connectivity, CRS/vertical alignment, license exceptions and survey-use conditions need resource-level checks. Converter semantic retention and WebGPU tile-loader compatibility are untested. The pilot budgets/tolerances are proposed limits, not benchmark results. The current simulation's art/performance release gaps and ongoing access-discovery work remain open.

## Follow-up: direct Open Data directory links

Rechecked the [MLIT Open Data directory](https://www.mlit.go.jp/plateau/open-data/) on 2026-10-10 after the user's follow-up. It displays last update **2026-05-11**, **306 locations**, and individual Tokyo ward links in addition to the older combined Tokyo23ku entry. This improves concrete resource discovery beyond the initial report:

- [Takeshiba Tokyo sample, catalog identifier 2023](https://www.geospatial.jp/ckan/dataset/plateau-tokyo-takeshiba-2023): candidate for a bounded waterfront/reference asset study.
- [Shinjuku ward, catalog identifier 2025](https://www.geospatial.jp/ckan/dataset/plateau-13104-shinjuku-ku-2025): candidate for station-cluster and mixed-height calibration.
- [Sumida ward, catalog identifier 2025](https://www.geospatial.jp/ckan/dataset/plateau-13107-sumida-ku-2025): candidate for compact ordinary urban fabric calibration.

These exact catalog destinations were verified by following the directory links, but all three catalog fetches returned HTTP 403. The year in an identifier is not independently verified acquisition vintage. Sizes, feature attributes, LOD extents and resource-specific licenses remain unresolved. The directory is a current official index into G Spatial downloads, not a separate download host bypassing that restriction. Prioritize these explicit ward resources over assuming the old combined Tokyo catalog is the latest option. Takeshiba is labeled a sample; its actual size is still unknown. The pilot remains offline and bounded, with no large download required or performed.
