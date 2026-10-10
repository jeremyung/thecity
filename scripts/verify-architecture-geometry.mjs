import { createHash } from 'node:crypto';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const cache = resolve(root, 'scripts/.architecture-geometry-cache');
const output = resolve(root, 'validation/architectural-geometry.json');

function building(kind, seed, floors) {
  const house = kind === 'home';
  return {
    id: seed,
    parcel_id: seed + 1_000,
    x: 0,
    z: 0,
    rotation: 0,
    width: house ? 14 : 22,
    depth: house ? 12 : 18,
    floors,
    kind,
    stage: 'open',
    progress: 1,
    capacity: 0,
    occupancy: 0,
    built_day: 0,
    name: '',
    appearance_seed: seed,
    history: [],
    anchor: false,
    vacant_days: 0,
  };
}

function inspectGroup(group, spec) {
  group.updateMatrixWorld(true);
  const bounds = new (globalThis.THREE.Box3)().setFromObject(group);
  let vertices = 0;
  const hash = createHash('sha256');
  for (const child of group.children) {
    const positions = child.geometry.attributes.position.array;
    vertices += positions.length / 3;
    for (const value of positions) {
      if (!Number.isFinite(value)) throw new Error(`${spec.kind}/${spec.seed} has non-finite geometry`);
    }
    hash.update(Buffer.from(positions.buffer, positions.byteOffset, positions.byteLength));
  }
  const tolerance = 1e-5;
  const withinParcel = bounds.min.x >= -spec.width / 2 - tolerance
    && bounds.max.x <= spec.width / 2 + tolerance
    && bounds.min.z >= -spec.depth / 2 - tolerance
    && bounds.max.z <= spec.depth / 2 + tolerance;
  if (!withinParcel) {
    throw new Error(`${spec.kind}/${spec.seed} escaped parcel bounds (${bounds.min.x},${bounds.max.x},${bounds.min.z},${bounds.max.z})`);
  }
  return {
    vertices,
    mergedMeshes: group.children.length,
    finiteGeometry: true,
    withinParcel,
    bounds: {
      minX: Number(bounds.min.x.toFixed(3)),
      maxX: Number(bounds.max.x.toFixed(3)),
      minZ: Number(bounds.min.z.toFixed(3)),
      maxZ: Number(bounds.max.z.toFixed(3)),
    },
    geometrySha256: hash.digest('hex'),
  };
}

await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  await build({
    entryPoints: [resolve(root, 'web/src/render/kit.ts'), resolve(root, 'web/src/render/materials.ts')],
    outdir: cache,
    bundle: true,
    format: 'esm',
    platform: 'node',
    packages: 'external',
    logLevel: 'silent',
  });
  const THREE = await import('three/webgpu');
  const [{ buildingKit }, { ArtMaterials }] = await Promise.all([
    import(pathToFileURL(resolve(cache, 'kit.js')).href),
    import(pathToFileURL(resolve(cache, 'materials.js')).href),
  ]);
  globalThis.THREE = THREE;
  const art = new ArtMaterials();
  const specs = [
    building('home', 13, 2),
    building('home', 29, 2),
    building('apartment', 55, 6),
    building('office', 72, 8),
  ];
  const cases = [];
  for (const spec of specs) {
    const first = buildingKit(spec, art);
    const result = inspectGroup(first, spec);
    first.traverse((object) => { if (object instanceof THREE.Mesh) object.geometry.dispose(); });

    const second = buildingKit(spec, art);
    const repeated = inspectGroup(second, spec);
    second.traverse((object) => { if (object instanceof THREE.Mesh) object.geometry.dispose(); });
    if (result.geometrySha256 !== repeated.geometrySha256) throw new Error(`${spec.kind}/${spec.seed} geometry was not deterministic`);

    cases.push({ kind: spec.kind, seed: spec.appearance_seed, floors: spec.floors, width: spec.width, depth: spec.depth, ...result, deterministicRepeat: true });
  }
  const homePlans = cases.filter((item) => item.kind === 'home');
  if (homePlans[0].geometrySha256 === homePlans[1].geometrySha256) throw new Error('Seeded home plans did not vary');
  art.dispose();

  const proof = {
    check: 'architectural geometry bounds and deterministic variation',
    source: 'web/src/render/kit.ts',
    checkedAt: new Date().toISOString(),
    policy: 'Open architectural geometry must have finite vertices and remain inside its parcel footprint; repeated appearance seeds must reproduce identical geometry.',
    cases,
    summary: {
      caseCount: cases.length,
      passed: true,
      totalMergedMeshes: cases.reduce((sum, item) => sum + item.mergedMeshes, 0),
      totalVertices: cases.reduce((sum, item) => sum + item.vertices, 0),
      seededHomePlansDiffer: true,
    },
  };
  await writeFile(output, `${JSON.stringify(proof, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(proof.summary)}\n`);
} finally {
  await rm(cache, { recursive: true, force: true });
}
