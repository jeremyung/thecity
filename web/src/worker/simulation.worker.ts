import {
  WORKER_PROTOCOL_VERSION,
  type Metrics,
  type WorkerInput,
  type WorkerOutput,
  type World,
} from '../types';
import { ControlSequenceGate } from './protocol';
import { ensurePresentationWorld } from './presentation-order';

type Engine = {
  free?(): void;
  step(ticks: number): void;
  snapshot(): string;
  presentation_revision(): string;
  movement_packet(): Uint32Array;
  train_doors(): Float32Array;
  inspect(id: number): string;
  save(): string;
  set_development_rate(rate: number): void;
};

type WasmModule = {
  default(input?: { module_or_path: string }): Promise<{ memory?: WebAssembly.Memory }>;
  CityEngine: {
    new (seed: number): Engine;
    restore(json: string): Engine;
  };
  memory?: WebAssembly.Memory;
};

type WorkerScope = {
  onmessage: ((event: MessageEvent<WorkerInput>) => void) | null;
  postMessage(message: WorkerOutput, transfer?: Transferable[]): void;
  setTimeout(handler: () => void, timeout?: number): number;
  clearTimeout(handle: number): void;
  performance: Performance;
};

const TICK_RATE = 20;
const TICK_MS = 1000 / TICK_RATE;
const MAX_STEPS_PER_TURN = 8;
const WORLD_PUBLISH_MS = 500;
const MAX_BACKLOG_TICKS = 160;

const scope = globalThis as unknown as WorkerScope;

/** A tiny transferable pool that also enforces the two-snapshot backpressure cap. */
export class MovementBufferPool {
  private inFlight = 0;
  private buffers: ArrayBuffer[] = [];

  acquire(requiredBytes: number): ArrayBuffer | null {
    if (this.inFlight >= 2) return null;
    const index = this.buffers.findIndex((buffer) => buffer.byteLength >= requiredBytes);
    const buffer = index >= 0 ? this.buffers.splice(index, 1)[0] : new ArrayBuffer(requiredBytes);
    this.inFlight++;
    return buffer;
  }

  release(buffer: ArrayBuffer): void {
    this.inFlight = Math.max(0, this.inFlight - 1);
    if (buffer.byteLength > 0 && this.buffers.length < 2) this.buffers.push(buffer);
  }

  reset(): void {
    this.inFlight = 0;
    this.buffers = [];
  }
}

/** Carries the newest development-rate command across asynchronous WASM startup. */
export class PendingDevelopmentControl {
  private epoch = 0;
  private pendingRate: number | null = null;

  reset(epoch: number): void {
    this.epoch = epoch;
    this.pendingRate = null;
  }

  queue(epoch: number, rate: number): boolean {
    if (epoch !== this.epoch) return false;
    this.pendingRate = normalizeDevelopmentRate(rate);
    return true;
  }

  apply(epoch: number, engine: Pick<Engine, 'set_development_rate'>): void {
    if (epoch !== this.epoch || this.pendingRate === null) return;
    engine.set_development_rate(this.pendingRate);
    this.pendingRate = null;
  }

  peek(epoch: number): number | null {
    return epoch === this.epoch ? this.pendingRate : null;
  }

  clear(epoch: number): void {
    if (epoch === this.epoch) this.pendingRate = null;
  }
}

/** Keeps the committed engine alive until a candidate world has validated. */
export class EngineSlot<T extends { free?(): void }> {
  current: T | null = null;
  private loadingEpoch: number | null = null;

  begin(epoch: number): T | null {
    this.loadingEpoch = epoch;
    return this.current;
  }

  isLoading(epoch: number): boolean {
    return this.loadingEpoch === epoch;
  }

  commit(epoch: number, candidate: T): boolean {
    if (this.loadingEpoch !== epoch) {
      this.release(candidate);
      return false;
    }
    const previous = this.current;
    this.current = candidate;
    this.loadingEpoch = null;
    if (previous !== candidate) this.release(previous);
    return true;
  }

  rollback(epoch: number): T | null {
    if (this.loadingEpoch !== epoch) return null;
    this.loadingEpoch = null;
    return this.current;
  }

  discard(candidate: T): void {
    if (candidate !== this.current) this.release(candidate);
  }

  private release(engine: T | null): void {
    try {
      engine?.free?.();
    } catch {
      /* release cannot interrupt a world swap */
    }
  }
}

function normalizeDevelopmentRate(rate: number): number {
  return Number.isFinite(rate) ? Math.max(0, Math.min(60, Math.round(rate))) : 1;
}

/** Bound accumulated work so a throttled tab cannot spend minutes catching up. */
export function capBacklog(
  current: number,
  incoming: number,
  limit = MAX_BACKLOG_TICKS,
): { backlog: number; dropped: number } {
  const total = Math.max(0, current) + Math.max(0, incoming);
  const backlog = Math.min(Math.max(0, limit), total);
  return { backlog, dropped: total - backlog };
}

let epoch = 0;
const controlSequence = new ControlSequenceGate();
const engineSlot = new EngineSlot<Engine>();
let engine: Engine | null = null;
type WasmRuntime = Pick<WasmModule, 'CityEngine'> & { memory?: WebAssembly.Memory };
let wasm: WasmRuntime | null = null;
let wasmLoading: Promise<WasmRuntime> | null = null;
let timer: number | null = null;
let paused = false;
let hidden = false;
let speed = 1;
let accumulator = 0;
let lastFrameAt = 0;
let lastWorldAt = 0;
let lastRevision = -1;
let lastPresentationRevision: string | undefined;
let lastMetricsAt = 0;
let metricsSampleAt = 0;
let metricsSampleTick = 0;
let droppedWallMs = 0;
const movementBuffers = new MovementBufferPool();
const pendingDevelopment = new PendingDevelopmentControl();
let tickCost = 0;
let previousTick = 0;

function post(message: WorkerOutput, transfer?: Transferable[]): void {
  scope.postMessage(message, transfer ?? []);
}

function sendError(message: string, code?: Extract<WorkerOutput, { type: 'error' }>['code']): void {
  post({
    type: 'error',
    protocolVersion: WORKER_PROTOCOL_VERSION,
    epoch,
    message,
    ...(code ? { code } : {}),
  });
}

function currentTick(): number {
  if (!engine) return 0;
  // Tick is carried by snapshots. Cache it whenever a reliable world is read.
  return previousTick;
}

function readWorld(): World {
  if (!engine) throw new Error('Simulation is not initialized');
  return parseWorldSnapshot(engine.snapshot());
}

function parseWorldSnapshot(json: string): World {
  const value: unknown = JSON.parse(json);
  if (!value || typeof value !== 'object')
    throw new Error('Engine returned an invalid world snapshot');
  const world = value as Partial<World>;
  if (
    world.schema_version !== 1 ||
    !Number.isFinite(world.tick) ||
    !Number.isFinite(world.revision) ||
    !Number.isFinite(world.development_rate) ||
    typeof world.presentation_revision !== 'string' ||
    !world.presentation_revision
  ) {
    throw new Error('Engine returned an invalid world snapshot');
  }
  return world as World;
}

function publishWorld(force = false): World | undefined {
  if (!engine) return;
  const now = scope.performance.now();
  if (!force && now - lastWorldAt < WORLD_PUBLISH_MS) return;
  const world = readWorld();
  if (!force && world.revision === lastRevision) return;
  lastWorldAt = now;
  lastRevision = world.revision;
  previousTick = world.tick;
  post({ type: 'world', protocolVersion: WORKER_PROTOCOL_VERSION, epoch, world });
  lastPresentationRevision = world.presentation_revision;
  return world;
}

function minuteAndDay(tick: number): { minute: number; day: number } {
  const ticksPerDay = 24_000;
  const elapsed = ((tick % ticksPerDay) * 1440) / ticksPerDay;
  return { minute: (360 + elapsed) % 1440, day: Math.floor(tick / ticksPerDay) };
}

function publishMovement(): void {
  if (!engine) return;
  lastPresentationRevision = ensurePresentationWorld(
    engine.presentation_revision(),
    lastPresentationRevision,
    () => publishWorld(true)?.presentation_revision,
  );
  const packet = engine.movement_packet();
  const neededBytes = packet.length * Uint32Array.BYTES_PER_ELEMENT;
  const buffer = movementBuffers.acquire(neededBytes);
  if (!buffer) return;
  if (neededBytes > 0) new Uint32Array(buffer, 0, packet.length).set(packet);
  const tick = currentTick();
  const clock = minuteAndDay(tick);
  post(
    {
      type: 'movement',
      protocolVersion: WORKER_PROTOCOL_VERSION,
      epoch,
      tick,
      wordCount: packet.length,
      trainDoors: engine.train_doors(),
      buffer,
      presentationRevision: lastPresentationRevision,
      minute: clock.minute,
      day: clock.day,
    },
    [buffer],
  );
}

function publishMetrics(now: number, backlog: number): void {
  if (!wasm) return;
  if (now - lastMetricsAt < 1000) return;
  const sampleMs = Math.max(1, now - metricsSampleAt);
  const tick = currentTick();
  const achievableSpeed = Math.max(0, ((tick - metricsSampleTick) * TICK_MS) / sampleMs);
  lastMetricsAt = now;
  metricsSampleAt = now;
  metricsSampleTick = tick;
  post({
    type: 'metrics',
    protocolVersion: WORKER_PROTOCOL_VERSION,
    epoch,
    metrics: {
      tickMs: tickCost,
      memoryBytes: wasm.memory?.buffer.byteLength ?? 0,
      backlog: Math.max(0, Math.floor(backlog)),
      tick,
      achievableSpeed,
      droppedWallMs,
    },
  });
}

function runFrame(): void {
  if (timer === null) return;
  timer = scope.setTimeout(runFrame, TICK_MS);
  const now = scope.performance.now();
  const elapsed = lastFrameAt === 0 ? TICK_MS : Math.max(0, now - lastFrameAt);
  lastFrameAt = now;

  if (!engine || paused || hidden) {
    accumulator = 0;
    return;
  }

  const bounded = capBacklog(accumulator, (elapsed / TICK_MS) * speed);
  accumulator = bounded.backlog;
  droppedWallMs += (bounded.dropped * TICK_MS) / Math.max(1, speed);
  const steps = Math.min(MAX_STEPS_PER_TURN, Math.floor(accumulator));
  try {
    if (steps > 0) {
      const startedAt = scope.performance.now();
      engine.step(steps);
      tickCost = (scope.performance.now() - startedAt) / steps;
      accumulator -= steps;
      previousTick += steps;
      publishMovement();
    }

    publishWorld(false);
    publishMetrics(now, accumulator);
  } catch (error) {
    if (timer !== null) scope.clearTimeout(timer);
    timer = null;
    accumulator = 0;
    sendError(error instanceof Error ? error.message : String(error));
  }
}

function startScheduler(): void {
  if (timer !== null) scope.clearTimeout(timer);
  timer = 0;
  lastFrameAt = 0;
  accumulator = 0;
  runFrame();
}

async function initialize(message: Extract<WorkerInput, { type: 'init' }>): Promise<void> {
  if (message.epoch < epoch) return;
  if (message.epoch === epoch && (engine || engineSlot.isLoading(message.epoch))) return;
  epoch = message.epoch;
  controlSequence.reset();
  pendingDevelopment.reset(epoch);
  const previousEngine = engineSlot.begin(epoch);
  movementBuffers.reset();
  lastRevision = -1;
  lastPresentationRevision = undefined;
  lastWorldAt = 0;
  previousTick = 0;
  droppedWallMs = 0;
  metricsSampleTick = 0;
  metricsSampleAt = scope.performance.now();
  lastMetricsAt = 0;
  accumulator = 0;
  if (timer !== null) scope.clearTimeout(timer);
  timer = null;

  let candidate: Engine | null = null;
  let committed = false;
  let requestedRate: number | null = null;
  try {
    if (!wasm) {
      wasmLoading ??= (async () => {
        const wasmUrl = new URL('/wasm/city_engine.js', self.location.origin).href;
        const module = (await import(/* @vite-ignore */ wasmUrl)) as WasmModule;
        const exports = await module.default({ module_or_path: '/wasm/city_engine_bg.wasm' });
        return { CityEngine: module.CityEngine, memory: exports.memory };
      })();
      wasm = await wasmLoading;
    }
    if (message.epoch !== epoch) return;
    candidate = message.save
      ? wasm.CityEngine.restore(message.save)
      : new wasm.CityEngine(message.seed >>> 0);
    if (message.epoch !== epoch) {
      engineSlot.discard(candidate);
      return;
    }
    requestedRate = pendingDevelopment.peek(epoch);
    pendingDevelopment.apply(epoch, candidate);
    const world = parseWorldSnapshot(candidate.snapshot());
    if (message.epoch !== epoch || !engineSlot.commit(message.epoch, candidate)) {
      if (message.epoch !== epoch) engineSlot.discard(candidate);
      return;
    }
    committed = true;
    engine = engineSlot.current;
    previousTick = world.tick;
    metricsSampleTick = world.tick;
    metricsSampleAt = scope.performance.now();
    lastRevision = world.revision;
    lastWorldAt = scope.performance.now();
    post({ type: 'world', protocolVersion: WORKER_PROTOCOL_VERSION, epoch, world });
    lastPresentationRevision = world.presentation_revision;
    // A paused restore still needs its actors at the saved positions.
    publishMovement();
    startScheduler();
  } catch (error) {
    if (message.epoch !== epoch) return;
    if (!wasm) wasmLoading = null;
    const reason = error instanceof Error ? error.message : String(error);
    if (committed) {
      if (timer !== null) scope.clearTimeout(timer);
      timer = null;
      sendError(reason);
      return;
    }

    if (candidate) engineSlot.discard(candidate);
    engine = engineSlot.rollback(message.epoch);
    if (engine) {
      try {
        if (requestedRate !== null) engine.set_development_rate(requestedRate);
        pendingDevelopment.clear(message.epoch);
        const fallbackWorld = readWorld();
        previousTick = fallbackWorld.tick;
        metricsSampleTick = fallbackWorld.tick;
        metricsSampleAt = scope.performance.now();
        lastRevision = fallbackWorld.revision;
        lastWorldAt = scope.performance.now();
        post({
          type: 'world',
          protocolVersion: WORKER_PROTOCOL_VERSION,
          epoch,
          world: fallbackWorld,
        });
        lastPresentationRevision = fallbackWorld.presentation_revision;
        publishMovement();
        sendError(`Could not load that world. The current city is still running. ${reason}`);
        startScheduler();
        return;
      } catch (fallbackError) {
        const fallbackReason =
          fallbackError instanceof Error ? fallbackError.message : String(fallbackError);
        pendingDevelopment.clear(message.epoch);
        sendError(
          `Could not load that world (${reason}); the current city could not be resumed (${fallbackReason}).`,
        );
        return;
      }
    }
    pendingDevelopment.clear(message.epoch);
    sendError(reason);
  }
}

const handleMessage = (event: MessageEvent<WorkerInput>) => {
  const message = event.data;
  if (!message || typeof message !== 'object') {
    sendError(
      'Invalid worker message. Reload the app so its interface and worker use matching files.',
    );
    return;
  }
  if (message.protocolVersion !== WORKER_PROTOCOL_VERSION) {
    const messageEpoch = Number.isSafeInteger(message.epoch) ? message.epoch : epoch;
    post({
      type: 'error',
      protocolVersion: WORKER_PROTOCOL_VERSION,
      epoch: messageEpoch,
      code: 'protocol-version',
      message: `Worker protocol mismatch: this worker requires version ${WORKER_PROTOCOL_VERSION}, but received ${String(message.protocolVersion)}. Reload the app so its worker and interface use matching files.`,
    });
    return;
  }
  if (message.type === 'init') {
    void initialize(message);
    return;
  }
  if (message.epoch !== epoch) return;

  switch (message.type) {
    case 'control': {
      if (!Number.isSafeInteger(message.sequence) || message.sequence <= 0) {
        sendError('Invalid simulation control sequence; reload the app to restore communication.');
        break;
      }
      const sequence = controlSequence.accept(message.sequence);
      if (!sequence.accepted) {
        if (message.sequence <= sequence.appliedSequence) {
          post({
            type: 'controlAck',
            protocolVersion: WORKER_PROTOCOL_VERSION,
            epoch,
            appliedSequence: sequence.appliedSequence,
            tick: currentTick(),
          });
        } else {
          sendError(
            'Invalid simulation control sequence; reload the app to restore communication.',
          );
        }
        break;
      }
      if (message.paused !== undefined) paused = message.paused;
      if (message.hidden !== undefined) hidden = message.hidden;
      if (message.speed !== undefined)
        speed = [1, 2, 4, 8].includes(message.speed) ? message.speed : 1;
      if (message.development !== undefined) {
        const rate = normalizeDevelopmentRate(message.development);
        if (engine && !engineSlot.isLoading(epoch)) engine.set_development_rate(rate);
        else pendingDevelopment.queue(epoch, rate);
      }
      accumulator = 0;
      lastFrameAt = scope.performance.now();
      // The acknowledgement confirms the worker accepted the absolute desired
      // control state. Development pace is staged here during async world loads
      // and is applied before that candidate's first snapshot is published.
      post({
        type: 'controlAck',
        protocolVersion: WORKER_PROTOCOL_VERSION,
        epoch,
        appliedSequence: sequence.appliedSequence,
        tick: currentTick(),
      });
      break;
    }
    case 'recycle': {
      movementBuffers.release(message.buffer);
      break;
    }
    case 'save': {
      try {
        if (!engine) throw new Error('Simulation is not initialized');
        post({
          type: 'save',
          protocolVersion: WORKER_PROTOCOL_VERSION,
          epoch,
          request: message.request,
          data: engine.save(),
        });
      } catch (error) {
        sendError(error instanceof Error ? error.message : String(error));
      }
      break;
    }
    case 'inspect': {
      try {
        if (!engine) throw new Error('Simulation is not initialized');
        post({
          type: 'inspect',
          protocolVersion: WORKER_PROTOCOL_VERSION,
          epoch,
          request: message.request,
          data: JSON.parse(engine.inspect(message.id)),
        });
      } catch (error) {
        sendError(error instanceof Error ? error.message : String(error));
      }
      break;
    }
    case 'resync': {
      try {
        publishWorld(true);
      } catch (error) {
        sendError(error instanceof Error ? error.message : String(error));
      }
      break;
    }
  }
};

if (
  typeof self !== 'undefined' &&
  typeof (self as unknown as WorkerScope).postMessage === 'function'
) {
  scope.onmessage = handleMessage;
}
