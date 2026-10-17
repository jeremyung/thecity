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
The catalog count is now reconciled: Taito contains 4,613 Building records plus
one geometry-free CityObjectGroup referencing an existing building. All three
sources have zero missing/duplicate building IDs, missing selected LOD0 geometry,
invalid polygons or unhandled nonbuilding geometry. This establishes supported
geometry extraction, not completeness of the real-world survey.

Source `9999` numeric values are retained in raw fields, labeled sentinel, and
excluded from statistics. [MLIT's work-procedure guidance](https://www.mlit.go.jp/plateau/file/libraries/doc/plateau_doc_0002_ver02.pdf)
discusses unknown-value markers including 9999 and source-specific null semantics.
This pilot conservatively excludes that exact marker; it does not establish the
meaning of every possible source code. Zero values have not been reinterpreted.

[Station diagram](station.svg), [compact-neighborhood diagram](ordinary.svg), and
[waterfront diagram](waterfront.svg) share the same 250 m extent and height legend.
Root rendered and visually checked all three SVGs in Chromium. Empty areas are
unclassified; they are not inferred lawns, roads or water polygons. `summary.json`
records linear-interpolated percentiles with known counts and missing values
excluded. Roof-edge gaps can include touching/overlapping eaves and must not be
used directly as wall, collision, fire-access or cadastral clearance rules.

| Candidate | Building area median / p90 | Height median / p90 | Known heights | Known above-ground floors |
| --- | --- | --- | ---: | ---: |
| Station | 114.67 / 651.21 m² | 17.45 / 32.86 m | 130 / 140 | 121 / 140 |
| Compact | 57.62 / 136.69 m² | 12.05 / 22.08 m | 452 / 452 | 417 / 452 |
| Waterfront | 203.92 / 787.53 m² | 7.20 / 31.40 m | 21 / 25 | 20 / 25 |

Area percentiles use full projected geometry of buildings intersecting the crop;
edge-intersection selection biases distributions toward larger buildings. Coverage
alone uses the exact clipped union. All 140 station and 452 compact selected
buildings expose direct LOD2, as do 22/25 waterfront records. Roof-family extraction,
verified context, street frontage, land-use masks and net coverage remain pending.

## Consequences for the generator

The compact sample demonstrates many small footprints with a long larger-building
tail, rather than a uniform larger lot. The mixed-row experiment's 2.5–4.3% area
gain from widening one row is not sufficient evidence to promote it. Next compare
matched 250 m synthetic crops, including separate parcel and rendered-roof unions,
area/height joint distributions, usable street access and resource counts. Keep
small-building rows; test occasional larger assembled sites in appropriate context.
Do not translate 452 intersecting source buildings into a whole-city entity target
or loosen engine clearances to match roof-edge gaps. Occupancy capacity needs its
own explicit units and lifecycle validation.

## Reproduce

Create a Python virtual environment and install `scripts/assets/requirements-plateau.txt`.
Retrieve each `catalogBuildingFile.url` in provenance with `Accept-Encoding: gzip`
without automatic decompression; verify the recorded compressed SHA-256. Save as
`.tools/plateau-pilot/thecity-plateau-{sourceIndex}.gml.gz`. Do not fetch whole wards.
Run `.tools/plateau-python/bin/python scripts/assets/build-plateau-reference.py` to verify
hashes, rebuild all three crops, reconcile catalog counts and generate the summaries
and SVGs. For individual crops, use `scripts/assets/extract-plateau-crop.py`; arguments
are recorded in provenance. The builder adds review status separately from extraction.

Twelve synthetic tests pass via `.tools/plateau-python/bin/python scripts/assets/test-plateau-crop.py`.
Root independently reran all three sources. Tests exercise holes, clipping,
identifiers, CRS rejection and missing/invalid attributes. Envelope inheritance, group references, positive nearest distances, overlapping
unions and unknown numeric markers also have focused checks. The extractor remains
a narrow offline pilot, not a general CityGML importer.

A repeat rebuild produced identical morphology, summary and all three SVGs. The
full per-building diagnostic JSON compresses to131,696bytes and remains offline;
it exceeds the proposed100KiB compact-pack target. The separate summary is below
1KiB gzip, but is not yet an adequate joint-distribution runtime archetype pack.
Do not bundle either as production calibration without the remaining review.
