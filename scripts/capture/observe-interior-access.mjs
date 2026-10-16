import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { CityEngine, initSync } from '../../public/wasm/city_engine.js';

const [input, output, horizonText = '32000'] = process.argv.slice(2);
const horizon = Number(horizonText);
if (
  !input ||
  !output ||
  resolve(input) === resolve(output) ||
  !Number.isSafeInteger(horizon) ||
  horizon < 1 ||
  horizon > 2000000
) {
  throw new Error(
    'Usage: node scripts/capture/observe-interior-access.mjs input-save.json distinct-report.json [ticks 1..2000000]',
  );
}
const source = await readFile(input, 'utf8');
const wasm = await readFile(new URL('../../public/wasm/city_engine_bg.wasm', import.meta.url));
const hash = (value) => createHash('sha256').update(value).digest('hex');
initSync({ module: wasm });
const engine = CityEngine.restore(source);
const observations = new Map();
const near = (a, b) => Number.isFinite(a) && Number.isFinite(b) && Math.abs(a - b) < 0.002;
const matchesLot = (parcel, lot) =>
  ['x', 'z', 'width', 'depth', 'rotation'].every((key) => near(parcel[key], lot[key]));
const requireInvariant = (condition, message) => {
  if (!condition) throw new Error(message);
};
function verifyReplay(save) {
  const left = CityEngine.restore(save),
    right = CityEngine.restore(save);
  try {
    requireInvariant(
      left.save() === save && right.save() === save,
      'Noncanonical checkpoint restore',
    );
    left.step(2000);
    right.step(2000);
    requireInvariant(left.save() === right.save(), 'Checkpoint replay differs');
  } finally {
    left.free();
    right.free();
  }
}
try {
  const initial = JSON.parse(engine.save());
  let state = initial;
  // Sample every eight ticks. Report the interval explicitly: absence of an
  // observation cannot establish that no shorter-lived plan ever existed.
  for (let elapsed = 0; elapsed < horizon;) {
    const step = Math.min(8, horizon - elapsed);
    engine.step(step);
    elapsed += step;
    const save = engine.save();
    state = JSON.parse(save);
    const plan = state.pending_interior_access;
    if (plan) {
      const branch = state.roads.find((road) => road.id === plan.branch_road_id);
      const project = state.projects.find((project) => project.id === plan.project_id);
      requireInvariant(
        branch && branch.stage !== 'open' && !branch.connected,
        'Pending branch became routable',
      );
      requireInvariant(
        project?.kind === 'road' && project.progress < 1,
        'Missing active branch construction',
      );
      requireInvariant(
        branch.from === plan.split_node_id && branch.to === plan.branch_endpoint_id,
        'Branch endpoints differ from reservation',
      );
      requireInvariant(
        plan.selected_lots.length >= 2,
        'Access street promises fewer than two lots',
      );
      requireInvariant(
        plan.selected_lots.every((lot) => !state.parcels.some((parcel) => matchesLot(parcel, lot))),
        'Reserved lot published before road opening',
      );
      if (!observations.has(plan.project_id)) {
        const snapshot = JSON.parse(engine.snapshot());
        requireInvariant(
          snapshot.roads.some((road) => road.id === branch.id && road.stage !== 'open'),
          'Construction absent from public snapshot',
        );
        verifyReplay(save);
        const checkpoint = `${output}.project-${plan.project_id}.pending.json`;
        await writeFile(checkpoint, save + '\n');
        observations.set(plan.project_id, {
          plan,
          firstObservedTick: state.tick,
          pendingCheckpoint: checkpoint,
          pendingReplayTicks: 2000,
          openedTick: null,
        });
      }
    }
    for (const observation of observations.values()) {
      if (
        observation.openedTick !== null ||
        state.pending_interior_access?.project_id === observation.plan.project_id
      )
        continue;
      const completed = observation.plan;
      const road = state.roads.find((road) => road.id === completed.branch_road_id);
      requireInvariant(
        road?.stage === 'open' && road.connected,
        'Reservation vanished without connected road',
      );
      for (const lot of completed.selected_lots) {
        const parcel = state.parcels.find((parcel) => matchesLot(parcel, lot));
        requireInvariant(parcel, 'Promised lot missing after completion');
        requireInvariant(
          state.parcel_frontages.some(
            (frontage) =>
              frontage.parcel_id === parcel.id &&
              frontage.road_id === road.id &&
              frontage.side === lot.side &&
              near(frontage.along, lot.along),
          ),
          'Completed lot has incorrect frontage',
        );
      }
      verifyReplay(save);
      observation.openedTick = state.tick;
      observation.completedReplayTicks = 2000;
      observation.completedCheckpoint = `${output}.project-${completed.project_id}.open.json`;
      await writeFile(observation.completedCheckpoint, save + '\n');
    }
  }
  requireInvariant(state.tick - initial.tick === horizon, 'Requested horizon was not simulated');
  const finalSave = engine.save();
  const finalCheckpoint = `${output}.final.json`;
  await writeFile(finalCheckpoint, finalSave + '\n');
  const report = {
    input,
    wasmSha256: hash(wasm),
    sourceSha256: hash(source.trim()),
    sampleEveryTicks: 8,
    finalCheckpoint,
    finalSaveSha256: hash(finalSave),
    startTick: initial.tick,
    endTick: state.tick,
    before: {
      buildings: initial.buildings.length,
      roads: initial.roads.length,
      parcels: initial.parcels.length,
    },
    after: {
      buildings: state.buildings.length,
      roads: state.roads.length,
      parcels: state.parcels.length,
    },
    observedPlans: observations.size,
    completedPlans: [...observations.values()].filter((value) => value.openedTick !== null).length,
    observations: [...observations.values()],
    limitation:
      'Node executes the public WASM binary. This is not a browser rendering or performance measurement; eight-tick sampling can miss shorter-lived plans.',
  };
  await writeFile(output, JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
} finally {
  engine.free();
}
