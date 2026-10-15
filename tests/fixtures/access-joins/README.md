# Regression fixtures with current route joins

These static test inputs derive from the historical captures named in
[manifest.json](manifest.json). The historical `validation/` files remain
byte-identical to their committed originals; they still document the old runs.

Regenerate with `python3 scripts/refresh-access-join-fixtures.py` from the repo
root. This is an offline test-data tool, not an application save migration.
Its endpoint calculation follows `Engine::route_endpoint_position`; native
restore validation checks the resulting inputs independently.

Across 18 fixtures (including the sibling bus-queue fixture), 7,505 connector joins are updated to meet the current route
endpoint and street-width policy. In five active paths across the fixture corpus, the corrected path is shorter
than saved progress; progress is adjusted
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

Current-format fixture normalization (2026-10-12): each world-state fixture now
uses `activity_schema_version` 5 and an empty `neighborhood_errands` ledger
with zeroed raw scan cursors. This is static test-fixture preparation, not a
runtime migration path. `manifest.json` records each fixture's prior activity
schema and updated SHA-256; the schema change was checked by removing the new
ledger and restoring/removing the activity-version field, then comparing the
remaining parsed JSON to the previous fixture. Geometry, actors, ordering, and
all other saved fields are unchanged. The seed-5 bus queue fixture has the same
normalization and its provenance record points to the updated fixture hash.

Route-policy refresh (2026-10-13): local/lane walkers use half road width minus
0.72 m; other road kinds use half width plus 0.72 m, matching the engine's shared
walking policy. Runtime restore validation remains unchanged. The generator now
reproduces activity schema/empty errand normalization as well as joins; its inverse
check restores all declared fields and proves semantic equality to each source.
The bus queue's immutable source is Git checkpoint e87392b, recorded in the same
manifest. Regeneration does not change routes, resident IDs, queue ordering, road
or building geometry, clocks, or random state. The historical validation sources
remain untouched. The business-activity test now generates a current-format city
through ordinary bounded steps and checks history, occupancy and replay directly;
it no longer asserts obsolete migration behavior.
