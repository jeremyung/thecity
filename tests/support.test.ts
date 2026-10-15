import { afterEach, describe, expect, it, vi } from "vitest";
import { AudioDirector } from "../web/src/audio/director";
import { saveWorld, loadWorld, parseViewerPreferences, saveViewerPreferences, loadViewerPreferences, validateWorldSave } from "../web/src/storage/saves";
import { SimulationClient } from "../web/src/worker/client";
import type { MovementSupport } from "../web/src/movement-support";
import { capBacklog, EngineSlot, MovementBufferPool, PendingDevelopmentControl } from "../web/src/worker/simulation.worker";
import { ControlSequenceGate } from "../web/src/worker/protocol";
import { WORKER_PROTOCOL_VERSION, type WorkerOutput, type World } from "../web/src/types";

class FakeWorker {
  static instances: FakeWorker[] = [];
  onmessage: ((event: MessageEvent<WorkerOutput>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  onmessageerror: (() => void) | null = null;
  sent: Array<{ message: unknown; transfer?: Transferable[] }> = [];
  terminated = false;

  constructor() { FakeWorker.instances.push(this); }
  postMessage(message: unknown, transfer?: Transferable[]): void { this.sent.push({ message, transfer }); }
  terminate(): void { this.terminated = true; }
  emit(message: WorkerOutput): void { this.onmessage?.({ data: message } as MessageEvent<WorkerOutput>); }
}

function sampleWorld(revision: number): World {
  return {
    schema_version: 1,
    engine_version: "test",
    seed: 7,
    tick: revision * 40,
    day: 0,
    minute_of_day: 360,
    development_rate: 4,
    revision,
    presentation_revision: `g${revision}:0:0:0`,
    stage: "Founding",
    bounds: { min_x: -400, max_x: 400, min_z: -400, max_z: 400 },
    terrain: { river: [], protected: [], gateways: [] },
    weather: { kind: "clear", intensity: 0 },
    nodes: [], roads: [], parcels: [], buildings: [], projects: [], trains: [], events: [],
    stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
  };
}

function sampleEngineSave(seed: number, tick: number): string {
  return JSON.stringify({
    schema_version: 1,
    engine_version: "0.1.0",
    seed,
    tick,
    rng_growth: 1,
    rng_schedule: 2,
    rng_weather: 3,
    development_rate: 4,
    development_accum: 0,
    revision: tick,
    next_id: 1,
    next_event_id: 1,
    nodes: [], roads: [], parcels: [], buildings: [], projects: [], travelers: [], trains: [], events: [],
    population: 0,
    demand: 0,
    growth_cursor: 0,
    station_online: false,
    weather_day: 0,
    weather_kind: "clear",
    weather_intensity: 0,
    last_departure_day: 0,
    last_renewal_day: 0,
  });
}

class MemoryRequest<T> {
  result!: T;
  error: DOMException | null = null;
  onsuccess: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
}

type MemoryRecord = { slot: number; data: string; seed: number; savedAt: number };
const memoryDatabases = new Map<string, Map<number, MemoryRecord>>();

class MemoryTransaction {
  oncomplete: ((event: Event) => void) | null = null;
  onabort: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  error: DOMException | null = null;
  private scheduled = false;

  constructor(private records: Map<number, MemoryRecord>) {}
  objectStore(): {
    getAll: () => IDBRequest<MemoryRecord[]>;
    put: (record: MemoryRecord) => IDBRequest<IDBValidKey>;
  } {
    return {
      getAll: () => {
        const request = new MemoryRequest<MemoryRecord[]>();
        queueMicrotask(() => {
          request.result = [...this.records.values()];
          request.onsuccess?.(new Event("success"));
          this.finishSoon();
        });
        return request as unknown as IDBRequest<MemoryRecord[]>;
      },
      put: (record) => {
        const request = new MemoryRequest<IDBValidKey>();
        queueMicrotask(() => {
          this.records.set(record.slot, record);
          request.result = record.slot;
          request.onsuccess?.(new Event("success"));
          this.finishSoon();
        });
        return request as unknown as IDBRequest<IDBValidKey>;
      },
    };
  }

  private finishSoon(): void {
    if (this.scheduled) return;
    this.scheduled = true;
    setTimeout(() => this.oncomplete?.(new Event("complete")), 0);
  }
}

function makeMemoryIndexedDb(): IDBFactory {
  return {
    open(name: string) {
      let stores = memoryDatabases.get(name);
      const request: Record<string, unknown> = { result: null, error: null, onupgradeneeded: null, onsuccess: null, onerror: null, onblocked: null };
      queueMicrotask(() => {
        const isNew = !stores;
        stores ??= new Map<number, MemoryRecord>();
        memoryDatabases.set(name, stores);
        const database = {
          objectStoreNames: { contains: () => true },
          createObjectStore: () => ({}),
          transaction: () => new MemoryTransaction(stores!) as unknown as IDBTransaction,
          close: () => undefined,
        };
        request.result = database;
        if (isNew) (request.onupgradeneeded as ((event: Event) => void) | null)?.(new Event("upgradeneeded"));
        (request.onsuccess as ((event: Event) => void) | null)?.(new Event("success"));
      });
      return request as unknown as IDBOpenDBRequest;
    },
  } as unknown as IDBFactory;
}

class FakeParam {
  value = 0;
  history: number[] = [];
  setTargetAtTime(value: number): void { this.value = value; this.history.push(value); }
  setValueAtTime(value: number): void { this.value = value; this.history.push(value); }
  linearRampToValueAtTime(value: number): void { this.value = value; this.history.push(value); }
  exponentialRampToValueAtTime(value: number): void { this.value = value; this.history.push(value); }
  cancelScheduledValues(): void {}
}

class FakeNode {
  connections: FakeNode[] = [];
  connect<T extends FakeNode>(target: T): T { this.connections.push(target); return target; }
}

class FakeGainNode extends FakeNode { gain = new FakeParam(); }
class FakeFilterNode extends FakeNode {
  frequency = new FakeParam();
  Q = new FakeParam();
  type = "lowpass";
}
class FakePannerNode extends FakeNode { pan = new FakeParam(); }
class FakeDynamicsCompressorNode extends FakeNode {
  threshold = new FakeParam();
  knee = new FakeParam();
  ratio = new FakeParam();
  attack = new FakeParam();
  release = new FakeParam();
}
class FakeOscillatorNode extends FakeNode {
  frequency = new FakeParam();
  type = "sine";
  started = false;
  stopped = false;
  start(): void { this.started = true; }
  stop(): void { this.stopped = true; }
}
class FakeBufferSourceNode extends FakeNode {
  buffer: AudioBuffer | null = null;
  loop = false;
  started = false;
  stopped = false;
  start(): void { this.started = true; }
  stop(): void { this.stopped = true; }
}

class FakeAudioContext {
  static instances: FakeAudioContext[] = [];
  static rejectResume = false;
  state: AudioContextState = "suspended";
  currentTime = 0;
  sampleRate = 8_000;
  destination = new FakeNode() as unknown as AudioDestinationNode;
  oscillators: FakeOscillatorNode[] = [];
  buffers: FakeBufferSourceNode[] = [];
  gains: FakeGainNode[] = [];
  panners: FakePannerNode[] = [];
  compressors: FakeDynamicsCompressorNode[] = [];
  filters: FakeFilterNode[] = [];
  createdBuffers = 0;
  closeCount = 0;

  constructor() { FakeAudioContext.instances.push(this); }
  createGain(): GainNode { const node = new FakeGainNode(); this.gains.push(node); return node as unknown as GainNode; }
  createBiquadFilter(): BiquadFilterNode { const node = new FakeFilterNode(); this.filters.push(node); return node as unknown as BiquadFilterNode; }
  createStereoPanner(): StereoPannerNode { const node = new FakePannerNode(); this.panners.push(node); return node as unknown as StereoPannerNode; }
  createDynamicsCompressor(): DynamicsCompressorNode { const node = new FakeDynamicsCompressorNode(); this.compressors.push(node); return node as unknown as DynamicsCompressorNode; }
  createOscillator(): OscillatorNode {
    const oscillator = new FakeOscillatorNode();
    this.oscillators.push(oscillator);
    return oscillator as unknown as OscillatorNode;
  }
  createBufferSource(): AudioBufferSourceNode {
    const source = new FakeBufferSourceNode();
    this.buffers.push(source);
    return source as unknown as AudioBufferSourceNode;
  }
  createBuffer(channels: number, length: number): AudioBuffer {
    this.createdBuffers++;
    const samples = Array.from({ length: channels }, () => new Float32Array(length));
    return { getChannelData: (channel: number) => samples[channel] } as unknown as AudioBuffer;
  }
  async resume(): Promise<void> { if (FakeAudioContext.rejectResume) throw new Error("resume denied"); this.state = "running"; }
  async suspend(): Promise<void> { this.state = "suspended"; }
  async close(): Promise<void> { this.closeCount++; this.state = "closed"; }
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  FakeWorker.instances = [];
  FakeAudioContext.instances = [];
  FakeAudioContext.rejectResume = false;
  memoryDatabases.clear();
});

describe("simulation worker support", () => {
  it("accepts newer absolute control snapshots and ignores duplicate or out-of-order sequences", () => {
    const gate = new ControlSequenceGate();
    expect(gate.accept(1)).toEqual({ accepted: true, appliedSequence: 1 });
    expect(gate.accept(3)).toEqual({ accepted: true, appliedSequence: 3 }); // Sequence 2 was superseded by a full desired-state snapshot.
    expect(gate.accept(3)).toEqual({ accepted: false, appliedSequence: 3 });
    expect(gate.accept(2)).toEqual({ accepted: false, appliedSequence: 3 });
    expect(gate.accept(Number.NaN)).toEqual({ accepted: false, appliedSequence: 3 });
    gate.reset();
    expect(gate.accept(1)).toEqual({ accepted: true, appliedSequence: 1 });
  });

  it("versions every client message and resets sequenced controls on a new world epoch", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient();
    client.init(4);
    const worker = FakeWorker.instances[0];
    const firstInit = worker.sent[0].message as { protocolVersion: number; epoch: number };
    const firstControl = worker.sent[1].message as { protocolVersion: number; epoch: number; sequence: number };
    expect(firstInit).toMatchObject({ protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1 });
    expect(firstControl).toMatchObject({ protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, sequence: 1 });

    client.controls({ paused: true });
    client.controls({ speed: 4 });
    const controls = worker.sent.filter(({ message }) => (message as { type?: string }).type === "control").map(({ message }) => {
      const control = message as { sequence: number; paused?: boolean; speed?: number };
      return { sequence: control.sequence, ...(control.paused !== undefined ? { paused: control.paused } : {}), ...(control.speed !== undefined ? { speed: control.speed } : {}) };
    });
    expect(controls.slice(-2)).toEqual([{ sequence: 2, paused: true }, { sequence: 3, paused: true, speed: 4 }]);
    worker.emit({ type: "controlAck", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, appliedSequence: 2, tick: 0 });
    worker.emit({ type: "controlAck", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, appliedSequence: 3, tick: 4 });
    client.init(5);
    const lastControl = worker.sent.filter(({ message }) => (message as { type?: string }).type === "control").at(-1)!.message as { epoch: number; sequence: number };
    expect(lastControl).toMatchObject({ epoch: 2, sequence: 1 });
    client.dispose();
  });

  it("reports a bounded control acknowledgement timeout without restarting or changing pause", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient();
    const errors: string[] = [];
    client.onError((error) => errors.push(error.message));
    client.init(8);
    const worker = FakeWorker.instances[0];
    client.controls({ paused: true });
    await vi.advanceTimersByTimeAsync(15_000);
    expect(errors.join(" ")).toContain("controls were not acknowledged within 15 seconds");
    expect(worker.terminated).toBe(false);
    const lastControl = worker.sent.filter(({ message }) => (message as { type?: string }).type === "control").at(-1)!.message as { paused?: boolean };
    expect(lastControl.paused).toBe(true);
    client.dispose();
  });

  it("stops safely on incompatible worker output without sending a buffer command", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient();
    const errors: string[] = [];
    client.onError((error) => errors.push(error.message));
    client.init(9);
    const worker = FakeWorker.instances[0];
    worker.emit({ type: "movement", protocolVersion: WORKER_PROTOCOL_VERSION + 1, epoch: 1, tick: 1, wordCount: 2, buffer: new ArrayBuffer(8), presentationRevision: "g0:0:0:0", trainDoors: new Float32Array(), minute: 360, day: 0 } as unknown as WorkerOutput);
    expect(errors[0]).toContain(`requires version ${WORKER_PROTOCOL_VERSION}`);
    expect(errors[0]).toContain("Reload the app");
    expect(worker.terminated).toBe(true);
    expect(worker.sent.some(({ message }) => (message as { type?: string }).type === "recycle")).toBe(false);
    client.dispose();
  });

  it("caps movement snapshots at two in flight and reuses returned buffers", () => {
    const pool = new MovementBufferPool();
    const first = pool.acquire(40);
    const second = pool.acquire(80);
    expect(first).toBeInstanceOf(ArrayBuffer);
    expect(second).toBeInstanceOf(ArrayBuffer);
    expect(pool.acquire(16)).toBeNull();

    pool.release(first!);
    expect(pool.acquire(24)).toBe(first);
    expect(pool.acquire(16)).toBeNull();
    pool.release(second!);
    pool.reset();
    expect(pool.acquire(16)).toBeInstanceOf(ArrayBuffer);
  });

  it("bounds slow-frame backlog and reports the discarded wall-time ticks", () => {
    expect(capBacklog(158, 12)).toEqual({ backlog: 160, dropped: 10 });
    expect(capBacklog(0, 12, 8)).toEqual({ backlog: 8, dropped: 4 });
  });

  it("decodes movement support and delivers train doors alongside the same tick", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient(), listener = vi.fn();
    client.subscribeMovement(listener); client.init(1);
    const worker = FakeWorker.instances[0];
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, world: sampleWorld(1) });
    const packet = new Uint32Array(13);
    const bits = (value: number) => new Uint32Array(new Float32Array([value]).buffer)[0];
    packet.set([1, 1, 42, bits(0), bits(23.5), bits(0), 2, bits(-1), 1, 1, 1, 10, 30]);
    const buffer = packet.buffer;
    const trainDoors = new Float32Array([42, 1, .5]);
    worker.emit({type:"movement", protocolVersion:WORKER_PROTOCOL_VERSION, epoch:1, tick:10, wordCount:packet.length, buffer, trainDoors, presentationRevision: sampleWorld(1).presentation_revision!, minute:360, day:0});
    expect(listener).toHaveBeenCalledWith(expect.any(Float32Array), 10, 360, 0, trainDoors, new Map<number, MovementSupport>([[42, { resolved: true, roads: [10], platforms: [30] }]]));
    expect(listener.mock.calls[0][0]).toEqual(new Float32Array([42, 0, 23.5, 0, 2, -1]));
    expect(worker.sent.at(-1)?.message).toMatchObject({type:"recycle", buffer});
    client.dispose();
  });

  it("rejects malformed word counts, emits an error, requests resync, and recycles", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient(), errors: string[] = [];
    client.onError(error => errors.push(error.message));
    client.init(1);
    const worker = FakeWorker.instances[0];
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, world: sampleWorld(1) });
    const buffer = new ArrayBuffer(4);
    worker.emit({ type: "movement", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, tick: 1, wordCount: 99, buffer, presentationRevision: sampleWorld(1).presentation_revision!, trainDoors: new Float32Array(), minute: 360, day: 0 });
    expect(errors.join(" ")).toContain("invalid word count");
    expect(worker.sent.some(({ message }) => (message as { type?: string }).type === "resync")).toBe(true);
    expect(worker.sent.some(({ message }) => (message as { type?: string; buffer?: ArrayBuffer }).type === "recycle" && (message as { buffer?: ArrayBuffer }).buffer === buffer)).toBe(true);
    client.dispose();
  });

  it("rejects movement from a stale presentation token and recycles it", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient(); client.init(1);
    const worker = FakeWorker.instances[0];
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, world: sampleWorld(1) });
    const buffer = new ArrayBuffer(8);
    worker.emit({ type: "movement", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, tick: 1, wordCount: 2, buffer, presentationRevision: "stale", trainDoors: new Float32Array(), minute: 360, day: 0 });
    expect(worker.sent.some(({ message }) => (message as { type?: string }).type === "resync")).toBe(true);
    expect(worker.sent.some(({ message }) => (message as { type?: string; buffer?: ArrayBuffer }).type === "recycle" && (message as { buffer?: ArrayBuffer }).buffer === buffer)).toBe(true);
    client.dispose();
  });

  it("rejects stale epochs and recycles stale movement buffers", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient();
    const seen: number[] = [];
    client.subscribeWorld((world) => seen.push(world.revision));
    client.init(1);
    const worker = FakeWorker.instances[0];
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, world: sampleWorld(1) });

    client.init(2);
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, world: sampleWorld(99) });
    const staleBuffer = new ArrayBuffer(24);
    worker.emit({ type: "movement", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 1, tick: 4, wordCount: 2, buffer: staleBuffer, presentationRevision: sampleWorld(1).presentation_revision!, trainDoors: new Float32Array([42, 1, .5]), minute: 360, day: 0 });
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: 2, world: sampleWorld(2) });

    expect(seen).toEqual([1, 2]);
    expect(worker.sent.some(({ message }) => (message as { type?: string; epoch?: number }).type === "recycle" && (message as { epoch?: number }).epoch === 1)).toBe(true);
    client.dispose();
  });

  it("restores the saved development rate without replaying the previous city's override", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient();
    client.controls({ paused: true, hidden: true, speed: 4, development: 16 });
    client.init(1);
    const worker = FakeWorker.instances[0];
    const oldEpoch = client.worldEpoch;

    client.init(2, sampleEngineSave(2, 80));
    const restoreEpoch = client.worldEpoch;
    const restoreControl = worker.sent.filter(({ message }) => (message as { type?: string }).type === "control").at(-1)!.message as { paused?: boolean; hidden?: boolean; speed?: number; development?: number };
    expect(restoreControl).toMatchObject({ paused: true, hidden: true, speed: 4 });
    expect(restoreControl).not.toHaveProperty("development");

    const stale = { ...sampleWorld(90), development_rate: 32 };
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: oldEpoch, world: stale });
    const restored = { ...sampleWorld(2), development_rate: 3 };
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: restoreEpoch, world: restored });
    client.init(3);
    const nextControl = worker.sent.filter(({ message }) => (message as { type?: string }).type === "control").at(-1)!.message as { development?: number };
    expect(nextControl.development).toBe(3);
    client.dispose();
  });

  it("keeps a development command issued while the restored engine is loading", () => {
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient();
    client.init(9, sampleEngineSave(9, 200));
    const worker = FakeWorker.instances[0];
    const epoch = client.worldEpoch;
    client.controls({ development: 12 });
    worker.emit({ type: "world", protocolVersion: WORKER_PROTOCOL_VERSION, epoch, world: { ...sampleWorld(4), development_rate: 2 } });

    client.init(10);
    const latestControl = worker.sent.filter(({ message }) => (message as { type?: string }).type === "control").at(-1)!.message as { development?: number };
    expect(latestControl.development).toBe(12);
    client.dispose();
  });

  it("retains only the current epoch's development command until the engine is ready", () => {
    const pending = new PendingDevelopmentControl();
    const applied: number[] = [];
    const engine = { set_development_rate: (rate: number) => applied.push(rate) };
    pending.reset(4);
    expect(pending.queue(3, 20)).toBe(false);
    expect(pending.queue(4, 8)).toBe(true);
    pending.reset(5);
    pending.apply(4, engine);
    expect(applied).toEqual([]);
    pending.queue(5, 6);
    pending.apply(5, engine);
    pending.apply(5, engine);
    expect(applied).toEqual([6]);
  });

  it("keeps the committed world saveable after a failed replacement and frees only stale candidates", () => {
    const freed: string[] = [];
    const slot = new EngineSlot<{ name: string; save: () => string; free: () => void }>();
    const makeEngine = (name: string) => ({ name, save: () => `save:${name}`, free: () => freed.push(name) });
    const current = makeEngine("current");
    slot.current = current;

    slot.begin(2);
    expect(slot.rollback(2)).toBe(current); // A restore/snapshot failure rolls back without freeing.
    expect(slot.current?.save()).toBe("save:current");
    expect(freed).toEqual([]);

    slot.begin(3);
    const staleCandidate = makeEngine("stale");
    slot.begin(4);
    expect(slot.commit(3, staleCandidate)).toBe(false);
    const winner = makeEngine("winner");
    expect(slot.commit(4, winner)).toBe(true);
    expect(slot.current).toBe(winner);
    expect(freed).toEqual(["stale", "current"]);
    expect(winner.save()).toBe("save:winner");
  });

  it("rejects unanswered requests after a bounded timeout", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("window", globalThis);
    vi.stubGlobal("Worker", FakeWorker);
    const client = new SimulationClient();
    client.init(3);
    const result = expect(client.save()).rejects.toThrow("timed out");
    await vi.advanceTimersByTimeAsync(15_000);
    await result;
    client.dispose();
  });
});

describe("local saves", () => {
  it("validates portable checkpoints and extracts their seed", () => {
    expect(validateWorldSave(sampleEngineSave(41, 900))).toEqual({ data: sampleEngineSave(41, 900), seed: 41 });
    expect(() => validateWorldSave(JSON.stringify({ schema_version: 1, engine_version: "x", seed: 41, tick: 9, nodes: [], roads: [], parcels: [], buildings: [], projects: [], travelers: [], trains: [], events: [] }))).toThrow("invalid engine schema");
    const malformedRows = JSON.parse(sampleEngineSave(41, 900)) as Record<string, unknown>;
    malformedRows.nodes = [null];
    expect(() => validateWorldSave(JSON.stringify(malformedRows))).toThrow("invalid engine schema");
    const wrongSeed = JSON.parse(sampleEngineSave(41, 900)) as Record<string, unknown>;
    wrongSeed.seed = -1;
    expect(() => validateWorldSave(JSON.stringify(wrongSeed))).toThrow("invalid engine schema");
  });

  it("loads versioned viewer preferences safely and falls back on corrupt or unknown data", () => {
    const storage = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
    });
    const preferences = { musicLevel: 0.72, cityLevel: 0.24, style: "miniature" as const, focus: false, quality: "balanced" as const, contrast: "high" as const, lens: "detail" as const };
    saveViewerPreferences(preferences);
    expect(loadViewerPreferences()).toEqual(preferences);
    const defaults = { musicLevel: 0.38, cityLevel: 0.55, style: "anime", focus: true, quality: "high", contrast: "standard", lens: "environment" };
    expect(parseViewerPreferences("not json")).toEqual(defaults);
    expect(parseViewerPreferences(JSON.stringify({ version: 9, musicLevel: 1, cityLevel: 1 }))).toEqual(defaults);
    expect(parseViewerPreferences(JSON.stringify({ version: 1, musicLevel: Infinity, cityLevel: -2, style: "neon", focus: "yes", quality: "ultra", contrast: "neon" }))).toEqual({
      ...defaults, cityLevel: 0,
    });
    expect(parseViewerPreferences(JSON.stringify({ version: 1, musicLevel: 0.7, cityLevel: 0.3, style: "miniature", focus: false, quality: "balanced" }))).toEqual({
      musicLevel: 0.7, cityLevel: 0.3, style: "miniature", focus: false, quality: "balanced", contrast: "standard", lens: "environment",
    });
    expect(parseViewerPreferences(JSON.stringify({ version: 1, lens: "portrait" }))).toEqual(defaults);
    expect(parseViewerPreferences(JSON.stringify({ version: 1, lens: "neighborhood" })).lens).toBe("neighborhood");
    storage.set("thecity-viewer-preferences", "{");
    expect(loadViewerPreferences()).toEqual(defaults);
  });

  it("rejects structurally invalid JSON saves before writing a slot", async () => {
    vi.stubGlobal("indexedDB", makeMemoryIndexedDb());
    await expect(saveWorld('{"valid_json":true}', 11)).rejects.toThrow("invalid engine schema");
    expect(memoryDatabases.get("thecity-worlds")).toBeUndefined();
  });

  it("rotates through two checkpoints and loads the newest valid record", async () => {
    vi.stubGlobal("indexedDB", makeMemoryIndexedDb());
    await saveWorld(sampleEngineSave(11, 1), 11);
    await new Promise((resolve) => setTimeout(resolve, 3));
    await saveWorld(sampleEngineSave(22, 2), 22);
    await new Promise((resolve) => setTimeout(resolve, 3));
    await saveWorld(sampleEngineSave(33, 3), 33);

    const database = memoryDatabases.get("thecity-worlds");
    expect(database?.size).toBe(2);
    expect(await loadWorld()).toEqual({ data: sampleEngineSave(33, 3), seed: 33, savedAt: expect.any(Number) });

    const newest = [...database!.values()].reduce((a, b) => a.savedAt >= b.savedAt ? a : b);
    database!.set(newest.slot, { ...newest, data: JSON.stringify({ valid_json: true }), savedAt: newest.savedAt + 100 });
    expect(await loadWorld()).toEqual({ data: sampleEngineSave(22, 2), seed: 22, savedAt: expect.any(Number) });
  });
});

describe("audio lifecycle", () => {
  function installAudioMocks(): void {
    vi.stubGlobal("window", globalThis);
    vi.stubGlobal("AudioContext", FakeAudioContext);
    vi.stubGlobal("OscillatorNode", FakeOscillatorNode);
    vi.stubGlobal("AudioBufferSourceNode", FakeBufferSourceNode);
  }

  it("keeps the source graph capped during dense activity updates and enables once", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = FakeAudioContext.instances[0];
    const initialOscillators = context.oscillators.length;
    const initialBuffers = context.buffers.length;
    await director.enable();
    await vi.advanceTimersByTimeAsync(30);
    const master = context.gains.find((gain) => gain.connections.includes(context.compressors[0]));
    const buses = context.gains.filter((gain) => gain.connections.includes(master!));
    const musicBus = buses.find((bus) => bus.gain.value === 0.58);
    expect(musicBus).toBeDefined();
    expect(context.compressors).toHaveLength(1);
    expect(context.compressors[0].threshold.value).toBe(-8);
    expect(context.compressors[0].ratio.value).toBe(12);
    expect(context.gains.some((gain) => gain.connections.includes(musicBus!) && gain.gain.value > 0)).toBe(true);

    const trains = Array.from({ length: 100 }, (_, id) => ({ id, x: id * 0.2, z: id * 0.1, speed: 0.5 }));
    const construction = Array.from({ length: 100 }, (_, id) => ({ id: id + 1_000, x: id * -0.2, z: id * 0.1 }));
    for (let frame = 0; frame < 20; frame++) {
      director.update({ hour: 19, rain: 0.4, camera: { x: 0, y: 30, z: 0 }, trains, construction }, 0.05);
    }

    expect(FakeAudioContext.instances).toHaveLength(1);
    expect(vi.getTimerCount()).toBe(1);
    expect(context.oscillators).toHaveLength(initialOscillators);
    expect(context.buffers).toHaveLength(initialBuffers);
    expect(initialOscillators).toBeGreaterThan(32);
    expect(initialBuffers).toBe(37); // 32 reusable emitters plus five fixed ambience/music loops.
    director.dispose();
  });

  it("rejects failed or disposed enable attempts instead of reporting sound as enabled", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    FakeAudioContext.rejectResume = true;
    const director = new AudioDirector();
    await expect(director.enable()).rejects.toThrow(/Could not resume Web Audio/);
    expect(vi.getTimerCount()).toBe(0);
    expect(FakeAudioContext.instances[0].state).toBe("suspended");

    FakeAudioContext.rejectResume = false;
    await director.enable();
    director.dispose();
    await expect(director.enable()).rejects.toThrow(/disposed/);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("keeps 32 simultaneous near emitters inside a bounded city mix", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = FakeAudioContext.instances[0];
    director.update({
      hour: 17,
      rain: 1,
      camera: { x: 0, y: 2.75, z: 0 },
      trains: Array.from({ length: 32 }, (_, id) => ({ id, x: 0, z: 0, speed: 1 })),
      traffic: [],
      construction: [],
      pedestrians: 64,
    }, 0.05);
    const emitterGains = context.panners.map((panner) => (panner.connections[0] as FakeGainNode).gain.value);
    expect(emitterGains).toHaveLength(32);
    expect(emitterGains.reduce((sum, gain) => sum + gain, 0)).toBeLessThanOrEqual(0.42 + 1e-9);
    expect(emitterGains.every((gain) => gain > 0)).toBe(true);
    expect(context.compressors[0].threshold.value).toBe(-8); // Final headroom catches unrelated correlated city/music peaks.
    director.dispose();
  });

  it("starts only on enable and keeps city emitters audible while music is muted and simulation is paused", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    const nearbyTrain = [{ id: 44, x: 8, z: 4, speed: 0.6 }];
    const scene = { hour: 19, rain: 0.2, camera: { x: 0, y: 25, z: 0 }, trains: nearbyTrain, construction: [] };
    for (let i = 0; i < 20; i++) director.update(scene, 0.05);
    expect(FakeAudioContext.instances).toHaveLength(0);
    expect(vi.getTimerCount()).toBe(0);

    await director.enable();
    const context = FakeAudioContext.instances[0];
    expect(context.createdBuffers).toBe(1); // One generated noise bed is shared by the whole graph.
    expect(context.buffers.every((source) => source.buffer !== null)).toBe(true);

    const master = context.gains.find((gain) => gain.connections.includes(context.compressors[0]));
    expect(master).toBeDefined();
    const buses = context.gains.filter((gain) => gain.connections.includes(master!));
    expect(buses).toHaveLength(2);
    director.setLevels(0, 0.74);
    expect(buses.some((bus) => bus.gain.value === 0)).toBe(true);
    expect(buses.some((bus) => bus.gain.value === 0.74)).toBe(true);

    director.update(scene, 0.05);
    const emitterOutputs = context.panners.map((panner) => panner.connections[0] as FakeGainNode);
    expect(emitterOutputs.some((gain) => gain?.gain.value > 0)).toBe(true);
    director.setPaused(true);
    expect(emitterOutputs.every((gain) => gain?.gain.value === 0)).toBe(true);
    expect(buses.some((bus) => bus.gain.value === 0.74)).toBe(true); // Quiet ambience remains available.
    director.setPaused(false);
    expect(emitterOutputs.some((gain) => gain?.gain.value > 0)).toBe(true);

    director.dispose();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("pans against camera orientation and attenuates emitters by 3D distance", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = FakeAudioContext.instances[0];
    const train = [{ id: 812, x: 12, z: 0, speed: 0.4 }];
    const frame = {
      hour: 16,
      rain: 0,
      camera: { x: 0, y: 2.75, z: 0, forward: { x: 0, z: -1 } },
      trains: train,
      construction: [],
    };

    director.update(frame, 0.05);
    const panner = context.panners.find((candidate) => ((candidate.connections[0] as FakeGainNode).gain.value > 0));
    expect(panner?.pan.value).toBeGreaterThan(0); // East is to the right while looking north (-Z).

    director.update({ ...frame, camera: { ...frame.camera, forward: { x: 1, z: 0 } } }, 0.05);
    expect(panner?.pan.value).toBeCloseTo(0); // East is ahead when looking east.

    const output = panner!.connections[0] as FakeGainNode;
    const closeLevel = output.gain.value;
    director.update({ ...frame, camera: { ...frame.camera, y: 90 } }, 0.05);
    expect(output.gain.value).toBeLessThan(closeLevel); // Height contributes to attenuation.
    director.dispose();
  });

  it("adds a gentle river wash that fades with distance on the independently controlled city bus", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = FakeAudioContext.instances[0];
    const master = context.gains.find((gain) => gain.connections.includes(context.compressors[0]));
    const bus = context.gains.find((gain) => gain.connections.some((node) => node instanceof FakeNode && node.connections.includes(master!)));
    expect(bus).toBeDefined();

    const waterSource = context.buffers[2];
    const pending = [...waterSource.connections];
    const visited = new Set<FakeNode>();
    const reachable: FakeNode[] = [];
    while (pending.length > 0) {
      const node = pending.pop()!;
      if (visited.has(node)) continue;
      visited.add(node);
      reachable.push(node);
      pending.push(...node.connections);
    }
    const waterOutput = reachable.find((node): node is FakeGainNode => node instanceof FakeGainNode && node.connections.includes(bus!));
    expect(waterOutput).toBeDefined();

    const frame = { hour: 12, rain: 0, camera: { x: 0, y: 0, z: 0 }, trains: [], construction: [] };
    director.update({ ...frame, riverDistance: 0 }, 0.05);
    const near = waterOutput!.gain.value;
    director.update({ ...frame, riverDistance: 100 }, 0.05);
    const far = waterOutput!.gain.value;
    expect(near).toBeGreaterThan(0);
    expect(far).toBeLessThan(near);

    director.setLevels(0, 0);
    expect(waterOutput!.gain.value).toBe(far); // The city level is controlled at its bus, independently of music.
    director.dispose();
  });

  it("uses a bounded nearby traffic emitter and aggregates pedestrians into the city bed", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = FakeAudioContext.instances[0];
    const frame = {
      hour: 14,
      rain: 0,
      camera: { x: 0, y: 0, z: 0, forward: { x: 0, z: -1 } },
      trains: [],
      traffic: [{ id: 66, x: 9, z: 0, speed: 0.4 }],
      pedestrians: 24,
      construction: [],
    };
    director.update(frame, 0.05);

    const activeVehicle = context.panners.find((panner) => (panner.connections[0] as FakeGainNode).gain.value > 0);
    expect(activeVehicle).toBeDefined();
    const activeOutput = activeVehicle!.connections[0] as FakeGainNode;
    expect(activeOutput.gain.value).toBeGreaterThan(0);

    const pedestrianSource = context.buffers[3];
    const pending = [...pedestrianSource.connections];
    const visited = new Set<FakeNode>();
    const audiblePath: FakeNode[] = [];
    while (pending.length > 0) {
      const node = pending.pop()!;
      if (visited.has(node)) continue;
      visited.add(node);
      audiblePath.push(node);
      pending.push(...node.connections);
    }
    const pedestrianGain = audiblePath.find((node): node is FakeGainNode => node instanceof FakeGainNode);
    expect(pedestrianGain?.gain.value).toBeGreaterThan(0);
    const sourceCount = context.buffers.length;
    for (let i = 0; i < 80; i++) director.update({ ...frame, traffic: frame.traffic.map((vehicle) => ({ ...vehicle, x: vehicle.x + i * 0.05 })) }, 0.05);
    expect(context.buffers).toHaveLength(sourceCount);
    expect(context.panners).toHaveLength(32);
    director.dispose();
  });

  it("plays only fresh station cues and suppresses loaded, paused, hidden, and accelerated events", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = FakeAudioContext.instances[0];
    const base = { hour: 12, rain: 0, camera: { x: 0, y: 0, z: 0 }, trains: [], construction: [] };
    const cueCount = () => context.gains.reduce((total, gain) => total + gain.gain.history.filter((value) => value === 0.008).length, 0);

    // Initial snapshots and a changed world establish a new baseline; saved events are history.
    director.update({ ...base, worldKey: 1, tick: 100, events: [{ id: 1, tick: 100, kind: "TrainStationStop", entity_id: 4 }] }, 0.05);
    expect(cueCount()).toBe(0);
    director.update({ ...base, worldKey: 1, tick: 101, events: [
      { id: 1, tick: 100, kind: "TrainStationStop", entity_id: 4 },
      { id: 2, tick: 101, kind: "TrainStationStop", entity_id: 4 },
    ] }, 0.05);
    expect(cueCount()).toBe(1);

    context.currentTime = 3;
    director.setPaused(true);
    director.update({ ...base, worldKey: 1, tick: 102, events: [{ id: 3, tick: 102, kind: "TrainStationStop", entity_id: 4 }] }, 0.05);
    director.setPaused(false);
    context.currentTime = 6;
    director.update({ ...base, worldKey: 1, tick: 103, events: [
      { id: 3, tick: 102, kind: "TrainStationStop", entity_id: 4 },
      { id: 4, tick: 103, kind: "TrainStationStop", entity_id: 4 },
    ], playbackRate: 4 }, 0.05);
    expect(cueCount()).toBe(1);

    director.setHidden(true);
    context.currentTime = 9;
    director.update({ ...base, worldKey: 1, tick: 104, events: [{ id: 5, tick: 104, kind: "TrainStationStop", entity_id: 4 }] }, 0.05);
    director.setHidden(false);
    context.currentTime = 12;
    director.update({ ...base, worldKey: 1, tick: 105, events: [
      { id: 5, tick: 104, kind: "TrainStationStop", entity_id: 4 },
      { id: 6, tick: 105, kind: "TrainStationStop", entity_id: 4 },
    ] }, 0.05);
    expect(cueCount()).toBe(2); // Only the live event after resume is played.

    context.currentTime = 15;
    director.update({ ...base, worldKey: 2, tick: 500, events: [{ id: 7, tick: 500, kind: "TrainStationStop", entity_id: 4 }] }, 0.05);
    expect(cueCount()).toBe(2); // Context changes never replay saved event history.
    context.currentTime = 18;
    director.update({ ...base, worldKey: 2, tick: 501, developmentRate: 16, events: [
      { id: 8, tick: 501, kind: "TrainStationStop", entity_id: 4 },
      { id: 9, tick: 450, kind: "TrainStationStop", entity_id: 4 },
    ] }, 0.05);
    expect(cueCount()).toBe(2);
    context.currentTime = 21;
    director.update({ ...base, worldKey: 2, tick: 502, events: [
      { id: 8, tick: 501, kind: "TrainStationStop", entity_id: 4 },
      { id: 9, tick: 450, kind: "TrainStationStop", entity_id: 4 },
    ] }, 0.05);
    expect(cueCount()).toBe(2); // Fast-forward and stale event IDs are consumed, never replayed.
    director.dispose();
  });

  it("fades and suspends when hidden, resumes, then clears sources and timers on dispose", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = FakeAudioContext.instances[0];
    for (let i = 0; i < 15; i++) director.setHidden(true);
    expect(vi.getTimerCount()).toBe(2); // one scheduler, one replaceable suspend timer.
    await vi.advanceTimersByTimeAsync(300);
    expect(context.state).toBe("suspended");

    director.setHidden(false);
    await Promise.resolve();
    expect(context.state).toBe("running");
    expect(vi.getTimerCount()).toBe(1);
    director.dispose();
    await Promise.resolve();
    expect(context.closeCount).toBe(1);
    expect(context.buffers.every((source) => source.stopped)).toBe(true);
    expect(context.oscillators.every((source) => source.stopped)).toBe(true);
    expect(vi.getTimerCount()).toBe(0);
  });
});
