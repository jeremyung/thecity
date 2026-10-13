"""Produce static regression inputs; never migrate application saves.

Historical captures remain untouched. The only permitted edits are connector
route joins and, where necessary, progress proportional to the changed active
connector length. The manifest records every edit and both file hashes.
"""
import hashlib
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'tests/fixtures/access-joins'
SOURCES = [
    'generalized-access-pending-save.json',
    'mature-world.json',
    'mature-world.activity-baseline.json',
    'early-reservation-wasm-87231-save.json',
    'spine-experiment-browser-review-save.json',
    'interior-access-verified-87231.json.project-3736.pending.json',
    'infill-oriented-continued.json',
    'mature-world.bounded-selection.json',
    'mature-world.multiblock-candidate.json',
    'spine-ranking-seed-87231-by-sites-false.json',
    'spine-ranking-seed-0-by-sites-false.json',
    'mature-world.cache-candidate.json',
    'mature-world.ranked-blocks.json',
    'ranked-blocks-seed0.json',
    'founder-topology-wasm-87231-save.json',
    'interior-access-verified-87231.json.final.json',
    'interior-access-verified-seed0.json.final.json',
]

def length(points):
    return sum(math.hypot(b['x']-a['x'], b['z']-a['z']) for a,b in zip(points, points[1:]))

def digest(data):
    return hashlib.sha256(data).hexdigest()

def write_changed(path, data):
    if not path.exists() or path.read_bytes() != data:
        path.write_bytes(data)

manifest = []
OUT.mkdir(parents=True, exist_ok=True)
for name in SOURCES:
    source = ROOT / 'validation' / name
    raw = source.read_bytes()
    world = json.loads(raw)
    nodes = {node['id']: node for node in world['nodes']}
    edits = []
    for traveler in world['travelers']:
        for start, key in [(True, 'origin_access'), (False, 'destination_access')]:
            path = traveler.get(key)
            if not path:
                continue
            route = traveler['route']
            assert len(route) >= 2
            edge = route[:2] if start else route[-2:]
            a,b = [nodes[node] for node in edge]
            road = next((road for road in world['roads'] if road['stage'] == 'open'
                         and road['kind'] != 'rail' and {road['from'],road['to']} == set(edge)), None)
            width = {'arterial':16.,'bridge':16.,'local':6.5,'lane':4.}.get(road['kind'] if road else '',9.)
            lateral = -1.65 if traveler['kind'] == 1 else (width/2+.72)*(1 if traveler['id']%2 == 0 else -1)
            dx,dz = b['x']-a['x'],b['z']-a['z']
            distance = max(math.hypot(dx,dz),1.)
            node = a if start else b
            expected = {'x':node['x']-dz/distance*lateral,'z':node['z']+dx/distance*lateral}
            index = len(path['points'])-1 if start else 0
            old = path['points'][index]
            if math.hypot(old['x']-expected['x'],old['z']-expected['z']) <= .01:
                continue
            old_length = length(path['points'])
            path['points'][index] = expected
            edit = {'actor':traveler['id'],'connector':key,'point':index,'before':old,'after':expected}
            new_length = length(path['points'])
            if traveler['access_phase'] == (0 if start else 2) and traveler['access_progress'] > new_length + traveler['speed']:
                # Preserve the stage of the journey, not a distance beyond its end.
                before = traveler['access_progress']
                traveler['access_progress'] = min(before/max(old_length,1e-6),1.)*new_length
                edit['progress'] = {'before':before,'after':traveler['access_progress']}
            edits.append(edit)
    output = (json.dumps(world,separators=(',',':'),ensure_ascii=False)+'\n').encode()
    restored_source = json.loads(output)
    actors = {actor['id']: actor for actor in restored_source['travelers']}
    for edit in edits:
        actor = actors[edit['actor']]
        actor[edit['connector']]['points'][edit['point']] = edit['before']
        if 'progress' in edit:
            actor['access_progress'] = edit['progress']['before']
    assert restored_source == json.loads(raw), 'Fixture edits must be limited to the manifest'
    write_changed(OUT/name, output)
    manifest.append({'source':str(source.relative_to(ROOT)), 'fixture':str((OUT/name).relative_to(ROOT)),
                     'sourceSha256':digest(raw),'fixtureSha256':digest(output),'edits':edits})
write_changed(OUT/'manifest.json', (json.dumps(manifest,indent=2)+'\n').encode())
print(json.dumps({'fixtures':len(manifest),'joins':sum(len(x['edits']) for x in manifest),
                  'progressChanges':sum('progress' in e for x in manifest for e in x['edits'])}))
