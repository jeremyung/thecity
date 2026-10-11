#!/usr/bin/env python3
"""Focused synthetic checks for extract-plateau-crop.py."""
import gzip
import importlib.util
import json
import math
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("extract-plateau-crop.py")
spec = importlib.util.spec_from_file_location("plateau_crop", SCRIPT)
pc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pc)

GML = pc.GML
CORE = pc.CORE
BLDG = pc.BLDG


def ring(coords):
    return "<gml:LinearRing><gml:posList srsDimension='3'>" + " ".join(
        f"{lat} {lon} 10" for lon, lat in coords
    ) + "</gml:posList></gml:LinearRing>"


def polygon(coords, holes=()):
    return "<gml:Polygon><gml:exterior>" + ring(coords) + "</gml:exterior>" + "".join(
        "<gml:interior>" + ring(h) + "</gml:interior>" for h in holes
    ) + "</gml:Polygon>"


def building(gid, geometry="", lod="lod0FootPrint", fields=""):
    geom = f"<bldg:{lod}><gml:MultiSurface><gml:surfaceMember>{geometry}</gml:surfaceMember></gml:MultiSurface></bldg:{lod}>" if geometry else ""
    return f"<bldg:Building gml:id='{gid}'>{fields}{geom}</bldg:Building>"


def dataset(buildings, crs="http://www.opengis.net/def/crs/EPSG/0/6697", dim="3", envelope=True):
    env = (f"<gml:boundedBy><gml:Envelope srsName='{crs}' srsDimension='{dim}'>"
           "<gml:lowerCorner>35.0 139.0 0</gml:lowerCorner><gml:upperCorner>36.0 140.0 100</gml:upperCorner>"
           "</gml:Envelope></gml:boundedBy>") if envelope else ""
    return (f"<core:CityModel xmlns:core='{CORE}' xmlns:gml='{GML}' xmlns:bldg='{BLDG}'>"
            + env
            + "".join(f"<core:cityObjectMember>{b}</core:cityObjectMember>" for b in buildings)
            + "</core:CityModel>")


class ExtractTests(unittest.TestCase):
    def run_doc(self, xml, center=(139.75, 35.68), size=250):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        path = Path(tmp.name) / "source.gml.gz"
        with gzip.open(path, "wt", encoding="utf8") as f:
            f.write(xml)
        return pc.extract(path, *center, size)

    def test_hole_preserved_and_area_is_finite(self):
        # Center in the building's approximate position; a small crop contains it.
        outer = [(139.749, 35.679), (139.751, 35.679), (139.751, 35.681), (139.749, 35.681), (139.749, 35.679)]
        hole = [(139.7495, 35.6795), (139.7505, 35.6795), (139.7505, 35.6805), (139.7495, 35.6805), (139.7495, 35.6795)]
        doc = self.run_doc(dataset([building("a", polygon(outer, [hole]))]), size=250)
        feature = doc["features"][0]
        self.assertEqual(len(feature["clippedGeometry"]["coordinates"]), 2)
        self.assertTrue(math.isfinite(feature["completeAreaM2"]))
        self.assertAlmostEqual(feature["completeAreaM2"], feature["clippedAreaM2"], delta=0.01)
        self.assertEqual(doc["nearestDistancesWindowCensored"], True)
        # First local coordinate lies within roughly half the 250m crop bounds.
        point = feature["clippedGeometry"]["coordinates"][0][0]
        self.assertLessEqual(abs(point[0]), 125.01)
        self.assertLessEqual(abs(point[1]), 125.01)

    def test_union_and_boundary_intersection(self):
        a = [(139.749, 35.679), (139.7505, 35.679), (139.7505, 35.681), (139.749, 35.681), (139.749, 35.679)]
        b = [(139.7505, 35.679), (139.752, 35.679), (139.752, 35.681), (139.7505, 35.681), (139.7505, 35.679)]
        doc = self.run_doc(dataset([building("a", polygon(a)), building("b", polygon(b))]), size=250)
        self.assertEqual(doc["selectedBuildingCount"], 2)
        self.assertGreater(doc["clippedUnionAreaM2"], 0)
        self.assertEqual(len(doc["nearestSelectedBuildingDistances"]), 2)
        self.assertTrue(all(row["distanceM"] == 0 for row in doc["nearestSelectedBuildingDistances"]))
        self.assertEqual(doc["nearestSelectedBuildingDistanceQuantilesM"]["p50"], 0)

    def test_dedup_missing_fields_and_lod0_fallback(self):
        coords = [(139.749, 35.679), (139.751, 35.679), (139.751, 35.681), (139.749, 35.681), (139.749, 35.679)]
        doc = self.run_doc(dataset([building("a", polygon(coords), lod="lod0RoofEdge"),
                                   building("a", polygon(coords)), building("missing")]))
        self.assertEqual(doc["counts"]["duplicateIds"], 1)
        self.assertEqual(doc["counts"]["missingGeometry"], 1)
        self.assertTrue(doc["coverageIncomplete"])
        self.assertEqual(doc["features"][0]["geometrySource"], "lod0RoofEdge")
        self.assertIsNone(doc["features"][0]["measuredHeight"])

    def test_crs_dimension_and_missing_envelope_fail_closed(self):
        with self.assertRaisesRegex(ValueError, "unsupported or missing source CRS"):
            self.run_doc(dataset([], crs="EPSG:4326"))
        with self.assertRaisesRegex(ValueError, "srsDimension=3"):
            self.run_doc(dataset([], dim="2"))
        with self.assertRaisesRegex(ValueError, "missing source CRS envelope"):
            self.run_doc(dataset([], envelope=False))

    def test_nested_crs_override_and_unclosed_hole_fail_closed(self):
        coords = [(139.749, 35.679), (139.751, 35.679), (139.751, 35.681), (139.749, 35.681), (139.749, 35.679)]
        override = polygon(coords).replace("<gml:Polygon>", "<gml:Polygon srsName='EPSG:4326'>")
        with self.assertRaisesRegex(ValueError, "nested geometry CRS"):
            self.run_doc(dataset([building("a", override)]))
        open_hole = [(139.7495, 35.6795), (139.7505, 35.6795), (139.7505, 35.6805), (139.7495, 35.6805)]
        doc = self.run_doc(dataset([building("a", polygon(coords, [open_hole]))]))
        self.assertEqual(doc["counts"]["invalidPolygons"], 1)
        self.assertTrue(doc["coverageIncomplete"])

    def test_missing_id_and_invalid_numeric_fields(self):
        coords = [(139.749, 35.679), (139.751, 35.679), (139.751, 35.681), (139.749, 35.681), (139.749, 35.679)]
        attrs = "<bldg:measuredHeight uom='ft'>12</bldg:measuredHeight><bldg:storeysAboveGround>1.5</bldg:storeysAboveGround><bldg:storeysBelowGround>-1</bldg:storeysBelowGround>"
        doc = self.run_doc(dataset([building("", polygon(coords)), building("a", polygon(coords), fields=attrs)]))
        self.assertTrue(doc["coverageIncomplete"])
        self.assertEqual(doc["counts"]["missingIds"], 1)
        feature = doc["features"][0]
        self.assertIsNone(feature["measuredHeight"])
        self.assertIsNone(feature["storeysAboveGround"])
        self.assertIsNone(feature["storeysBelowGround"])


if __name__ == "__main__":
    unittest.main()
