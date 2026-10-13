# River tessellation experiment — rejected 2026-10-11

Hypothesis: bank-only vertex rows flatten the cross-river displacement and cause
the broad angular patches seen in the fixed waterfront view. A temporary helper
replaced those rows with a 2 m grid, preserving all water shaders, reflection,
rain simulation and UV mappings. Bounds were 128 points, 1024 m per segment,
64 m half-width, positive z progression and 100,000 total cells.

Primary review corrected the proposed test to compare bank interpolation at the
same x coordinate as each fine triangle centroid; the earlier center-only
comparison was not valid evidence. Four focused tests passed, covering upward
winding, exact bend seams, bounded allocation and cross-river sine interpolation
below 3 mm at times 0 and 8000. Production build passed with the existing bundle
warning. These checks establish geometry behavior, not artistic improvement.

## Matched browser review

Normal UI import of `.tools/fidelity-review/fair-routes-87231.json`: seed 87231,
tick 160000, 1839 buildings, clear weather, paused, Anime, lighting preview 18:00,
50 mm lens, DOF off, WebGPU/high detail. Apply **Hold waterfront view**. Both
geometry queues reached zero through normal scheduling. No captured warning or
error logs. The new tab initially had a different viewport; resize settled before
the final candidate capture at 1099 × 1071, matching the baseline.

- [Baseline](screenshots/waterfront-preset/first.jpg)
- [Candidate](screenshots/water-mesh/candidate.jpg)
- [Queue diagnostics before viewport matching](screenshots/water-mesh/diagnostics.jpg)

The broad angular patches and repeated fine bands remain in the same places.
There is no convincing improvement at this observation scale. The six-segment
visible river, including its scenic tails, would use 46,530 vertices and 89,856
triangles, versus 396 vertices and 384 triangles before. Estimated position,
normal, UV and Uint16 index arrays total 2,028,096 bytes; this is source buffer
arithmetic, not measured GPU memory or frame cost.

## Decision

Reverted runtime integration and removed the experimental helper/tests. Runtime
water is again unchanged from HEAD. Do not retain extra geometry solely because
its analytic interpolation is more accurate. This experiment falsifies the
proposed explanation for the visible patches in this fixed view; it does not
prove that tessellation cannot improve other views or moving silhouettes.

Before another water intervention, isolate received shadows and planar reflection
in this same fixed view, one pass at a time. Identify the source of the angular
patterns before tuning normals, reflection gains or tessellation again. Complete
concept fidelity, other styles, motion and hardware performance remain open.
