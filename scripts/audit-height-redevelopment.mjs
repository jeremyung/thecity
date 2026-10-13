// Exercise admission, demolition and reconstruction through ordinary engine ticks.
// No edited fixture state or private method calls. Output saves support UI import.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const [output = '.tools/height-redevelopment', seedText = '87231', limitText = '240000'] = process.argv.slice(2);
const seed = Number(seedText), limit = Number(limitText);
if (!Number.isSafeInteger(seed) || seed < 0 || seed > 0xffffffff || !Number.isSafeInteger(limit) || limit < 1) {
  throw new Error('Expected uint32 seed and positive tick limit');
}
const { CityEngine, initSync } = await import(pathToFileURL(resolve('public/wasm/city_engine.js')));
initSync({ module: readFileSync('public/wasm/city_engine_bg.wasm') });
mkdirSync(output, { recursive: true });
const live = new CityEngine(seed);
live.set_development_rate(16);
const captures = [], captured = new Set();
let replay = null, replayChecks = 0, firstProject = null, completed = false;
let previousSave = live.save();
const started = performance.now();
try {
  for (let tick = 50; tick <= limit; tick += 50) {
    live.step(50);
    if (replay) {
      replay.step(50);
      if (live.save() !== replay.save()) throw new Error(`Live/restore state diverged at ${tick}`);
      replayChecks++;
    }
    const save = live.save(), state = JSON.parse(save);
    const pending = state.height_redevelopment;
    const building = pending ? state.buildings.find(b => b.id === pending.building_id) : null;
    if (pending && !firstProject) {
      writeFileSync(resolve(output, 'before-renewal.json'), previousSave);
      firstProject = { ...pending, tick, totalBuildings: state.buildings.length,
        realizedLots: state.masterplan.lot_parcels.filter(Boolean).length,
        plannedLots: state.masterplan.lot_parcels.length };
      replay = CityEngine.restore(save);
      if (replay.save() !== save) throw new Error('Immediate demolition restore differs');
    }
    let label = null;
    if (building?.stage === 'demolition') label = building.progress < .5 ? 'demolition-early' : 'demolition-late';
    else if (building) label = `construction-${building.stage}`;
    else if (firstProject) { label = 'completed'; completed = true; }
    if (label && !captured.has(label)) {
      captured.add(label);
      writeFileSync(resolve(output, `${label}.json`), save);
      const check = CityEngine.restore(save);
      if (check.save() !== save) throw new Error(`Phase restore differs: ${label}`);
      check.free();
      captures.push({ label, tick, building: building ?? state.buildings.find(b => b.id === firstProject.building_id) });
      console.log(`${label}: tick ${tick}`);
    }
    previousSave = save;
    if (completed) break;
  }
  const report = { seed, developmentRate: 16, tickLimit: limit, firstProject, completed,
    replayChecks, replayTicks: replayChecks * 50, captures, elapsedMs: performance.now() - started,
    scope: 'Normal fresh Node WASM growth and live/restored save comparison. Not browser or visual acceptance.' };
  writeFileSync(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ completed, firstProject, replayChecks, elapsedMs: report.elapsedMs }));
  if (!completed) process.exitCode = 1;
} finally { replay?.free(); live.free(); }
