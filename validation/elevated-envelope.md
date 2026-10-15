# Continuous expressway reservation

The segment-by-segment subtraction trial failed because it split usable block
frontage into small pieces. The second approach groups contiguous curve samples
into routes and clips each block against the route's offset tangent planes,
producing at most one convex piece per side of each route. This is a conservative
local envelope, not an exact polygon union. Expressways now exist before lot
partitioning, and new cut edges carry no street access labels. The existing final
curve clearance check remains authoritative.

| Seed | Prior lots | New lots | Prior footprint m² | New footprint m² | Late elevated rejections before → after |
| --- | ---: | ---: | ---: | ---: | --- |
|87231|1850|1861|236948|242349|78 → 21|
|0|1889|1924|234006|236552|106 → 24|
|5|2025|2072|233394|241112|99 → 30|

Footprint gains are approximately2.3%,1.1%,3.3%. Lot counts are supporting evidence,
not acceptance. Remaining rejections show the envelope does not replace the final
clearance guard. Street access, nonoverlap, density and corridor tests passed in
the13-test planner set. An additional test verifies that100collinear samples
produce two regions, preserve original frontage labels, mark new edges inaccessible,
and retain the expected area. No density or clearance thresholds were relaxed.

The production WASM build succeeded. Runtime checks and the initial browser
comparisons are recorded below; broad visual acceptance remains incomplete. Existing saved
plans change identity; obsolete-save compatibility remains out of scope.

Artifacts are under .tools/elevated-envelope; logs use elevated-envelope-*.

Runtime validation completed: six ordinary WASM growth cases (three seeds at
10000/83000ticks), each with100 exact-save continuation steps after restore;
two seeds covering building/road/rail/expressway construction replay and invalid
mapping rejection. Exact native/WASM plan digests match allthree seeds; exported
river-street conflicts are zero. These checks do not prove visual acceptance or
full simulation cross-target replay. The subsequent full-suite result is recorded below.

## First matched browser review

Loaded production WASM and seed0tick83000 save. Miniature,High,DOFoff,live17:00,
35mm,1280x720,position[430,450,130]target[150,0,-150]. Verified exact pose/tick and
all render queues zero before screenshots/elevated-envelope/seed0-overview-miniature.png.
Comparison is screenshots/curved-rail/seed0-mature-overview-miniature.png.
The regeneration changes building identities and construction history; this is
not an isolated same-world mesh edit.

Judgment: some additional development fits alongside elevated corridors, but the
overall composition change is modest. Wide decks, regular rows, rail-side grass
and abrupt city limits still dominate. Retain as incremental layout progress;
do not claim reference fidelity or multi-seed visual acceptance. Tall-building
placement also changes, so silhouette differences cannot be attributed solely to
recovered footprint. Further checks should include remaining seeds and street views.

Full native release suite:483passed,5failed,56ignored,72.72seconds. Failed names
exactly match the prior full-suite set; no new failures. Those five remain unresolved.
Ready browser UI:888draws,13,603,072triangles,39textures;73fps,p95 15.8ms. Uncontrolled
observation, not named-hardware performance acceptance. Higher triangle count
than prior seed0 snapshot remains a resource concern even with fewer draws.

## Waterfront comparison and stop decision

Captured seed87231, tick83000, Miniature, High, DOF off, live17:00, 50mm,
1280×720, position[60,150,140] looking at[-115,0,20]. All four geometry queues
were zero and the renderer reported ready. Capture and metadata:
`screenshots/elevated-envelope/seed87231-waterfront-miniature.{png,json}`.
The prior `screenshots/curved-rail/seed87231-waterfront-miniature.png` uses the
same framing. Regeneration changes building identities and history.

The foreground and far-bank building arrangement changes, but this view does
not establish a clear composition improvement. The broad diagonal expressway,
wide residual green strips, repetitive bank planting and conspicuous cloud
reflection remain dominant. Together with the modest seed0 overview result,
this is insufficient support for further envelope tuning as the primary visual
workstream. Retain the verified subdivision improvement; stop optimizing it for
lot count. No full multi-seed, both-style visual acceptance is claimed.

Next structural hypothesis: two independent full-domain elevated routes,
explicitly copied from an arterial row and column in masterplan.rs, impose too
much expressway on an800m neighborhood. Evaluate regional route placement and
hierarchy against the locked concepts before changing widths or adding detail.
Any route change must update the current specification's row/column contract
and preserve grade separation, boundary connections, fixed construction geometry,
clearance and deterministic replay. Do not simply remove visible decks in the
renderer. The developing seed5 and both-style comparisons remain outstanding.
