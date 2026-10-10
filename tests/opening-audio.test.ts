import { afterEach, describe, expect, it, vi } from "vitest";
import { AudioDirector, type AudioWorld } from "../web/src/audio/director";

class Param {
  value = 0;
  history: number[] = [];
  setTargetAtTime(value: number): void { this.value = value; this.history.push(value); }
  setValueAtTime(value: number): void { this.value = value; this.history.push(value); }
  linearRampToValueAtTime(value: number): void { this.value = value; this.history.push(value); }
  exponentialRampToValueAtTime(value: number): void { this.value = value; this.history.push(value); }
  cancelScheduledValues(): void {}
}

class Node {
  connections: Node[] = [];
  connect<T extends Node>(target: T): T { this.connections.push(target); return target; }
}
class Gain extends Node { gain = new Param(); }
class Filter extends Node { frequency = new Param(); Q = new Param(); type = "lowpass"; }
class Panner extends Node { pan = new Param(); }
class Compressor extends Node {
  threshold = new Param(); knee = new Param(); ratio = new Param(); attack = new Param(); release = new Param();
}
class Oscillator extends Node {
  frequency = new Param(); type = "sine";
  start(): void {}
  stop(): void {}
}
class BufferSource extends Node {
  buffer: AudioBuffer | null = null;
  loop = false;
  start(): void {}
  stop(): void {}
}
class Context {
  static instances: Context[] = [];
  state: AudioContextState = "suspended";
  currentTime = 0;
  sampleRate = 8_000;
  destination = new Node() as unknown as AudioDestinationNode;
  gains: Gain[] = [];
  panners: Panner[] = [];
  oscillators: Oscillator[] = [];
  buffers: BufferSource[] = [];
  constructor() { Context.instances.push(this); }
  createGain(): GainNode { const node = new Gain(); this.gains.push(node); return node as unknown as GainNode; }
  createBiquadFilter(): BiquadFilterNode { return new Filter() as unknown as BiquadFilterNode; }
  createStereoPanner(): StereoPannerNode { const node = new Panner(); this.panners.push(node); return node as unknown as StereoPannerNode; }
  createDynamicsCompressor(): DynamicsCompressorNode { return new Compressor() as unknown as DynamicsCompressorNode; }
  createOscillator(): OscillatorNode { const node = new Oscillator(); this.oscillators.push(node); return node as unknown as OscillatorNode; }
  createBufferSource(): AudioBufferSourceNode { const node = new BufferSource(); this.buffers.push(node); return node as unknown as AudioBufferSourceNode; }
  createBuffer(channels: number, length: number): AudioBuffer {
    const data = Array.from({ length: channels }, () => new Float32Array(length));
    return { getChannelData: (channel: number) => data[channel] } as unknown as AudioBuffer;
  }
  async resume(): Promise<void> { this.state = "running"; }
  async suspend(): Promise<void> { this.state = "suspended"; }
  async close(): Promise<void> { this.state = "closed"; }
}

function installAudioMocks(): void {
  Context.instances = [];
  vi.stubGlobal("window", globalThis);
  vi.stubGlobal("AudioContext", Context);
  vi.stubGlobal("OscillatorNode", Oscillator);
  vi.stubGlobal("AudioBufferSourceNode", BufferSource);
}

function world(overrides: Partial<AudioWorld> = {}): AudioWorld {
  return {
    hour: 12,
    rain: 0,
    camera: { x: 0, y: 0, z: 0, forward: { x: 0, z: -1 } },
    trains: [],
    construction: [],
    ...overrides,
  };
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("event-linked opening ambience", () => {
  it("plays a quiet spatial activity swell for a fresh nearby opening without allocating event sources", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = Context.instances[0];
    const sourceCounts = { buffers: context.buffers.length, oscillators: context.oscillators.length };
    const baseline = world({ worldKey: "openings", tick: 100, events: [
      { id: 1, tick: 100, kind: "BuildingOpened", entity_id: 4, x: 12, z: 0 },
    ] });
    director.update(baseline, 0.05); // Loaded-world events are history.
    const openingGain = (director as unknown as { openingGain: Gain }).openingGain;
    expect(openingGain.gain.history).toHaveLength(0);

    director.update(world({ worldKey: "openings", tick: 101, events: [
      { id: 1, tick: 100, kind: "BuildingOpened", entity_id: 4, x: 12, z: 0 },
      { id: 2, tick: 101, kind: "RoadOpened", entity_id: 8, x: 16, z: 0 },
    ] }), 0.05);
    const openingPanner = openingGain.connections.find((node): node is Panner => node instanceof Panner)!;
    expect(openingPanner.pan.value).toBeGreaterThan(0); // East of a north-facing listener.
    expect(openingGain.gain.history.some((value) => value > 0.00001 && value < 0.0035)).toBe(true);
    expect(context.buffers).toHaveLength(sourceCounts.buffers);
    expect(context.oscillators).toHaveLength(sourceCounts.oscillators);
    expect(context.panners).toHaveLength(33);

    const scheduled = openingGain.gain.history.length;
    context.currentTime = 0.5;
    director.update(world({ worldKey: "openings", tick: 102, events: [
      { id: 2, tick: 101, kind: "RoadOpened", entity_id: 8, x: 16, z: 0 },
    ] }), 0.05);
    expect(openingGain.gain.history).toHaveLength(scheduled); // Duplicate ID never replays.
    director.dispose();
  });

  it("consumes busy, stale, paused, fast-forward, unlocated, and distant events without a delayed cue parade", async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = Context.instances[0];
    const gain = (director as unknown as { openingGain: Gain }).openingGain;
    const eventCount = () => gain.gain.history.filter((value) => value > 0.00001).length;
    director.update(world({ worldKey: 7, tick: 50 }), 0.05);

    director.update(world({ worldKey: 7, tick: 51, events: [
      { id: 1, tick: 51, kind: "RoadOpened", entity_id: 8, x: 8, z: 0 },
      { id: 2, tick: 51, kind: "BuildingOpened", entity_id: 9, x: 9, z: 0 },
      { id: 3, tick: 51, kind: "BusinessOpened", entity_id: 10, x: 10, z: 0 },
    ] }), 0.05);
    expect(eventCount()).toBe(1); // One calm cue for a burst, with the rest consumed.
    const panner = gain.connections.find((node): node is Panner => node instanceof Panner)!;
    expect(panner.pan.value).toBeGreaterThan(0);

    context.currentTime = 3;
    director.update(world({ worldKey: 7, tick: 52, events: [
      { id: 1, tick: 51, kind: "RoadOpened", entity_id: 8, x: 8, z: 0 },
      { id: 2, tick: 51, kind: "BuildingOpened", entity_id: 9, x: 9, z: 0 },
      { id: 3, tick: 51, kind: "BusinessOpened", entity_id: 10, x: 10, z: 0 },
    ] }), 0.05);
    expect(eventCount()).toBe(1); // Snapshot coalescing does not turn into a later parade.

    context.currentTime = 6;
    director.update(world({ worldKey: 7, tick: 53, events: [
      { id: 4, tick: 1, kind: "BuildingOpened", entity_id: 11, x: 10, z: 0 },
      { id: 5, tick: 53, kind: "RoadOpened", entity_id: 12 },
      { id: 6, tick: 53, kind: "BuildingOpened", entity_id: 13, x: 400, z: 0 },
    ] }), 0.05);
    expect(eventCount()).toBe(1);

    context.currentTime = 9;
    director.setPaused(true);
    director.update(world({ worldKey: 7, tick: 54, events: [
      { id: 7, tick: 54, kind: "BusinessOpened", entity_id: 14, x: 4, z: 1 },
    ] }), 0.05);
    director.setPaused(false);
    context.currentTime = 12;
    director.update(world({ worldKey: 7, tick: 55, playbackRate: 4, events: [
      { id: 8, tick: 55, kind: "RoadOpened", entity_id: 15, x: 5, z: 1 },
    ] }), 0.05);
    context.currentTime = 15;
    director.update(world({ worldKey: 7, tick: 56, events: [
      { id: 7, tick: 54, kind: "BusinessOpened", entity_id: 14, x: 4, z: 1 },
      { id: 8, tick: 55, kind: "RoadOpened", entity_id: 15, x: 5, z: 1 },
    ] }), 0.05);
    expect(eventCount()).toBe(1); // Suppressed events are not replayed when normal playback resumes.
    context.currentTime = 18;
    director.update(world({ worldKey: 7, tick: 57, events: [
      { id: 9, tick: 57, kind: "BusinessReopened", entity_id: 16, x: 8, z: 2 },
    ] }), 0.05);
    expect(eventCount()).toBe(2); // Preserve the engine's emitted BusinessReopened spelling.
    expect(context.panners).toHaveLength(33);
    director.dispose();
  });
});
