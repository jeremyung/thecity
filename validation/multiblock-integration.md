# Successive-block integration — 2026-10-09

The rebuilt public WASM supports successive four-edge neighborhood plans. Completed street IDs persist separately from the one active corridor reservation, so retiring a plan does not change its future parcel rules. Restore validates plan shape before retirement and migrates completed legacy plans without moving buildings, roads, or residents.

The initial candidate exposed an exhausted infill quota that prevented further road planning. Growth now clears that priority only when no eligible unreserved lot remains. Offline tower reservations do not count as eligible infill. A usable ordinary lot still takes priority. No timing cooldown was retained.

## Same-target evidence

Both sides below were generated through the public WASM API. Parcel envelope and floor area are proxies, not actual rendered footprints or net developable coverage.

| Seed / horizon | Activity baseline | Multi-block candidate |
| --- | ---: | ---: |
| 87231, rate16, tick242000: buildings | 405 | 429 |
| Roads | 127 | 142 |
| Total road centerline | 8807 m | 9791 m |
| Nominal parcel envelopes | 70574 m² | 83318 m² |
| Gross envelope coverage | 11.027% | 13.018% |
| Nominal floor area | 248812 m² | 283648 m² |
| Housing occupants | 4367 | 4356 |
| 0, rate60, tick72000: buildings | 465 | 446 |
| Roads | 128 | 146 |
| Total road centerline | 9285 m | 10233 m |
| Nominal parcel envelopes | 80742 m² | 87318 m² |
| Population | 4791 | 4948 |

Seed87231 has 36 historic block-road IDs and 110 corresponding frontage parcels. Seed0 has 40 historic IDs. Road counts are not a general proof of completed-block count: historic IDs may include an active plan or shared streets. The saved 87231 candidate has no active plan.

The change increases coverage at an infrastructure cost; seed0 has fewer buildings on larger parcels. These results are not a universal density improvement or art acceptance. Native and WASM trajectories still differ, so native numbers are not substituted here.

## Verification

- Agent full native release suite:48 passed. Root independently reran the stale-quota regression.
- Root full JS/WASM suite:119 passed across21 files. An initial simultaneous generation/build run exceeded one default5s test timeout; the isolated WASM suite and subsequent complete suite passed without changing that timeout.
- Production build passes; existing large-bundle warning remains (about1.237MB JS /357KB gzip).
- Real legacy block migration test checks retained historic street IDs, unchanged geometry/history/random state, canonical second restore, and2000-tick twin continuation.
- Candidate generator independently verifies canonical restore and2000-tick twin continuation. Hashes live beside the candidate save.
- Browser WebGPU import showed429 buildings, paused at08:00, Miniature, depth of field off. Capture:`captures/multiblock-candidate-429.png`.

## Visual review

Latest shared-edge candidate: [803-building ready overview](captures/shared-edge-803-ready-overview.png), [diagnostics](captures/shared-edge-803-ready-diagnostics.png). Fresh seed87231/rate16/tick242000, paused08:00 Miniature DOFoff, zero geometry queues. Shared streets now occur naturally and selected lots survive save/resume; two-seed density/road-cost outcomes are mixed. This is functional reuse evidence, not accepted art or a general efficiency gain. See the opening [audit checkpoint](spec-audit.md).

Latest block-cadence checkpoint: [799-building ready overview](captures/block-cadence-799-ready-overview.png), [queue diagnostics](captures/block-cadence-799-ready-diagnostics.png). Fresh seed87231 at242000 ticks, rate16; paused08:00 Miniature, DOFoff, both geometry queues zero. This is the current candidate. Compared with fresh710/207, it has799 buildings/197 roads and24.22% nominal gross envelope coverage. The scheduling and rotated-corner road-clearance changes are both included. More continuous blocks do not close the architecture, open-ground, or atmosphere art gaps. See the opening [audit checkpoint](spec-audit.md).

Latest road-growth checkpoint (2026-10-10): [754-building ready overview](captures/road-benefit-754-ready-overview.png) and [queue diagnostics](captures/road-benefit-754-ready-diagnostics.png), paused08:00 Miniature, DOFoff. Both renderer queues were zero. Matched64000-tick continuation grows646/168 to754/202 versus an unchanged old-engine baseline; nominal gross envelope coverage rises17.81% to20.54%. This is continued growth, not a fresh-generation comparison or an art pass. Repeated architecture and open ground remain. See the opening [audit checkpoint](spec-audit.md) for provenance, validation, and performance limits.

Latest fully loaded capture: [646-building overview](captures/coalesced-646-ready-overview.png), with [zero-queue diagnostics](captures/coalesced-646-ready-diagnostics.png), paused08:00 Miniature DOFoff. Earlier screenshots did not establish loading completion and cannot prove missing simulation buildings. Saved-world audit confirms all coordinated blocks have ten built interior lots. The complete view still has excessive open ground and repeated building forms; art acceptance remains open.

Building-selection comparison (2026-10-10): [new646-building city](captures/bounded-selection-646-overview.png) and [prior678-building city](captures/before-bounded-selection-678-overview.png), both fresh seed87231 at242000 ticks, paused08:00, Miniature, DOFoff. Overview auto-framing differs; no fixed-camera A/B claim. Gross nominal envelope coverage falls19.07%→17.81%, floor proxy rises slightly, and roads fall186→168. Both remain below the artistic target. Selection is retained for bounded scoring and tested infill preference, not as visual-density acceptance.

Latest scenic-terrain check (2026-10-10): [Miniature 08:00, 658-building controlled fixture](captures/seeded-backdrop-miniature-0800.png). Seeded hills replace the flat outer backdrop; corrected distant rings no longer cut through those hills in this view. City geometry is unchanged. This is not acceptance of density, architectural variety, or authoritative geography.

Lighting follow-up (2026-10-10) uses the same 658-building save: [clear-air-miniature-0800.png](captures/clear-air-miniature-0800.png) and [clear-air-anime-1700.png](captures/clear-air-anime-1700.png). Reduced dry-weather haze and daytime ambient fill make façade orientation and shadows more legible. Exposed flat background terrain remains a substantial art deficit. This is not an accepted final look. A separate newly generated city reaches 678 buildings/186 roads; its data and tradeoffs are in the opening audit checkpoint, and it has not yet been visually reviewed.

Latest gap-aware frontage follow-up (2026-10-10): `gap-infill-candidate.json` reaches 658 buildings on the same 168 roads after a 64000-tick continuation from the 638-building fixture. The prior binary stays at 638. Gross nominal envelope coverage is 18.91984%. [gap-infill-658-overview.png](captures/gap-infill-658-overview.png) shows the paused import in Miniature at 08:00 lighting preview, depth of field disabled. This remains a modest gain and fails the art target. The opening [audit checkpoint](spec-audit.md) records the implementation, bounded work, hashes, and verification.

Earlier road-aligned infill follow-up (2026-10-10): continuing the 629-building city for 64000 ticks yields 638 buildings on the same 168 roads; the old binary remained at 629. Nominal envelope coverage rises to 18.49195% gross. [infill-oriented-638-overview.png](captures/infill-oriented-638-overview.png) shows the imported city, paused in Miniature with 08:00 lighting preview and depth of field disabled. Browser details confirm 638 buildings. The visual gain is small: broad gaps, repeated massing, and pale lighting remain. This does not pass the art gate.

Coordinated-block follow-up: `mature-world.blocks-candidate.json` has 629 buildings /168 roads, including 130 explicitly oriented block parcels (all developed). Gross nominal envelope coverage is 18.2557%. [blocks-candidate-629-overview.png](captures/blocks-candidate-629-overview.png) shows the 08:00 Miniature overview with depth of field disabled. This adds organized interiors but does not fix the scene's generic architecture, pale light, or large remaining gaps. See the opening audit checkpoint for current verification and limitations. Older candidate comparisons below are retained as historical evidence.

Earlier geometry/admission integration: `mature-world.geometry-candidate.json`, 582 buildings /172 roads, with 18.0734% nominal envelope coverage. See [geometry-candidate-582-overview.png](captures/geometry-candidate-582-overview.png) and the opening [audit checkpoint](spec-audit.md) for validation and limitations. This still fails artistic acceptance; counts alone do not close the density gap. The bridge candidate below remains a historical comparison.

Earlier bridge candidate: `mature-world.bridge-candidate.json`, seed 87231, rate 16, tick 242000, generated through public WASM. Its provenance records 580 buildings, 170 roads, 359 housing sites, 221 commercial sites, canonical restore, and 2000-tick twin continuation. `density-bridge-candidate.json` reports 116478 m² nominal parcel envelopes, or 18.1997% gross coverage; this is not rendered footprint or net developable coverage.

The browser capture [bridge-candidate-580-overview.png](captures/bridge-candidate-580-overview.png) shows development on both riverbanks at 08:00 in Miniature with depth of field disabled. Extent and continuity improved, but excessive green gaps, repeated façades, isolated towers, and pale lighting still fail the Tokyo reference. This is not a controlled A/B or performance benchmark. The candidate remains unpromoted; the established review fixture is unchanged. This save predates the subsequent bus-demand selection repair and does not prove that repair is integrated.

The procedural-generation proposals following this review are recorded in [the world-growth specification](../specs/02-world-growth.md#procedural-generation-research-and-proposed-next-work). They are future work, not implementation evidence.

The screenshot still fails the Tokyo reference: open lawns separate too many buildings, the two40-floor towers dominate, and façades/light remain generic. Its camera differs from the earlier405 capture, so it is not a controlled visual A/B. The established review fixture remains unchanged.

Selected commercial midrises now carry vertical tenant blades. One shared128×4096 atlas holds eight authored signs, with shared Anime/Miniature materials and the existing night/shop-light uniforms. The texture is owned and disposed by ArtMaterials. Geometry stays within the parcel apron and omits construction and explicitly inactive businesses. The architecture review offers an office camera and controlled night lighting. Day Anime and night Miniature captures confirm glyph proportions and illumination; these specimen views do not establish whole-city beauty or performance.
