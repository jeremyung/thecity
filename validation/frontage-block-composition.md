# Deep-site frontage block composition — design brief, 2026-10-15

Status: isolated descriptor and renderer completed, technically screened, then rejected on matched scene comparison. Candidate source/tests are archived outside the active source tree. The brief below records the hypothesis, not an accepted implementation.

## Witness and source

Use the retained ordinary frontage pose: camera(85,70,-150), target(20,4,-220), mature seed5/tick83000,13:00 clear preview,balanced1200×800,DOFoff; record FOV/material time and hold them identical. Retain interior/waterfront witnesses as weak-view controls and do not choose a lower camera to make a candidate pass. Compare against locked concepts01 and04: buildings, street edges and visible planting should make one composition.

A half-edge walk over the snapshot's connected open ground-road graph finds the face containing target(20,-220). Its boundary roads are29,31,100,1134,316,177; centerline area4581.34m². This is not net developable area or an engine-authoritative block identifier. It contains12 building centers and12 parcel centers. The diagram uses current primary massing functions; it excludes overhangs, balconies and detail geometry and is not a collision audit.

![Existing block plan](frontage-block-plan.svg)

| Building | Use / floors | Parcel width × depth (m) | Current composition |
| --- | --- | --- | --- |
| 93 | cafe / 1 | 9.10 × 7.18 | Compact shop-house |
| 95 | cafe / 1 | 18.28 × 7.87 | Broad shallow shop-house |
| 161 | cafe / 2 | 9.29 × 23.68 | Deep shop-house |
| 174 | cafe / 2 | 11.51 × 22.50 | Deep shop-house |
| 196 | home / 3 | 9.74 × 21.04 | Deep machiya family |
| 200 | apartment / 2 | 13.14 × 19.80 | Legacy full-depth volume |
| 354 | home / 3 | 14.75 × 24.09 | Broad terrace-home |
| 466 | home / 3 | 10.63 × 24.09 | Broad terrace-home |
| 512 | apartment / 3 | 14.35 × 24.09 | Legacy full-depth volume |
| 514 | cafe / 2 | 14.67 × 24.09 | Deep shop-house |
| 1158 | shop / 2 | 24.03 × 9.61 | Broad shallow shop-house |
| 1207 | office / 6 | 18.16 × 10.19 | Setback office |

## Diagnosis and correction to earlier inference

Citywide mature home parcel medians7.67–8.16m wide and13.71–14.01m deep do not describe this witness's larger sites. They rule against blanket shrinking, but cannot rule out a local mass-proportion problem. Several deep sites read as broad solid roof/body envelopes with incidental frontage space. Current mass floor-area proxies for354/466/512 are approximately792/571/792m² for engine capacities12/12/15. These sums describe rendered masses, not surveyed usable floor area, regulatory compliance or justification to change population.

Rear-court planting failed because it was hidden. Seeded commercial roof/facade substitution failed because it added plain caps. The new hypothesis is different: organize a visible planted court and the building around it as one street-facing composition, retaining a strong street wall elsewhere. Neither more shrubs behind the current masses nor another random roof selector addresses this.

## Proposed reusable composition

Eligible non-anchor home/apartment/cafe/shop sites:2–3floors, rendered envelope width>=10m, depth>=16m, and a remaining street wing>=6m wide. Study a front-corner court approximately28% of envelope width/depth, capped at4m ×6m, requiring at least2.8m ×4m. These are design parameters to screen, not Tokyo survey statistics. Apply only where the centered entrance and its complete saved access route can retain clearance. Any conflicting site must keep its existing family.

- **Mass and roof:** form an L-shaped main building around the front court, with a coherent rear wing and street wing. Preserve authoritative use, floors, capacity, parcel and entrance; quantify actual before/after occupied mass area. Do not inflate height or add residents to compensate. Keep the court open to the street rather than as a hidden rear void. Roof construction must follow the volumes, with no roof or foundation slab spanning the court.
- **Frontage and ground:** keep a continuous unobstructed entry path and a readable threshold. Replace full-envelope paving/plinth only within this family with ground surfaces matching the occupied masses and court. Carry a believable paved public edge into the court; avoid an isolated lawn rectangle or a decorative floor painted over the existing slab.
- **Visible planting:** compose one bounded crown/planting bed within the court's real space, clear of the entrance, service access, windows, canopy and parcel boundary. Home/apartment and commercial courts can have different ground treatment, but vegetation remains functional scenery within the same world. Do not merely increase a global tree count or reuse the rejected hidden rear-shrub screen as evidence.
- **Lifecycle:** use the same mass descriptor for open, construction, renovation and demolition. No fully grown planted court during initial foundations; rebuild/disposal must use existing bounded queues and shared materials.

The measured dimensional rule admits buildings174,200,354,466,512,514 in this witness. It does not yet validate access/prop/roof clearance. Across three saved worlds, open/renovating eligibility is14/66(seed0 developing/mature),21/50(seed5),25/69(seed87231). This is a targeted deep-site treatment. Narrow fronts and the office remain controls, not targets for arbitrary resizing.

## Acceptance and stop

Before broad implementation, build an isolated mass/ground/planting composition using these actual descriptors, then compare the existing ordinary frontage/interior/waterfront cameras. The result must read as a more convincing inhabited block, not merely less roof area or new empty ground. Check centered entry routes, plant/building intersections, minimum clear passage and real rendered bounds. Keep simulation state/light/lens/style/resolution fixed.

Reject if courts resemble arbitrary missing building corners, planting remains hidden, architectural identity becomes generic, narrow-site controls regress, or the normal frontage view has no meaningful gain. Do not spend a second variant on just roof color, plant height or camera changes. A promising screen alone does not authorize claiming acceptance: follow with three seeds, both stages/styles, lifecycle/production-scheduler and performance checks. Full urban fabric, presentation, activity/audio and release gates remain outstanding.

## Reproduction

Ignored local evidence at historical checkout `.tools/frontage-block-brief/`: `map.py`, `block.json`, `masses.mjs`, `block-masses.json`, `draw.py`, `scope.mjs`, `court-composition-scope.json`. Source world is `.tools/current-production-worlds/seed5-tick83000.world.json`. The small SVG is a reviewable source diagram; no bulk screenshots, recordings or saved worlds are added to version control.


## Descriptor implementation and audit — 2026-10-15

`web/src/render/front-court.ts` now implements an isolated pure descriptor; no production kit calls it. Rather than replacing the building with two full-height wings, it subtracts a front-corner rectangle from the existing mass descriptors. This retains rear service-wing heights and upper-floor setbacks, and cannot increase occupied mass floor area. Floor count, use, capacity and save state are unchanged. It requires the building's saved access path, transforms it to parcel-local coordinates and tests the entire polyline against the court expanded by0.9m. It tries the opposite side before fallback. The full-height street mass must still contain a centered2.4m-wide doorway zone; sliver fragments under1m are rejected. The planting disk is a bounded placement descriptor only.

TypeScript and four focused tests pass: subtraction/no court overlap, rear-wing preservation, alternative-side/fallback on crossing routes, rotated access coordinates, input immutability and missing-access/narrow-site fallback. Six saved-world audits retain14/66,21/50,25/69 eligible open/renovating buildings for seeds0/5/87231 at developing/mature stages. All six intended block sites remain eligible. The lowest retained mass floor-area ratio across those samples is0.90356; this is a geometric proxy, not legal usable area or a performance measurement. Clearance covers the saved access polyline and mass rectangles only, not yet windows, balconies, roofs, scaffolding or all rendered details.

![Isolated proposed mass composition](frontage-block-court-plan.svg)

Next implement the matching roof/facade/foundation/ground composition in an isolated renderer path. Remove full-envelope slabs over the court, put openings only on exposed faces, keep the authoritative centered entrance on the street wing and keep planting out of unfinished construction. Do not feed adjacent clipped masses blindly into the existing midrise renderer: it would add duplicate internal facades/entries and roof coping. Then run the ordinary matched scene screen before broad adoption. The descriptor alone is not a visual gain and does not complete the intervention.

Local audit: `.tools/frontage-block-brief/audit-composition.mjs`, `composition-audit.json`, `block-court-masses.json`. No simulation or production rendering behavior changed in this step.


## Front-court rendered screen — rejected, 2026-10-15

Completed an isolated assembled renderer for open buildings, including clipped mass foundations, exposed facades, exterior roof boundaries, centered entrances and visible court planting. Seven focused tests and TypeScript passed. The mature seed5 CPU audit built50 groups with no nonfinite coordinates or parcel-bound overflows;184,516 triangles is the candidate group total, not incremental scene cost. Construction/renovation/demolition were deliberately excluded from this screening renderer and never validated as complete lifecycle support.

WebGPU capture49417 completed with six images,50 replaced buildings, empty queues and no recorded page errors. Compared against the court-study before images: saved-world inputs, normalization, lens, preview time, quality, resolution, DOFoff and material time match; camera/target match within1e-5; source hashes unchanged. UI clocks differ, so these are matched renderer scenes rather than identical UI screenshots. Inspected three selected pairs: Miniature frontage, Anime interior and Miniature waterfront.

**Reject before broad validation.** The frontage cutouts expose planting, but replace balconies and commercial articulation with simpler repeated notched masses. The interior is effectively unchanged and waterfront gains are negligible. This does not establish a convincing complete neighborhood. Do not promote the descriptor merely because its access and geometry tests pass. No production kit imported it. The four isolated source/test files are archived under historical root `.tools/broad-home-family/rejected-front-court` and removed from the active source tree. Captures remain local in `front-court/`; verification is `.tools/frontage-block-brief/renderer-verification.json`. No recordings or bulk evidence added to commits.

Next primary investigation: resolve the scale and architectural articulation of the deep-site buildings as a whole, preserving use-specific fronts rather than replacing them with generic perimeter windows. Before further geometry, compare the measured block against the locked concepts at the same apparent building scale and identify which mass/roof/frontage relationships dominate the mismatch. The front-court result rules out corner clipping plus simplified facades as the solution; it does not prove that deeper notches, taller court trees or more eligible sites would help. Do not repeat those parameter variants. Keep any next implementation coupled to the complete block and require a meaningful ordinary-view gain before broad tests.


## Post-rejection proportion and identity audit — 2026-10-15

Reopened both locked concept boards and the retained ordinary frontage/interior/waterfront scenes. The boards distinguish compact street-facing homes and shop-houses from taller apartment/office masses; rooflines, openings and attached frontage are part of that identity. Their perspective, scale and generated imagery do not support extracting exact metre dimensions. The following numbers come exclusively from the actual saved block and current mass descriptors, not estimated reference dimensions.

Source diagnosis: `broadHomeMassing` supplies a full-depth two-floor lower body and an upper floor only2.4m shorter. `renderMidriseOpen` explicitly treats `terrace-home` as an apartment for facade generation. Conversely `midriseMassing` excludes apartments below4floors, leaving buildings200/512 on the generic legacy box path. The failed front-court renderer replaced these existing identities with one perimeter-window treatment, exacerbating rather than resolving the mismatch.

| Building | Use / current family | Largest floor plate (m²) | Plate depth / total wall height |
| --- | --- | ---: | ---: |
| 174 | cafe / shop-house | 144.34 | 2.09 |
| 200 | apartment / legacy | 196.92 | 2.56 |
| 354 | home / terrace-home | 274.97 | 2.08 |
| 466 | home / terrace-home | 198.28 | 2.08 |
| 512 | apartment / legacy | 264.04 | 2.08 |
| 514 | cafe / shop-house | 196.91 | 2.24 |
| 1158 | shop / shop-house | 128.65 | 0.89 |

These values establish which buildings dominate the low block's broad roof surfaces; they do not justify blanket shrinking or population changes. Local audit artifact: historical root `.tools/frontage-block-brief/block-proportion-audit.json`. The audit uses the retained block-masses snapshot; current source functions were reread. No new production geometry or aesthetic acceptance follows from these ratios.

### Next intervention contract

Keep the selected complete block and its cameras. Investigate use-specific street and service wings on deep sites, preserving the existing detailed frontage, centered entry, authoritative floors/use/capacity and occupied street edge. The hypothesis is a legible street-facing main volume plus lower attached rear/service volume, with roof construction and upper/rear openings following that composition. This must address homes, small apartments and deep shop-houses coherently; it must not make all three the same notched box or silently switch home fronts to apartment fronts. Keep compact existing sites and the accepted4–6floor apartment family intact.

Before rendering, quantify removed mass floor area and prove no added floor area or parcel/access intrusion. Reduced rendered floor-area proxy is a design consequence to report, not authority to change engine capacity. Keep rear ground occupied by meaningful service volume rather than introducing another empty grass strip. A shared descriptor must govern construction and renewal if the screen warrants integration; the first isolated screen may use open buildings but cannot claim lifecycle completion.

Expected visible gain: fewer broad uninterrupted caps, stronger distinctions between building uses, and a varied connected silhouette while retaining shopfronts/balconies. Stop if it reads as arbitrary stepped boxes, merely moves bare ground, loses frontage detail, or fails to improve the ordinary frontage and another weak view. Do not follow failure with a roof-color, shrub-height or eligibility-count variant. No broad validation until the scene screen succeeds.


## Deep-site street/service wings — historical candidate, rejected below

Implemented `deepSiteMassing` behind explicit `buildingKit(...,{deepSite:true})`; ordinary production callers do not enable it. Eligible non-anchor2–3floor homes, apartments, shops and cafes need at least9m rendered width and16m depth. Main street depth is capped at11m for homes,12m for apartments and10m for shops, with the remaining attached rear wing one storey. These are experimental composition parameters, not surveyed Tokyo dimensions. Street edge, centered access, floors/use/capacity remain authoritative. Existing home/shop frontage rendering is reused; small apartments use residential continuous balconies with rear ground openings omitted where the service wing attaches. Foundations follow the two volumes; construction, renovation and demolition consume the same descriptor. This is implementation coverage, not live lifecycle acceptance.

TypeScript passes;21 focused tests across deep-site, lowrise-massing and lowrise-kit pass. Added checks cover joined street/service volumes, unchanged input, eligibility fallback and complete parcel bounds through four stages/both styles. Initial test-only canvas failure was repaired using the existing shopfront test stub.

Mature seed5 audit finds71 open eligible buildings, no nonfinite/parcel-overflow geometry and156,244 candidate triangles total (not an incremental cost measurement). Rendered mass floor-area ratios range0.5698–0.9460 versus their current family; no added mass floor area. This substantial reduction must be judged explicitly rather than described as a cosmetic adjustment. Saved-world access and actor routes are not altered, but this CPU bounds audit does not prove every geometry/access interaction.

The matched deep-site capture completed. The following rejection supersedes the candidate plan; no capture is running and this candidate is not in the checkpoint.


## Deep-site screen decision — rejected, 2026-10-15

Capture34983 completed exit0 with six matched images and71 replaced open buildings. Inputs/normalization and all recorded scene controls match the court-study before baseline (camera tolerance1e-5); source hashes stayed frozen, queues drained and no page errors/failure were recorded. Selected Anime/Miniature frontage and Anime interior images inspected. UI clocks differ, despite matched rendering preview time.

**Rejected and reverted.** The shortened main volumes reveal a repetitive paved/service strip. The wide home gables and sparse side elevations read as warehouses; preserving the existing home kit did not preserve convincing domestic character at this larger size. Interior gain is negligible. No broad matrix or performance campaign is justified. Restored `kit.ts` byte-for-byte from the pre-study backup, retaining all earlier accepted/provisional edits; removed isolated helper/test from active source. Candidate and tests are archived at historical root `.tools/broad-home-family/rejected-deep-site`. Verification and CPU reports remain ignored local evidence.

Two successive block-massing hypotheses have now failed: front-corner clipping and shortened main/rear-wing composition. Stop this branch of parameter exploration. The evidence points to a kit-scale problem: apartment treatment on homes is not convincing, but stretching the current compact home kit across these sites is also not convincing. Do not try a third depth/roof/setback tweak. Next return to the reference architectural elevation and roof composition at the actual9–14m frontage scale, and establish a coherent full building treatment before changing more neighborhood mass. Keep the ordinary block as its scene-level acceptance gate; an attractive isolated asset is insufficient. The complete living-city goal and milestone1 remain incomplete.
