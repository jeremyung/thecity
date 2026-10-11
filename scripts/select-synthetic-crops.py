#!/usr/bin/env python3
"""Select fixed-size synthetic comparison crops without changing simulation state."""
import json, math, hashlib
from pathlib import Path
from shapely.geometry import Polygon,box
from shapely.ops import unary_union
source=Path('validation/early-reservation-wasm-87231-snapshot.json')
snapshot=json.loads(source.read_text())
shapes=[]
for b in snapshot['buildings']:
 c,s=math.cos(b['rotation']),math.sin(b['rotation']);w,d=b['width']/2,b['depth']/2
 geom=Polygon([(b['x']+c*x+s*z,b['z']-s*x+c*z) for x,z in [(-w,-d),(w,-d),(w,d),(-w,d)]])
 shapes.append((b,geom))
union=unary_union([g for b,g in shapes])
# Fixed 50m lattice, restricted to east-side land; select largest envelope union.
# Selection intentionally gives generator a strong candidate, not random coverage.
candidates=[]
for x in range(-50,151,50):
 for z in range(-400,151,50):
  bounds=[x,z,x+250,z+250];a=union.intersection(box(*bounds)).area
  candidates.append((a,x,z,bounds))
best=sorted(candidates,key=lambda v:(-v[0],v[1],v[2]))[0]
station=next(n for n in snapshot['nodes'] if n['kind']=='station')
crops=[('station',[station['x']-125,station['z']-125,station['x']+125,station['z']+125]),('compact',best[3]),('waterfront',[-237,-83,13,167])]
records=[]
for label,bounds in crops:
 region=box(*bounds);selected=[(b,g) for b,g in shapes if g.intersection(region).area>0]
 areas=sorted(g.area for b,g in selected)
 def q(p):
  t=(len(areas)-1)*p;lo=math.floor(t);hi=math.ceil(t);return areas[lo]+(areas[hi]-areas[lo])*(t-lo)
 records.append({'id':label,'bounds':bounds,'selectedBuildingCount':len(selected),'nominalEnvelopeUnionM2Exact':union.intersection(region).area,'nominalEnvelopeAreaQuantilesM2':{'p10':q(.1),'p50':q(.5),'p90':q(.9)},'buildingIds':[b['id'] for b,g in selected]})
 paths=[]
 for b,g in selected:
  clipped=g.intersection(region)
  if clipped.geom_type!='Polygon':raise ValueError('expected rectangle polygon crop')
  commands=[]
  for ring in [clipped.exterior,*clipped.interiors]:
   coords=[f'{40+(x-bounds[0])*2.56:.3f},{90+(z-bounds[1])*2.56:.3f}' for x,z in ring.coords]
   commands.append('M'+' L'.join(coords)+' Z')
  paths.append('<path d="'+' '.join(commands)+'" fill="#899c9c" stroke="#f7f4ed" stroke-width="0.5"/>')
 svg=f'''<svg xmlns="http://www.w3.org/2000/svg" width="720" height="810"><rect width="720" height="810" fill="#f7f4ed"/><g font-family="Arial" fill="#293c40"><text x="40" y="40" font-size="24">Synthetic {label} · 250 × 250 m</text><text x="40" y="66" font-size="13">Nominal building envelopes · not rendered roofs · seed87231</text><rect x="40" y="90" width="640" height="640" fill="#e5e3db"/>{''.join(paths)}<rect x="40" y="90" width="640" height="640" fill="none" stroke="#293c40"/><text x="40" y="760" font-size="14">{len(selected)} intersecting buildings · envelope union {records[-1]['nominalEnvelopeUnionM2Exact']/625:.2f}%</text><text x="40" y="785" font-size="12">+X right / +Z down · each side250m · blank areas unclassified</text></g></svg>'''
 Path(f'validation/synthetic-morphology/{label}-envelopes.svg').write_text(svg+'\n')
Path('validation/synthetic-morphology/crops.json').write_text(json.dumps({'source':str(source),'sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'seed':snapshot['seed'],'tick':snapshot['tick'],'selection':'Station centered on actual station node; waterfront center(-112,42); compact maximizes exact envelope union over east-side50m lattice with minX[-50,150],minZ[-400,150],ties minX then minZ. This favorable selection is not a random sample or matched land-use classification.','crops':records},indent=2)+'\n')
for r in records:print(r['id'],','.join(str(x) for x in r['bounds']),r['selectedBuildingCount'],r['nominalEnvelopeUnionM2Exact'])
