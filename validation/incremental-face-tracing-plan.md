# Incremental road-face tracing plan

## Scope and compatibility contract

`road_faces::trace_faces` is the reference operation. A resumable trace should be a pure job over a frozen node list, edge list, and optional proposed edge; it must not own world mutation, RNG, or selection policy. A convenience `trace_faces` wrapper can remain synchronous by constructing the job and draining it. The incremental job returns either the exact same ordered `Vec<RoadFace>` or the same first `RoadFaceError` as the reference for inputs within current caps. It never exposes partial faces as success.

Preserve input order throughout validation and half-edge creation. Do not sort or truncate input roads to fit a local window: a caller must supply a complete planar window, and current tests deliberately fail closed on the mature graph's crossing geometry. Output order remains the final ascending `RoadFace.identity` order. Preserve the f32 angle, cross, ray, and segment-distance calculations; preserve f64 signed-area accumulation order; preserve all existing epsilon and boundary comparisons.

The source has hard caps (`MAX_NODES=512`, `MAX_EDGES=512`, pairwise edge tests at most 131,072, half-edge walk at most 1,024 steps, adjacency lookup at most 524,288 steps, and at most 512 faces). Work must be charged inside loops, not once per stage or face. Work quotas control deterministic scheduling only; they do not change acceptance thresholds or geometry.

## Job/checkpoint shape

Use a versioned `RoadFaceTraceCheckpoint` with frozen `nodes`, `edges`, `proposed_edge`, `stage`, active cursors, and only the intermediate arrays required by that stage. The job takes ownership of input vectors at construction, so it does not synchronously clone them. Checkpoint serialization is an explicit persistence boundary and may serialize the frozen graph; record-count and encoded-size caps must be checked on restore. Keep fields private and validate stage/payload/cursor relationships before resuming.

Suggested top-level shape:

```rust
struct RoadFaceTraceCheckpoint {
    schema: u8,
    nodes: Vec<RoadFaceNode>,
    edges: Vec<RoadFaceEdge>,
    proposed_edge: Option<RoadFaceEdge>,
    stage: TraceStage,
    state: TraceState,
}

enum TraceStage {
    ValidateNodes,
    CheckCoincidentNodes,
    ValidateEdges,
    CheckPlanarity,
    BuildBridgeAdjacency,
    SortBridgeAdjacency,
    FindBridges,
    BuildHalfEdges,
    SortOutgoing,
    WalkFaces,
    EmitFace,
    SortFaces,
    RejectNestedFaces,
    Done,
    Failed(RoadFaceError),
}
```

`TraceState` should hold stable indices and explicit cursors rather than iterators: node/edge indices; nested pair `(first, second)`; endpoint-lookup cursor; uniqueness cursor; bridge DFS frame stack and `entered`/`low` arrays; current bridge-neighbor sort state; half-edge/outgoing vectors; current node and outgoing-sort merge state; visited flags; current half-edge start/current; reverse-adjacency scan; current walk vertices/edges; repeated-vertex and signed-area cursors; face emission/minimum-vertex/canonical-cycle cursors; stable face-sort merge state; and nested-face pair, direction, polygon-edge, and ray parity cursors. Store the first terminal error and do not continue to later validation stages.

An `advance(work_budget)` must return `work_used <= work_budget`; budget zero is a no-op. Debit one unit for each primitive comparison/visit/copy/append/rotation step, including sort comparisons and shifts, map-like lookups, output-vector construction, and every polygon-edge classification. Small constant arithmetic attached to one charged primitive is fine. Avoid opaque `sort_by`, BTree lookups, whole-polygon predicates, `collect`, `rotate_left`, recursive graph traversal, or face containment helpers inside `advance`; each hides a variable scan or copy. Restore performs full structural validation once, then continues at the exact next cursor. A saved job must not restart a previous prefix.

## Stages and exact ordering

1. **Validate nodes.** First reject over-cap counts and preserve the current empty-input fast path. Visit nodes in original order; reject non-finite coordinates and duplicate IDs in the same order as the current insertion loop. Keep their input indices as the canonical internal indices. `trace_faces` checks duplicate IDs before coincident positions.
2. **Check coincident nodes.** Replay the current nested `(left, right)` loop in lexicographic index order. Charge each distance comparison and fail on the first distance at or below `COINCIDENT_EPSILON`.
3. **Validate edges.** Visit original edges then the optional proposed edge. For each edge preserve check order: self-edge; `from` lookup; `to` lookup; duplicate edge ID; duplicate unordered endpoint pair; finite/nonzero endpoint delta; append indexed endpoint pair. Use a cursor-based bounded ID lookup and uniqueness search (or an equally resumable sorted index); do not put an uncharged `BTreeMap::get`/`BTreeSet::insert` inside a step. A lookup failure must be attributed to the same current edge and endpoint as the reference.
4. **Check planarity.** Compute the same pair count and return `WorkLimit` before testing pairs if it exceeds `MAX_PAIR_TESTS`. Then advance nested edge pairs in exactly the existing order. Shared-endpoint pairs keep the collinear-overlap test and its `cross`, `dot`, and `scale` operation order; nonincident pairs call the same segment-distance arithmetic. Stop on the first `NonPlanar` pair.
5. **Build and sort bridge adjacency.** Build two `(neighbor_index, edge_index)` entries per input edge in edge order. The reference sorts each neighbor list by tuple; perform a stable, resumable sort with charged comparisons/moves. This ordering is essential because it determines Tarjan DFS traversal and bridge discovery. Build/sort limits are small, but still must yield rather than run synchronously.
6. **Find graph bridges.** Replace recursive `graph_bridge_edges` with an explicit DFS frame stack that has the same recursive semantics. Start roots in ascending node index, visit each sorted neighbor in order, skip only the exact parent edge, assign `entered` and `low` at entry, propagate child `low` at return, and mark an edge iff `low[child] > entered[parent]`. For already-entered neighbors, use `min(low[node], entered[next])`, matching the current function. Charge every neighbor inspection and stack enter/return. The 512-node cap bounds stack depth; do not recurse on the simulation stack.
7. **Build half-edges.** Visit indexed edges in input order, skip bridges, and create forward then reverse records using the current `atan2(dz, dx)` formulas. Append to outgoing lists and `half_edges` in exactly the present order, and record each edge's pair of half-edge indices. Charge each append/copy.
8. **Sort outgoing adjacency.** Sort each node's outgoing half-edges by `angle.total_cmp`, then destination node ID, then original edge ID. Use a resumable stable merge/insertion algorithm and charge each comparator and move. Do not recompute angles or change tie breaks. The current face walk's clockwise predecessor depends on this exact order.
9. **Walk directed faces.** Start half-edges in ascending current index. For each walk, append `from` vertex and edge index, then scan the destination's outgoing list from position zero until the reverse half-edge is found, charging every lookup; this reproduces `adjacency_search_steps` and `InvalidEmbedding` precedence. Choose the immediately clockwise predecessor exactly as today. Charge every half-edge visit and retain `visited`, `current`, and walk vectors across yields. Exceeding the existing global half-edge or adjacency caps returns `WorkLimit`; encountering a previously visited non-start half-edge returns `InvalidEmbedding`.
10. **Validate and emit each walk.** Preserve the sequence: reject fewer than three vertices or repeated vertex; calculate signed area in the existing f64 accumulation order; ignore non-finite, exterior/negative, or area `<= MIN_FACE_AREA`; enforce `MAX_OUTPUT_FACES`; then construct IDs, polygon, and boundary IDs in walk order. Replace `has_repeated_vertex`'s hidden set scan with a charged mark-array check. Find the first minimum node ID, rotate the three arrays by cursor-driven copying, then compute `canonical_cycle` by evaluating forward and reversed rotations in the exact current order and using lexicographic comparison with the current best. Build `contains_proposed_edge` with a charged scan. Only append a face after its complete data is ready.
11. **Sort faces.** Stable-sort by `identity` using an incremental comparator that charges each compared ID and each merge/move. This is required for exact output ordering and cannot be a single library `sort_by` call.
12. **Reject nested disconnected cycles.** Replay the current sorted face-pair loop. For each pair, test the first polygon's first point in the second polygon, then (only if false) the reverse. Preserve boundary exclusion: for each polygon edge, first run the exact point-to-segment distance comparison against `COINCIDENT_EPSILON`, then the crossing parity test. Keep face-pair, direction, edge, and `inside` cursors. The first strict containment returns `NestedFacesUnsupported`; otherwise finish with the sorted face list.

The nested check has no explicit source cap today, but it is structurally bounded by the preceding caps: total bounded-face perimeter uses at most 1,024 half-edges, and at most 511 other faces are tested for each face edge in the conservative bound, or 523,264 polygon-edge checks. Charge each check; if a separately chosen lower work ceiling is introduced, document that it can return `WorkLimit` on an input the synchronous function previously completed. Prefer preserving the full current cap-derived bound.

## Hidden work in the current function

The obvious loops already have several explicit ceilings, but these operations are synchronous and need resumable implementations too:

- copying edges into `all_edges`; node-ID map insertion and endpoint map lookup; edge-ID/pair set insertion;
- pairwise coincident-node checks and edge planarity checks;
- bridge adjacency allocation/population/sorting, recursive Tarjan DFS, and bridge-set insertion;
- half-edge/outgoing construction and per-node `sort_by`;
- a linear reverse-half-edge search inside each walk (currently separately counted), plus `Vec` growth/copies;
- `has_repeated_vertex` set insertion, full signed-area scan, three output `collect`s, minimum search and `rotate_left`;
- `canonical_cycle`, which makes forward/reverse copies, checks every rotation, and may lexicographically compare long candidates;
- proposed-edge membership scan and final face sort;
- nested face-pair checks, each of which may scan a full polygon through `point_strictly_inside` and segment-distance calculations.

The current `graph_bridge_edges` recursion itself is bounded to 512 nodes, but is not resumable; the explicit frame stack both bounds stack usage and exposes each neighbor visit to the quota. `point_strictly_inside`, `canonical_cycle`, and sorting helpers should remain shared arithmetic/comparison helpers only if their incremental state is represented externally; a one-shot helper cannot be called from `advance` and counted as one unit.

## Tests and acceptance

Keep the current synchronous tests as a reference until the state machine is proven. Add a test-only one-shot frozen oracle (or retain the old body under a test-only name) and compare exact `Result<Vec<RoadFace>, RoadFaceError>` equality against the incremental wrapper for:

- single rectangle, irregular rotated face, adjacent rectangles, proposed edge marking;
- dangling exterior/interior branches and disconnected components;
- crossing, coincident, duplicate, missing-node, invalid edge, and nested disconnected cycles;
- max node/edge caps and mature non-planar fixture, checking exact first error;
- quota 1, 7, 48, and irregular budgets, with JSON checkpoint serialize/restore after every `advance` on small graphs; include budget zero and already-done calls;
- shuffled/reversed input where the existing canonical-output test guarantees equality, while preserving original-order error precedence on malformed input;
- forged checkpoints for every stage: out-of-range indices, wrong vector lengths, invalid parent/DFS stack relationships, malformed half-edge references, inconsistent visited/walk state, invalid sort cursors, impossible nested-check cursors, and terminal state with partial result;
- a capped stress graph that drains to completion under repeated small budgets and demonstrates `work_used` never exceeds the requested amount.

The mature exact-discovery diagnostic in `face_access_exact_tests.rs` calls `trace_faces` on a restored save and currently asserts a non-planar crossing in a separate mature graph test (`road_faces.rs` test `mature_connected_ordinary_road_graph_reports_unsupported_crossing`). Preserve both outcomes. The exact mature-face diagnostic compares downstream descriptor pages; once tracing is incremental, also capture and compare the complete `RoadFace` value sequence before using it for host preparation.

This plan only makes face discovery schedulable and replayable. It does not select a better graph window, repair crossings, claim density improvement, or change admission. The input graph still must be complete and planar under existing straight-line rules; unsupported geometry remains a no-op for growth.

## Implementation status

The separate `road_face_preflight` stage now passes root-verified focused tests and mature indexed-edge parity with deterministic quotas and JSON resume. Its restore validates structural state without replaying the full completed prefix. `road_face_bridges` now passes focused checks and the combined mature gate after repair of a partial insertion-sort restore defect:44 reference graph bridges,2,022 work units independent of quota, with JSON restore after every advance. Resumable half-edge construction/sorting/linking now passes focused checks and mature parity:2,427 units independent of quota and all49 mature faces recovered exactly by a diagnostic synchronous successor walk. The resumable walk, canonicalization, face sort and nested-containment stage now passes the mature complete49-face output comparison at quotas1/7/48, using19,630 work units per run. Live graph capture, target sampling, combined-job persistence/setup accounting and engine scheduling remain outstanding. None of these modules is connected to live road generation. Half-edge, walk, canonicalization and nested-face behavior have isolated parity evidence; composing them into one persisted job with budgeted handoffs is the next implementation step. Target selection and live scheduling remain outstanding requirements.
