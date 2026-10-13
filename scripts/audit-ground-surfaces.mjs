// CPU geometry diagnostic; this does not measure browser/GPU frame timing.
// Usage: node scripts/audit-ground-surfaces.mjs path/to/current-save.json
import { build } from 'esbuild';
import { readFileSync, mkdirSync } from 'node:fs';
import os from 'node:os';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

if (!process.argv[2]) throw Error('Pass a current-format city save');
mkdirSync('.tools', { recursive: true });
await build({
  stdin: {
    contents: "export {buildGroundSurfaces} from './web/src/render/ground-surfaces'; export {ArtMaterials} from './web/src/render/materials'; export {disposeGroup} from './web/src/render/kit';",
    resolveDir: process.cwd(),
  },
  bundle: true, platform: 'node', format: 'esm', packages: 'external',
  outfile: '.tools/ground-cost.mjs',
});
const { buildGroundSurfaces, ArtMaterials, disposeGroup } = await import('../.tools/ground-cost.mjs');
initSync({ module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)) });
const engine = CityEngine.restore(readFileSync(process.argv[2], 'utf8'));
let ground;
try { ground = JSON.parse(engine.snapshot()).ground; } finally { engine.free(); }
const art = new ArtMaterials(), times = [];
let batches = 0, triangles = 0;
try {
  for (let i = 0; i < 13; i++) {
    const start = performance.now();
    const group = buildGroundSurfaces(ground, art);
    const ms = performance.now() - start;
    try {
      if (i >= 3) times.push(ms);
      batches = group.children.length;
      triangles = group.children.reduce((n, mesh) => n + (mesh.geometry.index?.count ?? mesh.geometry.attributes.position.count) / 3, 0);
    } finally { disposeGroup(group); }
  }
} finally { art.dispose(); }
times.sort((a, b) => a - b);
console.log(JSON.stringify({
  scope: 'Node CPU geometry construction only; not browser/GPU performance',
  hardware: os.cpus()[0].model, platform: os.platform(), node: process.version,
  patches: ground.length, batches, triangles, samples: times.length,
  medianMs: times[Math.floor(times.length / 2)], maxMs: times.at(-1),
}, null, 2));
