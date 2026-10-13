# Authoritative elevated-road curves — 2026-10-11

## Deficiency and intervention

The user's crop exposed wedges between independent straight road slabs. A first
renderer-only repair closed those seams, but inferred tangents from realized
neighbors and allowed only 2 m deviation. User references then established a
larger target: flowing elevated routes embedded in dense fabric, with ramps and
traffic. The audit found lots were admitted without elevated-road clearance.

This checkpoint makes cubic controls part of the complete Rust master plan and
its digest. Realized roads publish the same controls from first construction to
maturity. Rendering consumes them directly for swept decks, barriers, medians,
paint, piers and lights. Progress clips arc length. Separate 11 m/20 m crossing
routes keep distinct nodes. Curve clearance is checked before final lot admission
using 32 chord capsules, 10.8 m half-width and a conservative second-derivative
sampling-error bound. The renderer does not determine parcel occupancy.

A regression witness found a >60-degree bend in seed 41 that the temporary helper
left unsmoothed. Broader one-third-chord handles replace the two-metre cap; the
planner now reserves the resulting envelope. Dense sampling tests independently
check lot clearance and shared endpoint directions on five seeds. Reversals over
120 degrees are not supported by this simple corridor helper; planned ramp/merge
geometry needs a separate construction.

## Density cost

Same native geography fixtures, same seeds, before versus after corridor admission:

| Seed | Lots before | Lots after | Parcel coverage before | After |
| --- | ---: | ---: | ---: | ---: |
| 0 | 1834 | 1747 | .450 | .423 |
| 7 | 1976 | 1893 | .441 | .420 |
| 41 | 1848 | 1769 | .433 | .410 |
| 2026 | 1942 | 1856 | .452 | .425 |
| 87231 | 1936 | 1849 | .460 | .432 |

This is parcel area over the plan domain, **not rendered-footprint acceptance**.
All five retain the existing >.36 engine coverage check. Discarding conflicting
lots is an interim admission rule; partitioning blocks around the curved corridor
should recover usable edge lots without restoring collisions.

## Verification

- Native release suite: 410 passed, 0 failed, 53 ignored. See
  [native log](expressway-curves-native.txt).
- Plan tests include conservative clearance, shared tangents, control digest
  sensitivity, fixed controls while new sections build, and original/restored
  construction continuation.
- Native mature seed 87231 at tick 160000 restores in rebuilt WASM with identical
  controls for all 16 elevated sections. Original/restored continuation matches
  for 800 ticks. Minimum sampled radius 22.499 m exceeds the deck half-width;
  this radius witness covers that checkpoint, not every seed. See
  [audit result](expressway-curves-audit.json) and run
  `node scripts/audit-expressway-curves.mjs <native-snapshot.json>` with its
  `.save.json` companion.
- Focused frontend geometry: 7 passed. TypeScript and production build pass with
  the existing large-chunk warning.
- Default frontend suite on the final WASM: 296 passed, 3 timeouts (lowrise, tower,
  movement), [full log](expressway-curves-web.txt). An isolated one-worker rerun
  of lowrise/midrise/tower passes 25 tests at unchanged limits; it does not turn
  the default suite green. Movement timeout remains unresolved.
- Browser normal import/scheduling reaches 0 buildings queued and 0 neighborhood
  batches queued. Paused seed 87231, tick 161693, 1849 buildings; 09:00 light
  preview, clear weather (intensity 0), DOF off, 50 mm, WebGPU/high detail. Final style comparison uses an
  Explore hold from the overview framing. Exact camera coordinates were not
  exported, so these are same-session comparisons, not an automated camera fixture.
- [Miniature](screenshots/authoritative-curves/miniature.jpg),
  [Anime](screenshots/authoritative-curves/anime.jpg), and
  [load diagnostics](screenshots/authoritative-curves/diagnostics.jpg).
  The diagnostics view precedes the final camera hold. Counters are observations,
  not controlled hardware performance measurements.

## Open work and decision

Keep the correction: it removes visible seams and building/deck conflicts and
passes the native invariants without flattening transport layers. Reject future
variants that fold the inner deck, break grade separation, alter completed curves
as construction advances, or lower coverage without a justified visual benefit.

The corridor still follows the arterial skeleton. The photographic target needs
longer route-level composition, a branching ramp, graded connections, believable
merges, slimmer structural details and purposeful expressway traffic. Routing and
construction costs still use endpoint-graph lengths. No ramp, traffic, motion,
all-seed curvature, artistic or performance acceptance is claimed here. Overall
concept fidelity, materials and architectural variety remain open.

Current master-plan saves prior to this change reject via digest; no migration.
Use `.tools/fidelity-review/curves-87231.json` for the local fresh checkpoint.
