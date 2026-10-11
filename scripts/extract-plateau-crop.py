#!/usr/bin/env python3
"""Extract a metric square of PLATEAU buildings from a gzipped CityGML file."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import math
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

try:
    from pyproj import Transformer
    from shapely.geometry import Polygon, box, mapping
    from shapely.ops import transform, unary_union
except ImportError as exc:  # pragma: no cover - environment diagnostic
    raise SystemExit("Requires shapely 2.0.7 and pyproj 3.6.1; use .tools/plateau-python/bin/python") from exc

GML = "http://www.opengis.net/gml"
XLINK = "http://www.w3.org/1999/xlink"
CORE = "http://www.opengis.net/citygml/2.0"
BLDG = "http://www.opengis.net/citygml/building/2.0"
NS = {"gml": GML, "b": BLDG}
SRC_CRS = "EPSG:6668"  # horizontal component of PLATEAU's EPSG:6697 coordinates
OUT_CRS = "EPSG:6677"  # plane rectangular CS IX, Tokyo


def local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def validate_declared_crs(elem: ET.Element, label: str) -> None:
    crs = elem.attrib.get("srsName")
    dim = elem.attrib.get("srsDimension")
    if crs and crs != "http://www.opengis.net/def/crs/EPSG/0/6697":
        raise ValueError("unsupported inherited CRS on " + label)
    if dim and dim != "3":
        raise ValueError("unsupported inherited coordinate dimension on " + label)


def has_geometry_payload(elem: ET.Element) -> bool:
    geometry_names = {"Polygon", "PolygonPatch", "Surface", "MultiSurface", "CompositeSurface", "Solid",
                      "MultiSolid", "Point", "MultiPoint", "LineString", "Curve", "MultiCurve",
                      "pos", "posList", "coordinates", "coord"}
    return any(local(node.tag) in geometry_names for node in elem.iter())


def rings_from_polygon(elem: ET.Element) -> Polygon | None:
    def ring(parent: ET.Element | None) -> list[tuple[float, float]] | None:
        if parent is None:
            raise InvalidGeometry("missing polygon ring")
        pos = parent.find(".//{%s}posList" % GML)
        if pos is not None and pos.text:
            vals = [float(v) for v in pos.text.split()]
            dim = int(pos.attrib.get("srsDimension", pos.attrib.get("{%s}srsDimension" % GML, "3")))
            if dim != 3 or len(vals) % dim or not all(math.isfinite(v) for v in vals):
                raise ValueError("unsupported or malformed coordinate dimensions")
            # EPSG:6697 axis order is latitude, longitude, height. Transformer
            # uses always_xy, so store the horizontal pair as longitude, latitude.
            coords = [(vals[i + 1], vals[i]) for i in range(0, len(vals), dim)]
            if len(coords) < 4 or coords[0] != coords[-1]:
                raise InvalidGeometry("unclosed or underspecified polygon ring")
            return coords
        pts = parent.findall(".//{%s}pos" % GML)
        if pts:
            out = []
            for p in pts:
                vals = [float(v) for v in (p.text or "").split()]
                if len(vals) != 3 or not all(math.isfinite(v) for v in vals):
                    raise ValueError("unsupported coordinate dimensions")
                out.append((vals[1], vals[0]))
            if len(out) < 4 or out[0] != out[-1]:
                raise InvalidGeometry("unclosed or underspecified polygon ring")
            return out
        raise InvalidGeometry("polygon ring has no coordinates")

    exterior = ring(elem.find("{%s}exterior" % GML))
    interiors = [ring(x) for x in elem.findall("{%s}interior" % GML)]
    return Polygon(exterior, interiors)


def building_geometry(building: ET.Element) -> tuple[object | None, str | None]:
    fp = building.find("{%s}lod0FootPrint" % BLDG)
    re = building.find("{%s}lod0RoofEdge" % BLDG)
    source = fp if fp is not None else re
    semantic = "lod0FootPrint" if fp is not None else "lod0RoofEdge"
    if source is None:
        return None, None
    if any("{%s}href" % XLINK in node.attrib for node in source.iter()):
        raise ValueError("xlink geometry is unsupported")
    for node in source.iter():
        declared_crs = node.attrib.get("srsName")
        declared_dim = node.attrib.get("srsDimension")
        if declared_crs and declared_crs != "http://www.opengis.net/def/crs/EPSG/0/6697":
            raise ValueError("unsupported nested geometry CRS")
        if declared_dim and declared_dim != "3":
            raise ValueError("unsupported nested coordinate dimension")
    polys = []
    for pe in source.iter():
        if pe.tag == "{%s}Polygon" % GML:
            p = rings_from_polygon(pe)
            if not p.is_valid:
                raise InvalidGeometry("invalid polygon")
            polys.append(p)
    if not polys:
        return None, semantic
    return unary_union(polys), semantic


class InvalidGeometry(Exception):
    pass


def numeric_field(building: ET.Element, name: str) -> tuple[float | int | None, str | None, str]:
    e = building.find("{%s}%s" % (BLDG, name))
    raw = e.text.strip() if e is not None and e.text else None
    if raw is None:
        return None, None, "missing"
    try:
        n = float(raw)
    except ValueError:
        return None, raw, "invalid"
    if not math.isfinite(n) or n < 0:
        return None, raw, "invalid"
    # PLATEAU uses 9999 as an unknown-value marker in several attributes.
    if n == 9999:
        return None, raw, "sentinel"
    if name == "measuredHeight":
        if e.attrib.get("uom") != "m":
            return None, raw, "invalid-unit"
        return n, raw, "valid"
    if not n.is_integer():
        return None, raw, "invalid"
    return int(n), raw, "valid"


def metrics_for_building(building: ET.Element, geom, semantic: str, to_metric, crop, cx, cy):
    geom = transform(to_metric, geom)
    clipped = geom.intersection(crop)
    if clipped.is_empty:
        return None
    rect = geom.minimum_rotated_rectangle
    coords = list(rect.exterior.coords) if rect.geom_type == "Polygon" else []
    edges = [math.dist(coords[i], coords[i + 1]) for i in range(4)] if len(coords) >= 5 else []
    aspect = max(edges) / min(edges) if edges and min(edges) > 0 else None
    gmlid = building.attrib.get("{%s}id" % GML)
    fields = {}
    for name in ("measuredHeight", "storeysAboveGround", "storeysBelowGround"):
        value, raw, status = numeric_field(building, name)
        fields[name] = value
        fields[name + "Raw"] = raw
        fields[name + "Status"] = status
    return {
        "id": gmlid,
        "geometrySource": semantic,
        "completeAreaM2": float(geom.area),
        "clippedAreaM2": float(clipped.area),
        "aspectRatio": float(aspect) if aspect is not None else None,
        **fields,
        "directLOD2": any(local(e.tag).startswith("lod2") for e in building),
        "touchesCropBoundary": bool(clipped.boundary.intersects(crop.boundary)),
        "clippedGeometry": mapping(transform(lambda x, y, z=None: (x - cx, y - cy), clipped)),
        "_completeGeometry": geom,
    }


def extract(input_path: Path, center_lon: float, center_lat: float, size: float) -> dict:
    if not all(math.isfinite(v) for v in (center_lon, center_lat, size)) or not -180 <= center_lon <= 180 or not -90 <= center_lat <= 90:
        raise ValueError("crop center and size must be finite geographic values")
    if size <= 0:
        raise ValueError("size must be positive")
    raw_hash = hashlib.sha256(input_path.read_bytes()).hexdigest()
    # Transform longitude/latitude to the Tokyo plane rectangular system, in metres.
    to_metric = Transformer.from_crs(SRC_CRS, OUT_CRS, always_xy=True).transform
    cx, cy = to_metric(center_lon, center_lat)
    if not math.isfinite(cx) or not math.isfinite(cy):
        raise ValueError("crop center is outside the supported projection domain")
    crop = box(cx - size / 2, cy - size / 2, cx + size / 2, cy + size / 2)
    seen: set[str] = set()
    features = []
    counts = {"buildings": 0, "duplicateIds": 0, "missingIds": 0, "missingGeometry": 0, "invalidPolygons": 0,
              "lod0FootPrint": 0, "lod0RoofEdge": 0, "directLOD2": 0}
    member_types: dict[str, int] = {}
    nonbuilding_objects = []
    unsupported_nonbuilding_geometry = 0
    with gzip.open(input_path, "rb") as stream:
        root = None
        stack = []
        envelope_seen = False
        for event, elem in ET.iterparse(stream, events=("start", "end")):
            if event == "start":
                stack.append(elem)
                if root is None:
                    root = elem
                    validate_declared_crs(elem, "CityModel root")
                continue
            parent = stack[-2] if len(stack) > 1 else None
            if elem.tag == "{%s}Envelope" % GML:
                if not (parent is not None and parent.tag == "{%s}boundedBy" % GML and
                        len(stack) >= 3 and stack[-3] is root):
                    raise ValueError("source envelope must be a direct root boundedBy child")
                if envelope_seen:
                    raise ValueError("multiple source envelopes are unsupported")
                envelope_seen = True
                if elem.attrib.get("srsName") != "http://www.opengis.net/def/crs/EPSG/0/6697":
                    raise ValueError("unsupported or missing source CRS; expected EPSG:6697")
                if elem.attrib.get("srsDimension") != "3":
                    raise ValueError("source envelope must declare srsDimension=3")
            if elem.tag == "{%s}cityObjectMember" % CORE:
                if not envelope_seen:
                    raise ValueError("source envelope must precede cityObjectMember records")
                validate_declared_crs(elem, "cityObjectMember")
                for obj in elem:
                    type_name = local(obj.tag)
                    member_types[type_name] = member_types.get(type_name, 0) + 1
                    validate_declared_crs(obj, "top-level city object")
                    if obj.tag != "{%s}Building" % BLDG:
                        refs = sorted({node.attrib["{%s}href" % XLINK] for node in obj.iter()
                                       if "{%s}href" % XLINK in node.attrib})
                        nonbuilding_objects.append({"id": obj.attrib.get("{%s}id" % GML),
                                                    "type": type_name, "parentReferences": refs})
                        if has_geometry_payload(obj):
                            unsupported_nonbuilding_geometry += 1
                        continue
                    counts["buildings"] += 1
                    ident = obj.attrib.get("{%s}id" % GML)
                    if not ident:
                        counts["missingIds"] += 1
                        continue
                    if ident in seen:
                        counts["duplicateIds"] += 1
                        continue
                    seen.add(ident)
                    direct_lod2 = any(local(e.tag).startswith("lod2") for e in obj)
                    counts["directLOD2"] += int(direct_lod2)
                    try:
                        geom, semantic = building_geometry(obj)
                    except InvalidGeometry:
                        counts["invalidPolygons"] += 1
                        continue
                    if geom is None:
                        counts["missingGeometry"] += 1
                        continue
                    counts[semantic] += 1
                    item = metrics_for_building(obj, geom, semantic, to_metric, crop, cx, cy)
                    if item:
                        features.append(item)
            if elem.tag == "{%s}cityObjectMember" % CORE:
                elem.clear()
                if root is not None:
                    root.remove(elem)
            stack.pop()
    if not envelope_seen:
        raise ValueError("missing source CRS envelope")
    from shapely.geometry import shape
    union = unary_union([shape(f["clippedGeometry"]) for f in features]) if features else None
    union_area = float(union.area) if union else 0.0
    # Each selected building is measured against all other selected complete
    # footprints. Other buildings outside the finite window remain unobserved.
    nearest_distances = []
    complete_geometries = [feature.pop("_completeGeometry") for feature in features]
    for i, feature in enumerate(features):
        geom = complete_geometries[i]
        others = [other for j, other in enumerate(complete_geometries) if i != j]
        nearest_distances.append({"id": feature["id"], "distanceM": float(min(geom.distance(other) for other in others)) if others else None})
    valid_distances = sorted(row["distanceM"] for row in nearest_distances if row["distanceM"] is not None)
    def percentile(fraction):
        if not valid_distances:
            return None
        pos = fraction * (len(valid_distances) - 1)
        low, high = int(pos), math.ceil(pos)
        return float(valid_distances[low] + (valid_distances[high] - valid_distances[low]) * (pos - low))
    return {
        "source": {"path": input_path.name, "sha256": raw_hash, "crs": "EPSG:6697"},
        "projection": {"inputHorizontal": SRC_CRS, "metric": OUT_CRS},
        "crop": {"centerLon": center_lon, "centerLat": center_lat, "sizeM": size,
                 "boundsMetric": [cx - size / 2, cy - size / 2, cx + size / 2, cy + size / 2],
                 "localCoordinateOriginMetric": [cx, cy], "localCoordinateOrigin": "crop center"},
        "counts": counts,
        "topLevelMemberTypeCounts": member_types,
        "topLevelNonBuildingObjects": nonbuilding_objects,
        "unsupportedNonBuildingGeometryCount": unsupported_nonbuilding_geometry,
        "selectedBuildingCount": len(features),
        "clippedUnionAreaM2": union_area,
        "nearestSelectedBuildingDistances": nearest_distances,
        "nearestSelectedBuildingDistanceQuantilesM": {"p10": percentile(.1), "p50": percentile(.5), "p90": percentile(.9)},
        "nearestDistancesWindowCensored": True,
        "coverageIncomplete": (unsupported_nonbuilding_geometry > 0 or
                               any(counts[k] > 0 for k in ("missingIds", "duplicateIds", "missingGeometry", "invalidPolygons"))),
        "features": features,
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--input", required=True, type=Path)
    ap.add_argument("--center-lon", required=True, type=float)
    ap.add_argument("--center-lat", required=True, type=float)
    ap.add_argument("--size", type=float, default=250)
    ap.add_argument("--output", required=True, type=Path)
    args = ap.parse_args(argv)
    try:
        result = extract(args.input, args.center_lon, args.center_lat, args.size)
        args.output.write_text(json.dumps(result, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf8")
    except Exception as exc:
        print(f"extract-plateau-crop: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
