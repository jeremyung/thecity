import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { chromium } from '@playwright/test';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const BASE_URL = process.env.LIVE_WORKER_BASE_URL ?? 'http://127.0.0.1:5173';
const FIXTURE_PATH = resolve(ROOT, 'validation/early-reservation-wasm-87231-save.json');
const REPORT_PATH = resolve(ROOT, 'validation/live-worker-profile.json');
const PROTOCOL_VERSION = 1;
const STARTUP_TIMEOUT_MS = Number(process.env.LIVE_WORKER_STARTUP_TIMEOUT_MS ?? 30_000);
const OVERALL_TIMEOUT_MS = Number(process.env.LIVE_WORKER_OVERALL_TIMEOUT_MS ?? 90_000);
const DURATION_MS = Number(process.env.LIVE_WORKER_DURATION_MS ?? 20_000);
const RUN_SPEED = Number(process.env.LIVE_WORKER_SPEED ?? 8);
const REQUIRE_LIVE_ACCESS = process.env.LIVE_WORKER_REQUIRE_LIVE_ACCESS !== '0';
const SEED = 87_231;
let activeBrowser;
let activeServer;

assert.ok(
  Number.isInteger(DURATION_MS) && DURATION_MS >= 1_000 && DURATION_MS <= 300_000,
  'LIVE_WORKER_DURATION_MS must be an integer from 1000 to 300000',
);
assert.ok([1, 2, 4, 8].includes(RUN_SPEED), 'LIVE_WORKER_SPEED must be one of 1, 2, 4, or 8');
assert.ok(
  STARTUP_TIMEOUT_MS >= 1_000 && OVERALL_TIMEOUT_MS >= DURATION_MS,
  'startup timeout must be at least 1000 ms and overall timeout must cover the run duration',
);

const fixture = readFileSync(FIXTURE_PATH, 'utf8');
const fixtureSave = JSON.parse(fixture);
assert.equal(fixtureSave.seed, SEED, 'mature fixture seed changed');

function deferred() {
  let resolvePromise;
  let rejectPromise;
  const promise = new Promise((resolve, reject) => {
    resolvePromise = resolve;
    rejectPromise = reject;
  });
  return { promise, resolve: resolvePromise, reject: rejectPromise };
}

async function withTimeout(promise, ms, label) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms} ms`)), ms);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

async function viteClientReady() {
  try {
    const response = await fetch(`${BASE_URL}/@vite/client`, {
      signal: AbortSignal.timeout(1_000),
    });
    return response.ok;
  } catch {
    return false;
  }
}

async function startOrReuseVite() {
  if (await viteClientReady()) return { child: null, owned: false };
  const child = spawn('npm', ['run', 'dev', '--', '--host', '127.0.0.1'], {
    cwd: ROOT,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: process.env,
  });
  let output = '';
  child.stdout.setEncoding('utf8').on('data', (chunk) => {
    output = `${output}${chunk}`.slice(-8_000);
  });
  child.stderr.setEncoding('utf8').on('data', (chunk) => {
    output = `${output}${chunk}`.slice(-8_000);
  });
  const ready = async () => {
    const deadline = Date.now() + STARTUP_TIMEOUT_MS;
    while (Date.now() < deadline) {
      if (child.exitCode !== null)
        throw new Error(`Vite exited before startup (code ${child.exitCode}).\n${output}`);
      if (await viteClientReady()) return;
      await delay(200);
    }
    throw new Error(`Vite startup timed out after ${STARTUP_TIMEOUT_MS} ms.\n${output}`);
  };
  try {
    await ready();
    return { child, owned: true };
  } catch (error) {
    await stopChild(child);
    throw error;
  }
}

async function stopChild(child) {
  if (!child || child.exitCode !== null) return;
  child.kill('SIGTERM');
  await Promise.race([
    new Promise((resolvePromise) => child.once('exit', resolvePromise)),
    delay(2_000).then(() => {
      if (child.exitCode === null) child.kill('SIGKILL');
    }),
  ]);
}

function summarizeWorld(world) {
  if (!world) return null;
  return {
    tick: world.tick,
    day: world.day,
    minute_of_day: world.minute_of_day,
    revision: world.revision,
    stage: world.stage,
    nodes: world.nodes?.length ?? null,
    roads: world.roads?.length ?? null,
    parcels: world.parcels?.length ?? null,
    buildings: world.buildings?.length ?? null,
    projects: world.projects?.length ?? null,
    residents: world.residents?.length ?? null,
    travelers: world.stats?.active_travelers ?? null,
    trains: world.trains?.length ?? null,
    buses: world.buses?.length ?? null,
    events: world.events?.length ?? null,
    stats: world.stats ?? null,
  };
}

async function main() {
  let server;
  let browser;
  let page;
  const overallStartedAt = Date.now();
  try {
    server = await withTimeout(startOrReuseVite(), STARTUP_TIMEOUT_MS + 2_000, 'Vite startup');
    activeServer = server;
    browser = await withTimeout(
      chromium.launch({ headless: true, timeout: STARTUP_TIMEOUT_MS }),
      STARTUP_TIMEOUT_MS,
      'Chromium startup',
    );
    activeBrowser = browser;
    const context = await browser.newContext();
    page = await context.newPage();
    page.setDefaultTimeout(STARTUP_TIMEOUT_MS);

    // A JSON document gives us the app origin without launching the viewer or
    // its own second simulation worker. The worker under test is loaded directly.
    const fixtureResponse = await withTimeout(
      page.goto(`${BASE_URL}/validation/early-reservation-wasm-87231-save.json`, {
        waitUntil: 'domcontentloaded',
      }),
      STARTUP_TIMEOUT_MS,
      'same-origin page navigation',
    );
    assert.equal(fixtureResponse?.status(), 200, 'fixture URL should be served by Vite');
    const workerUrl = `${BASE_URL}/web/src/worker/simulation.worker.ts`;
    const workerProbe = await fetch(workerUrl, { signal: AbortSignal.timeout(STARTUP_TIMEOUT_MS) });
    assert.equal(workerProbe.status, 200, `Vite worker source path failed: ${workerUrl}`);
    const workerSource = await workerProbe.text();
    assert.ok(
      workerSource.includes('WORKER_PROTOCOL_VERSION'),
      'Vite did not transform the requested worker source',
    );

    await page.evaluate(
      ({ workerUrl, protocolVersion, seed, save }) => {
        const worker = new Worker(workerUrl, { type: 'module', name: 'live-worker-profile' });
        const state = {
          worker,
          messages: [],
          worlds: [],
          metrics: [],
          errors: [],
          movements: {
            received: 0,
            recycled: 0,
            total_bytes: 0,
            max_buffer_bytes: 0,
            max_count: 0,
          },
          waiters: [],
          waitFor(type, epoch, timeoutMs, match = {}) {
            const matches = (message) =>
              message.type === type &&
              message.epoch === epoch &&
              Object.entries(match).every(([key, value]) => message[key] === value);
            const existing = this.messages.find(matches);
            if (existing) return Promise.resolve(existing);
            const priorError = this.errors.find((message) => message.epoch === epoch);
            if (priorError) return Promise.reject(new Error(priorError.message));
            return new Promise((resolveWait, rejectWait) => {
              const waiter = { type, epoch, match, resolve: resolveWait, reject: rejectWait };
              this.waiters.push(waiter);
              const timer = setTimeout(() => {
                this.waiters = this.waiters.filter((item) => item !== waiter);
                rejectWait(
                  new Error(`worker ${type} for epoch ${epoch} timed out after ${timeoutMs} ms`),
                );
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
          },
        };
        worker.onmessage = (event) => {
          const message = event.data;
          if (!message || typeof message !== 'object') return;
          if (message.type === 'movement' && message.buffer instanceof ArrayBuffer) {
            state.movements.received++;
            state.movements.total_bytes += message.buffer.byteLength;
            state.movements.max_buffer_bytes = Math.max(
              state.movements.max_buffer_bytes,
              message.buffer.byteLength,
            );
            state.movements.max_count = Math.max(state.movements.max_count, message.count ?? 0);
            worker.postMessage(
              { type: 'recycle', protocolVersion, epoch: message.epoch, buffer: message.buffer },
              [message.buffer],
            );
            state.movements.recycled++;
            return;
          }
          state.messages.push(message);
          if (message.type === 'world') state.worlds.push(message);
          if (message.type === 'metrics') state.metrics.push(message);
          if (message.type === 'error') state.errors.push(message);
          const ready = state.waiters.filter(
            (waiter) =>
              waiter.type === message.type &&
              waiter.epoch === message.epoch &&
              Object.entries(waiter.match).every(([key, value]) => message[key] === value),
          );
          state.waiters = state.waiters.filter((waiter) => !ready.includes(waiter));
          for (const waiter of ready) waiter.resolve(message);
          if (message.type === 'error') {
            for (const waiter of state.waiters) waiter.reject(new Error(message.message));
            state.waiters = [];
          }
        };
        worker.onerror = (event) => {
          for (const waiter of state.waiters)
            waiter.reject(new Error(event.message || 'worker crashed'));
          state.waiters = [];
        };
        window.__liveWorkerProfile = state;
        worker.postMessage({ type: 'init', protocolVersion, epoch: 1, seed, save });
        worker.postMessage({
          type: 'control',
          protocolVersion,
          epoch: 1,
          sequence: 1,
          paused: true,
          hidden: false,
          speed: 8,
        });
      },
      { workerUrl, protocolVersion: PROTOCOL_VERSION, seed: SEED, save: fixture },
    );

    const waitFor = (type, epoch, match = {}, timeoutMs = STARTUP_TIMEOUT_MS) =>
      page.evaluate(
        ({ type: wantedType, epoch: wantedEpoch, timeoutMs: wantedTimeout, match: wantedMatch }) =>
          window.__liveWorkerProfile.waitFor(wantedType, wantedEpoch, wantedTimeout, wantedMatch),
        { type, epoch, timeoutMs, match },
      );
    const post = (message) =>
      page.evaluate((value) => window.__liveWorkerProfile.worker.postMessage(value), message);

    const initialWorldMessage = await withTimeout(
      waitFor('world', 1),
      STARTUP_TIMEOUT_MS,
      'worker fixture restore',
    );
    const initialAck = await withTimeout(
      waitFor('controlAck', 1, { appliedSequence: 1 }),
      STARTUP_TIMEOUT_MS,
      'initial paused control acknowledgment',
    );
    assert.equal(initialAck.appliedSequence, 1);
    const initialWorld = initialWorldMessage.world;
    assert.equal(initialWorld.seed, SEED);
    assert.ok(initialWorld.tick >= fixtureSave.tick, 'restored worker tick precedes fixture tick');

    const runAckWait = waitFor('controlAck', 1, { appliedSequence: 2 });
    await post({
      type: 'control',
      protocolVersion: PROTOCOL_VERSION,
      epoch: 1,
      sequence: 2,
      paused: false,
      hidden: false,
      speed: RUN_SPEED,
    });
    const runAck = await withTimeout(
      runAckWait,
      STARTUP_TIMEOUT_MS,
      'speed control acknowledgment',
    );
    assert.equal(runAck.appliedSequence, 2);
    const runStartedAt = Date.now();

    // Save a checkpoint while speed 8 is still active. This captures live job
    // state if the mature workload has produced one without stopping the engine.
    const midpointElapsedTargetMs = Math.floor(DURATION_MS * 0.25);
    await withTimeout(
      delay(midpointElapsedTargetMs),
      midpointElapsedTargetMs + 1_000,
      'mid-run checkpoint target',
    );
    const midpointSaveWait = waitFor('save', 1, { request: 1 });
    await post({ type: 'save', protocolVersion: PROTOCOL_VERSION, epoch: 1, request: 1 });
    const midpointSaveReply = await withTimeout(
      midpointSaveWait,
      STARTUP_TIMEOUT_MS,
      'mid-run live checkpoint request',
    );
    const midpointCapturedElapsedMs = Date.now() - runStartedAt;
    const midpointCheckpoint = midpointSaveReply.data;
    assert.equal(typeof midpointCheckpoint, 'string');
    const midpointData = JSON.parse(midpointCheckpoint);
    assert.ok(
      midpointData.tick > initialWorld.tick,
      'mid-run checkpoint must show simulation progress',
    );
    const liveAccessCheckpoint = midpointData.live_access_checkpoint ?? null;
    const liveAccessBytes =
      liveAccessCheckpoint === null
        ? 0
        : Buffer.byteLength(JSON.stringify(liveAccessCheckpoint), 'utf8');
    if (REQUIRE_LIVE_ACCESS) {
      assert.ok(
        liveAccessCheckpoint !== null,
        'No active live_access_checkpoint was present in the ~25% mid-run save. Extend the run or investigate the mature planning workload; restoring an idle checkpoint does not verify live-job persistence.',
      );
    }

    // Account for the request/serialization time so the worker remains unpaused
    // for approximately the configured total run duration.
    const remainingRunMs = Math.max(0, DURATION_MS - (Date.now() - runStartedAt));
    if (remainingRunMs > 0)
      await withTimeout(
        delay(remainingRunMs),
        remainingRunMs + 1_000,
        'remaining simulation duration',
      );
    const runElapsedMs = Date.now() - runStartedAt;

    const pauseAckWait = waitFor('controlAck', 1, { appliedSequence: 3 });
    await post({
      type: 'control',
      protocolVersion: PROTOCOL_VERSION,
      epoch: 1,
      sequence: 3,
      paused: true,
      speed: RUN_SPEED,
    });
    const pauseAck = await withTimeout(pauseAckWait, STARTUP_TIMEOUT_MS, 'pause acknowledgment');
    assert.equal(pauseAck.appliedSequence, 3);

    const saveReplyWait = waitFor('save', 1, { request: 2 });
    await post({ type: 'save', protocolVersion: PROTOCOL_VERSION, epoch: 1, request: 2 });
    const saveReply = await withTimeout(saveReplyWait, STARTUP_TIMEOUT_MS, 'checkpoint request');
    const checkpoint = saveReply.data;
    assert.equal(typeof checkpoint, 'string');
    const checkpointBytes = Buffer.byteLength(checkpoint, 'utf8');
    const checkpointData = JSON.parse(checkpoint);
    assert.equal(checkpointData.seed, SEED);
    assert.ok(checkpointData.tick > initialWorld.tick, 'worker must advance simulation ticks');

    // Reinitialize the same module worker under a fresh epoch and require the
    // saved checkpoint to survive the worker's ordinary restore path exactly.
    const restoreWorldWait = waitFor('world', 2);
    await page.evaluate(
      ({ protocolVersion, seed, save }) => {
        const worker = window.__liveWorkerProfile.worker;
        worker.postMessage({ type: 'init', protocolVersion, epoch: 2, seed, save });
        worker.postMessage({
          type: 'control',
          protocolVersion,
          epoch: 2,
          sequence: 1,
          paused: true,
          hidden: false,
          speed: 8,
        });
      },
      { protocolVersion: PROTOCOL_VERSION, seed: SEED, save: checkpoint },
    );
    const restoredWorldMessage = await withTimeout(
      restoreWorldWait,
      STARTUP_TIMEOUT_MS,
      'new-epoch checkpoint restore',
    );
    assert.equal(restoredWorldMessage.world.seed, SEED);
    assert.equal(restoredWorldMessage.world.tick, checkpointData.tick);
    const restoredSaveWait = waitFor('save', 2, { request: 3 });
    await post({ type: 'save', protocolVersion: PROTOCOL_VERSION, epoch: 2, request: 3 });
    const restoredSaveReply = await withTimeout(
      restoredSaveWait,
      STARTUP_TIMEOUT_MS,
      'canonical restore checkpoint request',
    );
    assert.equal(restoredSaveReply.data, checkpoint, 'restore/save output was not byte-identical');

    let liveAccessRestore = {
      attempted: false,
      verified: false,
      epoch: 3,
      live_access_bytes: liveAccessBytes,
    };
    if (liveAccessCheckpoint !== null) {
      const liveRestoreWorldWait = waitFor('world', 3);
      await page.evaluate(
        ({ protocolVersion, seed, save }) => {
          const worker = window.__liveWorkerProfile.worker;
          worker.postMessage({ type: 'init', protocolVersion, epoch: 3, seed, save });
          worker.postMessage({
            type: 'control',
            protocolVersion,
            epoch: 3,
            sequence: 1,
            paused: true,
            hidden: false,
            speed: 8,
          });
        },
        { protocolVersion: PROTOCOL_VERSION, seed: SEED, save: midpointCheckpoint },
      );
      const liveRestoredWorld = await withTimeout(
        liveRestoreWorldWait,
        STARTUP_TIMEOUT_MS,
        'live-job new-epoch restore',
      );
      assert.equal(liveRestoredWorld.world.seed, SEED);
      assert.equal(liveRestoredWorld.world.tick, midpointData.tick);
      const liveRestoredSaveWait = waitFor('save', 3, { request: 4 });
      await post({ type: 'save', protocolVersion: PROTOCOL_VERSION, epoch: 3, request: 4 });
      const liveRestoredSave = await withTimeout(
        liveRestoredSaveWait,
        STARTUP_TIMEOUT_MS,
        'live-job canonical restore save',
      );
      assert.equal(
        liveRestoredSave.data,
        midpointCheckpoint,
        'mid-run live_access checkpoint did not restore/save byte-identically in epoch 3',
      );
      liveAccessRestore = {
        attempted: true,
        verified: true,
        epoch: 3,
        tick: midpointData.tick,
        live_access_bytes: liveAccessBytes,
        save_is_byte_identical: true,
      };
    }

    const telemetry = await page.evaluate(() => {
      const state = window.__liveWorkerProfile;
      const worlds = state.worlds;
      const metrics = state.metrics;
      return {
        metrics: metrics.filter((message) => message.epoch === 1).map((message) => message.metrics),
        initial_world: worlds.find((message) => message.epoch === 1)?.world ?? null,
        final_world: [...worlds].reverse().find((message) => message.epoch === 1)?.world ?? null,
        restored_world: worlds.find((message) => message.epoch === 2)?.world ?? null,
        live_restored_world: worlds.find((message) => message.epoch === 3)?.world ?? null,
        movements: state.movements,
        errors: state.errors,
      };
    });
    assert.equal(
      telemetry.errors.length,
      0,
      `worker reported errors: ${JSON.stringify(telemetry.errors)}`,
    );
    assert.ok(
      telemetry.metrics.length >= Math.max(1, Math.floor(DURATION_MS / 1_000) - 2),
      `too few once-per-second worker metric samples: ${telemetry.metrics.length}`,
    );
    assert.ok(telemetry.movements.received > 0, 'worker did not publish movement buffers');
    assert.equal(
      telemetry.movements.recycled,
      telemetry.movements.received,
      'not every movement buffer was recycled',
    );

    const report = {
      scope:
        'Chromium module-worker runtime and engine telemetry only; no renderer, browser frame, or GPU performance claims',
      endpoint: BASE_URL,
      worker_module_url: `${BASE_URL}/web/src/worker/simulation.worker.ts`,
      worker_path_probe: {
        status: workerProbe.status,
        content_type: workerProbe.headers.get('content-type'),
      },
      browser: { name: 'Chromium', version: browser.version() },
      wasm_sha256: createHash('sha256')
        .update(readFileSync(resolve(ROOT, 'public/wasm/city_engine_bg.wasm')))
        .digest('hex'),
      fixture: {
        path: 'validation/early-reservation-wasm-87231-save.json',
        seed: SEED,
        tick_before: fixtureSave.tick,
        bytes: Buffer.byteLength(fixture, 'utf8'),
      },
      configuration: {
        duration_ms: DURATION_MS,
        speed: RUN_SPEED,
        startup_timeout_ms: STARTUP_TIMEOUT_MS,
        overall_timeout_ms: OVERALL_TIMEOUT_MS,
        require_live_access_checkpoint: REQUIRE_LIVE_ACCESS,
      },
      run: {
        elapsed_ms: runElapsedMs,
        start_tick: initialWorld.tick,
        final_tick: checkpointData.tick,
        ticks_advanced: checkpointData.tick - initialWorld.tick,
      },
      metrics: {
        sample_interval: 'worker publishes approximately once per second',
        tick_ms_scope: 'per-batch mean supplied by the worker, not per-tick p95',
        samples: telemetry.metrics,
      },
      world_counts: {
        before: summarizeWorld(telemetry.initial_world),
        after_run: summarizeWorld(telemetry.final_world),
        restored_new_epoch: summarizeWorld(telemetry.restored_world),
      },
      movement_buffers: telemetry.movements,
      checkpoint: {
        present: checkpointBytes > 0,
        bytes_utf8: checkpointBytes,
        seed: checkpointData.seed,
        tick: checkpointData.tick,
        epoch: 1,
        paused_before_request: true,
        live_access_present: checkpointData.live_access_checkpoint != null,
        live_access_bytes:
          checkpointData.live_access_checkpoint == null
            ? 0
            : Buffer.byteLength(JSON.stringify(checkpointData.live_access_checkpoint), 'utf8'),
      },
      midrun_checkpoint: {
        captured_while_unpaused: true,
        target_elapsed_ms: midpointElapsedTargetMs,
        captured_elapsed_ms: midpointCapturedElapsedMs,
        tick: midpointData.tick,
        bytes_utf8: Buffer.byteLength(midpointCheckpoint, 'utf8'),
        live_access_present: liveAccessCheckpoint !== null,
        live_access_bytes: liveAccessBytes,
      },
      restore: {
        new_epoch: 2,
        world_published: true,
        tick_matches_checkpoint: true,
        save_is_byte_identical: true,
        live_access_checkpoint: liveAccessRestore,
      },
      errors: telemetry.errors,
      timeouts: {
        startup_ms: STARTUP_TIMEOUT_MS,
        overall_ms: OVERALL_TIMEOUT_MS,
        cleanup_runs_in_finally: true,
      },
    };
    writeFileSync(REPORT_PATH, `${JSON.stringify(report, null, 2)}\n`);
    console.log(JSON.stringify(report, null, 2));
  } finally {
    if (page) await page.close().catch(() => {});
    if (browser) await browser.close().catch(() => {});
    if (server?.owned) await stopChild(server.child);
    activeBrowser = undefined;
    activeServer = undefined;
    const totalMs = Date.now() - overallStartedAt;
    if (totalMs > OVERALL_TIMEOUT_MS)
      throw new Error(
        `Overall live-worker profile timeout exceeded (${totalMs} > ${OVERALL_TIMEOUT_MS} ms)`,
      );
  }
}

const overallTimer = setTimeout(() => {
  void activeBrowser?.close();
  if (activeServer?.owned && activeServer.child.exitCode === null)
    activeServer.child.kill('SIGTERM');
}, OVERALL_TIMEOUT_MS);
try {
  await withTimeout(main(), OVERALL_TIMEOUT_MS, 'overall live-worker profile');
} finally {
  clearTimeout(overallTimer);
}
