# Descriptor-based staged building chunk benchmark

Input: `validation/early-reservation-wasm-87231-snapshot.json` (seed 87231, tick 242000, 850 buildings), SHA-256 `0559e20d76403429dffcc8a13d0953b699accf0ce2c73df0e30401c4da40c94f`.

The initial 14,422-mesh source queue used **1171.3 ms** for buildingKit generation and **3121.2 ms** in set/outline preparation. The descriptor-based staged drain took **1,290 updates** (867.0 ms); per-update p50/p95/max was 0.526/1.774/9.676 ms.

Replacing building 13 in chunk `1:2` regenerated 33 neighboring source groups across 51 updates. Factory calls per update peaked at **1**; per-update p50/p95/max was 1.404/3.092/4.493 ms. The replacement added no triangle difference (2,135,638 before and after).

After both drains, 850 source records had zero pending or legacy group references; no active build remained. This is a structural reference check, not evidence that garbage collection reclaimed a particular number of bytes. Timings are single-run Node CPU observations on Apple M2 Max; they do not represent browser frame pacing, GPU upload, or memory. See the JSON for hashes, source counts, per-update distributions and limits.
