# Tencent texture pilot

Completed four approved 1K requests using `tencent/hy-image-v3.5-preview`, pinned to Tencent through OpenRouter. Reported cost: $0.024 each, $0.096 total against the $2 cap. No retries or extra generations. Exact prompts and returned usage are in `ledger.json`; endpoint pricing is preserved separately. Credentials were read only by the offline Node process and were not written to the ledger or browser assets.

Inspected the 512px derivatives and 3×3 repeated contact sheet:

| Candidate | Finding | Decision |
|---|---|---|
| Warm plaster | Attractive restrained trowel detail, but thin light border creates an obvious repeated grid; some surface relief is baked into colour. | Reject as-is; border cleanup and relighting review needed. |
| Cast concrete | Quiet fine mineral grain, least visible repetition, useful neutral colour. | Keep for subsequent in-scene evaluation; not yet accepted for shipping. |
| Painted steel | Useful desaturated blue-gray, but horizontal/vertical repeat seams and tonal bands remain. | Reject as-is; seam correction required. |
| Aged timber | Fine grain, but conspicuous light perimeter and recognizable patch repeat. | Reject as-is; border cleanup and tiling correction required. |

The pilot demonstrates inexpensive useful raw material generation, not reliable production-ready seamless textures. No candidate has been promoted into the city. Geometry, facade composition, density, lighting, and camera remain the larger art-fidelity work; surface maps cannot replace them.

All sources are 1024×1024. Four 512×512 candidate derivatives total 1,564,511 PNG bytes and an estimated 5,592,400 bytes (5.33 MiB) for one RGBA8 copy each including complete mip chains. This is an allocation estimate, not measured GPU usage, and excludes duplicate style copies and other render resources. Current added runtime memory is zero because these files remain outside public runtime assets.

Next acceptance step: correct seams locally without additional paid calls, then compare a single street at matched camera settings in both styles at dawn, noon, dusk, and night. Keep live glass, emission and directional illumination separate. Discard maps that only add noise.


## Facade review integration

Concrete512 is available at `/?review=architecture&material=concrete&webgl` for development review; the original plaster route remains the default. Candidate on shop toggles comparison. First Anime facade view loads successfully and shows restrained fine grain (`validation/captures/concrete-pilot-shop-anime-webgl.png`). No live-city promotion. The review creates a wall texture and facade clone, so budget two potential resident copies (~2.67MiB), not just the one source file. Production build passed (`validation/concrete-pilot-review-build.txt`). Both-style/night comparison remains outstanding.


## Promotion decision after direct comparison

Root compared the same facade/camera in Anime daylight, Miniature daylight, Miniature night, and Miniature daylight with the candidate disabled. Captures are `validation/captures/concrete-pilot-shop-{anime,miniature,miniature-night,baseline}-webgl.png`. The map adds restrained fine grain and responds to night lighting; it does not materially solve the flat facade character or weak style distinction. Keep as a development candidate, with no live-city promotion and no additional paid generation. The night specimen lighting is intentionally simple and is not equivalent to city lighting. Broader street/day-cycle/WebGPU acceptance remains incomplete.


## Review-path correction

The candidate facade path previously replaced Anime colorNode with raw triplanar base colour, dropping its painterly pigment treatment. It now multiplies candidate base colour by the same `paintedSurfaceTint()` used by painted Anime surfaces. Miniature retains the sampled base colour. Earlier paired captures remain evidence of the earlier path, not a fair final comparison of the corrected styles. Build verification is in `validation/candidate-style-preservation-build.txt`; refreshed visual comparison is pending. The decision to withhold live-city promotion remains in force.

## Corrected shader recheck — 2026-10-11

Revisited the concrete specimen after the Anime pigment-preservation fix using
`/?review=architecture&material=concrete` without forcing WebGL. Fixed Inspect shop
camera at 1280×720, open construction stage, no DOF; candidate on/off in both
styles and Miniature night. Captures: `validation/screenshots/texture-recheck/`.
The route requests WebGPU with fallback; no backend diagnostic was collected, so
these captures do not establish a backend-specific acceptance claim. No console
errors were observed. This static specimen has no simulation seed/tick.

The corrected Anime path preserves its surface treatment. Concrete adds restrained
grain but does not materially improve facade composition or the clean, sparse
shopfront impression. Night illumination remains separate; this simplified rig
is not a full city day-cycle comparison. Keep the map outside living-city use.
This closes the pending corrected-shader specimen review, not street-scale or
full lighting acceptance. No generation calls or additional spending occurred.

Next asset hypothesis: a shared street-detail atlas may add readable commercial
character where another generic wall map would add only grain. Before generation,
identify exact existing-kit surfaces, UV/instancing constraints and a baseline
street camera; preserve authored Japanese lettering and separate emission. Cap
a first experiment at one 1024-square atlas (about 5.33 MiB RGBA8 with mips per
resident copy), then accept only on visible gain in both styles and live lighting.
Do not generate a broad library or unique maps per building.

Refreshed `validation/texture-budget.json` now includes both public development
candidates: 9.329 MiB estimated source RGBA8 mip storage, excluding clones, canvas
textures and render targets. The 1254-square plaster still exceeds the proposed
1024 tile side limit. Neither candidate is runtime-listed; these figures are not
measured city GPU memory. The concrete review can have a wall and facade copy
(about 2.67 MiB together).
