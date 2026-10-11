// Diagnostic against the current station kit. Run after npm run wasm.
// This checks real movement output; it does not manufacture residents or queues.
import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';

const [savePath, reportPath] = process.argv.slice(2);
if (!savePath) throw new Error('Usage: node scripts/audit-station-passengers.mjs SAVE [REPORT]');
initSync({ module: readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)) });
const engine = CityEngine.restore(readFileSync(savePath, 'utf8'));
try {
  const state = JSON.parse(engine.save());
  const snapshot = JSON.parse(engine.snapshot());
  const station = state.nodes.find(node => node.kind === 'station');
  const layout = snapshot.rail_platforms.find(platform => platform.node_id === station?.id);
  if (!station || !layout) throw new Error('Checkpoint has no station platform descriptor');
  const { ux, uz } = layout;
  const waiting = new Set(state.residents.filter(resident => resident.transfer === 'waiting_station').map(resident => resident.id));
  if (!waiting.size) throw new Error('Checkpoint has no station queue; no acceptance evidence collected');

  // Explicit fixture for CityRenderer.rebuildRoads: box(5.2,1,23.5,5.2,2,49).
  // Abort when the kit changes so a stale reference cannot certify new geometry.
  const kit = readFileSync(new URL('../web/src/render/station-kit.ts', import.meta.url), 'utf8');
  if (!kit.includes('s.box(5.2, top - 1, 23.5, 5.2, 2, 49,')) throw new Error('Update audit to the changed station kit');
  const slab = { minX: 2.6, maxX: 7.8, minZ: -1, maxZ: 48 };
  const actors = [], movement = engine.movements();
  for (let offset = 0; offset < movement.length; offset += 6) {
    const [id, x, z, heading, kind, surfaceY] = movement.subarray(offset, offset + 6);
    if (!waiting.has(id)) continue;
    // Inverse of the kit's Three.js Y rotation, not the engine's queue formula.
    const localX = (x - station.x) * uz - (z - station.z) * ux;
    const localZ = (x - station.x) * ux + (z - station.z) * uz;
    actors.push({ id, x, z, heading, kind, surfaceY, localX, localZ,
      onPlatformSurface: Math.abs(surfaceY - layout.surface_y) < .001,
      insidePlatform: localX >= slab.minX && localX <= slab.maxX && localZ >= slab.minZ && localZ <= slab.maxZ });
  }
  if (actors.length !== waiting.size) throw new Error('Some queued residents are missing from movement output');
  const report = {
    revision: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    worktreeDirty: !!execFileSync('git', ['status', '--porcelain'], {encoding:'utf8'}).trim(),
    wasmSha256: createHash('sha256').update(readFileSync(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url))).digest('hex'),
    seed: state.seed, tick: state.tick, station: station.id,
    geometrySource: 'web/src/render/station-kit.ts: buildStationPlatform',
    slab, queued: waiting.size, incorrectHeight: actors.filter(actor => !actor.onPlatformSurface).length, outsidePlatform: actors.filter(actor => !actor.insidePlatform).length, actors,
    limitations: 'Queue XZ and surface height only. Approach paths, boarding, motion, and style fidelity require separate verification.',
  };
  const json = JSON.stringify(report, null, 2) + '\n';
  if (reportPath) writeFileSync(reportPath, json);
  console.log(json);
  if (report.outsidePlatform || report.incorrectHeight) process.exitCode = 1;
} finally {
  engine.free();
}
