// Audit planned elevated-road curves across a fixed seed corpus.
import { readFile } from 'node:fs/promises';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const wasm = await readFile(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url));
initSync({ module: wasm });
const seeds = [0, 7, 41, 2026, 87231, 1, 42, 1204, 1234, 65535];
const rows = [];

function geometry(world) {
  const nodes = new Map(world.nodes.map(n => [n.id, n]));
  const roads = new Map(world.roads.map(r => [r.id, r]));
  const decks = world.road_decks;
  let minRadius = Infinity, worstRoad = null, degenerateTangents = 0;
  const endpointTangents = new Map();
  for (const deck of decks) {
    const road = roads.get(deck.road_id), a = nodes.get(road.from), b = nodes.get(road.to);
    const [c, d] = deck.controls;
    for (let i = 0; i <= 128; i++) {
      const t = i / 128, s = 1 - t;
      const vx = 3*s*s*(c[0]-a.x) + 6*s*t*(d[0]-c[0]) + 3*t*t*(b.x-d[0]);
      const vz = 3*s*s*(c[1]-a.z) + 6*s*t*(d[1]-c[1]) + 3*t*t*(b.z-d[1]);
      const ax = 6*s*(d[0]-2*c[0]+a.x) + 6*t*(b.x-2*d[0]+c[0]);
      const az = 6*s*(d[1]-2*c[1]+a.z) + 6*t*(b.z-2*d[1]+c[1]);
      const speed = Math.hypot(vx, vz), cross = Math.abs(vx*az-vz*ax);
      if (speed <= 1e-4 || !Number.isFinite(speed)) degenerateTangents++;
      else if (cross > 1e-8) {
        const radius = speed**3 / cross;
        if (radius < minRadius) { minRadius = radius; worstRoad = road.id; }
      }
    }
    // Directions are travel-oriented: derivative at start, derivative at end.
    const start = [c[0]-a.x, c[1]-a.z];
    const end = [b.x-d[0], b.z-d[1]];
    for (const [node, vec, atStart] of [[road.from, start, true], [road.to, end, false]]) {
      const n = nodes.get(node);
      const key = `${node}:${n.y ?? n.height ?? n.elevation ?? 0}`;
      if (!endpointTangents.has(key)) endpointTangents.set(key, []);
      endpointTangents.get(key).push({ road: road.id, vec, atStart });
    }
  }
  let worstContinuityDegrees = 0, continuityWitness = null;
  for (const [key, ends] of endpointTangents) for (let i=0;i<ends.length;i++) for(let j=i+1;j<ends.length;j++) {
    const [x1,z1] = ends[i].vec, [x2,z2] = ends[j].vec;
    const l1=Math.hypot(x1,z1), l2=Math.hypot(x2,z2);
    if (l1<1e-8||l2<1e-8) { degenerateTangents++; continue; }
    // Two starts (or two ends) have opposite storage derivatives at a smooth
    // joint. Compare both away from the joint, then require opposition.
    const orientation=ends[i].atStart === ends[j].atStart ? -1 : 1;
    const dot=orientation*(x1*x2+z1*z2)/(l1*l2);
    const angle=Math.acos(Math.max(-1,Math.min(1,dot)))*180/Math.PI;
    if(angle>worstContinuityDegrees){worstContinuityDegrees=angle;continuityWitness={node:key.split(':')[0],roads:[ends[i].road,ends[j].road]};}
  }
  return { minRadiusM: Number.isFinite(minRadius) ? minRadius : null, worstRoad, worstContinuityDegrees, continuityWitness,
    degenerateTangents, innerDeckFold: minRadius <= 10.8 };
}

for (const seed of seeds) {
  const engine = new CityEngine(seed);
  try {
    engine.set_development_rate(16);
    let world = JSON.parse(engine.snapshot());
    while (world.road_decks.length < 16 && world.tick < 160000) {
      engine.step(200);
      world = JSON.parse(engine.snapshot());
    }
    rows.push({ seed, tick: world.tick, deckCount: world.road_decks.length, ...geometry(world) });
    console.log(JSON.stringify(rows.at(-1)));
  } finally { engine.free(); }
}
const failures = rows.filter(row => row.deckCount !== 16 || row.innerDeckFold || row.degenerateTangents > 0 || row.worstContinuityDegrees > .01);
const report = { method: 'Current compiled public WASM CityEngine; development rate 16; step in 200-tick batches until 16 road_decks or tick 160000. Cubic curvature sampled at 129 evenly spaced t values per deck. Same-height connected endpoint tangent comparison.', seeds, results: rows, failures };
await (await import('node:fs/promises')).writeFile(new URL('../validation/expressway-corpus.json', import.meta.url), `${JSON.stringify(report,null,2)}\n`);
if (failures.length) {
  console.error(`${failures.length} seed(s) failed:`, failures.map(row => `${row.seed} (decks=${row.deckCount}, fold=${row.innerDeckFold}, degenerate=${row.degenerateTangents})`).join('; '));
  process.exitCode = 1;
}
