# Mature building geometry budget

Measured the 850-building city at seed 87231, tick 242000. Input and transitive source hashes are recorded in `mature-building-geometry-budget.json`. Reproduce with `node scripts/audit-mature-building-geometry.mjs`.

The production kits author **2,135,638 triangles** across **14,422 meshes**. The production descriptor-based chunk path preserves every triangle in **417 batch meshes and 16 contour objects**, across 16 chunks. The staged drain takes 1290 update calls; this count is not a frame-time measurement.

After the drain, all source records have released their pending groups and retain no legacy groups. Source attributes and indices contribute **0 retained bytes**. Immutable building descriptors regenerate geometry when a chunk needs rebuilding. Persistent source contour copies occupy **3,845,280 bytes**, and batch arrays (including batch contours) occupy **215,649,748 bytes**. The resulting subtotal is **219,495,028 bytes (219.5 MB)**. Keeping this same source geometry as well would add **150,410,212 bytes**, without changing the rendered geometry.

Preserving batch indices saves **70.1 MB (24.5%)** compared with converting these same merged meshes to non-indexed geometry. This is a separate saving from releasing source groups.

This audit inspects production object references and typed-array lengths in Node. It does not measure garbage collection, browser heap, transient rebuild peaks, GPU VRAM, materials/textures, other scene geometry, actual draw calls, or frame rate. Geometry disposal alone does not prove memory release; the audit additionally checks that the chunk source records and active build no longer retain groups. Browser style-switch/reload validation remains necessary.

The JSON includes family contributions and heavy buildings for future LOD decisions. No geometry was removed by this memory change; visual fidelity still requires separate acceptance against the references.
