# Exhausted-face rejection audit — 2026-10-10

This test-only diagnostic explains why one face in the mature continuation fails
current straight-branch planning. It does not change simulation or rendering.
Machine-readable evidence: [exhausted-face-rejections.json](exhausted-face-rejections.json).

## Result

At tick 272,000, target face identity is
`[2656,2657,2772,2773,3131,3010,3011,2895]`. Fresh discovery returns 2,440
candidates. The exact proposal validator accepts none:

| Outcome | Candidates |
| --- | ---: |
| Branch conflict with reserved space or parcels | 2,401 |
| Branch road conflict | 22 |
| Endpoint collision | 10 |
| Insufficient lots | 7 |

An independent spatial classification of the 2,401 branch conflicts finds existing
parcel footprints in every case, with no reserved-lot-only cases. Probes use the
same `rect_segment_distance < 5.0` clearance as validation. Prefix counts are
cumulative, cover only those 2,401 candidates, and clip each prefix to ray length:

| Tested prefix length | Candidates already conflicting with a parcel |
| --- | ---: |
| 0 m (start point) | 0 |
| 5 m | 1,597 |
| 10 m | 1,782 |
| 20 m | 2,329 |

Thus 72 parcel-conflicting candidates have a clear first 20 m but hit parcels
farther along the straight route. This does **not** prove a bent route can fit,
connect safely or serve useful lots. It disproves the stronger interpretation
that every sampled entrance is itself blocked. Multiple rays can share one
entrance, so these are candidate counts, not unique entrances or parcels.

Discovery consumed 4,839,936 accounted work units; proposal validation consumed
2,511,626. Offline prefix probes made 8,928,052 rectangle-distance checks outside
those job counters. This diagnostic must not be placed inside a live tick.
Its input and candidate counts are bounded by existing capture/discovery limits;
it additionally fails after 6 million discovery or 100 million validator work units.
Final native release audit passed (1 test, 361 filtered out) in 0.89 seconds,
including prefix consistency assertions. This is not a browser performance claim.

## Reproduction

Input is the exact final city from the committed fragmented-frontage browser
profile. The temporary save is intentionally not committed. Regenerate it with
production WASM built from `e78c9a8`, Vite running, and:

```sh
LIVE_TICK_COUNT=30000 \
LIVE_TICK_REPORT_PATH=/tmp/thecity-density-source-profile.json \
LIVE_TICK_FINAL_SAVE_PATH=/tmp/thecity-fragmented-final.json \
LIVE_TICK_TRANSITION_SAMPLE_EVERY=20 \
LIVE_TICK_OVERALL_TIMEOUT_MS=180000 \
node scripts/profile-live-ticks.mjs
```

Expected save SHA-256 (including its trailing newline):
`5207f37c688888d9e6d49e293ba07a626e9bc5faba72019b86ab21ea8e77e9e3`.
Then run from the repository root:

```sh
DENSITY_AUDIT_SAVE=/tmp/thecity-fragmented-final.json \
DENSITY_AUDIT_REPORT=../validation/exhausted-face-rejections.json \
node scripts/test-engine.mjs --release diagnose_one_exhausted_face -- --ignored
```

The test runs from `engine/`, hence the relative report path. It asserts the
selected face identity rather than silently auditing a different face. It uses
saved exhaustion history to isolate the target and captures fresh geometry for
both discovery and validation. Its first 12 sample outcomes are illustrative;
the totals cover every candidate.

## Next implementation boundary

Investigate a bounded connector with an intermediate turn against this exact
geometry. First require an offline witness that checks every segment against
parcel envelopes, protected land, pending reservations, other roads, face bounds,
and endpoint rules, and supplies at least two fully viable connected lots. A
clear segment prefix alone is insufficient. Keep the existing 5 m clearance.
If a witness exists, production support requires an explicit versioned polyline
proposal, quota-stepped validation, atomic admission, connected construction order,
and save/resume for every segment. Do not pass a bent path through the current
single-segment proposal contract or draw roads absent from the simulation.

For future neighborhoods, coordinated spine reservations already exist. Earlier
road-opening access and arbitrary spine preference had negative or mixed measured
coverage; see [prior experiments](early-reservation-gap-diagnosis.md). Neither is
justified as a production fix by this report. This one-face result does not
establish city-wide accessibility, density, or artistic acceptance.
