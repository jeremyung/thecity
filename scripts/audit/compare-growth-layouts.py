"""Compare saved world layouts without assuming matching entity IDs imply identity."""
import argparse
import html
import json
import math
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--baseline', required=True)
p.add_argument('--candidate', required=True)
p.add_argument('--output', required=True)
p.add_argument('--title', default='Growth layouts · same seed and elapsed simulation time')
p.add_argument('--subtitle', default='Nominal building envelopes, not a rendered scene. IDs are not matched across divergent worlds.')
p.add_argument('--candidate-label', default='Deeper spine candidate — rejected')
p.add_argument('--footer', default='Spatial differences do not prove which planning decision caused them; no demolition or displacement is inferred.')
a = p.parse_args()
worlds = [json.loads(Path(path).read_text()) for path in [a.baseline, a.candidate]]
assert worlds[0]['seed'] == worlds[1]['seed'] and worlds[0]['tick'] == worlds[1]['tick']
assert all(w['bounds'] == {'min_x': -400, 'max_x': 400, 'min_z': -400, 'max_z': 400} for w in worlds), 'This comparison uses the 800 m fixture domain'
out = Path(a.output)
out.parent.mkdir(parents=True, exist_ok=True)
parts = ['<svg xmlns="http://www.w3.org/2000/svg" width="1260" height="760" viewBox="0 0 1260 760">', '<rect width="1260" height="760" fill="#f1efe6"/>', '<style>text{font-family:Arial,sans-serif;fill:#23383c}.muted{fill:#56676b}</style>', f'<text x="30" y="35" font-size="22">{html.escape(a.title)}</text>', f'<text x="30" y="60" font-size="13" class="muted">{html.escape(a.subtitle)}</text>']
for index, (label, w) in enumerate(zip(['Baseline', a.candidate_label], worlds)):
    x0, y0, scale = 30 + index * 620, 115, .725
    def pt(x, z):
        return x0 + (x + 400) * scale, y0 + (z + 400) * scale
    parts.append(f'<text x="{x0}" y="93" font-size="17">{html.escape(label)} · {len(w["buildings"])} buildings</text>')
    parts.append(f'<rect x="{x0}" y="{y0}" width="580" height="580" fill="#e1e7d7"/>')
    parts.append(f'<defs><clipPath id="map{index}"><rect x="{x0}" y="{y0}" width="580" height="580"/></clipPath></defs><g clip-path="url(#map{index})">')
    river = ' '.join(f'{pt(q["x"],q["z"])[0]:.2f},{pt(q["x"],q["z"])[1]:.2f}' for q in w['terrain']['river'])
    parts.append(f'<polyline points="{river}" fill="none" stroke="#8bb5bd" stroke-width="{w["terrain"]["river_half_width"]*2*scale}"/>')
    nodes = {n['id']: n for n in w['nodes']}
    for road in w['roads']:
        u, v = nodes[road['from']], nodes[road['to']]
        x1, y1 = pt(u['x'], u['z']); x2, y2 = pt(v['x'], v['z'])
        width = 3 if road['kind'] == 'rail' else 5
        parts.append(f'<path d="M{x1:.2f},{y1:.2f} L{x2:.2f},{y2:.2f}" stroke="#a6aaa3" stroke-width="{width*scale}"/>')
    for b in w['buildings']:
        c, s = math.cos(b['rotation']), math.sin(b['rotation'])
        corners = [pt(b['x']+c*x+s*z,b['z']-s*x+c*z) for x,z in [(-b['width']/2,-b['depth']/2),(b['width']/2,-b['depth']/2),(b['width']/2,b['depth']/2),(-b['width']/2,b['depth']/2)]]
        points = ' '.join(f'{x:.2f},{y:.2f}' for x,y in corners)
        color = '#374f5b' if b['floors'] >= 12 else '#617984' if b['floors'] >= 4 else '#829497'
        parts.append(f'<polygon points="{points}" fill="{color}"/>')
    parts.append('</g>')
    parts.append(f'<text x="{x0}" y="717" font-size="12" class="muted">800 × 800 m · seed {w["seed"]} · tick {w["tick"]} · darker footprints: taller buildings</text>')
parts.append(f'<text x="30" y="746" font-size="12" class="muted">{html.escape(a.footer)}</text></svg>')
out.write_text('\n'.join(parts))
print(out)
