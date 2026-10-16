import { afterEach, describe, expect, it, vi } from 'vitest';
import { AudioDirector, type AudioWorld } from '../web/src/audio/director';

class FakeParam {
  value = 0;
  history: number[] = [];
  setTargetAtTime(value: number): void {
    this.value = value;
    this.history.push(value);
  }
  setValueAtTime(value: number): void {
    this.value = value;
    this.history.push(value);
  }
  linearRampToValueAtTime(value: number): void {
    this.value = value;
    this.history.push(value);
  }
  exponentialRampToValueAtTime(value: number): void {
    this.value = value;
    this.history.push(value);
  }
  cancelScheduledValues(): void {}
}
class FakeNode {
  connections: FakeNode[] = [];
  connect<T extends FakeNode>(target: T): T {
    this.connections.push(target);
    return target;
  }
}
class FakeGainNode extends FakeNode {
  gain = new FakeParam();
}
class FakeFilterNode extends FakeNode {
  frequency = new FakeParam();
  Q = new FakeParam();
  type = 'lowpass';
}
class FakePannerNode extends FakeNode {
  pan = new FakeParam();
}
class FakeCompressorNode extends FakeNode {
  threshold = new FakeParam();
  knee = new FakeParam();
  ratio = new FakeParam();
  attack = new FakeParam();
  release = new FakeParam();
}
class FakeOscillatorNode extends FakeNode {
  frequency = new FakeParam();
  type = 'sine';
  started = false;
  stopped = false;
  start(): void {
    this.started = true;
  }
  stop(): void {
    this.stopped = true;
  }
}
class FakeBufferSourceNode extends FakeNode {
  buffer: AudioBuffer | null = null;
  loop = false;
  started = false;
  stopped = false;
  start(): void {
    this.started = true;
  }
  stop(): void {
    this.stopped = true;
  }
}
class FakeAudioContext {
  state: AudioContextState = 'suspended';
  currentTime = 0;
  sampleRate = 8_000;
  destination = new FakeNode() as unknown as AudioDestinationNode;
  oscillators: FakeOscillatorNode[] = [];
  buffers: FakeBufferSourceNode[] = [];
  gains: FakeGainNode[] = [];
  panners: FakePannerNode[] = [];
  compressors: FakeCompressorNode[] = [];
  createGain(): GainNode {
    const node = new FakeGainNode();
    this.gains.push(node);
    return node as unknown as GainNode;
  }
  createBiquadFilter(): BiquadFilterNode {
    return new FakeFilterNode() as unknown as BiquadFilterNode;
  }
  createStereoPanner(): StereoPannerNode {
    const node = new FakePannerNode();
    this.panners.push(node);
    return node as unknown as StereoPannerNode;
  }
  createDynamicsCompressor(): DynamicsCompressorNode {
    const node = new FakeCompressorNode();
    this.compressors.push(node);
    return node as unknown as DynamicsCompressorNode;
  }
  createOscillator(): OscillatorNode {
    const node = new FakeOscillatorNode();
    this.oscillators.push(node);
    return node as unknown as OscillatorNode;
  }
  createBufferSource(): AudioBufferSourceNode {
    const node = new FakeBufferSourceNode();
    this.buffers.push(node);
    return node as unknown as AudioBufferSourceNode;
  }
  createBuffer(channels: number, length: number): AudioBuffer {
    const samples = Array.from({ length: channels }, () => new Float32Array(length));
    return { getChannelData: (channel: number) => samples[channel] } as unknown as AudioBuffer;
  }
  async resume(): Promise<void> {
    this.state = 'running';
  }
  async suspend(): Promise<void> {
    this.state = 'suspended';
  }
  async close(): Promise<void> {
    this.state = 'closed';
  }
}

function installAudioMocks(): void {
  vi.stubGlobal('window', globalThis);
  vi.stubGlobal('AudioContext', FakeAudioContext);
  vi.stubGlobal('OscillatorNode', FakeOscillatorNode);
  vi.stubGlobal('AudioBufferSourceNode', FakeBufferSourceNode);
}

function world(overrides: Partial<AudioWorld> = {}): AudioWorld {
  return {
    hour: 15,
    rain: 0,
    camera: { x: 0, y: 0, z: 0, forward: { x: 0, z: -1 } },
    trains: [],
    traffic: [],
    buses: [],
    construction: [],
    ...overrides,
  };
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('bus sound', () => {
  it('uses a distinct soft spatial engine voice inside the shared 32-source mix', async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = (director as unknown as { context: FakeAudioContext }).context;
    const sourceCounts = {
      oscillators: context.oscillators.length,
      buffers: context.buffers.length,
    };

    const frame = world({
      buses: [{ id: 41, x: 4, z: 0, speed: 0 }],
      traffic: [{ id: 42, x: 8, z: 0, speed: 0 }],
    });
    director.update(frame, 0.05);
    expect(context.panners).toHaveLength(32);
    const active = context.panners.filter(
      (panner) => (panner.connections[0] as FakeGainNode).gain.value > 0,
    );
    expect(active).toHaveLength(2);
    expect(active[0].pan.value).toBeGreaterThan(0); // Bus east of the north-facing listener.
    expect(active[1].pan.value).toBeGreaterThan(active[0].pan.value); // Car remains spatially distinct.
    expect(context.oscillators[10].frequency.value).toBe(43); // Bus low engine; traffic begins at the next voice pair.
    expect(context.oscillators[12].frequency.value).toBe(62);
    expect(
      active.reduce((sum, panner) => sum + (panner.connections[0] as FakeGainNode).gain.value, 0),
    ).toBeLessThanOrEqual(0.42);

    const crowded = world({
      buses: Array.from({ length: 80 }, (_, id) => ({ id, x: id % 6, z: id % 7, speed: 0.4 })),
      traffic: Array.from({ length: 80 }, (_, id) => ({
        id: id + 100,
        x: id % 5,
        z: id % 8,
        speed: 0.3,
      })),
    });
    for (let i = 0; i < 40; i++) director.update(crowded, 0.05);
    expect(context.panners).toHaveLength(32);
    expect(context.oscillators).toHaveLength(sourceCounts.oscillators);
    expect(context.buffers).toHaveLength(sourceCounts.buffers);
    const mix = context.panners.reduce(
      (sum, panner) => sum + (panner.connections[0] as FakeGainNode).gain.value,
      0,
    );
    expect(mix).toBeLessThanOrEqual(0.42 + 1e-9);
    director.dispose();
  });

  it('plays one soft BusStop cue per bus cooldown and rejects history, duplicates, pause, hidden, and accelerated snapshots', async () => {
    installAudioMocks();
    vi.useFakeTimers();
    const director = new AudioDirector();
    await director.enable();
    const context = (director as unknown as { context: FakeAudioContext }).context;
    const cues = () =>
      context.gains.reduce(
        (total, gain) => total + gain.gain.history.filter((value) => value === 0.0055).length,
        0,
      );
    const base = world({
      worldKey: 'bus-town',
      tick: 100,
      buses: [{ id: 7, x: 4, z: 2 }],
      events: [{ id: 1, tick: 100, kind: 'BusStop', entity_id: 7 }],
    });

    director.update(base, 0.05); // Saved event history is never replayed on a world baseline.
    expect(cues()).toBe(0);
    director.update(
      world({
        ...base,
        tick: 101,
        events: [
          { id: 1, tick: 100, kind: 'BusStop', entity_id: 7 },
          { id: 2, tick: 101, kind: 'BusStop', entity_id: 7, x: 4, z: 2 },
        ],
      }),
      0.05,
    );
    expect(cues()).toBe(1);
    expect(context.oscillators[1].frequency.value).toBe(392);
    expect(context.oscillators[2].frequency.value).toBe(523.25);

    context.currentTime = 3;
    director.update(
      world({ ...base, tick: 102, events: [{ id: 2, tick: 101, kind: 'BusStop', entity_id: 7 }] }),
      0.05,
    );
    expect(cues()).toBe(1); // Duplicate event ID.
    context.currentTime = 4;
    director.update(
      world({ ...base, tick: 103, events: [{ id: 3, tick: 103, kind: 'BusStop', entity_id: 7 }] }),
      0.05,
    );
    expect(cues()).toBe(1); // Per-bus dwell cooldown even though global budget is ready.

    context.currentTime = 9;
    director.setPaused(true);
    director.update(
      world({ ...base, tick: 104, events: [{ id: 4, tick: 104, kind: 'BusStop', entity_id: 8 }] }),
      0.05,
    );
    director.setPaused(false);
    expect(cues()).toBe(1);
    context.currentTime = 12;
    director.setHidden(true);
    director.update(
      world({ ...base, tick: 105, events: [{ id: 5, tick: 105, kind: 'BusStop', entity_id: 9 }] }),
      0.05,
    );
    director.setHidden(false);
    expect(cues()).toBe(1);
    context.currentTime = 15;
    director.update(
      world({
        ...base,
        tick: 106,
        playbackRate: 4,
        events: [{ id: 6, tick: 106, kind: 'BusStop', entity_id: 10 }],
      }),
      0.05,
    );
    expect(cues()).toBe(1);
    context.currentTime = 18;
    director.update(
      world({
        ...base,
        tick: 107,
        events: [
          { id: 4, tick: 104, kind: 'BusStop', entity_id: 8 },
          { id: 5, tick: 105, kind: 'BusStop', entity_id: 9 },
          { id: 6, tick: 106, kind: 'BusStop', entity_id: 10 },
          { id: 7, tick: 107, kind: 'BusStop', entity_id: 11 },
        ],
      }),
      0.05,
    );
    expect(cues()).toBe(2); // Only the new live event after resume is played.
    context.currentTime = 19.5;
    director.update(
      world({ ...base, tick: 108, events: [{ id: 8, tick: 108, kind: 'BusStop', entity_id: 12 }] }),
      0.05,
    );
    expect(cues()).toBe(2); // The shared cue budget coalesces even a different bus.
    context.currentTime = 21;
    director.update(
      world({ ...base, tick: 109, events: [{ id: 9, tick: 109, kind: 'BusStop', entity_id: 12 }] }),
      0.05,
    );
    expect(cues()).toBe(3);
    director.dispose();
  });
});
