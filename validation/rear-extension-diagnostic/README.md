# Rear extension feasibility — 2026-10-11

This is a **test-only geometry experiment**, not an implemented redevelopment
rule. The source is the 871-building seed87231 save at tick272000, SHA-256
`74db8e9f9ac2bef4a828bd877d6d31a124ba522d9365033a50c51bc963add01d`.

All 562 eligible parcels pass an unchanged-footprint control. Independent depth
increases of 4, 8 and 12 m admit 143, 88 and 48 parcels respectively. These counts
must not be added: they describe overlapping alternatives. A stable parcel-ID
greedy pass selects the largest feasible increase for each parcel, updates its
collision envelope before evaluating the next, and finds **145 mutually
compatible extensions adding 13,422.74 m² of nominal envelope area**.

Street-facing edges stay fixed. Candidate rectangles contain the old footprint;
road, river, protected-land, neighbor, reservation and world-boundary clearances
are retained. Explicit orientation, 1–8 floors, an open building and connected
frontage are required. Anchors and reserved parcels are excluded. Legacy parcels
with ambiguous orientation are outside this experiment. Root caught and corrected
an initial collision-index error: legacy parcels contribute two envelopes. The
mixed-envelope regression and delta-zero control now guard that assumption.

[Comparison map](layout.png) shows the existing and hypothetical nominal
envelopes. Root inspected its rendered PNG: extensions make some developed rows
deeper, but broad residual spaces remain. This does not establish Tokyo visual
fidelity, rendered coverage, demand, floor capacity or economic feasibility. The
hypothetical snapshot changes building rectangles solely to illustrate spatial
potential; it must never be loaded as a completed simulation save.

## Next implementation gate

Use this as evidence for bounded, demand-driven phased extensions, not immediate
bulk resizing. Reserve the additional land before construction; keep the existing
building and its entrance usable while an annex is built, or explicitly relocate
occupants if the chosen construction method requires closure. Preserve stable
identity and history. Add capacity only at completion. Persist pending geometry,
revalidate before admission, bound evaluation/construction work, and test replay,
movement continuity, conflicting proposals, and renderer updates in both styles.
Occupied-site redevelopment with relocation remains a separate requirement where
an extension cannot preserve occupancy. Confirm appearance and performance before
promoting a production density policy.

## Reproduce

```sh
REAR_EXTENSION_REPORT="$PWD/validation/rear-extension-diagnostic/report.json" REAR_EXTENSION_SNAPSHOT_DIR="$PWD/.tools/rear-extension-review" node scripts/test-engine.mjs --release rear_extension_diagnostic -- --include-ignored
```

`CITY_TRACE_SAVE` optionally selects another absolute save path; the default is
`.tools/narrow-frontage-review/final-save.json`. Raw snapshots remain ignored under
`.tools/rear-extension-review`. Root independently ran both focused tests: **2
passed**. Source serialization is unchanged. The map tool's default output remains
byte-identical to the earlier deep-spine comparison; optional labels clearly mark
this new hypothetical comparison.
