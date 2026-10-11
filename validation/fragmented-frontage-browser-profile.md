# Fragmented frontage sampling — 2026-10-10

The synchronous and resumable host selectors now preserve their existing
midpoint samples and append at most two inset edge samples for fragmented free
spans. Total splits remain capped at six per host. Single-span output is
unchanged. Exact collision, lot, connectivity and admission gates are unchanged.

The incremental selector has explicit charged edge-comparison cursors, persisted
in host-preparation checkpoint schema 4 (previously 3). Older checkpoints that
contain that stage are rejected; ordinary saves without it remain compatible.

## Evidence

- Full native release suite: 321 passed, zero failures, 39 ignored.
- The ignored mature golden descriptor comparison also passes unchanged.
- A geometric regression places off-host blockers across the previous midpoint
  rays and verifies that newly sampled edge rays clear them and produce candidate
  descriptors. This is candidate coverage, not a full construction acceptance.
- Quota-1/7 JSON replay matches synchronous samples, including short spans that
  produce no extra edges. Existing samples remain the prefix; extras are bounded.
- Production WASM builds. The matched 30,000-tick Chromium continuation restores
  its final save exactly. Script syntax and diff whitespace checks pass.

## Mature-city result: no density improvement

[The new report](fragmented-frontage-browser-profile.json) uses the same mature
fixture, tick count and probe intervals as [the prior sweep run](face-sweep-browser-profile.json).
Both finish with 853 buildings, 853 parcels and 210 roads. Both observe branch
4003 during construction at tick 244500 and open/connected at 244700, with two
frontages in the final city. Both sample 11 selected faces and finish with ten
exhausted identities.

The exported final JSON files are byte-identical, SHA-256
`5207f37c688888d9e6d49e293ba07a626e9bc5faba72019b86ab21ea8e77e9e3`.
The broader sample coverage therefore does not change this continuation's
outcome. It must not be presented as improved city density or visual fidelity.

Measured synchronous step calls: p95 0.40 ms, p99 10.10 ms,
max 29.60 ms. Snapshot/save probes, application scheduling and rendering
are excluded. Host timing differences do not establish a performance regression.

Next density work needs evidence about rejection causes or the upstream block
layout, rather than further speculative sampling expansion. Visual references
and remaining render gaps are recorded in screenshots/latest/README.md.
