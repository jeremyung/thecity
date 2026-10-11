# PR #1 review fixes — 2026-10-10

All nine inline review items were addressed in code. This remains an engineering checkpoint; the composed planner is not connected to live growth.

- **4239677765:** Bounded target headings must reach their target plus frontage margin within the branch limit and clear parcel footprints before consuming a slot. Exhaustive descriptor enumeration deliberately defers collision checks to the separate validator. Regression tests cover unreachable targets and blocked target routes with usable corner tangents.
- **4239677932:** Reserve each host an equal share of the existing 48-unit work cap. Count work only after checking the host's output allowance; skip duplicate host visits. The square-face regression now requires all four hosts, including the last host in edge order, to produce candidates.
- **4239678058:** Restore the fourth sample to `end - span / 6` in both synchronous and incremental host preparation. Test the sample next to a parcel projection explicitly.
- **4239678156:** Remove the duplicate inward-heading fallback.
- **4239678244:** Construction and CopyGraph restore now both validate graph IDs/endpoints and membership in the candidate-node snapshot. Tests cover dangling references, duplicate IDs, missing snapshot nodes, and valid CopyGraph round trips.
- **4239678322:** Remove the unreachable Projects validator phase; ActiveProjects already rejects active road work.
- **4239678414:** Remove the redundant road_opened revision mark while retaining the project-completion and independent mutation marks.
- **4239678520:** Centralize revision increment/exhaustion handling on WorldState for runtime mutation and save migration.
- **4239678651:** Recompute local bounds from current positions before contour prefiltering. Test a rewritten shared geometry with a stale small cached bounding box against the legacy edge output.

## Reference changes and limitations

Restoring the fourth sample changes the mature descriptor set from 2,069 to 2,134: 1,581 unchanged, 488 removed, 553 added. All first-three-sample endpoint sets are unchanged. Native output was compared with freshly built WASM before adopting the new reference. Input JSON remains unchanged. The migration record includes hashes; the old golden is retained in Git at 7db65cc.

The revised set has **zero accepted proposals on this mature fixture**, matching the independent synchronous evaluator across all 45 pages. The previous sole accepted proposal came from the removed end-minus-1m sample. This exposes a finite-sampling coverage limitation; it must be addressed before live growth integration. Success coverage remains explicit: `accepted-access-candidate.json` retains that legal two-lot candidate independently of sampling, and both native and WASM validators accept it with matching output across quotas and save/resume. The paged-search diagnostic now reports zero results without claiming construction success; existing explicit admission/construction tests remain in the native suite.

The revised fixture also exposed a constructor inconsistency: borrowed host preparation normalized using hypot while owned preparation used access_math::length. Both now use the shared explicit vector calculation. Native uninterrupted, quota-stepped, and restored descriptor output match exactly. The temporary JSON parsing hypothesis was rejected; no dependency change was needed.

Diagnostic checkpoint schemas were incremented for changed sampling/phase semantics. Old diagnostic jobs must restart; ordinary city saves do not contain these jobs.

## Validation

- Native release suite: 299 passed, 39 ignored.
- Web suite: 262 passed across 51 files.
- Production TypeScript/Vite and WASM builds pass; existing compiler/bundle warnings remain.
- Separate mature descriptor, proposal, planning, and pipeline gates pass, including serialized resumes.
- Fresh WASM discovery: exact 2,134-descriptor parity at quotas 48, 4096, and 16384, including resumes.
- Fresh WASM proposal validation: 2,134 generated descriptors plus the independent accepted candidate; exact native proposal parity at quotas 48 and 4096.

See `pr-review-*.txt`, `access-planning-mature.json`, `access-descriptor-incremental-check.json`, and the WASM profile JSON files for evidence. Earlier reports describe the previous sampling policy and are historical.

## Follow-up: split coverage and repeated hosts

- **Edge split samples.** Single-interval hosts keep the four reviewed samples and append up to two edge samples: 1m inside an end bounded by host end clearance, 3m inside an end bounded by a parcel projection or node (leaving the 5m parcel clearance after the 2m projection margin). Samples within 1m of the quartile offset are omitted. Bounded scans reach them only with spare host budget. The old end-minus-1m sample found the mature route because road 3317's free interval ends at host end clearance, not at a parcel.
- **Repeated hosts.** A road bounding a face on both sides is scanned on both sides again; the two traversals share one per-road work allowance. The skipped-duplicate behaviour diverged from incremental host preparation, which never deduplicated.
- **Reference migration.** Mature descriptors grow from 2,134 to 3,180: all 2,134 retained, 1,046 added. The descriptor at index 2824 is bit-identical to `accepted-access-candidate.json` and is the only accepted descriptor. Reference SHA-256 `9cea2591…cd811` → `53364380…83fee`; input JSON preserved verbatim. Composed planning: 67 pages, 1 accepted, 10,486,289 work units (was 45 pages, 0 accepted, 8,239,083).
- **Checkpoint schemas** for host preparation, descriptor search, discovery and planning jobs were incremented.
- **Validation (Linux x86_64, rustc 1.97):** native release suite 298 passed, 1 failed, 39 ignored; the failure, `frontier_proposal_scan_is_read_only_and_plan_road_replays`, also fails on `main` in this environment. Mature descriptor, discovery, proposal, pipeline, planning and paged-search gates pass. Fresh WASM discovery and proposal output match native exactly. Web: TypeScript passes; vitest 258/262, the four failures are fixed in-test timeouts that also time out on the unmodified PR head in this container.
