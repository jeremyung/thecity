import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';

const root = resolve(import.meta.dirname, '..');
const inputPath = resolve(process.argv[2] ?? 'validation/early-reservation-wasm-87231-snapshot.json');
const outputPath = resolve(process.argv[3] ?? 'validation/building-outline-comparison.json');
const cache = resolve(root, 'node_modules/.cache', `thecity-outline-compare-${process.pid}`);
const inputBytes = await readFile(inputPath);
const world = JSON.parse(inputBytes.toString('utf8'));
if (!Array.isArray(world.buildings) || world.buildings.length !== 850) throw new Error(`Expected mature 850-building fixture; received ${world.buildings?.length ?? 'no buildings'}`);
const sha256 = (data) => createHash('sha256').update(data).digest('hex');
const codeFiles = [
  'web/src/render/kit.ts',
  'web/src/render/building-chunks.ts',
  'web/src/render/incremental-geometry-merge.ts',
  'web/src/render/incremental-geometry-transform.ts',
  'web/src/render/materials.ts',
];

globalThis.document = { createElement(name) {
  if (name !== 'canvas') throw new Error(`Unexpected DOM element request: ${name}`);
  return { width: 0, height: 0, getContext: () => ({ fillRect() {}, strokeRect() {}, fillText() {}, beginPath() {}, ellipse() {}, fill() {} }) };
} };

function frozenLegacyOutlines(THREE, group) {
  const values = [];
  group.updateMatrixWorld(true);
  group.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    const edges = new THREE.EdgesGeometry(object.geometry, 42);
    edges.applyMatrix4(object.matrixWorld);
    const points = edges.getAttribute('position');
    for (let i = 0; i < points.count; i += 2) {
      const dx = points.getX(i + 1) - points.getX(i);
      const dy = points.getY(i + 1) - points.getY(i);
      const dz = points.getZ(i + 1) - points.getZ(i);
      if (dx * dx + dy * dy + dz * dz < 12.25) continue;
      for (let j = 0; j < 2; j++) values.push(points.getX(i + j), points.getY(i + j), points.getZ(i + j));
    }
    edges.dispose();
  });
  return new Float32Array(values);
}

function updateHash(hash, array) {
  hash.update(Buffer.from(array.buffer, array.byteOffset, array.byteLength));
}

await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  const bundle = await build({
    entryPoints: codeFiles.map((file) => resolve(root, file)), outdir: cache,
    bundle: true, format: 'esm', platform: 'node', packages: 'external',
    logLevel: 'silent', metafile: true,
  });
  const THREE = await import('three/webgpu');
  const [{ buildingKit, disposeGroup }, { ArtMaterials }, { BuildingChunks }] = await Promise.all([
    import(pathToFileURL(resolve(cache, 'kit.js')).href),
    import(pathToFileURL(resolve(cache, 'materials.js')).href),
    import(pathToFileURL(resolve(cache, 'building-chunks.js')).href),
  ]);
  const art = new ArtMaterials();
  const chunks = new BuildingChunks((material) => art.buildingBatch(material), (dispose) => dispose(), art.contours);
  const referenceHash = createHash('sha256'), optimizedHash = createHash('sha256');
  let authoredMs = 0, legacyOutlinesMs = 0, optimizedSetMs = 0;
  let referenceFloats = 0, optimizedFloats = 0, perBuildingMismatches = 0;
  const mismatchIds = [];

  for (const building of world.buildings) {
    const authoredStart = performance.now();
    const group = buildingKit(building, art);
    group.position.set(building.x, 0, building.z);
    group.rotation.y = building.rotation;
    authoredMs += performance.now() - authoredStart;

    const legacyStart = performance.now();
    const legacy = frozenLegacyOutlines(THREE, group);
    legacyOutlinesMs += performance.now() - legacyStart;
    referenceFloats += legacy.length;
    updateHash(referenceHash, legacy);

    const setStart = performance.now();
    chunks.set(building.id, group, { createGroup() { throw new Error('Factory not used in outline-only comparison'); }, disposeGroup });
    optimizedSetMs += performance.now() - setStart;
    const optimized = chunks.sources.get(building.id).contours;
    optimizedFloats += optimized.length;
    updateHash(optimizedHash, optimized);

    const sameBytes = legacy.length === optimized.length && Buffer.from(legacy.buffer, legacy.byteOffset, legacy.byteLength)
      .equals(Buffer.from(optimized.buffer, optimized.byteOffset, optimized.byteLength));
    if (!sameBytes) { perBuildingMismatches++; mismatchIds.push(building.id); }
  }

  const hashBeforeFinish = referenceHash.digest('hex');
  const hashAfterOptimization = optimizedHash.digest('hex');
  const transitiveCode = Object.keys(bundle.metafile.inputs)
    .map((file) => file.replaceAll('\\', '/'))
    .filter((file) => file.startsWith('web/src/render/'))
    .sort();
  const transitiveSourceHashes = {};
  for (const file of transitiveCode) transitiveSourceHashes[file] = sha256(await readFile(resolve(root, file)));
  const report = {
    title: 'Mature fixture building outline byte-parity comparison',
    measuredAt: new Date().toISOString(),
    fixture: {
      file: inputPath.replace(`${root}/`, ''), sha256: sha256(inputBytes), bytes: inputBytes.byteLength,
      seed: world.seed, tick: world.tick, buildings: world.buildings.length,
    },
    transitiveSourceHashes,
    oracle: {
      description: 'Frozen pre-optimization BuildingChunks.outlines algorithm: EdgesGeometry(geometry, 42), apply world matrix, omit edge pairs where world-space squared length is below 12.25, append each retained pair to Float32Array.',
      edgeAngleDegrees: 42, minimumEdgeLengthMeters: 3.5, thresholdComparison: 'strictly less than 12.25 m² is omitted',
    },
    output: {
      legacyReference: { floatCount: referenceFloats, byteCount: referenceFloats * Float32Array.BYTES_PER_ELEMENT, orderedFloat32Sha256: hashBeforeFinish },
      optimized: { floatCount: optimizedFloats, byteCount: optimizedFloats * Float32Array.BYTES_PER_ELEMENT, orderedFloat32Sha256: hashAfterOptimization },
      orderedBytesIdentical: referenceFloats === optimizedFloats && hashBeforeFinish === hashAfterOptimization && perBuildingMismatches === 0,
      perBuildingMismatches, mismatchBuildingIds: mismatchIds,
    },
    timing: {
      mode: 'one Node process, same generated groups; legacy reference first, optimized BuildingChunks.set second for each building',
      units: 'milliseconds',
      nodeWallMs: { buildingKitGeneration: authoredMs, frozenLegacyOutlineReference: legacyOutlinesMs, optimizedSetOutlinePath: optimizedSetMs },
      note: 'The optimized path timing includes BuildingChunks.set bookkeeping as well as outline extraction; timings are single-run CPU wall time and not browser frame pacing or GPU performance.',
    },
    environment: {
      platform: process.platform, architecture: process.arch, node: process.version,
      threeVersion: JSON.parse(await readFile(resolve(root, 'node_modules/three/package.json'), 'utf8')).version,
      sourceModulesFromEsbuildMetafile: transitiveCode,
    },
    limitations: [
      'This verifies contour Float32 byte parity for the saved mature fixture, not rendered appearance on WebGPU or WebGL.',
      'The oracle is the frozen prior algorithm embedded above; it runs before the optimized path on each identical generated group.',
      'The script does not drain chunk merges and does not measure garbage collection, browser heap, renderer uploads, GPU work or frame rate.',
      'Both hash streams preserve world/building order and each source contour float order; mismatching building IDs are listed if parity fails.',
    ],
  };
  if (!report.output.orderedBytesIdentical) throw new Error(`Contour outputs differ: ${JSON.stringify(report.output)}`);
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify({ outputPath, fixture: report.fixture, transitiveSourceHashes, output: report.output, timing: report.timing }, null, 2)}\n`);
  chunks.clear();
  art.dispose();
} finally {
  await rm(cache, { recursive: true, force: true });
}
