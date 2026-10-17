# Checkpoint notes moved from specs/03-simulation.md

Dated candidate, checkpoint and status notes, moved verbatim on 2026-10-16 so the
spec describes current design only. Newest decisions win over older notes.

Implementation candidate (2026-10-12): residual households now dispatch bounded
local walking errands with saved outbound, dwelling and returning phases. Each
outing represents one person; the home participant remains reserved until arrival
home. Shop/cafe reservations and actual entrance routes precede admission. Current
parameters cap128 outings and180m per outbound walking leg, with09:00–21:00
admissions. These are calibration candidates, not acceptance targets. Broader
job-derived demand, universal visitor accounting, fixed route-expansion budgets
and multi-seed/browser acceptance remain outstanding. See the
[daily activity review](../validation/archive/2026-10/fidelity-review-current.md).

Implementation checkpoint (2026-10-10): the diagnostic geometry-derived access search now discovers an admissible two-lot branch in the mature seed87231 fixture without hand-picked route coordinates. It enumerates2,069 descriptors in44 pages of at most48 and passes shared admission, validation, and canonical restore. The discovered road also completes through normal engine ticks (observed open and connected at242400), with dependent lots withheld until opening and save-identical mid-construction replay. This remains test-only. The first resumable stage now reproduces44 mature-face host split samples bit-for-bit at quotas1,7,and48 with JSON resume checks; A separately tested ray/segment primitive now budgets individual polygon-edge checks and preserves JSON resume; the composed heading/endpoint preparer now matches all2,069 frozen mature descriptors at quotas1,7,48,and irregular partitions, including149 JSON restores per run. All runs charge2,422,472 work units; the largest observed checkpoint is451,864 JSON bytes. This proves native sequence/resume parity on this fixture, not full runtime acceptance. Ray initialization now shares a polygon validated once per job; host lookup uses canonical binary search. Every-step small-fixture restore validation and the remaining setup/evaluator budget review are still required before promotion. A pure graph pipeline now reproduces all49 mature reference faces through budgeted preflight, bridge detection, topology and complete face walking at quotas1,7,48 with JSON resumes. Its stage work counts are119,232 /2,022 /2,427 /19,630 respectively; this is native frozen-input evidence. Live graph capture, target sampling and pipeline scheduling remain outstanding. Before runtime promotion, incrementally budget live face/target discovery and exact evaluation; persist deterministic cursors and frozen inputs, validate restored bounds, and recheck current geometry at admission. Latest integration evidence: the persisted frozen discovery wrapper now runs through a separate WASM diagnostic boundary, with native and WASM producing the same2,069 ordered descriptor bits after shared algebraic heading construction. Checkpointed and uninterrupted WASM results agree. Native and WASM logical work counts still differ, so cross-target scheduling equivalence is not established. A frozen quota-stepped proposal validator now independently matches the existing evaluator on all2,069 descriptors with JSON resume; production capture, stale-geometry checks, split preflight and live scheduling are still pending. See `validation/spec-audit.md` and `validation/incremental-access-admission-plan.md`. A single successful face is not evidence of general density improvement. See `validation/face-access-paged-search.json` and `validation/face-access-paged-root-test.txt`.

Implementation checkpoint (2026-10-11): bus and inbound train passengers can
alight into a persisted, visible waiting-at-stop state when the final walking
route is unavailable. Recovery caps are 256 waiting residents and eight examined
entries/route queries per tick; unchanged failed-route geometry suppresses repeat
searches. Reconnection preserves the physical waiting position and original trip
intent. Native and WASM replay evidence is recorded in
[the handoff](../validation/archive/2026-10/HANDOFF.md). A permanently unavailable route still
needs an explicit cancellation/replanning policy; this checkpoint does not close
the complete travel requirement.

Implementation evidence (2026-10-10): shallow-frontage continuation is repeatable within the tested WASM binary, but native and WASM are not evolution-identical. Matched 200-tick checkpoints first differ in frontage geometry at tick 243200 and choose different candidates by tick 251600. Do not use native continuation counts as exact browser predictions. See `validation/shallow-native-wasm-diagnostic.md`; the new `tests/shallow-infill-wasm.test.ts` covers real construction, connected access, and same-target replay in the browser engine.
