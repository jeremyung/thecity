#!/usr/bin/env python3
"""Offline area accounting, not an engine parcel-admissibility oracle."""
import json,math,hashlib
from pathlib import Path
from shapely.geometry import Polygon,LineString,box
from shapely.ops import unary_union
manifest=json.loads(Path('validation/synthetic-morphology/crops.json').read_text())
raw=Path(manifest['source']).read_bytes()
assert hashlib.sha256(raw).hexdigest()==manifest['sha256'], 'source snapshot hash mismatch'
snapshot=json.loads(raw);nodes={n['id']:n for n in snapshot['nodes']}
buildings=[]
for b in snapshot['buildings']:
 c,s=math.cos(b['rotation']),math.sin(b['rotation']);w,d=b['width']/2,b['depth']/2
 buildings.append(Polygon([(b['x']+c*x+s*z,b['z']-s*x+c*z) for x,z in [(-w,-d),(w,-d),(w,d),(-w,d)]]))
built=unary_union(buildings)
# All open roads, matching collision_context (including rail). Round end caps.
segments=[LineString([(nodes[r['from']]['x'],nodes[r['from']]['z']),(nodes[r['to']]['x'],nodes[r['to']]['z'])]) for r in snapshot['roads'] if r['stage']=='open']
roads=unary_union([g.buffer(5,quad_segs=32) for g in segments])
road12=unary_union([g.buffer(12,quad_segs=32) for g in segments])
building_halo=built.buffer(2,quad_segs=32)
rows=[]
for crop in manifest['crops']:
 region=box(*crop['bounds']);b=built.intersection(region);r=roads.intersection(region)
 remainder=region.difference(b.union(r))
 halo=remainder.intersection(building_halo)
 after_halo=remainder.difference(building_halo)
 rows.append({'id':crop['id'],'grossM2':region.area,'buildingEnvelopeUnionM2':b.area,'openRoad5mBufferUnionM2':r.area,'envelopeRoadBufferOverlapM2':b.intersection(r).area,'unclassifiedOutsideBothM2':remainder.area,'remainderWithin2mOfBuildingM2':halo.area,'remainderBeyond2mBuildingWithin12mRoadM2':after_halo.intersection(road12).area,'remainderBeyond2mBuildingBeyond12mRoadM2':after_halo.difference(road12).area})
Path('validation/synthetic-morphology/land-budget.json').write_text(json.dumps({'sourceSha256':manifest['sha256'],'method':'Exact polygon boolean operations; road buffer round caps approximated with32segments/quadrant.5m centerline distance matches open-road parcel rule. Unclassified remainder includes water, parks, pending development, reservations, other constraints and unmodeled access; NOT developable area. The2m Euclidean building halo and12m road-distance bands are descriptive distances, not the engine SAT clearance or frontage access test. This does not evaluate parcel fit or collision-context legacy alternate envelopes.','crops':rows},indent=2)+'\n')
for row in rows:print(row)
