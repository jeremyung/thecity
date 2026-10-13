import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { build } from 'esbuild';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
const inputPath = resolve(process.argv[2] ?? '/tmp/city-ceramic-mature.json');
const outputPath = resolve(process.argv[3] ?? 'validation/ceramic-roof-cost.json');
const cache = resolve(root, 'node_modules/.cache', `thecity-ceramic-roof-${process.pid}`);
const inputBytes = await readFile(inputPath);
const world = JSON.parse(inputBytes.toString('utf8'));
if (!Array.isArray(world.buildings)) throw new Error('Snapshot is missing buildings[]');
const sha256 = (data) => createHash('sha256').update(data).digest('hex');
const baselineRevision = '8e9f5e196659c29c9c6a5b9dcd6bed43fd1de2f7';
const baselineKit = execFileSync('git', ['show', `${baselineRevision}:web/src/render/kit.ts`], { cwd: root, encoding: 'utf8' });

function installCanvasStub() {
  globalThis.document = {
    createElement(name) {
      if (name !== 'canvas') throw new Error(`Unexpected DOM element request: ${name}`);
      const context = {
        fillRect() {}, strokeRect() {}, fillText() {}, beginPath() {}, ellipse() {}, fill() {},
        moveTo() {}, quadraticCurveTo() {}, stroke() {},
      };
      return { width: 0, height: 0, getContext: () => context };
    },
  };
}

function measure(THREE, group) {
  const stats = { meshes: 0, triangles: 0, geometryTypedArrayBytes: 0 };
  group.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    stats.meshes++;
    const geometry = object.geometry;
    const position = geometry.getAttribute('position');
    stats.triangles += geometry.index ? geometry.index.count / 3 : (position?.count ?? 0) / 3;
    for (const attribute of Object.values(geometry.attributes)) stats.geometryTypedArrayBytes += attribute.array.byteLength;
    if (geometry.index) stats.geometryTypedArrayBytes += geometry.index.array.byteLength;
  });
  return stats;
}

function dispose(THREE, group) {
  group.traverse((object) => {
    if (object instanceof THREE.Mesh) {
      object.geometry.dispose();
      if (object.userData.ownedMaterial) object.material.dispose();
    }
    object.userData.ownedTexture?.dispose();
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
      plugins: variant === 'before' ? [{
        name: 'head-kit-baseline',
        setup(build) {
          build.onLoad({ filter: /\/render\/kit\.ts$/ }, () => ({ contents: baselineKit, loader: 'ts', resolveDir: resolve(root, 'web/src/render') }));
        },
      }] : [],
    });
    outputs[variant] = dir;
  }

  const THREE = await import('three/webgpu');
  installCanvasStub();
  const results = {};
  for (const variant of ['before', 'after']) {
    const [{ buildingKit }, { ArtMaterials }] = await Promise.all([
      import(pathToFileURL(resolve(outputs[variant], 'kit.js')).href),
      import(pathToFileURL(resolve(outputs[variant], 'materials.js')).href),
    ]);
    const art = new ArtMaterials();
    const totals = { buildings: world.buildings.length, meshes: 0, triangles: 0, geometryTypedArrayBytes: 0 };
    for (const building of world.buildings) {
      const group = buildingKit(building, art);
      const stats = measure(THREE, group);
      totals.meshes += stats.meshes;
      totals.triangles += stats.triangles;
      totals.geometryTypedArrayBytes += stats.geometryTypedArrayBytes;
      dispose(THREE, group);
    }
    art.dispose();
    results[variant] = totals;
  }
  const percentDelta = (before, after) => before === 0 ? null : (after - before) / before * 100;
  const report = {
    baseline: { revision: baselineRevision, kitSha256: sha256(baselineKit) },
    input: { path: inputPath, sha256: sha256(inputBytes), seed: world.seed ?? world.worldSeed ?? null, tick: world.tick ?? world.currentTick ?? null, buildingCount: world.buildings.length },
    method: 'For each snapshot building, generate production buildingKit geometry in Node for pinned baseline kit.ts (before) and the working kit.ts (after), count Mesh objects, triangles and typed-array bytes, then dispose the geometry before continuing. No renderer or browser is used.',
    units: { triangles: 'Index count/3 or non-indexed position count/3.', geometryTypedArrayBytes: 'Sum of resident BufferGeometry attribute and index typed-array byteLength values.', meshes: 'THREE.Mesh objects in generated building groups.' },
    before: results.before,
    after: results.after,
    delta: {
      meshes: results.after.meshes - results.before.meshes,
      triangles: results.after.triangles - results.before.triangles,
      geometryTypedArrayBytes: results.after.geometryTypedArrayBytes - results.before.geometryTypedArrayBytes,
      percent: {
        meshes: percentDelta(results.before.meshes, results.after.meshes),
        triangles: percentDelta(results.before.triangles, results.after.triangles),
        geometryTypedArrayBytes: percentDelta(results.before.geometryTypedArrayBytes, results.after.geometryTypedArrayBytes),
      },
    },
    limitations: ['CPU-side authored Three.js geometry accounting; not GPU memory, rendering performance, or browser heap measurement.'],
  };
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ outputPath, input: report.input, before: report.before, after: report.after, delta: report.delta }, null, 2)}\n`);
} finally {
  await rm(cache, { recursive: true, force: true });
}
