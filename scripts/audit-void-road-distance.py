#!/usr/bin/env python3
"""Measure saved unbuilt-mask cells against actual open road centerlines.

Run from the repository root:
  python3 scripts/audit-void-road-distance.py

This is a read-only diagnostic. It uses the exact 2 m raster runs saved in
validation/density-void-audit.json and endpoint geometry in that report's save.
It does not infer street spacing from road segment lengths.
"""

import hashlib
import json
import math
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = ROOT / "validation/density-void-audit.json"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def point_segment_distance(x: float, z: float, a: dict, b: dict) -> float:
    dx = b["x"] - a["x"]
    dz = b["z"] - a["z"]
    length_squared = dx * dx + dz * dz
    if length_squared <= 0.0:
        return math.hypot(x - a["x"], z - a["z"])
    t = max(0.0, min(1.0, ((x - a["x"]) * dx + (z - a["z"]) * dz) / length_squared))
    return math.hypot(x - a["x"] - t * dx, z - a["z"] - t * dz)


def quantile(sorted_values: list[float], fraction: float) -> float:
    return sorted_values[math.floor((len(sorted_values) - 1) * fraction)]


def summarize(values: list[float]) -> dict:
    values.sort()
    return {
        "cellCount": len(values),
        "minM": round(values[0], 2),
        "p50M": round(quantile(values, 0.50), 2),
        "p90M": round(quantile(values, 0.90), 2),
        "p95M": round(quantile(values, 0.95), 2),
        "maxM": round(values[-1], 2),
        "over25m": sum(value > 25.0 for value in values),
        "over50m": sum(value > 50.0 for value in values),
        "over100m": sum(value > 100.0 for value in values),
    }


def main() -> None:
    report = json.loads(REPORT_PATH.read_text())
    save_path = (ROOT / report["input"]).resolve()
    if not save_path.is_file():
        raise SystemExit(f"Missing source save: {save_path}")
    world = json.loads(save_path.read_text())
    nodes = {node["id"]: node for node in world["nodes"]}

    roads = []
    for road in world["roads"]:
        if road.get("stage") != "open" or not road.get("connected"):
            continue
        if road.get("kind") not in ("local", "arterial"):
            continue
        a = nodes.get(road["from"])
        b = nodes.get(road["to"])
        if a is None or b is None:
            continue
        roads.append((road, a, b))
    if not roads:
        raise SystemExit("Source save has no connected open local/arterial roads")

    regions = []
    for index, region in enumerate(report["top3VoidComponents"]):
        distances = []
        for z_index, runs in region["void_mask_2m_rle"]["x_cell_index_runs_by_z_index"].items():
            z = int(z_index) * 2.0 + 1.0
            for first_x, last_x in runs:
                for x_index in range(first_x, last_x + 1):
                    x = x_index * 2.0 + 1.0
                    distances.append(min(point_segment_distance(x, z, a, b) for _, a, b in roads))
        boundary_ids = set(region.get("boundary_road_ids", []))
        boundary = [road for road, _, _ in roads if road["id"] in boundary_ids]
        coordinated_boundary = sum(road["id"] in set(world.get("neighborhood_road_ids", [])) for road in boundary)
        face = region.get("road_face_polygon_xz", [])
        perimeter = sum(
            math.hypot(face[(i + 1) % len(face)][0] - face[i][0], face[(i + 1) % len(face)][1] - face[i][1])
            for i in range(len(face))
        ) if face else 0.0
        regions.append({
            "regionIndex": index,
            "maskAreaM2": region["area"],
            "maskBoundsXZ": region["bbox"],
            "roadFaceAreaM2": round(region["face_area"], 2),
            "roadFaceCharacteristicWidth2AreaOverPerimeterM": round(2.0 * region["face_area"] / perimeter, 2) if perimeter else None,
            "openBoundaryRoadCount": len(boundary),
            "coordinatedTaggedBoundaryRoadCount": coordinated_boundary,
            "otherBoundaryRoadCount": len(boundary) - coordinated_boundary,
            "distanceToNearestOpenRoadCenterline": summarize(distances),
        })

    neighborhood_roads = set(world.get("neighborhood_road_ids", []))
    ordinary = [road for road, _, _ in roads]
    classes = {
        "coordinatedTagged": {"roadCount": 0, "centerlineLengthM": 0.0, "frontageRecords": 0},
        "other": {"roadCount": 0, "centerlineLengthM": 0.0, "frontageRecords": 0},
    }
    for road, a, b in roads:
        key = "coordinatedTagged" if road["id"] in neighborhood_roads else "other"
        classes[key]["roadCount"] += 1
        classes[key]["centerlineLengthM"] += math.hypot(b["x"] - a["x"], b["z"] - a["z"])
    road_by_id = {road["id"]: road for road in ordinary}
    for frontage in world.get("parcel_frontages", []):
        road = road_by_id.get(frontage["road_id"])
        if road is None:
            continue
        key = "coordinatedTagged" if road["id"] in neighborhood_roads else "other"
        classes[key]["frontageRecords"] += 1
    total_length = sum(values["centerlineLengthM"] for values in classes.values())
    total_frontages = sum(values["frontageRecords"] for values in classes.values())
    for values in classes.values():
        values["centerlineLengthM"] = round(values["centerlineLengthM"], 2)
        values["roadLengthSharePercent"] = round(100.0 * values["centerlineLengthM"] / total_length, 2) if total_length else 0.0
        values["frontageSharePercent"] = round(100.0 * values["frontageRecords"] / total_frontages, 2) if total_frontages else 0.0
        values["frontagesPerRoadKm"] = round(values["frontageRecords"] / (values["centerlineLengthM"] / 1000.0), 2) if values["centerlineLengthM"] else None

    output = {
        "method": "2 m void-mask cell centers to finite connected open local/arterial road segments; exact point-to-segment distance",
        "inputs": {
            "voidAudit": str(REPORT_PATH.relative_to(ROOT)),
            "voidAuditSha256": sha256(REPORT_PATH),
            "sourceSave": str(save_path.relative_to(ROOT)),
            "sourceSaveSha256": sha256(save_path),
            "seed": report["seed"],
            "tick": report["tick"],
        },
        "roadGraph": {
            "connectedOpenLocalOrArterialSegments": len(roads),
            "distanceDoesNotInferSpacingFromSegmentLengths": True,
        },
        "frontageSplit": {
            "basis": "neighborhood_road_ids tag, not an independent proof that every tagged road is a completed coordinated block edge",
            "totalFrontageRecords": total_frontages,
            "classes": classes,
        },
        "regions": regions,
        "limitations": [
            "The saved masks are approximate 2 m road-face raster components; they exclude parcel envelopes and road corridors but not terrain or protected geometry.",
            "Point-to-centerline distance is not parcel buildability, developable net land, or a measurement of street-to-street spacing.",
            "neighborhood_road_ids tagging does not prove that every tagged road bounds a completed coordinated block interior.",
            "The road graph and parcels are from a mature snapshot; this audit cannot establish what was available when each frontage was first admitted.",
        ],
    }
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
