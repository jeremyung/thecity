# Batched cladding dark-wall correction

PR19 head `9002612` rendered some Miniature clad walls nearly black in a mature
city although the authored wall material was pale. The earlier isolated facade
review did not exercise the shared batch shader. Anime's generally darker palette
was already present in the older comparison; it is not evidence of the same
new regression.

The correction evaluates the four derivative-based cladding patterns outside
conditional shader blocks and then masks by the per-vertex finish. It preserves
base colours, geometry, lighting, room windows and cladding detail. A runtime
cladding bypass first localized the problem; the branchless candidate restores
Miniature walls without removing their surface pattern. This demonstrates a
conditional-evaluation defect; the exact driver/compiler mechanism has not been
proven. All four patterns now execute for clad fragments; GPU cost has not been
separately benchmarked.

## Reproduction and regression

With the app's Vite server running, execute:

```sh
CLADDING_URL='http://127.0.0.1:5173/?webgl=1' node scripts/audit/check-cladding-batching.mjs
CLADDING_URL='http://127.0.0.1:5173/' CLADDING_OUTPUT=.tools/cladding-parity-webgpu node scripts/audit/check-cladding-batching.mjs
```

The browser check compares authored and batch materials on identical boxes,
colours, geometry, camera and lighting, covering siding, tile, ALC and concrete
in both styles. Output is local and ignored. It checks the actual backend in its
report, collects console/page errors, and rejects an RGB mean absolute difference
over 1/255 or more than 0.5% of channels differing by over 10/255. It is a scene-pass
shader regression, not full post-processing or simulation acceptance.

On Apple M2 Max/macOS, Chromium 145.0.7632.6:

- Original shader: Miniature mean RGB difference 9.386/255, 8.985% large differences;
  the check fails. Original source was restored only for this check, then the
  candidate was restored automatically.
- Final corrected WebGL/ANGLE Metal: Anime 0.000689/255, Miniature zero; no large
  differences or console/page errors.
- Final corrected native WebGPU: Anime 0.000540/255, Miniature zero; no large
  differences or console/page errors.
- Full web suite: 125 suites, 689 tests pass. TypeScript and 34 focused surface,
  building-chunk and construction-batch tests pass. No Rust/WASM change.

## City evidence and limits

Six matched captures cover seed0/tick83000 frontage, interior and overview in
both styles with depth of field off. Input hash, camera/target, FOV, quality,
resolution and empty render queues match the previous PR-head capture. Miniature
frontage and interior restore the expected pale walls; close-up matrix captures
on both backends retain the material detail. Evidence stays local under
`.tools/cladding-fixed-city`, `.tools/cladding-matrix`, and
`.tools/cladding-matrix-webgpu` in the original workspace.

The city harness recorded one unidentified resource 404, but no other errors;
a fresh startup trace did not reproduce it. The material checks recorded none.
The city comparison injects an unchanged historical snapshot into the renderer;
it is not current scheduler, loading/replay, three-seed fidelity, or performance
acceptance. Existing open fabric, repeated massing and infrastructure dominance
remain separate work. This fix does not establish completion of the living-city
goal or user acceptance against the locked concepts.
