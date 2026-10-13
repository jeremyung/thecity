// CPU-only production CameraDirector shot-selection diagnostic. It does not
// instantiate OrbitControls or run a browser renderer/GPU.
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import { build } from 'esbuild';
import { performance } from 'node:perf_hooks';
import * as THREE from 'three';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const savePath = '.tools/midrise-cladding-postmerge/after-opening.save.json';
const growthPath = '.tools/movement-support/browser/growth-575.json';
const bundlePath = '.tools/watch-cost/director.mjs';
const outputPath = 'validation/watch-shot-cost.json';
const rounds = 5;
const fov50mm = 2 * Math.atan(12 / 50) * 180 / Math.PI;
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const quantile = (values, q) => [...values].sort((a, b) => a - b)[Math.min(values.length - 1, Math.floor(values.length * q))];
const summarize = values => ({
  n: values.length,
  p50Ms: values.length ? quantile(values, .5) : 0,
  p95Ms: values.length ? quantile(values, .95) : 0,
  maxMs: values.length ? Math.max(...values) : 0,
});
const normalize = value => {
  if (value === null || typeof value !== 'object') return value;
  if (value.isVector3) return { x: value.x, y: value.y, z: value.z };
  if (Array.isArray(value)) return value.map(normalize);
  return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalize(item)]));
};

const [saveBytes, growthBytes, wasmBytes] = await Promise.all([
  readFile(savePath), readFile(growthPath), readFile('public/wasm/city_engine_bg.wasm'),
]);
await mkdir('.tools/watch-cost', { recursive: true });
await Promise.all([
  build({ entryPoints: ['web/src/camera/director.ts'], bundle: true, platform: 'node', format: 'esm',
    packages: 'external', outfile: bundlePath }),
  build({ entryPoints: ['web/src/camera/watch-visibility.ts'], bundle: true, platform: 'node', format: 'esm',
    packages: 'external', outfile: '.tools/watch-cost/watch-visibility.mjs' }),
]);
const [{ CameraDirector }, { chooseStreetWatchPosition: chooseStreet }] = await Promise.all([
  import('../.tools/watch-cost/director.mjs'),
  import('../.tools/watch-cost/watch-visibility.mjs'),
]);
initSync({ module: wasmBytes });

function restoreWorld(bytes, label) {
  const engine = CityEngine.restore(bytes.toString());
  try { return { label, world: JSON.parse(engine.snapshot()), compatible: true }; }
  finally { engine.free(); }
}

const inputs = [
  { label: 'after-opening-save', path: savePath, bytes: saveBytes },
  { label: 'growth-575', path: growthPath, bytes: growthBytes },
];
const restored = inputs.map(input => {
  try { return { ...input, ...restoreWorld(input.bytes, input.label) }; }
  catch (error) { return { ...input, compatible: false, error: String(error) }; }
});

function makeHarness(world) {
  const director = Object.create(CameraDirector.prototype);
  director.camera = new THREE.PerspectiveCamera(fov50mm, 847 / 764, .1, 2_000);
  director.cameraObstacles = director.cameraObstacleEnvelopes(world);
  director.obstacleRevision = 1;
  director.skylineClusterCache = undefined;
  director.anchors = director.buildAnchors(world);
  director.desiredTarget = new THREE.Vector3();
  director.desiredPosition = new THREE.Vector3();
  director.streetWatchShot = null;
  return director;
}

function measureWorld(world) {
  const director = makeHarness(world);
  const buildAnchorsMs = [];
  for (let round = 0; round < rounds + 1; round++) {
    const start = performance.now();
    const anchors = director.buildAnchors(world);
    const elapsed = performance.now() - start;
    if (round > 0) buildAnchorsMs.push(elapsed);
    if (round === rounds) director.anchors = anchors;
  }
  const districts = director.anchors.filter(anchor => anchor.id.startsWith('district:') && anchor.streetForward);
  const shotMs = [];
  for (let round = 0; round < rounds + 1; round++) {
    for (const anchor of districts) {
      director.streetWatchShot = null;
      const distance = director.anchorDistance(anchor);
      const start = performance.now();
      director.setAnchorPose(anchor, distance);
      const elapsed = performance.now() - start;
      if (round > 0) shotMs.push(elapsed);
    }
  }

  let visibleDistricts = 0;
  let noClearDistricts = 0;
  let candidateChecks = 0;
  const visibleRoadIds = new Set();
  for (const anchor of districts) {
    const subjects = anchor.streetSubjects ?? [{ x: anchor.x, z: anchor.z, roadId: anchor.streetRoadId, forward: anchor.streetForward }];
    let visible = false;
    const distance = director.anchorDistance(anchor);
    for (const subject of subjects) {
      candidateChecks++;
      const result = chooseStreet({ x: subject.x, y: 1, z: subject.z }, { x: subject.forward.x, z: subject.forward.z },
        new THREE.Vector3(.65, .28, .7).normalize(), distance, director.cameraObstacles);
      if (result.visible) { visible = true; visibleRoadIds.add(subject.roadId); break; }
    }
    if (visible) visibleDistricts++;
    else noClearDistricts++;
  }

  return {
    world: { seed: world.seed, tick: world.tick, buildings: world.buildings.length, roads: world.roads.length,
      anchors: director.anchors.length, streetDistrictAnchors: districts.length },
    timings: { buildAnchors: summarize(buildAnchorsMs), coldSetAnchorPose: summarize(shotMs),
      roundsPerDistrict: rounds },
    visibility: { visibleDistrictAnchors: visibleDistricts, noClearDistrictAnchors: noClearDistricts,
      testedStreetCandidates: candidateChecks, distinctChosenRoads: visibleRoadIds.size },
    sample: districts.slice(0, 5).map(anchor => {
      director.streetWatchShot = null;
      director.setAnchorPose(anchor, director.anchorDistance(anchor));
      return { id: anchor.id, subjectCount: anchor.streetSubjects?.length ?? 1,
        target: normalize(director.desiredTarget), camera: normalize(director.desiredPosition) };
    }),
  };
}

const worlds = restored.filter(item => item.compatible).map(item => ({ label: item.label, report: measureWorld(item.world) }));
const report = {
  schemaVersion: 1,
  scope: 'CPU-only measurement of CameraDirector.buildAnchors and cold district setAnchorPose calls on WASM-restored snapshots; no browser, renderer, GPU, or frame-time claim.',
  host: { cpu: os.cpus()[0]?.model ?? 'unknown', platform: os.platform(), arch: os.arch(), node: process.version },
  camera: { viewport: [847, 764], lensFocalLengthMm: 50, sensorHeightMm: 24, fovDegrees: fov50mm, near: .1, far: 2_000 },
  inputs: { wasmSha256: sha256(wasmBytes), directorBundle: bundlePath,
    files: restored.map(({ label, path, bytes, compatible, error }) => ({ label, path, sha256: sha256(bytes), compatible, ...(error ? { error } : {}) })) },
  measurement: { warmupRounds: 1, measuredRounds: rounds, districtShotsPerRound: 'one cold setAnchorPose per district street anchor', worlds },
  limitations: [
    'Uses Object.create(CameraDirector.prototype), production obstacle-envelope and anchor methods, and live clearance/visibility helpers; no controls or transition travel are simulated.',
    'The helper visibility count reports geometric line clearance for candidate street samples, not a rendered image or human judgment of composition.',
    'Node process timings are CPU-only and do not represent browser main-thread contention, garbage collection during a long session, GPU work, or camera-transition frame cost.',
  ],
};
await writeFile(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ outputPath, inputs: report.inputs.files, measurement: report.measurement }, null, 2));
