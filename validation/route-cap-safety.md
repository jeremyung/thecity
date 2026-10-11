# Reject incomplete route suffixes — 2026-10-11

The routing adjacency equivalence tests exposed an existing correctness bug:
reconstruction stopped at the 128-node cap, then returned the goal-side suffix
even when it did not reach the requested origin. Consumers treated that suffix
as a valid trip. A delivery van or resident could therefore start partway along
the journey, and bus-stop selection could use the wrong walking distance.

`route_avoiding` now returns an empty result if reconstruction ends before
reaching `start`. Exactly 128 nodes remain valid; 129-node paths fail closed.
The search, cost ordering and node cap are otherwise unchanged. This does not
find a more expensive shorter-node alternative or implement segmented journeys.

The frozen old oracle remains in the test module. Its known oversized suffix is
normalized to rejection for equivalence assertions; a separate boundary test
explicitly demonstrates the old 129-node witness (128 returned nodes starting
at node 1 instead of node 0). Another test dispatches to an active shop over
that chain: the oversized trip creates no traveler or delivery-cursor advance;
moving its destination one node closer dispatches from the actual gateway.
The other route tests retain filter, tie, sparse-ID and mutation coverage.

Call-site review found guards for empty routes in resident departures, delivery
dispatch, rail departure, bus-stop selection, bus trip creation, bus line
construction and return alighting. Queue-slot selection before some checks is
read-only. Existing inability-to-alight handling can keep a passenger aboard;
bounded retry/cancellation for genuinely unreachable journeys remains separate
unfinished work under the travel specification. This fix prevents starting an
invalid partial path; it does not claim complete disruption recovery.

No save schema changes. Existing saved traveler paths are not rewritten.
Native release suite: **361 passed, 43 ignored**. The six focused routing
tests pass. Production WASM rebuild succeeds; the fresh seed-1204,
development-rate-60, 300,000-tick run satisfies collection caps and exact restore,
and its final save hash is unchanged from the pre-optimization baseline.
See [WASM evidence](route-cap-wasm.json). Timing is excluded because the native
suite ran concurrently. The full frontend suite was not rerun for this Rust-only
change; this is not a claim that its previous timeouts are resolved.

No visual change is claimed. Live visual review was attempted again this turn,
but the Mac remains locked and the screenshot API returned `Unable to capture
screenshot`; the compact-midrise renderer study remains unpromoted.
