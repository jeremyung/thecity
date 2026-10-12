# Tokyo street-network reference — 2026-10-11

Reference measurements of Tokyo's street fabric, compared with the master-plan generator. These are calibration evidence only. No PLATEAU or OpenStreetMap geometry is a runtime asset.

## Sources

**PLATEAU** (MLIT, CC BY 4.0): LOD1 road-surface (`tran`) files for five 1 km third-order meshes, plus LOD0 building roof edges for Kyojima and Taito. They were fetched gzip-compressed from the PLATEAU CityGML API (22 MB total) and cached in the ignored `.tools/plateau-reference/`.

| Mesh | Ward | Character |
| --- | --- | --- |
| 53394665 | Sumida (Kyojima) | Pre-war organic fabric with roji and diagonal arterials |
| 53393537 | Shinagawa (Togoshi) | Irregular long blocks with many dead-end lanes |
| 53394653 | Taito | Post-war land-readjustment grid |
| 53394526 | Shinjuku | Station district |
| 53393680 | Chuo/Minato | Waterfront |

**OpenStreetMap** (© OpenStreetMap contributors, ODbL): `highway` ways for the same mesh bounds, from the Overpass API. Only derived statistics are kept here.

Derived figure-ground maps of the PLATEAU meshes are in this folder: [Kyojima](kyojima.svg), [Togoshi](togoshi.svg), [Taito](taito.svg), [Shinjuku](shinjuku.svg) and [Minato](minato.svg). The same maps for generated plans are [seed 87231](seed87231.svg), [seed 0](seed0.svg) and [seed 7](seed7.svg). [comparison.png](comparison.png) shows seed 87231 beside Kyojima and Togoshi. The images are not drawn to the same scale: the plan is 800 m and the meshes about 1.1 × 0.9 km.

## Measures

- **Orientation order** (Boeing 2019): length-weighted bearings in 36 bins. 1 is a perfect grid and 0 is uniform.
- **Junction type:** junction degree from OSM centerlines, or plan nodes for generated plans. The share of three-way (T) versus four-way crossings.
- **Dead ends:** degree-1 nodes per km².
- **Blocks:** the mesh area minus the road surface. Blocks touching the edge or larger than 30,000 m² are excluded.

Full values are in [street-metrics.json](street-metrics.json).

| | Orientation order (streets) | T-junction share | Dead ends / km² | Junctions / km² | Median segment | Non-four-sided blocks | Block rectangularity p50 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Kyojima | 0.14 | 83% | 98 | 491 | 38 m | 81% | 0.74 |
| Togoshi | 0.52 | 81% | 96 | 502 | 39 m | 59% | 0.88 |
| Taito | 0.64 | 65% | 48 | 459 | 42 m | 44% | 0.95 |
| Shinjuku | 0.27 | 75% | 16 | 231 | 47 m | 91% | 0.90 |
| Minato | 0.63 | 70% | 35 | 217 | 55 m | 66% | 0.96 |
| Plan before (seeds 87231 / 0) | 0.83 / 0.79 | 90% | 4–5 | 371–398 | 44–47 m | 51–56% | 0.82–0.85 |
| **Plan now (seeds 87231 / 0 / 7)** | **0.58 / 0.55 / 0.56** | **92–93%** | **22–38** | **488–566** | **37–38 m** | **64–70%** | **0.76–0.78** |

The earlier plan was more grid-aligned than even Taito's land-readjustment grid. Three generator changes, informed by these maps, bring it into Togoshi's range:

1. **District orientation.** Each superblock's streets turn by a seeded angle from its edges: up to about 35° in compact districts, less near the station. Blocks therefore meet arterials at angles.
2. **Bent and diagonal arterials.** Interior arterial junctions get seeded slopes and offsets, with a convexity guard. The river-bank roads, the rail corridor and the outer ring stay fixed.
3. **Dead-end roji.** 4 m alleys reach two-thirds of the way into about a third of deep blocks.

## Remaining gaps

- Dead ends are at 22–38/km², against Tokyo's 48–98. More roji per block, or roji into shallower blocks, would close this.
- The T-junction share (92–93%) is higher than Tokyo's 65–83%, because cuts end at existing streets. Occasional straight-through cuts would add four-way crossings.
- Street length per km² (34–37 km) is lower than the reference (45–64 km). OSM counts service roads and separate carriageways, so part of the gap is method.
- The fixed river, rail corridor and outer ring still frame the city as a square.

## Reproduce

```
python3 -m venv .tools/plateau-python && .tools/plateau-python/bin/pip install shapely pyproj numpy
# Fetch the tran/bldg files from the PLATEAU CityGML API (https://api.plateauview.mlit.go.jp/datacatalog/citygml/m:<mesh>)
# and OSM highways via Overpass for each mesh bbox into .tools/plateau-reference/.
cd engine && for s in 87231 0 7; do PLAN_JSON=$PWD/../.tools/plateau-reference/plan-$s.json PLAN_SEED=$s cargo test --release masterplan::export -- --ignored; done && cd ..
.tools/plateau-python/bin/python scripts/plateau-street-reference.py --out validation/street-reference \
  --source kyojima=53394665,.tools/plateau-reference/kyojima_tran.gml.gz,.tools/plateau-reference/kyojima_bldg.gml.gz \
  --osm kyojima=53394665,.tools/plateau-reference/osm_kyojima.json \
  --plan seed87231=.tools/plateau-reference/plan-87231.json   # and the other meshes/seeds
```
