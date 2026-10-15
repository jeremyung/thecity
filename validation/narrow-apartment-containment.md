# Narrow apartment containment — 2026-10-15

A planner-admitted5m apartment site could receive a6m minimum body, exceeding
its site during construction and after opening. This affected ordinary4–11floor
apartments; parcel admission, use and capacity were already authoritative.

Keep the existing1.3m side allowance without the minimum body width for this
family, and accept the resulting3.7m body in bounded midrise massing. This keeps
construction, balconies, entry and renovation on the same supported family path.
Towers, anchors and2–3floor apartments retain their prior behavior. No engine,
WASM, save, simulation, demand or generation change.

The new regression fails on PR19 head5e98142 at5×8m/4floors/seed32/structure:
xmin−3.325m exceeds the site's−2.5m boundary. After the fix,384 combinations
cover both styles, widths5/6/7m, depths8/16m, floors4/5/8/11, two family seeds,
and structure/surface/open/renovation. It checks actual whole-kit bounds and
an unobstructed front-glass ray hit for open buildings. Related five suites:
38tests pass; TypeScript passes. Full integrated web suite:125 suites /689tests passed in114s on Apple M2 Max/macOS. Diff whitespace checks pass.

Earlier root-only evidence: six unchanged production snapshots contain308 open
narrow apartments;193 nominal-site overflow failures become zero. Matched
isolated neighborhood captures show narrower silhouettes with retained balcony/
stair identity; narrow roofs may omit the old tank. Those captures used the root
renderer, simplified road slabs and WebGL fallback, not this integrated PR's
materials or full application. They are containment evidence, not acceptance
of the urban-fabric milestone. Raw captures remain local and are not committed.
