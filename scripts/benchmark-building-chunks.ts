import * as THREE from 'three/webgpu';
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { BuildingChunks, buildingChunkKey } from '../web/src/render/building-chunks';

const BUILDING_COUNT = 128;
const MESHES_PER_BUILDING = 12;
const BUILDINGS_PER_UPDATE = 2;

type Work = {
  updateCalls: number;
  rebuildCount: number;
  updateTotalMs: number;
  updateMaxMs: number;
  rebuildTotalMs: number;
  rebuildMaxMs: number;
  geometryClones: number;
  nonIndexedVertices: number;
};

function sourceGroups(material: THREE.Material): THREE.Group[] {
  return Array.from({ length: BUILDING_COUNT }, (_, buildingIndex) => {
    const group = new THREE.Group();
    group.position.set(10 + (buildingIndex % 16) * 11, 0, 12 + Math.floor(buildingIndex / 16) * 20);
    group.name = `Synthetic building ${buildingIndex}`;
    for (let meshIndex = 0; meshIndex < MESHES_PER_BUILDING; meshIndex += 1) {
      const width = 0.8 + (meshIndex % 3) * 0.14;
      const height = 1 + (meshIndex % 5) * 0.2;
      const depth = 0.7 + (meshIndex % 4) * 0.13;
      const geometry = new THREE.BoxGeometry(width, height, depth);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set((meshIndex % 4) * 0.7, height * 0.5, Math.floor(meshIndex / 4) * 0.7);
      group.add(mesh);
    }
    return group;
  });
}

function makeChunks() {
  return new BuildingChunks((material) => material, (dispose) => dispose());
}

function updateOnce(chunks: BuildingChunks, work: Work, pendingKeys?: ReadonlySet<string>) {
  const beforeClones = counters.geometryClones;
  const beforeVertices = counters.nonIndexedVertices;
  const pendingBefore = chunks.pendingCount;
  const started = performance.now();
  chunks.update(pendingKeys);
  const elapsed = performance.now() - started;
  const committed = chunks.pendingCount < pendingBefore;
  work.updateCalls += 1;
  work.updateTotalMs += elapsed;
  work.updateMaxMs = Math.max(work.updateMaxMs, elapsed);
  if (committed) {
    work.rebuildCount += 1;
    work.rebuildTotalMs += elapsed;
    work.rebuildMaxMs = Math.max(work.rebuildMaxMs, elapsed);
  }
  work.geometryClones += counters.geometryClones - beforeClones;
  work.nonIndexedVertices += counters.nonIndexedVertices - beforeVertices;
}

const counters = { geometryClones: 0, nonIndexedVertices: 0 };

function instrumentGeometryWork<T>(run: () => T): T {
  const clone = THREE.BufferGeometry.prototype.clone;
  const toNonIndexed = THREE.BufferGeometry.prototype.toNonIndexed;
  THREE.BufferGeometry.prototype.clone = function (...args: Parameters<typeof clone>) {
    counters.geometryClones += 1;
    return clone.apply(this, args);
  };
  THREE.BufferGeometry.prototype.toNonIndexed = function (...args: Parameters<typeof toNonIndexed>) {
    const geometry = toNonIndexed.apply(this, args);
    counters.nonIndexedVertices += geometry.getAttribute('position')?.count ?? 0;
    return geometry;
  };
  try {
    return run();
  } finally {
    THREE.BufferGeometry.prototype.clone = clone;
    THREE.BufferGeometry.prototype.toNonIndexed = toNonIndexed;
  }
}

function emptyWork(): Work {
  return {
    updateCalls: 0, rebuildCount: 0, updateTotalMs: 0, updateMaxMs: 0,
    rebuildTotalMs: 0, rebuildMaxMs: 0, geometryClones: 0, nonIndexedVertices: 0,
  };
}

function finalBatchFacts(chunks: BuildingChunks) {
  let meshCount = 0;
  let mergedVertices = 0;
  chunks.root.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    meshCount += 1;
    mergedVertices += object.geometry.getAttribute('position')?.count ?? 0;
  });
  return { meshCount, mergedVertices };
}

function disposeSources(groups: THREE.Group[]) {
  for (const group of groups) {
    group.traverse((object) => {
      if (object instanceof THREE.Mesh) object.geometry.dispose();
    });
  }
}

async function main() {
  const outputPath = resolve(process.argv[2] ?? 'validation/building-chunks-before.json');
  const material = new THREE.MeshBasicMaterial({ color: '#c9c4b7' });
  const groups = sourceGroups(material);
  const sourceVertexCount = groups[0].children.reduce((sum, child) => {
    const geometry = (child as THREE.Mesh).geometry;
    return sum + (geometry.index ? geometry.index.count : geometry.getAttribute('position').count);
  }, 0);

  const progressiveChunks = makeChunks();
  const progressiveWork = emptyWork();
  let progressiveSetMs = 0;
  let progressiveTotalMs = 0;
  instrumentGeometryWork(() => {
    const scenarioStarted = performance.now();
    for (let start = 0; start < groups.length; start += BUILDINGS_PER_UPDATE) {
      const setStarted = performance.now();
      for (let i = start; i < Math.min(start + BUILDINGS_PER_UPDATE, groups.length); i += 1) {
        progressiveChunks.set(i, groups[i]);
      }
      progressiveSetMs += performance.now() - setStarted;
      updateOnce(progressiveChunks, progressiveWork);
    }
    progressiveTotalMs = performance.now() - scenarioStarted;
  });
  const progressiveFacts = finalBatchFacts(progressiveChunks);
  progressiveChunks.clear();

  const finalChunks = makeChunks();
  const finalWork = emptyWork();
  let finalSetMs = 0;
  let finalTotalMs = 0;
  instrumentGeometryWork(() => {
    const scenarioStarted = performance.now();
    const setStarted = performance.now();
    for (let i = 0; i < groups.length; i += 1) finalChunks.set(i, groups[i]);
    finalSetMs = performance.now() - setStarted;
    updateOnce(finalChunks, finalWork);
    finalTotalMs = performance.now() - scenarioStarted;
  });
  const finalFacts = finalBatchFacts(finalChunks);
  finalChunks.clear();

  const coalescedChunks = makeChunks();
  const coalescedWork = emptyWork();
  let coalescedSetMs = 0;
  let coalescedTotalMs = 0;
  const pendingChunk = new Set([buildingChunkKey(groups[0].position.x, groups[0].position.z)]);
  instrumentGeometryWork(() => {
    const scenarioStarted = performance.now();
    for (let start = 0; start < groups.length; start += BUILDINGS_PER_UPDATE) {
      const setStarted = performance.now();
      const end = Math.min(start + BUILDINGS_PER_UPDATE, groups.length);
      for (let i = start; i < end; i += 1) coalescedChunks.set(i, groups[i]);
      coalescedSetMs += performance.now() - setStarted;
      const finalArrival = end === groups.length;
      updateOnce(coalescedChunks, coalescedWork, finalArrival ? undefined : pendingChunk);
    }
    coalescedTotalMs = performance.now() - scenarioStarted;
  });
  const coalescedFacts = finalBatchFacts(coalescedChunks);
  coalescedChunks.clear();

  disposeSources(groups);
  material.dispose();

  const report = {
    benchmark: 'BuildingChunks synthetic CPU batching workload',
    measuredAt: new Date().toISOString(),
    runtime: { node: process.version, platform: process.platform, arch: process.arch },
    scope: 'Synthetic CPU-only comparison. This does not measure WebGPU/WebGL upload, shader compilation, browser frame pacing, or real authored buildings.',
    workload: {
      buildings: BUILDING_COUNT,
      meshesPerBuilding: MESHES_PER_BUILDING,
      totalSourceMeshes: BUILDING_COUNT * MESHES_PER_BUILDING,
      sourceVerticesPerBuilding: sourceVertexCount,
      chunkSizeMeters: 200,
      allBuildingsShareOneChunk: true,
      sourceGroupsSharedBetweenScenarios: true,
      scenarioCadence: 'Progressive adds two buildings then calls update() without a pending key; one-final sets all groups before one update(); coalesced adds two groups per call and passes its chunk key as pending until the last group arrives. The production 12-update deferral cap remains active.',
    },
    progressive: {
      setMs: progressiveSetMs,
      updateMs: progressiveWork.updateTotalMs,
      maxSingleUpdateMs: progressiveWork.updateMaxMs,
      rebuildMs: progressiveWork.rebuildTotalMs,
      maxRebuildMs: progressiveWork.rebuildMaxMs,
      totalMs: progressiveTotalMs,
      updateCalls: progressiveWork.updateCalls,
      rebuilds: progressiveWork.rebuildCount,
      geometryCloneCount: progressiveWork.geometryClones,
      nonIndexedVerticesPresentedToMerge: progressiveWork.nonIndexedVertices,
      derivedSourceTraversalVisits: progressiveWork.geometryClones
        + progressiveWork.geometryClones / MESHES_PER_BUILDING,
      finalBatchMeshes: progressiveFacts.meshCount,
      finalMergedVertices: progressiveFacts.mergedVertices,
    },
    oneFinalUpdate: {
      setMs: finalSetMs,
      updateMs: finalWork.updateTotalMs,
      maxSingleUpdateMs: finalWork.updateMaxMs,
      rebuildMs: finalWork.rebuildTotalMs,
      maxRebuildMs: finalWork.rebuildMaxMs,
      totalMs: finalTotalMs,
      updateCalls: finalWork.updateCalls,
      rebuilds: finalWork.rebuildCount,
      geometryCloneCount: finalWork.geometryClones,
      nonIndexedVerticesPresentedToMerge: finalWork.nonIndexedVertices,
      derivedSourceTraversalVisits: finalWork.geometryClones
        + finalWork.geometryClones / MESHES_PER_BUILDING,
      finalBatchMeshes: finalFacts.meshCount,
      finalMergedVertices: finalFacts.mergedVertices,
    },
    coalesced: {
      setMs: coalescedSetMs,
      updateMs: coalescedWork.updateTotalMs,
      maxSingleUpdateMs: coalescedWork.updateMaxMs,
      rebuildMs: coalescedWork.rebuildTotalMs,
      maxRebuildMs: coalescedWork.rebuildMaxMs,
      totalMs: coalescedTotalMs,
      updateCalls: coalescedWork.updateCalls,
      rebuilds: coalescedWork.rebuildCount,
      geometryCloneCount: coalescedWork.geometryClones,
      nonIndexedVerticesPresentedToMerge: coalescedWork.nonIndexedVertices,
      derivedSourceTraversalVisits: coalescedWork.geometryClones
        + coalescedWork.geometryClones / MESHES_PER_BUILDING,
      finalBatchMeshes: coalescedFacts.meshCount,
      finalMergedVertices: coalescedFacts.mergedVertices,
    },
    ratios: {
      cloneWorkProgressiveOverSingle: finalWork.geometryClones > 0 ? progressiveWork.geometryClones / finalWork.geometryClones : null,
      mergeInputVerticesProgressiveOverSingle: finalWork.nonIndexedVertices > 0 ? progressiveWork.nonIndexedVertices / finalWork.nonIndexedVertices : null,
      updateCpuProgressiveOverSingle: finalWork.updateTotalMs > 0 ? progressiveWork.updateTotalMs / finalWork.updateTotalMs : null,
      totalCpuProgressiveOverSingle: finalTotalMs > 0 ? progressiveTotalMs / finalTotalMs : null,
      cloneWorkCoalescedOverProgressive: progressiveWork.geometryClones > 0 ? coalescedWork.geometryClones / progressiveWork.geometryClones : null,
      mergeInputVerticesCoalescedOverProgressive: progressiveWork.nonIndexedVertices > 0 ? coalescedWork.nonIndexedVertices / progressiveWork.nonIndexedVertices : null,
      updateCpuCoalescedOverProgressive: progressiveWork.updateTotalMs > 0 ? coalescedWork.updateTotalMs / progressiveWork.updateTotalMs : null,
    },
  };

  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
}

main().catch((error) => {
  process.stderr.write(`${error instanceof Error ? error.stack ?? error.message : String(error)}\n`);
  process.exitCode = 1;
});
