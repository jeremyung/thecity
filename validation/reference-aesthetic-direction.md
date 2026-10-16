# Priority update — camera, shaders and rendering

User steering now makes camera, shaders and rendering the primary workstream. This supersedes architecture-first next-action statements below; historical results remain valid. The full living-city scope is unchanged. The daylight matrix and subsequent local screens are closed; consult the current checkpoint and material-study summary for outcomes. Assess lens/framing/focus and shader/rendering behavior against the locked concepts. Prioritize substantial whole-scene gains, including temporal stability and browser cost. No building-kit or parcel micro-variant should displace this pass. See the synchronized living-city goal and specification08 for acceptance gates.

# Reference-led aesthetic direction — 2026-10-15

## Current decision

Follow `archive/2026-10/HANDOFF.md` for active source and validation status. Historical investigation sections below do not describe current running jobs. Retained candidates are street/roof material separation, broad terrace homes, closed gables and provisional restrained AO. The AO static matrix, production day/night preview and camera-pose sweeps have completed; full temporal/weather/performance and artistic acceptance remain outstanding. Neighborhood fidelity is unaccepted. Stop isolated compact-home details, local planting variants and AO scalar tuning. The apartment workstream reached a retained provisional checkpoint: a proportion-qualified slab family is implemented and locally screened, with the80-image static screen and bounded production review complete. All eight production previews were inspected; growth stalls remain unresolved (candidate max457ms, baseline375ms), and five-second stage samples do not validate construction/renewal. The bounded profile identified repeated park-tree/utility-pole instance recreation; reuse now cuts measured pipeline creation597→14 while preserving detail. Remaining frame/world-update spikes are unresolved; see HANDOFF. Preserve authoritative city state and judge complete scenes; no whole-neighborhood acceptance is claimed. See the latest study and handoff for scope and stop conditions.


The shared tree-crown orientation screen is rejected. Twelve captures completed with matched controls/no errors; selected frontage/interior/waterfront pairs show negligible whole-scene gain despite changing summed top-projected triangle area8–9%→60%. The tilted-card candidate never entered production source. Stop foliage-card/alpha/placement-count tuning. Next investigate complete-neighborhood scale/composition: repeated large roof/building masses, broad infrastructure surfaces and narrow incidental planted edges. Identify actual descriptors and camera proportions against both locked concept boards before a coupled intervention; preserve authoritative state and existing witnesses. Do not treat a more flattering camera as a geometry fix.


Latest user steering explicitly asks for more realistic streets/buildings and look/feel against concepts, beyond topology and allocation. Goal and specification08 milestones now assess complete scenes and select the largest visible gap. Original full living-city scope is retained. This supersedes the prior strict subsystem ladder in historical notes.

## Evidence inspected

Reopened both locked concept boards: `concepts/01-little-tokyo.png` and `concepts/04-last-train-first-light.png`. Compared them with mature seed5 frontage/loss-site captures from the merged979469c renderer and frozen baseline/v1/v2 worlds. These have different compositions from the concept boards; they support qualitative findings, not pixel or dimensional equivalence. The concepts' day panels show smaller articulated roof/facade masses, readable material differences, layered street edges, substantial planting and buildings embedded in a larger landscape. Warm directional light is present without every surface becoming uniformly brown.

Current ordinary frontage has close-set buildings and existing details, but repeated large dark roofs, strongly repeated trim/window patterns, broad pavement and incidental grass dominate. The issue is not wholesale absence of sidewalks, balconies, cladding or utility wires. Do not add duplicate systems based on old reviews.

## Existing implementation inspected (merged PR19 renderer)

- `web/src/render/materials.ts`: asphalt already has world-space aggregate/patch/crack/wear shaders and wet roughness; base color is `#747f7b`. Its appearance must be tested under fixed lighting, not blamed on missing textures.
- `web/src/render/painted-surface.ts`: shared painted tint mixes cool/warm pigment with world-space strokes and wash. Large-scale tint can affect material separation; any change must preserve individual/batched parity and both backends.
- `web/src/render/building-surfaces.ts`: use-specific cladding and room windows already exist. Preserve pane tags, occupancy behavior and batched material fixes from PR19.
- `web/src/render/lowrise-massing.ts`: lowrise frontage mass consumes96% of body width and72–76% of body depth, with a rear service mass; roof family depends on clearance/seed. Investigate actual visible descriptors before changing these ratios or claiming they explain every long roof.
- `web/src/render/sidewalk-kit.ts`: reference sidewalk extrudes0.35m above a0.125m origin. Runtime uses `CitySidewalks/SidewalkBuild`; inspect that production path before changing curb geometry. Do not infer a universal real-world target from this value alone.

## Next bounded visual intervention

Use the unchanged mature seed5 baseline world and its ordinary frontage/interior cameras, plus one closer view selected to expose the same street/building edges. First identify the actual visible building families and production pavement/curb materials. Compare the same world, camera and light before/after. Improve the shared street/building material and construction hierarchy, not simulation allocation: credible asphalt/concrete/wall/roof separation, restrained wear, and appropriately scaled roof/facade/curb edges. Change only the evidenced contributor(s); this is not authorization for a blanket palette wash or blanket extra detail.

Visible payoff: a complete ordinary street reads as constructed buildings and public space, closer to the concepts at normal distance. Reject a variant if it only looks better in an isolated asset, erases style distinction, needs blur to work, creates shimmer or violates batching/performance/clearance. Begin with a small comparison, then expand only after a visible gain. Do not generate a broad texture pack or reopen parcel-fitting to pursue this pass.

## Access candidate disposition

V1 complete160-image matrix has valid capture controls but visible lawn regressions in seeds5/87231. V2 restores some mass in seed5's rejectedFrontage view; ordinary frontage remains substantially similar and no aesthetic milestone is accepted. Its capture session18929 completed; six-frame evidence is local at `.tools/flag-access-v2/render/candidate/seed5`. Full seed87231/correctness/visual acceptance remains outstanding. Preserve the isolated candidate without promotion or a third fitting tweak. This is deferred work, not a declaration that legal access is optional.


## Broad-house elevation diagnosis — 2026-10-15

Rechecked the isolated `street-building-realism` candidate against its mature seed5 Miniature frontage capture (`.tools/street-building-realism/candidate/seed5/83000-miniature-frontage.jpg`, local evidence). The material-only candidate remains unaccepted. Broad dark gables and blank end walls remain prominent; repeating a roof-only variation is not the next action.

Source inspection supplies a more specific hypothesis. `lowrise-massing.ts` chooses `quietSides: [side]` from the appearance seed. `renderSideElevation` skips that entire elevation without an adjacency query. This is a seeded architectural convention, not evidence of a real party wall. The visible blank wall must be checked against actual orientation/neighbors before changing its treatment. Do not indiscriminately put windows through true shared walls.

`renderHouseFacade` uses exactly two upper-floor openings, capped at 2.3m (2.12m glazing in the detailed path), across all supported lowrise widths. For confirmed home354's 13.591m frontage, the two openings occupy approximately31% of frontage width; home466's 9.800m frontage gives43%. These are horizontal coverage ratios, not facade glazing-area ratios or target standards. At ground level, the centered entry and two windows occupy a compact central cluster even on broad homes. Side elevations, when enabled, also retain exactly two openings over a roughly15.377m depth. This supports investigating a compact-house facade being stretched across a broader building, rather than asserting missing textures or missing roof meshes.

Next bounded candidate: define a coherent broad-home elevation family with span-appropriate room/bay rhythm and deliberate treatment of exposed end walls, preserving simulated building identity, footprint, floors, occupancy, entries and access. First confirm actual exposed faces on home354/home466 and inspect full-building street views. Reuse existing window materials/occupancy tags and construction paths. Do not change roof geometry, lighting, lens or simulation during this comparison. Reject if additional bays merely create a repetitive office grid, introduce neighbor-facing conflicts, or fail to improve the complete street scene. A meaningful local screen is required before the three-seed/stage/style validation. No geometry implementation or visual acceptance is claimed by this diagnosis.


### Completed facade screen and family evidence

The span-aware-window candidate has now been rejected after six matched captures (session75073 completed0, comparison check reports no errors). Three-to-six upper-floor bays made the street more repetitive without resolving the large roofs or blank end walls. Source/tests were archived locally and the isolated worktree returned to material-only. This is the second failed architecture variant, following the hip roof; stop tweaking roof shape/window counts separately.

Frozen mature seed5 descriptors confirm home354 and home466 are both open, three-storey `home` entities, each capacity12 and occupancy12. Their parcels are14.746641×24.086472m and10.633784×24.086466m; both rotation1.8066885. Capacity does not prove household count or apartment use, so do not reinterpret them as twelve apartments. Existing `midriseMassing` explicitly accepts apartment/office kinds; a simple renderer-kind substitution would cross an architectural boundary and needs review. Next investigate a coherent broad residential family, preserving kind, occupancy behavior and entry/access. Use the references to choose massing/circulation first, then facade and roof together; no third incremental window/roof variant.


## Family reassessment against locked concepts — 2026-10-15

Reopened both locked boards, including their daytime panels. The target combines compact pitched-roof houses/shop-houses with taller narrow plaster/concrete residential masses, articulated service spaces and restrained roofs; it does not imply converting every large lowrise to a balcony slab. Vegetation, winding infrastructure and foreground/background composition remain major scene-wide differences. A broad-home family is a bounded architectural improvement, not the full fidelity milestone.

The current engine's `choose_building_profile` sets home capacity to `6 + floors * 2`; capacity12 is therefore a floor-count consequence, not evidence of twelve dwellings. The renderer's `midriseMassing` accepts only apartment/office kinds and floors4–11. Existing walkup geometry is useful implementation material, but cannot be activated by silently changing a home's kind. Home occupancy, pane schedule, authoritative entries and construction identity must survive any renderer family choice.

A source-driven scope probe evaluates the existing lowrise descriptors, selecting home/frontage masses with3floors, width>=9m and depth>=12m. This is a candidate study filter, not an accepted production threshold. Results (developing24000 / mature83000): seed0=6/26, seed5=7/20, seed87231=6/24. Mature city totals are1796,1814,1795. Raw script and per-building records remain local at `.tools/broad-home-family/{scope.mjs,scope.json}`. This limits scope to roughly1–1.5% of mature buildings, although the frontage witness gives some of them disproportionate screen area. Do not present it as a city-wide solution.

Implementation brief for the next bounded whole-family candidate:
- Keep one home entity and its centered authoritative street entrance; no fabricated separate dwelling entrances or kind conversion.
- Compose an entrance/stair core and a residential wing, with a modest upper-storey setback/usable terrace instead of one uninterrupted shallow gable. Use dimensions needed to fit the circulation and rooms to determine applicability; inspect compact and threshold cases.
- Design facade openings, terrace parapets and roof services with that massing. Preserve existing occupancy-tagged pane materials and neutral palette. Do not stack the rejected extra-window patch onto it.
- Treat construction and renovation as the same mass descriptor, with all built stages inside the parcel. Existing construction code assumes base-floor0 for lowrise masses; an elevated upper tier needs explicit handling or the established midrise lifecycle rather than only changing the completed mesh.
- First compare the two real witnesses and the ordinary frontage/interior scenes against material-only, fixed world/camera/time. Reject if it becomes a generic apartment slab or the gain is too small. Broader validation follows only a clear complete-building/street gain.

This turn establishes scope and implementation constraints; it does not add a third visual candidate. Active worktree remains material-only. No captures running, no visual milestone accepted.
