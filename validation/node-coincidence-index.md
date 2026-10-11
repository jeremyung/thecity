# Source-ordered node coincidence index — 2026-10-11

Preflight now builds a transient spatial index alongside its validated node-ID index. Each query checks nine neighboring cells and returns the smallest eligible source index through ordered-tree lower bounds. It keeps the original f32 subtraction/hypot predicate and lexicographic pair order. It does not allocate a candidate vector or scan a whole crowded bucket.

Cells are 2e-5 units wide, twice the 1e-5 coincidence threshold. Exact f32 coordinates are converted to f64 for cell assignment. Within the supported ±100,000 coordinate domain, a pair accepted by the f32 distance predicate cannot span more than one cell on either axis: subtraction rounding at the threshold is far smaller than the cell-width margin. Tests exercise positive/negative cell boundaries, neighboring f32 representations, randomized inputs, and dense and sparse layouts. Unsupported coordinates latch a full-scan fallback; the accepted input domain is unchanged.

The caller's existing 512-node cap bounds index storage. One work unit charges a nine-cell query and at most one exact pair test. Insertion and lookup use logarithmic tree operations; logical units are not equal-cost CPU instructions. Dense local populations can still require quadratic pair testing. Full index reconstruction during checkpoint restore is outside advance accounting.

## Checkpoints and determinism

The index is derived from the validated node prefix and is not serialized. Preflight schema remains 3: coincidence cursors retain their source-index meaning, including old all-pairs cursors pointing at pairs that the index skips. Tests check that such a cursor finds the same first failing pair and that JSON restoration reconstructs the complete runtime state at quotas 1/7/48. Work scheduling changes across builds; this is not a promise of identical cross-version admission timing.

## Evidence

- Full native release suite: 351 passed, zero failed, 42 ignored, 49.17 seconds. Six focused coincidence tests pass. The additional old-cursor assertion was then checked with the full preflight test group.
- Mature fixture: 159 nodes, 206 edges, unchanged indexed roads and 49 reference faces. Preflight uses **5,692 logical units**, versus 18,095 before this change (68.54% fewer), for quotas 1/7/48. See `road-face-preflight-mature.json`.
- Rebuilt production WASM preserves all **3,180 descriptors bit-for-bit** against the native reference, across quotas 48/4096/16384 and JSON resume. Total discovery work is **7,380,416**, versus 7,392,819 (about 0.17% fewer). See `access-discovery-wasm-profile.json`.
- Actual prior-binary schema2 save migrates successfully in Node WASM, preserving non-job world fields, exact save round-trip, and deterministic 24-tick continuation. See `legacy-preflight-schema2-wasm.json`.

These are planning correctness and logical-work results. No new renderer, browser-worker timing, overall FPS, density or art-quality improvement is claimed. The coupled 512 graph limits remain. Remaining scaling work includes graph capture, restore, topology and face processing; visual acceptance still needs denser urban form, architectural variety, water and lighting refinement.
