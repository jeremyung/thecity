#!/usr/bin/env python3
"""Render offline morphology diagrams from the committed pilot JSON (stdlib only)."""
import argparse
import html
import json
import math
from pathlib import Path


def quantiles(values):
    values = sorted(v for v in values if v is not None)
    def q(f):
        if not values:
            return None
        p = f * (len(values) - 1)
        lo, hi = math.floor(p), math.ceil(p)
        return values[lo] + (values[hi] - values[lo]) * (p - lo)
    return {'known': len(values), 'p10': q(.1), 'p50': q(.5), 'p90': q(.9)}


def diagram(sample):
    parts = ['<svg xmlns="http://www.w3.org/2000/svg" width="720" height="850" viewBox="0 0 720 850">',
             '<rect width="720" height="850" fill="#f7f4ed"/>',
             '<g font-family="Arial,sans-serif" fill="#293c40">',
             f'<text x="40" y="40" font-size="24">{html.escape(sample["sampleId"].title())} · 250 × 250 m</text>',
             '<text x="40" y="66" font-size="13">PLATEAU roof-edge projections · measured height shading</text>',
             '<rect x="40" y="90" width="640" height="640" fill="#e5e3db"/>']
    size = sample['crop']['sizeM']
    def point(x, y):
        # Projection uses east/north; SVG y grows downward.
        return f'{40 + (x / size + .5) * 640:.3f},{90 + (.5 - y / size) * 640:.3f}'
    palette = ['#bfcebd', '#8facaa', '#5d878f', '#33576a']
    for feature in sample['features']:
        height = feature['measuredHeight']
        color = '#cdbca7' if height is None else palette[0 if height < 10 else 1 if height < 25 else 2 if height < 60 else 3]
        geom = feature['clippedGeometry']
        if geom['type'] not in ('Polygon', 'MultiPolygon'):
            raise ValueError('Unsupported diagram geometry: ' + geom['type'])
        polys = [geom['coordinates']] if geom['type'] == 'Polygon' else geom['coordinates']
        commands = []
        for polygon in polys:
            for ring in polygon:
                commands.append('M' + ' L'.join(point(*p[:2]) for p in ring) + ' Z')
        parts.append(f'<path d="{" ".join(commands)}" fill="{color}" fill-rule="evenodd" stroke="#f7f4ed" stroke-width="0.5"><title>{html.escape(feature["id"])}; height {height}</title></path>')
    parts += ['<rect x="40" y="90" width="640" height="640" fill="none" stroke="#293c40"/>',
              '<path d="M646 128 L646 105 L642 113 M646 105 L650 113" fill="none" stroke="#293c40"/><text x="657" y="116" font-size="12">N*</text>',
              '<path d="M40 750 L168 750 M40 746 L40 754 M168 746 L168 754" stroke="#293c40"/><text x="180" y="755" font-size="12">50 m · *grid north</text>']
    coverage = sample['clippedUnionAreaM2'] / size ** 2 * 100
    parts.append(f'<text x="40" y="783" font-size="14">{sample["selectedBuildingCount"]} buildings · gross roof-edge coverage {coverage:.2f}%</text>')
    parts.append('<text x="40" y="807" font-size="11">Height: pale &lt;10 m · medium 10–25 m · blue 25–60 m · dark ≥60 m · tan unknown</text>')
    parts.append('<text x="40" y="829" font-size="10">Project PLATEAU / MLIT · CC BY 4.0 · cropped and projected. Blank areas are unclassified.</text></g></svg>')
    return '\n'.join(parts) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, default=Path('validation/plateau-pilot'))
    args = parser.parse_args()
    data = json.loads((args.directory / 'morphology.json').read_text())
    summaries = []
    for sample in data['samples']:
        (args.directory / (sample['sampleId'] + '.svg')).write_text(diagram(sample))
        summaries.append({'sampleId': sample['sampleId'], 'selectedBuildingCount': sample['selectedBuildingCount'],
                         'grossRoofEdgeCoverage': sample['clippedUnionAreaM2'] / sample['crop']['sizeM'] ** 2,
                         'completeAreaM2': quantiles([f['completeAreaM2'] for f in sample['features']]),
                         'aspectRatio': quantiles([f['aspectRatio'] for f in sample['features']]),
                         'heightM': quantiles([f['measuredHeight'] for f in sample['features']]),
                         'storeysAboveGround': quantiles([f['storeysAboveGround'] for f in sample['features']]),
                         'selectedDirectLOD2Count': sum(f['directLOD2'] for f in sample['features']),
                         'nearestSelectedBuildingDistanceQuantilesM': sample['nearestSelectedBuildingDistanceQuantilesM'],
                         'nearestDistancesWindowCensored': True})
    (args.directory / 'summary.json').write_text(json.dumps({'quantileMethod': 'linear interpolation, index=(n-1)*p, missing excluded', 'calibrationAccepted': False, 'samples': summaries}, indent=2) + '\n')


if __name__ == '__main__':
    main()
