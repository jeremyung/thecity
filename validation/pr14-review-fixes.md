# PR14 corrective follow-up

These fixes apply to PR14 head `676fd615cef35c78216ba5eede3a0b54e9e4ea77`.
They are isolated from the uncommitted curved-rail, parcel, expressway and fixture
changes in the living-city working branch. No simulation or save changes.

## Fixed

- Keep four-sample scene AA and supply resolved view normals to GTAO. This avoids
  the invalid multisampled depth-normal WGSL path seen on WebGPU.
- Compile style variants against the live scene-pass render target and MRT layout;
  restore renderer state immediately after synchronous submission. Skip unchanged
  shared materials, avoiding unnecessary water viewport-copy hooks.
- Detach obsolete pipeline owners immediately but defer their disposal until
  pending compilation settles. Release the sky environment through that same
  lifetime mechanism during teardown.
- Correct the water advection blend: the sample whose phase wraps has zero weight.
  Cover both half-cycle boundaries with continuity tests.
- Omit the DOF node and its seven passes while DOF is disabled. This reduces
  inactive work; it is not a claim of a demonstrated frame-rate improvement.

## Validation and scope

The exact PR source plus these fixes builds after rebuilding its own WASM.
Six focused files  / 15 tests pass, covering style warmup, pipeline retirement,
water continuity and normals, skyline and backdrop forest.

Earlier production-app checks on the larger local integration used the Codex
in-app browser WebGPU backend on a MacBook Pro M2 Max / 32 GB. Both style-switch
directions on a mature wet city completed without captured GPU warnings/errors;
night, dawn, morning and resize checks also produced no captured diagnostics.
These runtime results include additional local city work and are not a clean
runtime benchmark of this isolated PR commit.

## Full-suite comparison

Fixed PR source:616 passed / 12 failed across 115 files with one worker. Re-running
those 11 failing files on unchanged head 676fd61 using the identical WASM artifact
reproduces 11 assertion failures (legacy-save fixtures and skyline camera framing).
The remaining fixed-run failure is a 5 s movement-test timeout; the unchanged head
passes that file. Re-running that file on the fixed source passes all five tests
at unchanged timeout limits. See the full-run, original-head and recheck logs
alongside this note.
No timeout limits were increased and no failing assertions were removed.

## Remaining acceptance limits

- Real GPU teardown *during* pending compilation has not been verified. The
  focused ownership test passes; an attempted browser harness was blocked while
  loading its module, before it could exercise the renderer.
- Performance is not accepted. Local integrated Anime samples reached 52–54 fps
  at 1600×900 versus 61 fps at 2048×1152 for a pre-PR renderer overlay on the same
  mature scene. Adaptive quality differed, so this is a concern rather than an
  exact regression percentage. A fixed-quality comparison and effect profiling
  remain required before claiming the browser performance target.
- These fixes do not establish concept-fidelity or full specification 08 acceptance.

The original compilation/resource/flow defects have corrections and regression
coverage. Merging this rendering checkpoint means retaining the explicit runtime
and performance follow-ups above; it does not mean those gates have passed.
