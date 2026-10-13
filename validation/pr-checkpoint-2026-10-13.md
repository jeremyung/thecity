# Living-city review checkpoint — 2026-10-13

This draft PR captures the accumulated branch, including the earlier e87392b
checkpoint and subsequent neighborhood work. It is not merge-ready or artistic
acceptance. Remote main was fetched and is an ancestor of this branch.

## Scope

- Living-city movement/access, elevated roads and rail, construction presentation,
  camera/DOF and incremental rendering from the preceding checkpoint.
- Causal local errands, station approaches and road walking offsets, station
  construction phases and clearance, neighborhood building families and glazing,
  and bounded bank planting.
- Current-format fixture updates, diagnostic scripts, reference studies, visual
  evidence and the updated living-city goal. Rejected experiment patches are
  historical evidence, not production changes.

## Fresh verification

- `npm run build`: passes; existing >500 kB bundle warning.
- `npm run test:engine -- --release`: **443 passed, 27 failed, 54 ignored**.
- First `npm test`: **605 passed, 12 failed**, across 110 files. Ten failures report
  traveler origin-connector/route-endpoint mismatch while restoring fixtures
  (including the camera test before its framing assertions); one asserts a legacy
  activity schema is absent, and one movement check exceeds its 5-second timeout.
- `npm run wasm`: passes. Post-rebuild `npm test`: **606 passed, 11 failed**
  across 110 files. The ten restore failures and legacy-schema assertion persist;
  the movement timeout did not recur. This is still a failing default suite.
- Changed/new file credential-pattern scan found no matching private keys,
  OpenRouter keys or GitHub tokens; this is a bounded scan, not a security audit.
- Source whitespace check passes. Archived patch files intentionally retain diff
  context prefixes and can produce whitespace warnings when staged as new files.

Detailed command evidence is in `pr-checkpoint-2026-10-13/`.

## Merge blockers and outstanding review

Resolve native/web fixture and restore failures without weakening access invariants
or deleting behavioral regression coverage. Backward-save compatibility is not
required; replacement current-format fixtures must still exercise the behaviors.
Do not attribute failures to stale WASM, baseline behavior or machine load without
separate evidence. The historical green checkpoint does not validate this tree.

Office side-facade final stills and sampled orbits support a modest improvement;
live developing/construction review remains open. Existing office parcel overhangs
need diagnosis. Unisolated frame spikes around 80 ms, normal load/growth behavior,
actual day cycles, narrow views, audio, named-hardware performance and user artistic
review remain open. See HANDOFF.md, office-side-facades.md and living-city-goal.md.
