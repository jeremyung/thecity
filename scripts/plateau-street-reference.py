"""Street-network morphology from PLATEAU transportation (tran) meshes.

Offline reference tooling only. For each 1 km third-order mesh it:
  * reads LOD1 road-surface polygons from tran:Road features,
  * projects them to EPSG:6677 metres and unions them,
  * derives blocks as the mesh area not covered by road surface,
  * measures road share, block size/shape and street orientation entropy,
  * optionally overlays LOD0 building footprints for a figure-ground SVG.

The same measures run on a synthetic plan exported by
`cargo test --release masterplan::svg::export_plan_json -- --ignored`.

Usage:
  .tools/plateau-python/bin/python scripts/plateau-street-reference.py \
      --source NAME=MESHCODE,tran.gml.gz[,bldg.gml.gz] ... \
      --plan NAME=plan.json ... --out validation/street-reference
"""

import argparse
import gzip
import json
import math
import os
import xml.etree.ElementTree as ET

import numpy as np
from pyproj import Transformer
from shapely import affinity
from shapely.geometry import LineString, Polygon, box
from shapely.ops import unary_union

GML = "{http://www.opengis.net/gml}"
TRAN = "{http://www.opengis.net/citygml/transportation/2.0}"
BLDG = "{http://www.opengis.net/citygml/building/2.0}"
TO_METRIC = Transformer.from_crs("EPSG:6668", "EPSG:6677", always_xy=True)
ORIENTATION_BINS = 36
GRID_ENTROPY = math.log(4)
MAX_ENTROPY = math.log(ORIENTATION_BINS)


def mesh_bounds(code):
    """Lon/lat bounds of a JIS X 0410 third-order mesh code."""
    p, u, q, v, r, w = int(code[:2]), int(code[2:4]), int(code[4]), int(code[5]), int(code[6]), int(code[7])
    lat = p / 1.5 + q * 5 / 60 + r * 30 / 3600
    lon = 100 + u + v * 7.5 / 60 + w * 45 / 3600
    return lon, lat, lon + 45 / 3600, lat + 30 / 3600


def ring(pos_list):
    values = [float(x) for x in pos_list.split()]
    points = [TO_METRIC.transform(values[i + 1], values[i]) for i in range(0, len(values), 3)]
    return points if len(points) >= 4 else None


def polygons(path, feature_tag, surface_tag):
    """Exterior rings of LOD surfaces under each feature."""
    out = []
    with gzip.open(path, "rb") as handle:
        for _, elem in ET.iterparse(handle, events=("end",)):
            if elem.tag != feature_tag:
                continue
            for surface in elem.iter(surface_tag):
                for polygon in surface.iter(f"{GML}Polygon"):
                    exterior = polygon.find(f"{GML}exterior/{GML}LinearRing/{GML}posList")
                    if exterior is None:
                        continue
                    points = ring(exterior.text)
                    if points:
                        shape = Polygon(points).buffer(0)
                        if not shape.is_empty and shape.area > 0.5:
                            out.append(shape)
            elem.clear()
    return out


def orientation_entropy(geometries):
    """Length-weighted entropy of edge bearings over 36 bins (Boeing 2019)."""
    histogram = np.zeros(ORIENTATION_BINS)
    for geometry in geometries:
        coords = list(geometry.coords)
        for (x0, y0), (x1, y1) in zip(coords, coords[1:]):
            length = math.hypot(x1 - x0, y1 - y0)
            if length < 2.0:
                continue
            bearing = math.degrees(math.atan2(x1 - x0, y1 - y0)) % 180.0
            histogram[int(bearing // (180.0 / ORIENTATION_BINS)) % ORIENTATION_BINS] += length
    if histogram.sum() == 0:
        return None, histogram
    p = histogram / histogram.sum()
    p = p[p > 0]
    return float(-(p * np.log(p)).sum()), histogram


def block_metrics(domain, road_union, edge_margin=1.0, park_area=30000.0):
    """Blocks are the domain minus road surface; edge-touching pieces are truncated."""
    remainder = domain.difference(road_union)
    pieces = list(getattr(remainder, "geoms", [remainder]))
    inner = domain.buffer(-edge_margin)
    blocks = [b for b in pieces if b.area >= 40.0 and b.within(inner) and b.area <= park_area]
    areas = np.array([b.area for b in blocks]) if blocks else np.array([0.0])
    rectangularity = []
    vertices = []
    boundaries = []
    for block in blocks:
        simplified = block.simplify(1.5, preserve_topology=True)
        rectangularity.append(block.area / simplified.minimum_rotated_rectangle.area)
        vertices.append(len(simplified.exterior.coords) - 1)
        boundaries.append(simplified.exterior)
    entropy, histogram = orientation_entropy(boundaries)
    order = None
    if entropy is not None:
        order = 1 - ((entropy - GRID_ENTROPY) / (MAX_ENTROPY - GRID_ENTROPY)) ** 2
    q = lambda values, p: float(np.percentile(values, p)) if len(values) else None
    return {
        "blocks": len(blocks),
        "blocksPerKm2": len(blocks) / (domain.area / 1e6),
        "blockAreaM2": {"p10": q(areas, 10), "p50": q(areas, 50), "p90": q(areas, 90)},
        "rectangularityP50": q(rectangularity, 50),
        "rectangularShare": float(np.mean(np.array(rectangularity) > 0.92)) if rectangularity else None,
        "verticesP50": q(vertices, 50),
        "nonQuadShare": float(np.mean(np.array(vertices) != 4)) if vertices else None,
        "orientationEntropy": entropy,
        "orientationOrder": order,
        "orientationHistogram": [round(float(x), 1) for x in histogram],
    }, blocks


def svg(path, domain, road_union, blocks, buildings, title):
    minx, miny, maxx, maxy = domain.bounds
    width = maxx - minx
    height = maxy - miny

    def path_of(geometry):
        parts = []
        for poly in getattr(geometry, "geoms", [geometry]):
            if poly.is_empty:
                continue
            for ring_coords in [poly.exterior.coords] + [r.coords for r in poly.interiors]:
                parts.append("M" + " L".join(f"{x - minx:.1f},{maxy - y:.1f}" for x, y in ring_coords) + "Z")
        return " ".join(parts)

    body = [f'<rect width="{width:.0f}" height="{height:.0f}" fill="#e9e6dc"/>']
    body.append(f'<path d="{path_of(unary_union(blocks))}" fill="#d6d1c2"/>' if blocks else "")
    body.append(f'<path d="{path_of(road_union)}" fill="#ffffff" stroke="#b8b4a8" stroke-width="0.4"/>')
    if buildings:
        body.append(f'<path d="{path_of(unary_union(buildings))}" fill="#4a5a60"/>')
    with open(path, "w") as handle:
        handle.write(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width:.0f} {height + 28:.0f}" width="{width:.0f}" height="{height + 28:.0f}">'
            f'<g transform="translate(0,28)">{"".join(body)}</g>'
            f'<text x="6" y="19" font-family="sans-serif" font-size="16">{title} · {width:.0f} × {height:.0f} m</text></svg>'
        )


def analyse_source(name, code, tran_path, bldg_path, out_dir):
    roads = polygons(tran_path, f"{TRAN}Road", f"{TRAN}lod1MultiSurface")
    road_union = unary_union(roads)
    lon0, lat0, lon1, lat1 = mesh_bounds(code)
    corners = [TO_METRIC.transform(x, y) for x, y in [(lon0, lat0), (lon1, lat0), (lon1, lat1), (lon0, lat1)]]
    domain = Polygon(corners)
    road_union = road_union.intersection(domain)
    metrics, blocks = block_metrics(domain, road_union)
    buildings = []
    if bldg_path:
        buildings = [b.intersection(domain) for b in polygons(bldg_path, f"{BLDG}Building", f"{BLDG}lod0RoofEdge")]
        if not buildings:
            buildings = [b.intersection(domain) for b in polygons(bldg_path, f"{BLDG}Building", f"{BLDG}lod0FootPrint")]
        metrics["buildingCoveragePercent"] = 100 * unary_union(buildings).area / domain.area
    metrics["roadSharePercent"] = 100 * road_union.area / domain.area
    metrics["domainM2"] = domain.area
    svg(os.path.join(out_dir, f"{name}.svg"), domain, road_union, blocks, buildings, f"PLATEAU {name}")
    return metrics


def analyse_plan(name, plan_path, out_dir):
    plan = json.load(open(plan_path))
    nodes = plan["nodes"]
    widths = {"arterial": 21.0, "bridge": 18.8, "local": 6.5, "lane": 4.0}
    lines = []
    for road in plan["roads"]:
        width = widths.get(road["kind"])
        if width is None:
            continue
        a, b = nodes[road["a"]], nodes[road["b"]]
        lines.append(LineString([(a[0], a[1]), (b[0], b[1])]).buffer(width / 2, cap_style=2))
    extent = plan["extent"]
    domain = box(-extent, -extent, extent, extent)
    river = LineString([tuple(p) for p in plan["river"]]).buffer(plan["riverHalfWidth"])
    road_union = unary_union(lines + [river]).intersection(domain)
    metrics, blocks = block_metrics(domain, road_union)
    lots = []
    for lot in plan["lots"]:
        x, z, w, d, fx, fz = lot
        rect = box(-w / 2, -d / 2, w / 2, d / 2)
        rect = affinity.rotate(rect, math.degrees(math.atan2(fx, fz)) * -1 + 0, origin=(0, 0))
        lots.append(affinity.translate(rect, x, z))
    metrics["lotCoveragePercent"] = 100 * unary_union(lots).area / domain.area
    metrics["roadSharePercent"] = 100 * unary_union(lines).intersection(domain).area / domain.area
    metrics["domainM2"] = domain.area
    svg(os.path.join(out_dir, f"{name}.svg"), domain, road_union, blocks, lots, f"Synthetic {name}")
    return metrics


DRIVE = {"motorway", "trunk", "primary", "secondary", "tertiary", "unclassified", "residential", "living_street", "service",
         "motorway_link", "trunk_link", "primary_link", "secondary_link", "tertiary_link", "road"}
WALK = DRIVE | {"pedestrian", "footway", "path", "steps"}


def network_metrics(points, edges, domain, edge_margin=30.0):
    """Junction and segment statistics for a planar street graph.

    `points` maps node id -> (x, y); `edges` is a list of (a, b) pairs.
    Nodes within `edge_margin` of the domain boundary are ignored for degree
    statistics because their neighbours were clipped away.
    """
    degree = {}
    for a, b in edges:
        degree[a] = degree.get(a, 0) + 1
        degree[b] = degree.get(b, 0) + 1
    inner = domain.buffer(-edge_margin)
    from shapely.geometry import Point
    interior = {n for n in degree if inner.contains(Point(points[n]))}
    junctions = [n for n in interior if degree[n] >= 3]
    three = sum(1 for n in junctions if degree[n] == 3)
    four = sum(1 for n in junctions if degree[n] == 4)
    dead = sum(1 for n in interior if degree[n] == 1)
    km2 = inner.area / 1e6
    lines = [LineString([points[a], points[b]]) for a, b in edges]
    entropy, histogram = orientation_entropy(lines)
    # Segment length between junctions/dead ends: walk chains through degree-2 nodes.
    adjacency = {}
    for a, b in edges:
        adjacency.setdefault(a, []).append(b)
        adjacency.setdefault(b, []).append(a)
    seen = set()
    segments = []
    for start in degree:
        if degree[start] == 2:
            continue
        for nxt in adjacency[start]:
            key = (min(start, nxt), max(start, nxt))
            if key in seen:
                continue
            length = 0.0
            prev, cur = start, nxt
            seen.add(key)
            length += math.dist(points[prev], points[cur])
            while degree.get(cur) == 2:
                following = [n for n in adjacency[cur] if n != prev]
                if not following:
                    break
                prev, cur = cur, following[0]
                seen.add((min(prev, cur), max(prev, cur)))
                length += math.dist(points[prev], points[cur])
            segments.append(length)
    order = None if entropy is None else 1 - ((entropy - GRID_ENTROPY) / (MAX_ENTROPY - GRID_ENTROPY)) ** 2
    return {
        "junctionsPerKm2": len(junctions) / km2,
        "threeWayShare": three / max(1, len(junctions)),
        "fourWayShare": four / max(1, len(junctions)),
        "deadEndsPerKm2": dead / km2,
        "segmentLengthP50": float(np.percentile(segments, 50)) if segments else None,
        "streetOrientationEntropy": entropy,
        "streetOrientationOrder": order,
        "streetLengthKmPerKm2": sum(line.length for line in lines) / 1000 / (domain.area / 1e6),
    }


def analyse_osm(name, code, path):
    data = json.load(open(path))
    lon0, lat0, lon1, lat1 = mesh_bounds(code)
    domain = Polygon([TO_METRIC.transform(x, y) for x, y in [(lon0, lat0), (lon1, lat0), (lon1, lat1), (lon0, lat1)]])
    nodes = {e["id"]: TO_METRIC.transform(e["lon"], e["lat"]) for e in data["elements"] if e["type"] == "node"}
    result = {}
    for label, kinds in (("drive", DRIVE), ("walk", WALK)):
        edges = set()
        for way in (e for e in data["elements"] if e["type"] == "way"):
            tags = way.get("tags", {})
            if tags.get("highway") not in kinds or tags.get("area") == "yes":
                continue
            if label == "drive" and tags.get("service") in {"parking_aisle", "drive-through"}:
                continue
            refs = [r for r in way["nodes"] if r in nodes]
            for a, b in zip(refs, refs[1:]):
                if a != b:
                    edges.add((min(a, b), max(a, b)))
        result[label] = network_metrics(nodes, sorted(edges), domain)
    return result


def analyse_plan_network(plan_path):
    plan = json.load(open(plan_path))
    points = {i: (p[0], p[1]) for i, p in enumerate(plan["nodes"])}
    edges = [(r["a"], r["b"]) for r in plan["roads"] if r["kind"] in {"arterial", "local", "lane", "bridge"}]
    extent = plan["extent"]
    return network_metrics(points, edges, box(-extent, -extent, extent, extent))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", action="append", default=[])
    parser.add_argument("--plan", action="append", default=[])
    parser.add_argument("--osm", action="append", default=[], help="NAME=MESHCODE,overpass.json")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)
    report = {
        "method": "Blocks = mesh/extent minus road surface (PLATEAU tran LOD1, or synthetic carriageway+sidewalk buffers plus river). Blocks touching the domain edge or larger than 30,000 m2 are excluded. Orientation entropy uses length-weighted bearings of 1.5 m-simplified block boundaries in 36 bins (Boeing 2019); grid = ln 4, uniform = ln 36. Orientation order 1 = perfect grid, 0 = uniform.",
        "sources": {},
        "osm": {},
        "plans": {},
        "attribution": "PLATEAU / MLIT (CC BY 4.0); OpenStreetMap contributors (ODbL), Overpass API extracts. Derived statistics only.",
    }
    for spec in args.osm:
        name, paths = spec.split("=", 1)
        code, path = paths.split(",")
        report["osm"][name] = analyse_osm(name, code, path)
        print("osm", name, json.dumps(report["osm"][name]))
    for spec in args.source:
        name, paths = spec.split("=", 1)
        code, tran, *rest = paths.split(",")
        report["sources"][name] = analyse_source(name, code, tran, rest[0] if rest else None, args.out)
        print(name, {k: v for k, v in report["sources"][name].items() if k != "orientationHistogram"})
    for spec in args.plan:
        name, path = spec.split("=", 1)
        report["plans"][name] = analyse_plan(name, path, args.out)
        report["plans"][name]["network"] = analyse_plan_network(path)
        print(name, {k: v for k, v in report["plans"][name].items() if k != "orientationHistogram"})
    with open(os.path.join(args.out, "street-metrics.json"), "w") as handle:
        json.dump(report, handle, indent=2)


if __name__ == "__main__":
    main()
