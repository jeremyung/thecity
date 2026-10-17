# Building surfaces benchmark (2026-10-14)

`node scripts/benchmark/benchmark-building-surfaces.mjs <save.json> <port> <label>` on the seed 87231 review city (tick 242000, development rate 16), run against `origin/main` and `render/building-surfaces` served side by side. Hardware WebGL through ANGLE/OpenGL ES on an NVIDIA RTX PRO 6000 Blackwell, 1600×900, balanced quality. Each row renders the scene pass with shadows and calls `gl.finish`, 120 samples after 30 warm-up frames; the post pipeline is left out because its `render()` returns without drawing in headless Chromium. Compare rows, not absolutes. Raw numbers: [building-surfaces-benchmark.json](building-surfaces-benchmark.json).

| Style | Hour | View | main median | branch median | main draws | branch draws | main triangles | branch triangles |
|---|---|---|---|---|---|---|---|---|
| Anime | 13:00 | street | 1.10 ms | 1.10 ms | 98 | 99 | 817,751 | 854,111 |
| Anime | 13:00 | neighborhood | 2.00 ms | 2.00 ms | 217 | 214 | 2,841,561 | 2,968,197 |
| Anime | 13:00 | overview | 3.40 ms | 3.30 ms | 397 | 389 | 5,606,017 | 5,762,041 |
| Anime | 21:00 | street | 1.10 ms | 1.10 ms | 98 | 98 | 817,751 | 819,983 |
| Anime | 21:00 | neighborhood | 2.00 ms | 2.00 ms | 218 | 215 | 2,841,561 | 2,840,541 |
| Anime | 21:00 | overview | 3.40 ms | 3.30 ms | 397 | 389 | 5,580,961 | 5,572,609 |
| Miniature | 13:00 | street | 1.20 ms | 1.20 ms | 96 | 96 | 817,751 | 819,983 |
| Miniature | 13:00 | neighborhood | 2.20 ms | 2.20 ms | 210 | 207 | 2,841,561 | 2,840,541 |
| Miniature | 13:00 | overview | 3.60 ms | 3.60 ms | 381 | 373 | 5,580,961 | 5,572,609 |
| Miniature | 21:00 | street | 1.20 ms | 1.20 ms | 96 | 96 | 817,751 | 819,983 |
| Miniature | 21:00 | neighborhood | 2.20 ms | 2.20 ms | 210 | 207 | 2,841,561 | 2,840,541 |
| Miniature | 21:00 | overview | 3.70 ms | 3.60 ms | 381 | 373 | 5,580,961 | 5,572,609 |

Frame time is unchanged within the 0.1 ms timer resolution, and draw calls change by between one more (Anime street) and eight fewer (overview). All clad walls in a neighbourhood share one batch (the cladding kind is a vertex attribute), and every window pane shares one batch (its light schedule is packed into the pane data), replacing the plain glass, light glass and per-schedule glow batches. A first version that gave each cladding kind and each schedule its own batch cost 5–54 extra draws and up to 0.5 ms at overview distance.

Triangle counts differ slightly: the street and neighbourhood views gain the side-window frames and panes that were previously hidden inside their frame boxes, and towers lose their separate glowing overlay boxes.

## Limits

- One desktop GPU through WebGL; WebGPU and mobile frame cost are not measured.
- The window shader is the most expensive building material per pixel. It is cheap here because windows cover a small share of the screen; a camera filling the view with glazing has not been profiled separately.
