import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const BASE_URL = process.env.LIVE_TICK_BASE_URL ?? 'http://127.0.0.1:5173';
const FIXTURE_PATH = resolve(
  ROOT,
  'tests/fixtures/validation/early-reservation-wasm-87231-save.json',
);
const WASM_PATH = resolve(ROOT, 'public/wasm/city_engine_bg.wasm');
const REPORT_PATH = resolve(
  process.env.LIVE_TICK_REPORT_PATH ?? 'validation/live-tick-profile.json',
);
const FINAL_SAVE_PATH = process.env.LIVE_TICK_FINAL_SAVE_PATH
  ? resolve(process.env.LIVE_TICK_FINAL_SAVE_PATH)
  : null;
const TICK_COUNT = Number(process.env.LIVE_TICK_COUNT ?? 6_000);
const YIELD_EVERY = 100;
const CHECKPOINT_EVERY = 200;
const TRANSITION_SAMPLE_EVERY = Number(
  process.env.LIVE_TICK_TRANSITION_SAMPLE_EVERY ?? CHECKPOINT_EVERY,
);
const STARTUP_TIMEOUT_MS = Number(process.env.LIVE_TICK_STARTUP_TIMEOUT_MS ?? 30_000);
const OVERALL_TIMEOUT_MS = Number(process.env.LIVE_TICK_OVERALL_TIMEOUT_MS ?? 90_000);
const SEED = 87_231;
let activeBrowser;

assert.ok(
  Number.isInteger(TICK_COUNT) && TICK_COUNT >= 200 && TICK_COUNT <= 100_000,
  'LIVE_TICK_COUNT must be an integer from 200 to 100000',
);
assert.ok(
  STARTUP_TIMEOUT_MS >= 1_000 && OVERALL_TIMEOUT_MS >= STARTUP_TIMEOUT_MS,
  'timeouts must be positive and overall timeout must cover startup',
);
assert.ok(
  Number.isInteger(TRANSITION_SAMPLE_EVERY) && TRANSITION_SAMPLE_EVERY >= 1,
  'LIVE_TICK_TRANSITION_SAMPLE_EVERY must be a positive integer',
);

const fixtureText = readFileSync(FIXTURE_PATH, 'utf8');
const fixture = JSON.parse(fixtureText);
assert.equal(fixture.seed, SEED, 'mature fixture seed changed');
const wasmHash = createHash('sha256').update(readFileSync(WASM_PATH)).digest('hex');

function withTimeout(promise, ms, label) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms} ms`)), ms);
    }),
  ]).finally(() => clearTimeout(timer));
}

function percentile(sorted, p) {
  return sorted[Math.ceil((sorted.length - 1) * p)];
}

async function main() {
  let browser;
  let page;
  try {
    const vite = await fetch(`${BASE_URL}/@vite/client`, {
      signal: AbortSignal.timeout(STARTUP_TIMEOUT_MS),
    });
    assert.equal(vite.status, 200, `Vite must already be running at ${BASE_URL}`);
    browser = await withTimeout(
      chromium.launch({ headless: true, timeout: STARTUP_TIMEOUT_MS }),
      STARTUP_TIMEOUT_MS,
      'Chromium startup',
    );
    activeBrowser = browser;
    page = await browser.newPage();
    const fixtureResponse = await withTimeout(
      page.goto(`${BASE_URL}/tests/fixtures/validation/early-reservation-wasm-87231-save.json`, {
        waitUntil: 'domcontentloaded',
      }),
      STARTUP_TIMEOUT_MS,
      'same-origin page navigation',
    );
    assert.equal(fixtureResponse?.status(), 200, 'fixture URL should be served by Vite');

    const workerSource = String.raw`
      let wasm;
      let engine;
      function summarize(saveText, completedBranchIds = []) {
        const save = JSON.parse(saveText);
        const world = JSON.parse(engine.snapshot());
        const live = save.live_access_checkpoint ?? null;
        const pending = save.pending_interior_access ?? null;
        const roadsById = new Map((save.roads ?? []).map(road => [road.id, road]));
        const projectsById = new Map((save.projects ?? []).map(project => [project.id, project]));
        const branch = pending === null ? null : roadsById.get(pending.branch_road_id) ?? null;
        const project = pending === null ? null : projectsById.get(pending.project_id) ?? null;
        const discovery = live?.planning?.discovery ?? null;
        return {
          tick: save.tick,
          stage: world.stage ?? null,
          revision: save.revision,
          counts: {
            nodes: save.nodes?.length ?? 0,
            roads: save.roads?.length ?? 0,
            parcels: save.parcels?.length ?? 0,
            buildings: save.buildings?.length ?? 0,
            projects: save.projects?.length ?? 0,
            travelers: save.travelers?.length ?? 0,
            residents: save.residents?.length ?? 0,
            trains: save.trains?.length ?? 0,
            buses: save.buses?.length ?? 0,
            events: save.events?.length ?? 0,
          },
          save_bytes: new TextEncoder().encode(saveText).byteLength,
          live_access_present: live !== null,
          live_access_bytes: live === null ? 0 : new TextEncoder().encode(JSON.stringify(live)).byteLength,
          live_access_phase: live?.planning?.phase ?? null,
          discovery_phase: discovery?.stage == null ? null : Object.keys(discovery.stage)[0],
          exhausted_face_count: save.exhausted_access_faces?.length ?? 0,
          selected_face_identity: discovery?.selected_face_identity ?? null,
          pending_interior_branch: pending === null ? null : {
            branch_road_id: pending.branch_road_id,
            road_stage: branch?.stage ?? null,
            project_id: pending.project_id,
            project_stage: project?.stage ?? null,
            planned_lot_count: pending.selected_lots?.length ?? 0,
            actual_frontage_count: branch === null ? null
              : (save.parcel_frontages ?? []).filter(frontage => frontage.road_id === branch.id).length,
          },
          completed_interior_branch_ids: [...completedBranchIds].sort((a, b) => a - b),
          observed_opened_branches: [...completedBranchIds].sort((a, b) => a - b).map(id => ({
            road_id: id,
            stage: roadsById.get(id)?.stage ?? null,
            connected: roadsById.get(id)?.connected ?? false,
            frontage_parcel_ids: (save.parcel_frontages ?? [])
              .filter(frontage => frontage.road_id === id)
              .map(frontage => frontage.parcel_id).sort((a, b) => a - b),
          })),
        };
      }
      self.onmessage = async event => {
        const message = event.data;
        try {
          if (message.type === 'init') {
            const wasmUrl = new URL('/wasm/city_engine.js', self.location.origin).href;
            wasm = await import(wasmUrl);
            if (typeof wasm.CityEngine.prototype.split_open_road !== 'undefined') {
              throw new Error('Production WASM must not expose the profiling mutation API');
            }
            await wasm.default({ module_or_path: new URL('/wasm/city_engine_bg.wasm', self.location.origin).href });
            engine = wasm.CityEngine.restore(message.save);
            const state = summarize(engine.save());
            self.postMessage({ type: 'ready', tick: state.tick, stage: state.stage, counts: state.counts });
            return;
          }
          if (message.type === 'run') {
            if (!engine) throw new Error('engine was not initialized');
            const startTick = JSON.parse(engine.save()).tick;
            const durationsMs = [];
            const checkpoints = [];
            let checkpointOverheadMs = 0;
            let transitionObservationOverheadMs = 0;
            const transitions = [];
            const openedBranchIds = new Set();
            const seenInteriorProjects = new Set();
            const admittedInteriorProjects = new Set();
            const initialSnapshot = JSON.parse(engine.snapshot());
            const initialSave = JSON.parse(engine.save());
            const initialPendingProjectId = initialSave.pending_interior_access?.project_id ?? null;
            const interiorReason = 'A neighborhood access street is being built from an established corridor';
            for (const project of initialSnapshot.projects ?? []) {
              if (project.reason === interiorReason) seenInteriorProjects.add(project.id);
            }
            function observeTransitions(snapshot) {
              const roadsByEndpoints = new Map();
              for (const road of snapshot.roads ?? []) {
                roadsByEndpoints.set(String(road.from) + ':' + String(road.to), road);
                roadsByEndpoints.set(String(road.to) + ':' + String(road.from), road);
              }
              for (const project of snapshot.projects ?? []) {
                if (project.reason !== interiorReason) continue;
                const road = roadsByEndpoints.get(String(project.from) + ':' + String(project.to)) ?? null;
                if (!seenInteriorProjects.has(project.id)) {
                  seenInteriorProjects.add(project.id);
                  admittedInteriorProjects.add(project.id);
                  transitions.push({
                    type: 'construction_admission_observed', tick: snapshot.tick, project_id: project.id,
                    branch_road_id: road?.id ?? null, project_stage: project.stage,
                    branch_stage: road?.stage ?? null,
                  });
                }
                const admissionObservedInRun = admittedInteriorProjects.has(project.id);
                const wasPendingAtRunStart = project.id === initialPendingProjectId;
                if ((admissionObservedInRun || wasPendingAtRunStart)
                    && road?.stage === 'open' && road.connected && !openedBranchIds.has(road.id)) {
                  openedBranchIds.add(road.id);
                  transitions.push({
                    type: 'connected_opening_observed', tick: snapshot.tick, project_id: project.id,
                    branch_road_id: road.id, project_stage: project.stage, branch_stage: road.stage,
                    admission_observed_in_run: admissionObservedInRun,
                  });
                }
              }
            }
            for (let index = 1; index <= message.count; index++) {
              const started = performance.now();
              engine.step(1);
              const ended = performance.now();
              durationsMs.push(ended - started);
              if (index % ${TRANSITION_SAMPLE_EVERY} === 0) {
                const observationStarted = performance.now();
                observeTransitions(JSON.parse(engine.snapshot()));
                transitionObservationOverheadMs += performance.now() - observationStarted;
              }
              if (index % ${CHECKPOINT_EVERY} === 0) {
                const checkpointStarted = performance.now();
                const saveText = engine.save();
                const summary = summarize(saveText, openedBranchIds);
                summary.step_index = index;
                summary.elapsed_tick_index = index;
                checkpoints.push(summary);
                checkpointOverheadMs += performance.now() - checkpointStarted;
              }
              if (index % ${YIELD_EVERY} === 0) await new Promise(resolve => setTimeout(resolve, 0));
            }
            const finalSave = engine.save();
            const final = summarize(finalSave, openedBranchIds);
            const restored = wasm.CityEngine.restore(finalSave);
            const canonicalRestoreEqual = restored.save() === finalSave;
            restored.free?.();
            engine.free?.();
            self.postMessage({ type: 'complete', startTick, count: message.count, durationsMs,
              checkpoints, checkpointOverheadMs, transitionObservationOverheadMs, transitions,
              transitionSampleEvery: ${TRANSITION_SAMPLE_EVERY}, finalSave, final,
              canonicalRestoreEqual });
          }
        } catch (error) {
          self.postMessage({ type: 'error', message: error instanceof Error ? error.message : String(error) });
        }
      };
    `;

    await page.evaluate(
      ({ source }) => {
        const blobUrl = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
        const worker = new Worker(blobUrl, { type: 'module', name: 'live-tick-profile' });
        const state = { worker, blobUrl, messages: [], waiters: [], error: null };
        state.waitFor = (type, timeoutMs) => {
          const existing = state.messages.find((message) => message.type === type);
          if (existing) return Promise.resolve(existing);
          if (state.error) return Promise.reject(new Error(state.error));
          return new Promise((resolveWait, rejectWait) => {
            const waiter = { type, resolve: resolveWait, reject: rejectWait };
            state.waiters.push(waiter);
            const timer = setTimeout(() => {
              state.waiters = state.waiters.filter((item) => item !== waiter);
              rejectWait(new Error(`diagnostic worker ${type} timed out after ${timeoutMs} ms`));
            }, timeoutMs);
            waiter.resolve = (value) => {
              clearTimeout(timer);
              resolveWait(value);
            };
            waiter.reject = (error) => {
              clearTimeout(timer);
              rejectWait(error);
            };
          });
        };
        worker.onmessage = (event) => {
          const message = event.data;
          state.messages.push(message);
          if (message.type === 'error') state.error = message.message;
          const matched = state.waiters.filter(
            (waiter) => waiter.type === message.type || message.type === 'error',
          );
          state.waiters = state.waiters.filter((waiter) => !matched.includes(waiter));
          for (const waiter of matched) {
            if (message.type === 'error') waiter.reject(new Error(message.message));
            else waiter.resolve(message);
          }
        };
        worker.onerror = (event) => {
          state.error = event.message || 'diagnostic worker crashed';
          for (const waiter of state.waiters) waiter.reject(new Error(state.error));
          state.waiters = [];
        };
        window.__liveTickProfile = state;
      },
      { source: workerSource },
    );

    const waitFor = (type, timeoutMs) =>
      page.evaluate(
        ({ type: wantedType, timeout }) => window.__liveTickProfile.waitFor(wantedType, timeout),
        { type, timeout: timeoutMs },
      );
    const readyWait = waitFor('ready', STARTUP_TIMEOUT_MS);
    await page.evaluate(
      (save) => window.__liveTickProfile.worker.postMessage({ type: 'init', save }),
      fixtureText,
    );
    const ready = await withTimeout(readyWait, STARTUP_TIMEOUT_MS, 'WASM fixture restore');
    assert.equal(ready.tick, fixture.tick);

    const completeWait = waitFor('complete', OVERALL_TIMEOUT_MS - STARTUP_TIMEOUT_MS);
    const runStartedAt = Date.now();
    await page.evaluate(
      (count) => window.__liveTickProfile.worker.postMessage({ type: 'run', count }),
      TICK_COUNT,
    );
    const result = await withTimeout(
      completeWait,
      OVERALL_TIMEOUT_MS - STARTUP_TIMEOUT_MS,
      'per-tick WASM profile',
    );
    const elapsedMs = Date.now() - runStartedAt;
    assert.equal(result.count, TICK_COUNT, 'not all requested step(1) calls completed');
    assert.equal(result.durationsMs.length, TICK_COUNT);
    assert.equal(
      result.final.tick,
      fixture.tick + TICK_COUNT,
      'final world tick does not match timed calls',
    );
    assert.equal(
      result.canonicalRestoreEqual,
      true,
      'final save did not restore and serialize byte-identically',
    );
    assert.equal(result.checkpoints.length, Math.floor(TICK_COUNT / CHECKPOINT_EVERY));

    const sorted = [...result.durationsMs].sort((a, b) => a - b);
    const topTicks = result.durationsMs
      .map((durationMs, index) => ({ tick: result.startTick + index + 1, duration_ms: durationMs }))
      .sort((a, b) => b.duration_ms - a.duration_ms)
      .slice(0, 20);
    const report = {
      artifacts: { report_path: REPORT_PATH, final_save_path: FINAL_SAVE_PATH },
      scope:
        'Production WASM CityEngine.step(1) call timings from a dedicated headless Chromium module worker; excludes the application simulation worker scheduler, renderer, browser frames, and worker message scheduling',
      runtime: {
        browser: 'Chromium',
        version: browser.version(),
        wasm_sha256: wasmHash,
        wasm_module_url: `${BASE_URL}/wasm/city_engine.js`,
      },
      fixture: {
        path: 'tests/fixtures/validation/early-reservation-wasm-87231-save.json',
        seed: fixture.seed,
        start_tick: fixture.tick,
        bytes_utf8: Buffer.byteLength(fixtureText, 'utf8'),
      },
      measurement: {
        requested_ticks: TICK_COUNT,
        completed_ticks: result.count,
        start_tick: result.startTick,
        final_tick: result.final.tick,
        elapsed_wall_ms_including_checkpoints_and_yields: elapsedMs,
        raw_step_call_ms: {
          p50: percentile(sorted, 0.5),
          p95: percentile(sorted, 0.95),
          p99: percentile(sorted, 0.99),
          max: sorted.at(-1),
        },
        top_20_ticks: topTicks,
        checkpoint_interval_ticks: CHECKPOINT_EVERY,
        yield_interval_ticks: YIELD_EVERY,
        transition_observation_interval_ticks: result.transitionSampleEvery,
        checkpoint_and_snapshot_overhead_ms_excluded_from_raw_step_call_times:
          result.checkpointOverheadMs,
        per_tick_transition_snapshot_overhead_ms_excluded_from_raw_step_call_times:
          result.transitionObservationOverheadMs,
        call_scope:
          'Each sample brackets one synchronous JS-to-WASM CityEngine.step(1) call with performance.now(). Save serialization, JSON parsing, snapshot generation, checkpoint inspection, and yields are outside each sample.',
      },
      checkpoint_observations: result.checkpoints,
      construction_transitions: result.transitions,
      final_world: result.final,
      restore: { same_save_restores_and_serializes_byte_identically: result.canonicalRestoreEqual },
      observation_limits: `Pending-plan, exhausted-face count, and selected-face identity are read from serialized saves every ${CHECKPOINT_EVERY} ticks, so face switches between checkpoints may be missed. Construction admission/opening observations come from snapshots every ${result.transitionSampleEvery} tick(s); interval 1 records the exact observed step, while coarser intervals can miss or delay transitions. An observed open connected road does not prove which planner caused it or that any dependent lots were released. Snapshot probes are outside raw step timings but increase overall run time.`,
      claims:
        'Per-call WASM timing only. These measurements do not predict whole-app tick throughput or browser frame rate.',
    };
    mkdirSync(dirname(REPORT_PATH), { recursive: true });
    writeFileSync(REPORT_PATH, `${JSON.stringify(report, null, 2)}\n`);
    if (FINAL_SAVE_PATH) {
      mkdirSync(dirname(FINAL_SAVE_PATH), { recursive: true });
      writeFileSync(FINAL_SAVE_PATH, `${result.finalSave}\n`);
    }
    console.log(JSON.stringify(report, null, 2));
  } finally {
    if (page) {
      await page
        .evaluate(() => {
          const state = window.__liveTickProfile;
          state?.worker.terminate();
          if (state?.blobUrl) URL.revokeObjectURL(state.blobUrl);
        })
        .catch(() => {});
      await page.close().catch(() => {});
    }
    if (browser) await browser.close().catch(() => {});
    activeBrowser = undefined;
  }
}

const overallTimer = setTimeout(() => {
  void activeBrowser?.close();
}, OVERALL_TIMEOUT_MS);
try {
  await withTimeout(main(), OVERALL_TIMEOUT_MS, 'overall live-tick profile');
} finally {
  clearTimeout(overallTimer);
}
