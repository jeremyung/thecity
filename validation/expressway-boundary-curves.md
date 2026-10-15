# Regional boundary curve correction — 2026-10-11

The ten-seed geometry audit found a real fold risk missed by the original seed
87231 review. On seed 1234, road 2262 was a 10 m outer segment, from
(-382, 44.143692) to (-392, 44.143692). Resetting that segment to a horizontal
bearing made the inherited tangent bend sharply. Its minimum sampled curvature
radius was 10.7131 m, below the 10.8 m reserved corridor half-width.

The planner now extrapolates the adjacent interior bearing to the regional
boundary for each of the four route ends. Interior nodes and their corridor
layout remain intact. Endpoint lateral coordinates remain within the finite
boundary. Curves, clearance and traffic continue to use the same plan geometry.
No renderer-only seam cover or acceptance threshold reduction was introduced.

Acceptance for this correction: all ten corpus seeds retain 16 deck segments,
finite bounded endpoints, continuous tangents and no sampled inner-edge folds;
the known failing boundary tail is straight rather than hooked. Abandon this
approach if it changes interior topology or invalidates clearance/replay.

## Evidence

- Full native release suite: 418 passed, 0 failed, 53 ignored.
- New native regression checks boundary-tail collinearity and finite bounded
  endpoints over the ten fixed seeds.
- Rebuilt WASM corpus: all ten seeds have 16 realized decks, zero sampled
  degenerate tangents, and sampled radius above 10.8 m. Seed 1234 improves to
  23.33799 m; the smallest radius across this corpus is still 11.46012 m.
- Three focused frontend files: four tests passed, including actual saved
  regional trips and their geometry/replay contract.
- Seed 1234 native snapshot at tick 30000 restores in WASM with identical deck
  controls and exact 800-tick live/restored continuation.

Files: `expressway-corpus-before.json`, `expressway-corpus.json`,
`expressway-boundary-native.txt`, `expressway-boundary-native-wasm.json`.
Run `node scripts/audit-expressway-corpus.mjs` after rebuilding WASM; it writes
its report and exits nonzero on missing decks, degenerate tangents or folds.

This is sampled corpus evidence, not a proof over every seed. Broad-radius
expressway realism and visual acceptance remain open. Some interior curves are
still tighter than the photographic reference. Do not extend the highway
feature indefinitely before returning to the larger architectural, material,
lighting and atmosphere gaps.

The altered plan digest intentionally rejects preceding checkpoints. Current
reproducible local witness: `.tools/fidelity-review/boundary-1234.json`.
Previous curve/traffic screenshot reports retain historical validity, but their
saved cities need regeneration before using the latest engine. No migrations.
# Follow-up join verification — 2026-10-11

The latest elevated-waterway reference requires broad, continuous routes, not
merely closed seams. A new renderer regression checks emitted Float32 deck,
median and barrier endpoint sections on adjacent cubics, including reversed
road storage. All four corners agree within 0.0001 m. Focused road-curve,
expressway-furniture and elevation suites pass 9 tests across 3 files.

The corpus audit previously skipped shared starts/ends and reported tangent
errors without making them failures. It now handles both storage orientations
and fails above 0.01 degrees. Rerunning the current compiled WASM on the ten
existing seeds passes: worst endpoint disagreement is about 0.000135 degrees,
with no sampled inner-deck folds. Updated results are in
[the corpus report](expressway-corpus.json).

This does not demonstrate broad-curve fidelity: the minimum sampled radius is
still 11.460 m on seed 41. Route-scale curvature, branching ramps and gradual
merges remain unfinished. No renderer or simulation behavior changed in this
follow-up, and no new visual acceptance is claimed.
