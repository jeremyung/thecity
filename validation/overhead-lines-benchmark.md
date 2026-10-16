# Overhead lines benchmark (2026-10-15)

`node scripts/benchmark/benchmark-overhead-lines.mjs <save.json>` on the seed 87231 review city (tick 242000, development rate 16) generated and served from `codex/city-runtime-review` with this branch on top. Hardware WebGL through ANGLE/OpenGL ES on an NVIDIA RTX PRO 6000 Blackwell, 1600×900, balanced quality. Each row renders the scene pass with shadows and calls `gl.finish`, 120 samples after 30 warm-up frames. The post pipeline is left out because its `render()` returns without drawing in headless Chromium. Use the differences between rows, not the absolute times. The script prints the raw numbers as JSON on its last line.

## Layer

| | Count |
|---|---|
| Utility poles | 758 (188 with a transformer) |
| Meshes | 9 (5 instanced, 4 wire batches in 420 m cells) |
| Wire triangles | 62,380 |
| Rebuild, world to group | 23.4 ms median, 30.6 ms max of 7 |

The rail catenary is not part of this layer: the rail corridor (`rail-corridor.ts`) draws it along the curved alignment.

## Frame cost

| Style | View | Lines | Median | p95 | Draws | Triangles |
|---|---|---|---|---|---|---|
| Anime | street | off | 3.40 ms | 3.60 ms | 317 | 4,185,463 |
| Anime | street | on | 3.50 ms | 3.70 ms | 328 | 4,383,847 |
| Anime | neighborhood | off | 2.00 ms | 2.20 ms | 164 | 2,614,449 |
| Anime | neighborhood | on | 2.10 ms | 2.20 ms | 173 | 2,780,441 |
| Anime | overview | off | 4.50 ms | 4.80 ms | 428 | 5,977,437 |
| Anime | overview | on | 4.50 ms | 4.80 ms | 441 | 6,206,741 |
| Miniature | street | off | 4.30 ms | 4.60 ms | 307 | 4,185,463 |
| Miniature | street | on | 4.50 ms | 4.90 ms | 318 | 4,383,847 |
| Miniature | neighborhood | off | 2.50 ms | 2.60 ms | 158 | 2,614,449 |
| Miniature | neighborhood | on | 2.60 ms | 2.70 ms | 167 | 2,780,441 |
| Miniature | overview | off | 5.70 ms | 6.00 ms | 412 | 5,977,437 |
| Miniature | overview | on | 5.80 ms | 6.10 ms | 425 | 6,206,741 |

The layer adds 9–13 draw calls, about 0.2 M triangles (shadow pass included) and 0–0.2 ms per frame on this GPU.

## Limits

- One desktop GPU through WebGL; WebGPU and mobile frame cost are not measured.
- The rebuild runs with the park layer, which rebuilds when roads, decks, platforms, open spaces, building entries or bus stops change. Sampling the curved rail and expressway keep-outs raised it from about 16 ms on `main` to about 23 ms here; on a growing city that is a periodic one-frame hitch, not yet profiled in a long live session.
- Wire shimmer was reviewed in software WebGL captures (four frames 0.4 m apart), not on hardware with the post pipeline's anti-aliasing.
