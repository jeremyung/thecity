# PLATEAU bounded source pilot — 2026-10-11

This is source evidence, **not an accepted generator calibration**. Production
city generation and rendering are unchanged.

Three individual CityGML files and their metadata use 26,254,659 compressed
bytes, below the 50 MiB retrieval limit. Raw sources are cached only in ignored
`.tools/plateau-pilot/`. `provenance.json` records exact URLs, resource metadata
license statements, compressed/decoded SHA-256 hashes and actual sizes. Two
catalog decoded sizes disagree with the downloaded files; both values are kept.
Each source's metadata states CC BY 4.0. Attribution: Project PLATEAU / MLIT,
selected Shinjuku, Taito and Minato resources. Derived crops are modifications.
Dataset year 2025 does not establish the actual survey date.

The [official per-file API](https://docs.plateauview.mlit.go.jp/datasets/citygml/)
returned overlapping mesh entries under neighboring wards. Only one source per
mesh is selected. All four corners of each metric 250 m square were independently
checked inside its selected standard mesh boundary. Geographic context labels
are sampling hypotheses, not verified land-use classifications.

| Candidate context | Selected buildings | Roof-edge union / gross 62,500 m² |
| --- | ---: | ---: |
| Shinjuku station edge | 140 | 28,773.689 m² (46.04%) |
| Taito compact neighborhood | 452 | 30,295.410 m² (48.47%) |
| Minato waterfront | 25 | 6,282.792 m² (10.05%) |

These are **LOD0 roof-edge projections**, not cadastral lots or ground footprints.
No net-developable denominator is established. The waterfront number includes
all crop area; water and transport exclusions have not been measured. Height and
storey missingness stays explicit; geometry cannot establish jobs or occupancy.
Nearest-building distances use full geometry of selected buildings and remain
window-censored. LOD2 availability is recorded, but roof forms are not classified.

`morphology.json` retains per-building source IDs and local clipped geometry.
Its extractor completeness flag covers recognized direct Building records only:
Taito yields 4,613 records versus 4,614 catalog features. This discrepancy remains
unresolved, so all samples are marked unaccepted for calibration. No source
metrics should become production targets before this and source semantics are
reviewed. Diagrams, source/context visual review, distributions and matched
synthetic benchmarks remain pending.

## Reproduce

Create a Python virtual environment and install `scripts/requirements-plateau.txt`.
Retrieve each `catalogBuildingFile.url` in provenance with `Accept-Encoding: gzip`
without automatic decompression; verify the recorded compressed SHA-256. Save as
`.tools/plateau-pilot/thecity-plateau-{sourceIndex}.gml.gz`. Do not fetch whole wards.
Run `scripts/extract-plateau-crop.py` with `--input`, `--center-lon`, `--center-lat`,
`--size 250`, and `--output`; crop arguments are in provenance. The JSON adds
review status separately from raw extractor output.

Six synthetic tests pass via `.tools/plateau-python/bin/python scripts/test-plateau-crop.py`.
Root independently reran all three sources. Tests exercise holes, clipping,
identifiers, CRS rejection and missing/invalid attributes. Further hardening is
needed for envelope inheritance and non-Building members; the extractor is a
narrow offline pilot, not a general CityGML importer.
