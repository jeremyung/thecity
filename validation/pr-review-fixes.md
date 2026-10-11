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
