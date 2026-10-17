import { createHash } from 'node:crypto';
import { build } from 'esbuild';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
const inputPath = resolve(
  process.argv[2] ?? 'validation/early-reservation-wasm-87231-snapshot.json',
);
const outputPath = resolve(process.argv[3] ?? 'validation/mature-building-geometry-budget.json');
const compactMidrise = process.argv.slice(4).includes('--compact-midrise');
// Keep the temporary bundle beneath node_modules so its ESM imports resolve the
// exact installed Three.js package used by the project.
const cache = resolve(root, 'node_modules/.cache', `thecity-building-geometry-${process.pid}`);
const inputBytes = await readFile(inputPath);
const world = JSON.parse(inputBytes.toString('utf8'));
if (!Array.isArray(world.buildings) || world.buildings.length !== 850) {
  throw new Error(
    `Expected the current 850-building snapshot; found ${world.buildings?.length ?? 'no building list'}`,
  );
}
const sha256 = (data) => createHash('sha256').update(data).digest('hex');

function installCanvasStub() {
  globalThis.document = {
    createElement(name) {
      if (name !== 'canvas') throw new Error(`Unexpected DOM element request: ${name}`);
      const context = {
        fillRect() {},
        strokeRect() {},
        fillText() {},
        beginPath() {},
        ellipse() {},
        fill() {},
      };
      return { width: 0, height: 0, getContext: () => context };
    },
  };
}

function freshStats() {
  return {
    buildings: 0,
    meshes: 0,
    triangles: 0,
    geometryGroups: 0,
    attributeArrayBytes: 0,
    indexArrayBytes: 0,
    uniqueBackingArrayBytes: 0,
    attributeBytesByName: {},
    materialMeshUses: {},
  };
}

function inspectRoot(THREE, rootObject, options = {}) {
  const stats = {
    meshes: 0,
    lineSegments: 0,
    triangles: 0,
    groups: 0,
    attributeArrayBytes: 0,
    indexArrayBytes: 0,
    attributeBytesByName: {},
    uniqueBackingArrayBytes: 0,
    materialMeshUses: {},
  };
  const buffers = new Set();
  const addAttribute = (name, attribute, index = false) => {
    if (!attribute?.array) return;
    const array = attribute.array;
    if (index) stats.indexArrayBytes += array.byteLength;
    else {
      stats.attributeArrayBytes += array.byteLength;
      stats.attributeBytesByName[name] = (stats.attributeBytesByName[name] ?? 0) + array.byteLength;
    }
    if (!buffers.has(array.buffer)) {
      buffers.add(array.buffer);
      stats.uniqueBackingArrayBytes += array.buffer.byteLength;
    }
  };
  rootObject.traverse((object) => {
    if (object instanceof THREE.LineSegments) {
      stats.lineSegments++;
      addAttribute('line.position', object.geometry.getAttribute('position'));
      return;
    }
    if (!(object instanceof THREE.Mesh)) return;
    stats.meshes++;
    const geometry = object.geometry;
    const position = geometry.getAttribute('position');
    const index = geometry.index;
    stats.triangles += index ? index.count / 3 : (position?.count ?? 0) / 3;
    stats.groups += geometry.groups.length;
    for (const [name, attribute] of Object.entries(geometry.attributes))
      addAttribute(name, attribute);
    if (index) addAttribute('index', index, true);
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    for (const material of materials) {
      const artClass = material.userData?.artBuildingClass?.key;
      const key =
        material.userData?.artKey ??
        (artClass ? `building-class:${artClass}` : (material.name ?? material.uuid));
      stats.materialMeshUses[key] = (stats.materialMeshUses[key] ?? 0) + 1;
    }
  });
  return stats;
}

function measureNonIndexedBatch(THREE, rootObject) {
  const stats = { triangles: 0, attributeArrayBytes: 0, indexArrayBytes: 0 };
  rootObject.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    const geometry = object.geometry.index
      ? object.geometry.toNonIndexed()
      : object.geometry.clone();
    stats.triangles += geometry.getAttribute('position').count / 3;
    for (const attribute of Object.values(geometry.attributes))
      stats.attributeArrayBytes += attribute.array.byteLength;
    if (geometry.index) stats.indexArrayBytes += geometry.index.array.byteLength;
    geometry.dispose();
  });
  return stats;
}

function disposeSource(THREE, group) {
  group.traverse((object) => {
    if (object instanceof THREE.Mesh) {
      object.geometry.dispose();
      if (object.userData.ownedMaterial) object.material.dispose();
      if (object.userData.ownedTexture) object.userData.ownedTexture.dispose();
    }
  });
}

await rm(cache, { recursive: true, force: true });
await mkdir(cache, { recursive: true });
try {
  const bundle = await build({
    entryPoints: [
      resolve(root, 'web/src/render/kit.ts'),
      resolve(root, 'web/src/render/materials.ts'),
      resolve(root, 'web/src/render/building-chunks.ts'),
    ],
    outdir: cache,
    bundle: true,
    format: 'esm',
    platform: 'node',
    packages: 'external',
    logLevel: 'silent',
    metafile: true,
  });
  const THREE = await import('three/webgpu');
  installCanvasStub();
  const [{ buildingKit }, { ArtMaterials }, { BuildingChunks }] = await Promise.all([
    import(pathToFileURL(resolve(cache, 'kit.js')).href),
    import(pathToFileURL(resolve(cache, 'materials.js')).href),
    import(pathToFileURL(resolve(cache, 'building-chunks.js')).href),
  ]);
  const art = new ArtMaterials();
  const byFamily = new Map();
  const perBuilding = [];
  const towers = freshStats();
  const sourceBuffers = new Set();
  const source = freshStats();
  const chunks = new BuildingChunks(
    (material) => art.buildingBatch(material),
    (release) => release(),
    art.contours,
  );

  function makeDescriptor(building) {
    const snapshot = { ...building, history: [...building.history] };
    return {
      createGroup() {
        const group = buildingKit(snapshot, art, { compactMidrise });
        group.position.set(snapshot.x, 0, snapshot.z);
        group.rotation.y = snapshot.rotation;
        return group;
      },
      disposeGroup(group) {
        disposeSource(THREE, group);
      },
    };
  }

  for (const building of world.buildings) {
    const group = buildingKit(building, art, { compactMidrise });
    group.position.set(building.x, 0, building.z);
    group.rotation.y = building.rotation;
    const family = byFamily.get(building.kind) ?? freshStats();
    family.buildings++;
    const stats = inspectRoot(THREE, group);
    family.meshes += stats.meshes;
    family.triangles += stats.triangles;
    family.maxFloors = Math.max(family.maxFloors ?? 0, building.floors);
    family.towerBuildings =
      (family.towerBuildings ?? 0) +
      (building.floors >= 12 && ['apartment', 'office'].includes(building.kind) ? 1 : 0);
    family.floorCounts ??= {};
    family.floorCounts[building.floors] = (family.floorCounts[building.floors] ?? 0) + 1;
    perBuilding.push({
      id: building.id,
      kind: building.kind,
      floors: building.floors,
      triangles: stats.triangles,
      attributeArrayBytes: stats.attributeArrayBytes,
      indexArrayBytes: stats.indexArrayBytes,
    });
    if (building.floors >= 12 && ['apartment', 'office'].includes(building.kind)) {
      towers.buildings++;
      towers.triangles += stats.triangles;
      towers.attributeArrayBytes += stats.attributeArrayBytes;
      towers.indexArrayBytes += stats.indexArrayBytes;
    }
    family.geometryGroups += stats.groups;
    family.attributeArrayBytes += stats.attributeArrayBytes;
    family.indexArrayBytes += stats.indexArrayBytes;
    family.uniqueBackingArrayBytes += stats.uniqueBackingArrayBytes;
    for (const [key, uses] of Object.entries(stats.materialMeshUses)) {
      family.materialMeshUses[key] = (family.materialMeshUses[key] ?? 0) + uses;
    }
    for (const [name, bytes] of Object.entries(stats.attributeBytesByName)) {
      family.attributeBytesByName[name] = (family.attributeBytesByName[name] ?? 0) + bytes;
    }
    byFamily.set(building.kind, family);

    const before = inspectRoot(THREE, group);
    source.buildings++;
    source.meshes += before.meshes;
    source.triangles += before.triangles;
    source.geometryGroups += before.groups;
    source.attributeArrayBytes += before.attributeArrayBytes;
    source.indexArrayBytes += before.indexArrayBytes;
    for (const [key, uses] of Object.entries(before.materialMeshUses)) {
      source.materialMeshUses[key] = (source.materialMeshUses[key] ?? 0) + uses;
    }
    for (const [name, bytes] of Object.entries(before.attributeBytesByName)) {
      source.attributeBytesByName[name] = (source.attributeBytesByName[name] ?? 0) + bytes;
    }
    group.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      for (const attribute of Object.values(object.geometry.attributes))
        sourceBuffers.add(attribute.array.buffer);
      if (object.geometry.index) sourceBuffers.add(object.geometry.index.array.buffer);
    });
    const descriptor = makeDescriptor(building);
    chunks.set(building.id, group, descriptor);
  }
  for (const buffer of sourceBuffers) source.uniqueBackingArrayBytes += buffer.byteLength;
  sourceBuffers.clear();

  let rebuildUpdates = 0;
  // Staged work need not finish a chunk on each update. Bound the drain so a
  // scheduling regression fails rather than hanging the resource audit.
  while (chunks.pendingCount > 0) {
    if (++rebuildUpdates > 50_000)
      throw new Error('BuildingChunks failed to drain within 50000 staged updates');
    chunks.update();
  }
  const chunkRebuilds = chunks.root.children.length;
  const batched = inspectRoot(THREE, chunks.root);
  const nonIndexedBatch = measureNonIndexedBatch(THREE, chunks.root);
  const internals = chunks;
  let retainedSourceContourBytes = 0;
  let retainedSourceGeometryBytes = 0;
  let retainedSourceGroups = 0;
  for (const sourceRecord of internals.sources.values()) {
    retainedSourceContourBytes += sourceRecord.contours.byteLength;
    for (const group of [sourceRecord.pendingGroup, sourceRecord.legacyGroup]) {
      if (!group) continue;
      retainedSourceGroups++;
      const stats = inspectRoot(THREE, group);
      retainedSourceGeometryBytes += stats.attributeArrayBytes + stats.indexArrayBytes;
    }
  }
  if (internals.active !== null || retainedSourceGroups !== 0) {
    throw new Error('Production descriptor path retained source groups after drain');
  }
  const familyRows = [...byFamily.entries()]
    .map(([kind, value]) => ({ kind, ...value }))
    .sort((a, b) => b.triangles - a.triangles);
  const materialRows = Object.entries(source.materialMeshUses)
    .map(([key, meshes]) => ({ key, meshes }))
    .sort((a, b) => b.meshes - a.meshes);
  const sources = {
    input: {
      file: inputPath.replace(`${root}/`, ''),
      sha256: sha256(inputBytes),
      bytes: inputBytes.byteLength,
      seed: world.seed,
      tick: world.tick,
      buildings: world.buildings.length,
      roads: world.roads.length,
    },
    // Include every bundled local dependency, so a kit/helper change cannot
    // silently invalidate comparison with a previous geometry measurement.
    code: Object.fromEntries(
      await Promise.all(
        Object.keys(bundle.metafile.inputs)
          .sort()
          .map(async (file) => [file, sha256(await readFile(resolve(root, file)))]),
      ),
    ),
  };
  const report = {
    title: 'Mature authored building geometry budget',
    compactMidriseReview: compactMidrise,
    measuredAt: new Date().toISOString(),
    method:
      'Production buildingKit() and BuildingChunks with the production ArtMaterials resolver; Node CPU-side array accounting. Geometry is generated from every snapshot building, placed at its saved translation/rotation, and merged into actual 200 m chunks.',
    sources,
    units: {
      bytes:
        'Typed-array byteLength in resident JS BufferGeometry attributes/indexes; unique backing bytes count each backing ArrayBuffer once.',
      triangles:
        'Triangle index count/3 or non-indexed position count/3; line contours excluded from triangle total.',
    },
    sourceGeometry: {
      ...source,
      uniqueBackingArrayBytes: source.uniqueBackingArrayBytes,
      drawSubmissionObjects: source.meshes,
    },
    authoredFamilyContribution: familyRows,
    materialAndGroupSummary: {
      sourceUniqueMaterialKeys: materialRows.length,
      sourceMaterialMeshUses: source.meshes,
      sourceTopMaterialKeys: materialRows.slice(0, 12),
      sourceGeometryGroups: source.geometryGroups,
      batchedUniqueMaterialKeys: Object.keys(batched.materialMeshUses).length,
      batchedMaterialMeshUses: batched.materialMeshUses,
      batchedGeometryGroups: batched.groups,
    },
    heaviestAuthoredBuildings: perBuilding.sort((a, b) => b.triangles - a.triangles).slice(0, 20),
    towerSubset: {
      ...towers,
      triangleShare: towers.triangles / source.triangles,
      attributeShare: towers.attributeArrayBytes / source.attributeArrayBytes,
    },
    batchedGeometry: {
      ...batched,
      chunkCount: chunkRebuilds,
      rebuildUpdates,
      drawSubmissionObjects: batched.meshes + batched.lineSegments,
      retainedSourceContourBytes,
    },
    retainedCpuGeometryArrays: {
      sourceAttributesAndIndexes: retainedSourceGeometryBytes,
      retainedSourceGroups,
      sourceContourCopies: retainedSourceContourBytes,
      batchedAttributesAndIndexes: batched.attributeArrayBytes + batched.indexArrayBytes,
      batchedContourPositions: batched.attributeBytesByName['line.position'] ?? 0,
      totalCountedBytes:
        retainedSourceGeometryBytes +
        retainedSourceContourBytes +
        batched.attributeArrayBytes +
        batched.indexArrayBytes,
    },
    comparison: {
      triangleDelta: batched.triangles - source.triangles,
      sourceToBatchTriangleRatio: source.triangles / batched.triangles,
      authoredMeshesToBatchedMeshesRatio: source.meshes / batched.meshes,
      attributeByteMultiplierAfterBatch: batched.attributeArrayBytes / source.attributeArrayBytes,
    },
    indexedBatchChange: {
      equivalentNonIndexedBatchBytes:
        nonIndexedBatch.attributeArrayBytes +
        nonIndexedBatch.indexArrayBytes +
        (batched.attributeBytesByName['line.position'] ?? 0),
      currentIndexedBatchBytes: batched.attributeArrayBytes + batched.indexArrayBytes,
      bytesSaved:
        nonIndexedBatch.attributeArrayBytes +
        nonIndexedBatch.indexArrayBytes +
        (batched.attributeBytesByName['line.position'] ?? 0) -
        (batched.attributeArrayBytes + batched.indexArrayBytes),
      percentSaved:
        ((nonIndexedBatch.attributeArrayBytes +
          nonIndexedBatch.indexArrayBytes +
          (batched.attributeBytesByName['line.position'] ?? 0) -
          (batched.attributeArrayBytes + batched.indexArrayBytes)) /
          (nonIndexedBatch.attributeArrayBytes +
            nonIndexedBatch.indexArrayBytes +
            (batched.attributeBytesByName['line.position'] ?? 0))) *
        100,
      trianglesBeforeAndAfter: { before: nonIndexedBatch.triangles, after: batched.triangles },
      note: 'Non-indexed reference is measured by applying BufferGeometry.toNonIndexed() to the same actual merged batch meshes; contours are unchanged. This isolates index-preservation array savings on current geometry.',
    },
    limitations: [
      'These are CPU-side authored Three.js typed-array totals, not a measurement of GPU VRAM, driver allocations, WebGPU repacking, shader/material/texture storage, or actual frame rate.',
      'Source geometry is transient: production descriptors regenerate it for rebuilds. The retained total inspects pending/legacy source-group references after drain and counts persistent contour copies plus batches. This is structural reachability accounting, not a garbage-collector or browser heap measurement.',
      'Draw submission object counts are geometry-bearing source meshes or batch meshes plus line objects, not measured renderer draw calls; shadows, scene infrastructure, material passes, and backend behavior are excluded.',
      'The report counts actual generated triangles, not visible pixels or perceptual detail.',
    ],
  };
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(
    `${JSON.stringify({ outputPath, ...sources.input, sourceMeshes: source.meshes, sourceTriangles: source.triangles, sourceAttributeBytes: source.attributeArrayBytes, sourceIndexBytes: source.indexArrayBytes, batchMeshes: batched.meshes, batchTriangles: batched.triangles, batchAttributeBytes: batched.attributeArrayBytes, batchIndexBytes: batched.indexArrayBytes, families: familyRows.map(({ kind, buildings, triangles, attributeArrayBytes }) => ({ kind, buildings, triangles, attributeArrayBytes })) }, null, 2)}\n`,
  );

  chunks.clear();
  art.dispose();
} finally {
  await rm(cache, { recursive: true, force: true });
}
