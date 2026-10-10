# Building source-geometry retention plan

`BuildingChunks` currently retains a full authored `THREE.Group` for every building in its `sources` map after the merged chunk is committed. `CityRenderer.buildings` also retains each group. The source groups let a later dirty chunk rebuild from the original geometry; active scene rendering, picking, and style compilation use the separate chunk batch. This creates a removable steady-state CPU copy, but also makes the rebuild inputs implicit in the retained meshes.

## Proposed ownership boundary

Keep a compact deterministic source descriptor and a regeneration callback; keep authored geometry only until its first batch has consumed it. A typed contract can be introduced without coupling `BuildingChunks` to application types:

```ts
export interface RebuildableBuildingSource {
  createGroup(): THREE.Group;
  disposeGroup(group: THREE.Group): void;
}

set(id: number, group: THREE.Group, source: RebuildableBuildingSource): void;
```

The source record should hold `{ key, createGroup, disposeGroup, pendingGroup?, contours }`. `pendingGroup` is the first authored group already produced by the existing per-frame building queue. `createGroup()` closes over an immutable copy of the relevant `Building` snapshot and the live `ArtMaterials` instance; the city keeps that descriptor and render signature in its ID map rather than a group reference. The factory sets the same translation and rotation as current `animate()` before returning a fresh group. Copy the snapshot, including the `history` array, so a later world snapshot cannot change the rebuild input.

On each chunk build, consume `pendingGroup` once or call `createGroup()` for a previously committed source. Transform/clone its meshes into the chunk merge inputs, then dispose that temporary authored group as soon as its meshes have been copied. Keep only the small builder record and the already-computed contour positions for subsequent rebuilds. If building or merging fails, dispose temporary groups and merge inputs, retain the prior visible batch, and requeue the dirty chunk; the callback can recreate the source. On commit, the old batch still follows the existing deferred-retirement path through `CityRenderer.retire()` so active async style compilation cannot observe disposed batch geometry.

Replacement/removal should drop the previous descriptor and dispose an unconsumed `pendingGroup`; its old rendered batch remains visible until the replacement chunk commits. `clear()` must dispose any pending authored groups, clear all callbacks/descriptors and contours, and retire active batches exactly once. `CityRenderer.resetWorld()` then clears metadata rather than disposing groups a second time. Only `disposeGroup()`-owned geometry, plus explicitly owned materials/textures, is released: cached `ArtMaterials` surfaces remain shared and alive. The regeneration callback must use the current `ArtMaterials` style so newly rebuilt chunks match the active style; geometry, vertex colors, material classes, and floor/sign details remain deterministic and unchanged.

## Expected resource change

The measured 850-building fixture has **148,742,812 bytes** of unique authored source attribute/index arrays. Current retained CPU typed arrays total **365,912,312 bytes** after indexed batching, including 3,861,024 bytes of source contours and 213,308,476 bytes of live batch arrays/indices/contours. Releasing authored source arrays after their batch commit would bring that counted steady-state total to approximately **217,169,500 bytes**, a reduction of **148.7 MB** (about **141.9 MiB**). The contour copies remain needed to redraw the outlines without rebuilding meshes. Builder closures, descriptors, Three.js objects, temporary merge inputs, renderer allocations, WASM memory, and other scene resources are outside this typed-array estimate. This is not a browser process-memory or GPU measurement, and it does not establish the full `<300 MiB` desktop CPU budget in `specs/08-delivery.md`.

There is a rebuild-cost trade-off. Initial authored geometry generation remains spread across the current bounded per-frame building queue. A later dirty chunk must regenerate each committed source in that chunk before merge; that may add a significant synchronous cost even though the existing merge already processes the whole chunk at once. The source factory must not eagerly regenerate every building on an unrelated style switch; existing chunk geometry and `ArtMaterials` variants already support style changes. Add per-chunk generation/merge timing diagnostics before rollout, and keep the previous batch live while replacement work proceeds. If a regenerated chunk exceeds its frame budget, split its work into bounded build phases while preserving the existing coalescing, overdue-commit, and one-chunk-commit-per-frame behavior rather than retaining all authored sources as a permanent workaround.

## Regression coverage required before integration

- Compare pre-change and regenerated batch geometry for a fixed selection from the fixture: identical triangle order/count, positions after translation/rotation (including negative scale if supported), normals, UVs, per-vertex linear colors, material class/style, `castShadow`, and `receiveShadow`.
- Verify source groups and owned per-group resources dispose once after consumption; shared `ArtMaterials` resources do not dispose. Cover a dirty-chunk rebuild using the callback, a source replacement before first commit, removal, clear/world replacement, and a failed factory/merge that keeps the previous batch and can retry.
- Assert the old batch remains attached until commit and its geometry is retired once through the renderer's delayed-disposal callback, including while a style compile is pending.
- Raycast both before and after regeneration; face indices must continue to map to the same source building IDs after indexed merging. Repeated style changes and world replacements must not increase retained source-geometry counts.
- Exercise the real 850-building fixture after all batches drain. Confirm zero source geometry arrays remain, triangle/material/picking totals match, and report per-chunk regeneration and merge costs. Separately inspect browser resource diagnostics; typed-array arithmetic alone cannot verify the CPU-memory release or GPU resource behavior.

Relevant implementation: [`building-chunks.ts`](../web/src/render/building-chunks.ts), [`city.ts`](../web/src/render/city.ts), and source-group ownership in [`kit.ts`](../web/src/render/kit.ts). Current measurement: [`mature-building-geometry-budget.json`](./mature-building-geometry-budget.json).
