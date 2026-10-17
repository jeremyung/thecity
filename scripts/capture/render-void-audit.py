"""Render approximate empty-space masks against authoritative saved parcels."""
import json
import math
from pathlib import Path

report = json.loads(Path('tests/fixtures/validation/density-void-audit.json').read_text())
world = json.loads(Path(report['input']).read_text())
nodes = {n['id']: n for n in world['nodes']}
svg = ['<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="1000" viewBox="0 0 1080 1000">',
       '<rect width="1080" height="1000" fill="#13202b"/>',
       '<g font-family="sans-serif" fill="#e5eef1"><text x="40" y="40" font-size="25">Mature city · unbuilt-space audit</text>',
       f'<text x="40" y="66" font-size="14">Seed {report["seed"]} · tick {report["tick"]} · 2 m sampling · highlighted areas are not proven buildable</text></g>',
       '<g transform="translate(440 490)"><rect x="-400" y="-400" width="800" height="800" fill="#26383c" stroke="#6a858b"/>']
colors = ['#e6a561', '#58c2bf', '#b59ae7']
for v, color in zip(report['top3VoidComponents'], colors):
    for z, runs in v['void_mask_2m_rle']['x_cell_index_runs_by_z_index'].items():
        for a, b in runs:
            svg.append(f'<rect x="{a*2}" y="{int(z)*2}" width="{(b-a+1)*2}" height="2" fill="{color}"/>')
for r in world['roads']:
    if r['stage'] != 'open': continue
    a, b = nodes[r['from']], nodes[r['to']]
    svg.append(f'<path d="M{a["x"]},{a["z"]}L{b["x"]},{b["z"]}" stroke="#829096" stroke-width="{3 if r["kind"]=="rail" else 7 if r["kind"]=="arterial" else 4}"/>')
for p in world['parcels']:
    angle = p.get('rotation', 0)*180/math.pi
    # Three.js rotation about Y maps local x,z to (cos*x+sin*z,-sin*x+cos*z).
    svg.append(f'<rect x="{-p["width"]/2}" y="{-p["depth"]/2}" width="{p["width"]}" height="{p["depth"]}" transform="translate({p["x"]} {p["z"]}) rotate({-angle})" fill="#c8ccbe" stroke="#26383c" stroke-width=".6"/>')
for i, (v, color) in enumerate(zip(report['top3VoidComponents'], colors), 1):
    x, z = v['centroid']
    svg.append(f'<circle cx="{x}" cy="{z}" r="13" fill="#13202b" stroke="{color}" stroke-width="2"/><text x="{x}" y="{z+5}" text-anchor="middle" font-family="sans-serif" font-size="14" fill="white">{i}</text>')
svg.append('</g><g font-family="sans-serif" font-size="14" fill="#e5eef1">')
for i,(v,color) in enumerate(zip(report['top3VoidComponents'],colors),1):
    y=130+i*76
    svg.append(f'<text x="865" y="{y}" fill="{color}">{i} · {v["area"]:,} m²</text><text x="865" y="{y+23}" font-size="12">Unbuilt mask area</text>')
svg.extend(['<text x="40" y="930">Light footprints: saved parcel envelopes · gray lines: open roads</text>',
            '<text x="40" y="955">Large components contain narrow passages and holes. Total area is not a usable lot or road envelope.</text>',
            '</g></svg>'])
Path('validation/density-void-map.svg').write_text('\n'.join(svg))
