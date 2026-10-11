# Incremental face viability and target selection plan

## Scope and reference behavior

This stage follows the resumable face-walk pipeline and precedes `AccessHostPreparation`. It is a pure, deterministic search over a frozen world/face snapshot. It must not mutate roads, parcels, plans, RNG, demand, or history. The current reference is the ignored offline diagnostic `exhaustive_prepared_pages_discover_mature_face_access_candidate` in `engine/src/face_access_exact_tests.rs`, especially its face sampling and ranking at lines 547–614 and target selection at lines 615–650. Keep that exact behavior as a parity oracle before changing any heuristic.

The oracle operates on the seed 87231 save at tick 242,000. It finds 49 faces, then selects the face with the most viable 8 m square centers; ties go to greater face area, then lexicographically smaller `face.identity`. That face has 34 viable centers and 11 ordinary-road boundary hosts. It builds 2,069 ordered access descriptors in 44 pages of at most 48. These are fixture measurements, not guaranteed limits or proof that selecting one face is a good citywide policy.

`build_candidate_context` does not exist under that name in the current source. Its relevant work is `Engine::collision_context` (`engine/src/lib.rs:1681`) plus predicates that continue to read live `Engine` state. A resumable job must freeze all of those inputs rather than calling those helpers against a changing world after a yield.

## Frozen inputs

Capture a versioned input record at a structural revision boundary. Preserve the reference iteration/order contracts:

- Complete `RoadFace` sequence from the prior trace stage, including polygon coordinates, vertex IDs, boundary edge IDs, area, and identity. Face order is the exact order emitted by `FaceWalkSearch` (identity-sorted); do not re-sort it here.
- Ordinary open connected local/arterial roads in `state.roads` order, their endpoint IDs/coordinates, and all nodes in `state.nodes` order. The former maps boundary edge IDs to target hosts; the latter two lists support the oracle's final 20 m exclusion and downstream preparation.
- The collision projection produced by `collision_context`: parcel footprints in parcel order, open-road segments in road order, and pending road-project segments in project order. Preserve legacy and rendered parcel envelopes where `rotation` is absent; `collision_context` intentionally records both.
- River control-point segments and the exact constants/domain limits needed by `parcel_clear_of_river`, `distance_to_river`, and bounds checks. The river test is a minimum over every consecutive river-control-point segment, not a local nearest-segment shortcut.
- Protected-land rectangle, pending interior-plan selected lots, and neighborhood-plan state needed by `interior_plan_allows_parcel` and `neighborhood_corridor_clear_for_parcel`. Preserve `next_segment`, coordinated-block interior, and the existing behavior for missing plan node IDs. Pre-resolve plan segment endpoint lookups into frozen segments while capturing; missing endpoints currently make that segment pass the parcel-corridor predicate.
- A source structural revision/save identity for diagnostics and stale-result handling. If the live revision changes before admission, discard/revalidate the proposed access against current geometry; do not silently apply a frozen decision to a newer world.

For exact parity, frozen arrays should retain source order. Any ID index may accelerate queries only if it preserves the current first-match behavior for duplicate IDs and duplicate IDs are already excluded by valid engine state. Immutable JSON checkpoint payloads should carry schema and caps; runtime-only constructors should be the only way to build trusted job state.

## Work hidden by the current diagnostic

The sampling loop is bounded by face limits from the tracing pipeline, but its nested geometry predicates are not a single work unit. Under the current graph caps, one face can have up to 1,024 boundary vertices and up to 24×24 grid cells. Across 512 faces this can approach 294,912 centers before geometry filtering. The center loop tries two rotations, and `rect_inside_face` invokes point-in-polygon four times per orientation. Each point-in-polygon can visit every polygon edge.

For an accepted inside center, `progressive_frontage_geometry_clear` first calls `oriented_parcel_geometry_clear_with_context` and then `progressive_frontage_center_clear` (`lib.rs:1793–1886`). Preserve its short-circuit order exactly:

1. Validate rectangle and river distance. River distance scans each river segment.
2. Test domain bounds and the protected rectangle.
3. Test every parcel footprint with SAT overlap (`geometry::rects_overlap`), preserving the 2 m gap.
4. Test the frozen interior-access reservations, also with 2 m gap.
5. Test each open road and then each pending road with rectangle-to-segment distance, thresholds 5 m and 7 m.
6. Check neighborhood plan corridor/interior constraints. `neighborhood_corridor_clear_for_parcel` (`lib.rs:4551`) scans the remaining plan segments and may perform node ID lookups; make those visits explicit and bounded.
7. Test center-to-open-road distances against `max(max(width,depth)/2+4,9)` and then center-to-pending-road distances against that value plus 2 m.

The oracle’s own operations also require cursors: polygon bounds min/max scan; row/column cell construction; face sort comparisons; viable-center count; target-clearance scans; target sorting; final all-node and selected-target separation. Do not debit only once per center and hide these loops inside `diagnostic_point_in_face`, `rect_segment_distance`, `point_segment_distance`, iterator chains, `all`, `map`, `collect`, or `sort_by`.

Each fixed-size SAT/segment-distance arithmetic primitive can be charged as one geometry test after freezing its inputs; every scan over a variable-length array (parcel, road, river, plan, polygon, node, selected-target, face) must debit each visited element. Use the existing f32 operation order and epsilon/comparison rules. Do not quantize candidate coordinates or clearance scores.

## Suggested job stages and cursors

Use a versioned `FaceTargetCheckpoint`, with private runtime state and a validated serialized DTO. `advance(budget)` must be a zero-work no-op at budget zero, return `work_used <= budget`, and make no uncharged data-sized clones, sorts, or predicates. Establish frozen-input caps from engine state limits and face-trace caps; reject excess input at job creation/restore rather than truncating it.

1. **Capture/freeze.** Build collision and river/plan projections from the engine at one tick boundary, with work counters for every source entity inspected and every projected entry copied. This setup cannot be hidden in the first `advance`. It may be a distinct capture job or a serialized setup stage. Avoid repeated BTree construction/lookups inside later work. The saved cursor must identify the frozen structural revision.
2. **Face bounds and grid.** For each face in emitted order, scan vertices to calculate bounds, then derive `columns = ceil(width/8).clamp(1,24)` and `rows = ceil(depth/8).clamp(1,24)` exactly as the test. Store row/column cursor and preserve the original coordinate arithmetic.
3. **Center-in-face.** Generate one row-major center at a time. Run the exact boundary-inclusive point-in-polygon arithmetic with an edge cursor. Keep `inside`, prior-edge index, and all f32 intermediates needed to resume without recomputing an unbounded prefix.
4. **Corner containment.** For an inside center, test rotations `0`, then `PI/2`; test four corners in `OrientedRect::corners()` order, each by resumable boundary-inclusive polygon scan. Stop on first outside corner as the oracle does.
5. **Geometry viability.** Apply the ordered checks above with cursors for parcel footprints, interior reservations, open roads, pending roads, river segments, plan segments, and center clearances. Keep the active center and rotation. Count a center once if either rotation passes. Do not append duplicate viable-center entries when both rotations pass.
6. **Per-face ranking.** Once its grid is exhausted, retain `(viable_count, area, identity, face_index)` and move to the next face. Incrementally compare/sort face ranks by viable count descending, area `total_cmp` descending, identity ascending. Exclude zero-viability faces as the oracle does.
7. **Winning-face rescan.** Select only the first ranked face for compatibility, then rescan that face’s grid and geometry predicates to rebuild its viable centers. This bounded second pass avoids storing every face’s candidate centers (worst-case hundreds of thousands) in every checkpoint. If retaining candidates instead, explicitly cap and account for their checkpoint bytes. Exact parity must prove the chosen center sequence matches the original single-pass buffer.
8. **Target score.** For each winning-face viable center in original row-major order, compute clearance as the minimum of (a) distance to every parcel center minus half its width/depth diagonal, clamped at zero, and (b) point-to-segment distance to every open road. The oracle does not include pending roads in this score. Preserve `f32::INFINITY` when both sets are empty and `total_cmp` behavior.
9. **Target ranking and spacing.** Incrementally sort all viable centers by clearance descending, x ascending, z ascending. Then scan in that order; retain only centers at least 20 m from every `state.nodes` item and at least 18 m from already selected targets, stopping at 12. The scan must continue past rejected high-score centers until it has 12 or exhausts the list. A top-12-only heap is not equivalent because rejected targets may require lower-ranked fallbacks.
10. **Publish.** Emit the selected face, viable count, ordered target list, matching ordinary boundary hosts in face boundary-edge order, all nodes for downstream preparation, and frozen parcel footprints. Publish only after the full selection is ready; no partial target list is actionable.

Checkpoint validation must check stage-specific vector lengths/caps, active face/grid indices, polygon edge/corner cursors, geometry-array cursors, rank/sort state, and target scan indices before any indexing or arithmetic. Do not authenticate the entire prior work history by replaying it; structural validation must be bounded by frozen-input caps. Persist the frozen input and cursor if exact resume is required, not just a reference to live engine state. A completed snapshot can be consumed once by the downstream host-preparation stage.

## Runtime attachment and policy limits

Production attachment should start from the captured ordinary graph and feed the existing face pipeline: `RoadFacePreflight` → bridge detection → topology → face walk → this viability/target job → `AccessHostPreparation` → paged descriptors → exact proposal/admission. Reuse the already frozen arrays across these stages where compatible; avoid separately rebuilding collision context for every face or every tick. A result is still a proposal: before reservation/admission, check the selected host/branch and lots against current graph revision, capacity, connectivity, river/protected land, parcel and pending-road reservations. If stale, discard and deterministically recapture/restart at a declared cadence.

Persist the running job state as required by specs 03 save completeness. Advance a fixed number of primitive work units per simulation tick; never choose how many candidates to process based on elapsed wall time. Keep selection and completion at deterministic tick boundaries, and bound simultaneous frozen jobs and serialized payload size. This complements, but does not replace, existing per-tick candidate/route work caps.

The current ranking is a discovery heuristic with a clear starvation risk: it chooses exactly one face by **count of viable 8 m grid centers**, then area, then identity. It does not consider district density deficit, live served-catchment vacancy, demand, accessible frontage, or whether its boundary hosts produce a usable descriptor. A face with many tiny separated sample points can beat one with better compact developable land; a large viability winner can repeatedly starve other faces, and a winner whose hosts yield no admissible project does not fall through to the next face. The at-most-12 targets (six in this fixture) are only spatially distributed inside that winning face, not across districts. Preserve this for baseline parity first. A later policy may evaluate a bounded ranked face window or fall through on exhausted descriptors, but that is a separate deterministic policy experiment with matched-seed evidence and must not silently alter this geometry task.

## Acceptance evidence

- Extract the current ignored diagnostic’s complete frozen output as a golden: all 49 ordered `RoadFace` values; per-face viable counts/ranks; selected face identity; all 34 winning-face centers in their original order; ordered scored centers (clearance and coordinate bit patterns); final targets; hosts; and downstream 2,069 ordered descriptor bits. The current test-only reference omits some intermediate goldens, so add them when implementation begins rather than inferring from final count alone.
- Compare exact outputs against the original synchronous oracle for the mature fixture and focused worlds with no parcels/roads, legacy unrotated parcels, pending road projects, an active interior plan, a coordinated neighborhood plan, river-adjacent candidates, protected-land overlaps, equal-score ties, and centers rejected by node/target spacing.
- Run quotas 1, 7, 16, 48 and irregular partitions; serialize/restore at every advance for small inputs and periodically for the mature fixture. Assert work bounds, completion, exact output/order, and no partial publication.
- Add malformed-checkpoint tests per phase and a stress cap case. Measure maximum checkpoint size and total work; these are bounded-work measurements, not density claims.
- Keep the existing mature diagnostic’s results (34 sites, 11 hosts, 2,069 descriptors, selected two-lot proposal, normal connected construction/replay) as downstream compatibility. Also retain adversarial tests for unsupported nonplanar input and no-access faces.
- After runtime integration, compare save/replay continuation and deterministic admission, ensure stale revisions revalidate, then run matched seeds and overhead/neighborhood review. The face/target search itself does not establish density or visual acceptance.

## Initial implementation evidence

`face-target-reference.json` now records the complete intermediate oracle, with provenance in `face-target-reference-provenance.json`. The mature fixture produces six final targets under the twelve-target cap. The existing ignored diagnostic compares this frozen output unless explicit export is requested. `face_target_containment.rs` supplies shared immutable polygon queries with one charged edge visit per step. Root focused tests pass and the mature grid/corner gate checks44,370 queries per quota1/7/48 with exact predicate parity and sampled JSON resume. Collision viability, face ranking, target scoring/spacing, live capture and production scheduling remain to be implemented.

The frozen viability primitive is now independently checked against the actual mature Engine predicate over9,860 rectangles per quota1/7/48, including complete context JSON restore and sampled query restore. All runs accept242 rectangles with6,052,352 work units. This does not include polygon containment, face selection, or live capture; the next sampler composes those geometry queries in reference order.

`face_target_sampling.rs` now reproduces all49 per-face center lists exactly against the frozen oracle:121 centers and3,079,373 logical work units per run, independent of quotas1/7/48. Root also verified focused small-face JSON resume tests. Face winner selection and winning-center clearance/ranking/spacing remain separate pending composition stages. Shared-context capture and runtime admission remain outside this evidence.

Streaming winner selection and target ranking now have independent mature parity evidence. `FaceSelection` retains only the best sampled centers and compares count, area, then identity without sorting all faces; it matches the selected34 centers at3,079,688 logical units. `TargetRanking` reproduces six final targets at51,852 units, preserving full candidate scanning after spacing rejects. Both pass quotas1/7/48 with sampled JSON resume. Full multi-stage composition and live engine attachment remain outstanding.
