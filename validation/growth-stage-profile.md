# Growth stage profile — 2026-10-12

Temporary opt-in instrumentation captured normal WebGPU UI growth after importing
seed 87231, tick 18918, 441 buildings, clear weather, growth 16× and speed 8×.
Miniature, Overview, DOF off. Raw records and computed summary are retained in
`growth-stage-profile-raw.json` and `growth-stage-profile-summary.json`.

For 103 world updates after the imported tick, synchronous setWorld time had
median 103.9 ms, p95 118.1 ms, maximum 150.2 ms. Stage medians:

| Stage | Median ms | p95 ms |
| --- | ---: | ---: |
| Roads, including bridge/signature preparation | 61.9 | 68.0 |
| Tree update | 22.7 | 27.8 |
| Transit/access owners | 18.3 | 22.8 |
| Ground surfaces | 1.7 | 3.3 |
| Building inputs | 0.4 | 0.5 |
| Sidewalk input | 0.1 | 0.2 |

This establishes substantial synchronous structural-update work. It does not
measure GPU time or isolate individual operations within these stages. Browser
hardware was not identified, so this is diagnostic evidence rather than target
hardware acceptance. Timer/log overhead was not subtracted.

The bounded frame recorder exhausted all 120 entries before growth resumed:
its last tick is 18918. It additionally recorded only frames above 25 ms.
Consequently it cannot rank growth-frame render submission versus world rebuilds,
or supply an unbiased frame percentile. Do not infer these from the raw log.

Instrumentation is archived in `growth-stage-instrumentation.txt` and removed
from production source. No scheduling/performance fix was implemented here.

Next bounded intervention: investigate retaining unchanged road geometry across
construction updates, preserving batching, style changes and dependent bridge,
platform, junction, bank-rail and light invalidation. Compare the same growth
fixture before adopting a cache. Do not lower visual quality or relax acceptance.
The original screenshot's exact camera/motion reproduction remains open.
