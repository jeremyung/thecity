# Street/building material and rendering study — checkpoint 2026-10-16

This decision summary replaces the chronological experiment log. Raw captures, scripts, source fingerprints and the original log remain local under ignored `.tools/`; they are not PR dependencies or portable CI evidence. All experiments below are closed. The living-city goal remains incomplete.

## Retained work

- Optional depth-aware tilt-shift with position, angle, plane tilt, width and blur controls; ordinary depth of field and unblurred inspection remain available.
- Stable scene submission avoids changing Three's render-call depth on first focus activation. Focused/unblurred pipelines share upstream effects; owned intermediate textures and AO noise have explicit disposal. Tree/pole instance identity is reused when capacity permits.
- Daylight key is stronger and more neutral; night is unchanged. Ambient occlusion is reduced. Horizontal painted surfaces use fine neutral grain. The distant flat roof-grid pattern causing square artifacts is removed.
- Broad three-storey homes have a bounded terrace family; facade/service-side and flat-roof material distinctions are retained. The family affects only 20–26 mature homes in the inspected seeds, not the entire low-rise city.
- Shared-street entrance height follows the local/lane datum. Fallback gable ends now have infill, correctly oriented fascia and vents. Ray tests establish closure; ordinary-scene regression was checked, but neighboring buildings prevented detailed end-wall visual confirmation.
- Canopy cores retain continuous foliage silhouettes. The later bank-planting experiment is not part of this work.

## Camera evidence and limits

Browser studies used Chromium 145.0.7632.6, WebGPU/Metal, 1200×800 balanced, generally a paused mature seed0 fixture. They are local diagnostics, not live-production performance or complete acceptance.

- One Watch study reduced measured first activation from 4715 to 59 ms in Anime and 13658 to 50 ms in Miniature. Earlier steady p95 was 33–42 ms; this does not establish 60 fps.
- A 24-state focus/style/resize/wet-dry lifecycle returned texture counts to 51 Anime / 56 Miniature. Counts are not GPU memory bytes.
- Sixteen day/dusk/night/dawn × style × focus conditions checked actual shader state; only selected images were visually inspected.
- Eight weather-motion states (two styles × clear day/rainy night × focus off/on) sampled 484–566 frames each, 108.169 m camera displacement and finite state. Endpoint images support local appearance, not absence of transient shimmer/halos. rAF p95 25.3–26.5 ms included input/capture overhead; maxima reached 90.8 ms.
- A separate paused, stationary 360-frame/state check gave median 8.4 ms, p95 16.8–16.9 ms. It does not measure GPU cost or live simulation throughput.
- Serialized GPU queries summed every render submission, rather than reporting only the last internal frame group. Sixty samples/state gave the following medians; ordering/noise makes the rainy deltas inconclusive.

| State | Focus off GPU ms | Focus on GPU ms | Render passes off → on |
| --- | ---: | ---: | ---: |
| Anime day | 12.71 | 15.93 | 17 → 29 |
| Anime rainy night | 16.84 | 16.06 | 23 → 35 |
| Miniature day | 17.56 | 19.53 | 17 → 29 |
| Miniature rainy night | 22.68 | 19.73 | 23 → 35 |

Per-pass attribution, temporal artifact review and live simulation remain outstanding. The counterbalanced follow-up below does not establish a precise focus cost. Do not tune blur to solve unattributed frame spikes.

## Generated textures: what is actually integrated

Runtime vehicle and train atlases are integrated through their material modules, with live glass, lamps and wet response. Metadata identifies Qwen vehicle elevations and Hunyuan train elevations. Train details are readable, but baked shading/perspective still warrant close-view and day/night review.

The ivory plaster and 512px concrete wall images are development-review assets, not ordinary city wall materials. Ivory plaster is 1254px and has tactile detail but broad mottling and illumination-like relief; it would need a smaller derivative and lighting/scale review. Tencent plaster, steel and timber have visible repeat/perimeter patterns and are rejected as-is. Keep the source references; do not confuse asset availability with successful integration.

Current wall cladding is procedural albedo with derivative-based detail suppression. It has no finish-specific normal/roughness maps. Miniature uses material-class roughness plus wetness; Anime uses toon bands and pigment. This is a real material limitation, but previous fine-detail experiments have not demonstrated a scene-level benefit.

## Rejected experiments and lessons

| Experiment | Evidence / decision | Consequence |
| --- | --- | --- |
| Existing 512px concrete grain | Four matched frontage/interior pairs, both styles, seed0 day13, focus off; shared 2m projection and normalized luminance preserved palettes and source/batch parity. Inspected Anime frontage and Miniature interior showed negligible benefit. Removed. | Does not justify shipping city-wide or generating more grain tiles. Close-range/color-replacement/normal-map use remains untested. |
| Direct-light cloud attenuation | Four views per version; moved cloud attenuation from albedo to direct light. No meaningful scene gain. Cloud phase was not recorded and UI clocks differed, limiting attribution. Restored original code. | A cleaner light model alone did not close the reference gap. |
| Layered bank planting | Four views per version, both styles. Shorter trees and small shrubs reduced presence and read as dots against the same rigid edge. Restored source; removed candidate test. | Coordinate bank cross-section, promenade and planting areas. Do not repeat height/count/card-angle tweaks in the narrow envelope. |
| Street canopy / small planting beds | Two local variants failed ordinary-scene review; larger crowns also raised clearance constraints. Removed. | Planting needs real space and coherent mass, without violating traffic, wires or entrances. |
| Room-bay / window-count variants | Matched ordinary views remained repetitive; helpers/tests removed. | Treat wall divisions, openings, roof edge and service side together. |
| Hip roof, front court, deep-site wings, low-rise apartment eligibility | Local screens failed composition or changed too few visible buildings; not promoted. | Broad neighborhood fidelity is not proven by one isolated family. See frontage design study. |
| Drainage bands, finish roughness/normal, cladding filtering, facade averaging, foliage-card orientation, global night fill, foothill boxes, water filtering/glints | Rejected or deferred after inadequate visible benefit or regression. | Do not revive as a sequence of small parameter variants without a new causal diagnosis. |

## Next acceptance contract

Use locked concepts and ordinary, unblurred frontage/interior/waterfront views. The dominant gaps remain coherent architectural layering, street proportions and enclosure, meaningful planting, material organization and composed illumination. Improve a complete visible scene relationship before expanding to three seeds, two stages, both styles, weather, motion and lifecycle/performance validation. Freeze or record cloud phase as well as camera, time and exposure for material comparisons. A passing numeric capture check is not proof that the intended subject is visible.

No additional image generation was needed for this assessment. No recent rejected treatment remains in runtime source. Full living-city scope, visual acceptance and browser performance gates remain open.

## Counterbalanced GPU follow-up — 2026-10-16

Previous goal turn was interrupted before changes; this follow-up completed a new measurement. Source hashes remained unchanged throughout. The same paused mature seed0/frontage, 1200×800 balanced WebGPU scene was measured in Chromium 145.0.7632.6. Adapter reports Apple / metal-3; exact device model was not exposed. Each style/weather stratum used focus **off/on/on/off**, with 10 warmup and 30 measured serialized submissions per block: 16 blocks, 480 measured submissions, no page errors. All actual focus/output/wet/preview checks passed.

| State | Off block 1 median ms | On block 2 | On block 3 | Off block 4 |
| --- | ---: | ---: | ---: | ---: |
| Anime day | 32.83 | 38.14 | 26.67 | 22.74 |
| Anime rainy night | 26.48 | 16.32 | 17.63 | 20.25 |
| Miniature day | 19.07 | 20.58 | 19.92 | 18.74 |
| Miniature rainy night | 23.07 | 19.73 | 19.92 | 22.74 |

All blocks consistently recorded 17→29 render passes in daylight and 23→35 in rain. Summed render-query durations still show strong drift in Anime day and reversed rainy deltas in both styles. ABBA ordering alone does not resolve the anomaly. Do not interpret it as blur making the renderer faster, a precise overhead, or delivered frame rate. The next performance investigation must attribute timings to specific passes and validate the query methodology; do not repeat broad aggregate timing runs or tune artistic settings from these totals. Live simulation and temporal visual acceptance remain separate open gates.

Harness, raw report and source hashes remain ignored under `.tools/broad-home-family/focus-gpu-counterbalanced` in the historical checkout. No recordings, screenshots or runtime changes were generated by this check.

### Named-pass attribution — 2026-10-16

A bounded follow-up completed on the same paused mature seed0 Miniature rainy-night view: four ABBA blocks, 10 warmup and 10 measured submissions each. A diagnostic wrapper associated query IDs with renderer object/material and render-target names, then restored the methods. No source files changed; original source hashes and actual state checks passed, with no page errors. The raw report remains local under `.tools/broad-home-family/focus-gpu-attribution`.

Block median summed durations were 22.81 / 19.60 / 19.79 / 22.81 ms. In the first off/on pair, the main scene median was 3.67 ms in both, shadow 1.11 ms and water reflector 1.38 ms. AO changed from 3.74 to 0.33 ms; reflection tracing from 4.03 to 0.66 ms; the first two reflection mip passes from about 4.00 each to 0.66/0.72 ms. Conversely the first DOF CoC and horizontal Gaussian passes each measured about 3.47 ms. Most bloom passes were small. Pass labels locate where timings differ; they do not prove isolated shader execution cost.

This distribution is consistent with scheduling/dependency effects when the graph order changes, but does not establish that cause: raw start/end intervals were not retained, so overlap or wait attribution cannot be checked. Summing pass durations must not be interpreted as end-to-end GPU latency or a focus speedup. Do not reduce SSR, AO or blur quality on this evidence. Stop aggregate profiling iterations; if performance becomes a demonstrated blocker, capture raw intervals and queue completion alongside the production frame timeline on named hardware. Meanwhile the explicit visual/temporal acceptance gates remain the next useful fidelity work.
