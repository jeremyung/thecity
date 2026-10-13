# Regression fixtures with current route joins

These static test inputs derive from the historical captures named in
[manifest.json](manifest.json). The historical `validation/` files remain
byte-identical to their committed originals; they still document the old runs.

Regenerate with `python3 scripts/refresh-access-join-fixtures.py` from the repo
root. This is an offline test-data tool, not an application save migration.
Its endpoint calculation follows `Engine::route_endpoint_position`; native
restore validation checks the resulting inputs independently.

Across 17 fixtures, 6,648 connector joins are updated to meet the current route
endpoint and street-width policy. In two active paths in the founder topology
fixture, the corrected path is shorter than saved progress; progress is adjusted
proportionally to retain the journey stage. The manifest lists original and
new values for every edit, plus source and output SHA-256 hashes.

Generation reverses all listed edits in memory and checks semantic equality
with the source. All unlisted state—including road/lot/building geometry, actor
identities, route sequences, random state, clocks and planner state—is unchanged.
No runtime fixture patching or relaxed save-validation branch is used.

Existing topology, construction, queue, density and replay assertions remain.
These derived fixtures establish regression coverage, not new visual or growth
measurements of the original historical runs. They are test assets and are not
part of the browser runtime asset pack.
