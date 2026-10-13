import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { build } from 'esbuild';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
const inputPath = resolve(root, '.tools/waterfront-growth/fully-realized.json');
const outputPath = resolve(root, 'validation/office-slab-cost.json');
const cache = resolve(root, 'node_modules/.cache', `thecity-office-slab-${process.pid}`);
const inputBytes = await readFile(inputPath);
const source = inputBytes.toString('utf8');
const sha256 = (data) => createHash('sha256').update(data).digest('hex');
const baselineRevision = '8e9f5e196659c29c9c6a5b9dcd6bed43fd1de2f7';
const baselineMassing = execFileSync('git', ['show', `${baselineRevision}:web/src/render/building-massing.ts`], { cwd: root, encoding: 'utf8' });
const currentMassing = await readFile(resolve(root, 'web/src/render/building-massing.ts'), 'utf8');
const slabBranch = /\n  \/\/ A seeded minority of offices[\s\S]*?\n  }\n(?=\n  const podiumFloors)/;
if (!slabBranch.test(currentMassing) || currentMassing.replace(slabBranch, '') !== baselineMassing) {
  throw new Error('Current building-massing.ts must differ from the pinned baseline only by the office slab branch');
}

const wasmBytes = await readFile(resolve(root, 'public/wasm/city_engine_bg.wasm'));
const wasm = await import(pathToFileURL(resolve(root, 'public/wasm/city_engine.js')).href);
await wasm.default({ module_or_path: wasmBytes });
const engine = wasm.CityEngine.restore(source);
let world;
let canonicalSave;
try {
  world = JSON.parse(engine.snapshot());
  canonicalSave = engine.save();
} finally {
  engine.free?.();
}
const eligible = world.buildings.filter((b) => b.kind === 'office' && !b.anchor && Number.isInteger(b.floors) && b.floors >= 4 && b.floors <= 11);

function installCanvasStub() {
  globalThis.document = { createElement(name) {
    if (name !== 'canvas') throw new Error(`Unexpected DOM element request: ${name}`);
    const context = { fillRect() {}, strokeRect() {}, fillText() {}, beginPath() {}, ellipse() {}, fill() {}, moveTo() {}, quadraticCurveTo() {}, stroke() {} };
    return { width: 0, height: 0, getContext: () => context };
  } };
}
function measure(THREE, group) {
  const stats = { meshes: 0, triangles: 0, geometryTypedArrayBytes: 0 };
  group.traverse((o) => {
    if (!(o instanceof THREE.Mesh)) return;
    stats.meshes++;
    const g = o.geometry;
    const pos = g.getAttribute('position');
    stats.triangles += g.index ? g.index.count / 3 : (pos?.count ?? 0) / 3;
    for (const a of Object.values(g.attributes)) stats.geometryTypedArrayBytes += a.array.byteLength;
    if (g.index) stats.geometryTypedArrayBytes += g.index.array.byteLength;
  });
  return stats;
}
function dispose(THREE, group) {
  group.traverse((o) => {
    if (o instanceof THREE.Mesh) {
      o.geometry.dispose();
      if (o.userData.ownedMaterial) o.material.dispose();
    }
    o.userData.ownedTexture?.dispose();
  });
}

await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  const outputs = {};
  for (const variant of ['before', 'after']) {
    const dir = resolve(cache, variant);
    await mkdir(dir, { recursive: true });
    await build({
      entryPoints: [resolve(root, 'web/src/render/kit.ts'), resolve(root, 'web/src/render/materials.ts')],
      outdir: dir, bundle: true, format: 'esm', platform: 'node', packages: 'external', logLevel: 'silent',
      plugins: variant === 'before' ? [{ name: 'head-office-slab-baseline', setup(build) {
        build.onLoad({ filter: /\/render\/building-massing\.ts$/ }, () => ({ contents: baselineMassing, loader: 'ts', resolveDir: resolve(root, 'web/src/render') }));
      } }] : [],
    });
    outputs[variant] = dir;
  }
  const THREE = await import('three/webgpu');
  installCanvasStub();
  const results = {};
  const byBuilding = {};
  for (const variant of ['before', 'after']) {
    const [{ buildingKit }, { ArtMaterials }] = await Promise.all([
      import(pathToFileURL(resolve(outputs[variant], 'kit.js')).href),
      import(pathToFileURL(resolve(outputs[variant], 'materials.js')).href),
    ]);
    const art = new ArtMaterials();
    const totals = { buildings: eligible.length, meshes: 0, triangles: 0, geometryTypedArrayBytes: 0 };
    for (const b of eligible) {
      const group = buildingKit(b, art);
      const stats = measure(THREE, group);
      byBuilding[b.id] ??= {};
      byBuilding[b.id][variant] = stats;
      totals.meshes += stats.meshes;
      totals.triangles += stats.triangles;
      totals.geometryTypedArrayBytes += stats.geometryTypedArrayBytes;
      dispose(THREE, group);
    }
    art.dispose();
    results[variant] = totals;
  }
  const deltaFor = (a, b) => b - a;
  const perBuilding = eligible.map((b) => {
    const before = byBuilding[b.id].before;
    const after = byBuilding[b.id].after;
    return { id: b.id, triangles: deltaFor(before.triangles, after.triangles), geometryTypedArrayBytes: deltaFor(before.geometryTypedArrayBytes, after.geometryTypedArrayBytes) };
  });
  const report = {
    baseline: { revision: baselineRevision, buildingMassingSha256: sha256(baselineMassing), verification: 'Current file equals the pinned baseline after removing only the inserted office slab branch.' },
    input: { path: '.tools/waterfront-growth/fully-realized.json', sourceHash: sha256(inputBytes), saveHash: sha256(canonicalSave), seed: world.seed ?? null, tick: world.tick ?? null, buildingCount: world.buildings.length, eligibleOfficeCount: eligible.length },
    method: 'Restore the normal saved checkpoint with production WASM and record source and canonical restore/save hashes; build production buildingKit and ArtMaterials, overriding only building-massing.ts to the pinned baseline for before; measure eligible nonanchor offices only; dispose each generated group and ArtMaterials. No browser is used.',
    units: { triangles: 'Index count/3 or non-indexed position count/3.', geometryTypedArrayBytes: 'Sum of BufferGeometry attribute and index typed-array byteLength values.', meshes: 'THREE.Mesh objects in generated building groups.' },
    before: results.before,
    after: results.after,
    delta: {
      triangles: deltaFor(results.before.triangles, results.after.triangles),
      geometryTypedArrayBytes: deltaFor(results.before.geometryTypedArrayBytes, results.after.geometryTypedArrayBytes),
      maxPerBuilding: {
        triangles: Math.max(...perBuilding.map((x) => Math.abs(x.triangles)), 0),
        geometryTypedArrayBytes: Math.max(...perBuilding.map((x) => Math.abs(x.geometryTypedArrayBytes)), 0),
      },
      changedBuildingCount: perBuilding.filter((x) => x.triangles !== 0 || x.geometryTypedArrayBytes !== 0).length,
    },
    limitations: ['CPU-side authored Three.js geometry accounting; not GPU memory, rendering performance, or browser heap measurement.'],
  };
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ outputPath, input: report.input, before: report.before, after: report.after, delta: report.delta }, null, 2)}\n`);
} finally {
  await rm(cache, { recursive: true, force: true });
}
