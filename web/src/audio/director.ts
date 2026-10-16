type Point3 = { x: number; y: number; z: number };

export type AudioWorld = {
  hour: number;
  rain: number;
  camera: Point3 & { forward?: { x: number; z: number } };
  /** Ground distance from the listener's target to the nearest river segment. */
  riverDistance?: number;
  trains: Array<{ id: number; x: number; z: number; speed?: number }>;
  traffic?: Array<{ id: number; x: number; z: number; speed?: number }>;
  buses?: Array<{ id: number; x: number; z: number; speed?: number }>;
  pedestrians?: number;
  construction: Array<{ id: number; x: number; z: number }>;
  events?: Array<{
    id: number;
    tick: number;
    kind: string;
    entity_id: number;
    x?: number;
    z?: number;
  }>;
  worldKey?: number | string;
  tick?: number;
  developmentRate?: number;
  playbackRate?: number;
};

type EmitterKind = 'train' | 'traffic' | 'bus' | 'construction';

type Emitter = {
  id: number;
  kind: EmitterKind | null;
  x: number;
  z: number;
  speed: number;
  seen: boolean;
  distanceSq: number;
  level: number;
  pan: StereoPannerNode;
  gain: GainNode;
  filter: BiquadFilterNode;
  noiseGain: GainNode;
  humGain: GainNode;
  secondaryGain: GainNode;
  hum: OscillatorNode;
  secondary: OscillatorNode;
  noise: AudioBufferSourceNode;
};

type OpeningEvent = NonNullable<AudioWorld['events']>[number] & {
  kind: 'RoadOpened' | 'BuildingOpened' | 'BusinessOpened' | 'BusinessReopened';
};
type AudioEvent = NonNullable<AudioWorld['events']>[number];
type CueEvent = AudioEvent & {
  kind:
    | 'TrainStationStop'
    | 'BusStop'
    | 'RoadOpened'
    | 'BuildingOpened'
    | 'BusinessOpened'
    | 'BusinessReopened';
};

const MAX_EMITTERS = 32;
const MAX_EMITTER_MIX = 0.42;
const BUS_CUE_COOLDOWN_SECONDS = 8;
const clamp01 = (value: number) => Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
function isCueEvent(event: AudioEvent): event is CueEvent {
  return (
    event.kind === 'TrainStationStop' ||
    event.kind === 'BusStop' ||
    event.kind === 'RoadOpened' ||
    event.kind === 'BuildingOpened' ||
    event.kind === 'BusinessOpened' ||
    event.kind === 'BusinessReopened'
  );
}
function isOpeningEvent(event: CueEvent): event is OpeningEvent {
  return (
    event.kind === 'RoadOpened' ||
    event.kind === 'BuildingOpened' ||
    event.kind === 'BusinessOpened' ||
    event.kind === 'BusinessReopened'
  );
}

/** Procedural, user-initiated city ambience and a small original lo-fi score. */
export class AudioDirector {
  private context: AudioContext | null = null;
  private master: GainNode | null = null;
  private limiter: DynamicsCompressorNode | null = null;
  private musicBus: GainNode | null = null;
  private cityBus: GainNode | null = null;
  private ambienceBus: GainNode | null = null;
  private musicNodes: AudioNode[] = [];
  private emitters: Emitter[] = [];
  private noiseBuffer: AudioBuffer | null = null;
  private scheduler: number | null = null;
  private hideTimer: number | null = null;
  private nextStep = 0;
  private currentStep = -1;
  private currentHour = 12;
  private rain = 0;
  private musicLevel = 0.58;
  private cityLevel = 0.62;
  private paused = false;
  private hidden = false;
  private disposed = false;
  private rainGain: GainNode | null = null;
  private cityNoiseGain: GainNode | null = null;
  private windGain: GainNode | null = null;
  private waterGain: GainNode | null = null;
  private pedestrianGain: GainNode | null = null;
  private openingGain: GainNode | null = null;
  private openingPanner: StereoPannerNode | null = null;
  private cueGain: GainNode | null = null;
  private cueOscillators: OscillatorNode[] = [];
  private worldKey: number | string | undefined;
  private hasWorldKey = false;
  private lastWorldTick: number | null = null;
  private lastCueTime = -Infinity;
  private readonly lastBusCueTimes = new Map<number, number>();
  private readonly seenEventIds = new Set<number>();
  private readonly eventIdOrder: number[] = [];
  private musicVoiceGains: GainNode[] = [];
  private musicVoiceOscs: OscillatorNode[] = [];
  private bassGain: GainNode | null = null;
  private bassOsc: OscillatorNode | null = null;
  private leadGain: GainNode | null = null;
  private leadOsc: OscillatorNode | null = null;
  private kickGain: GainNode | null = null;
  private kickOsc: OscillatorNode | null = null;
  private hatGain: GainNode | null = null;
  private snareGain: GainNode | null = null;

  /** Call directly from a user gesture such as the sound button. */
  async enable(): Promise<void> {
    if (this.disposed) throw new Error('Audio director has been disposed');
    if (!this.context) this.createGraph();
    const context = this.context;
    if (!context) throw new Error('Web Audio is unavailable in this browser');

    if (context.state !== 'running') {
      try {
        await context.resume();
      } catch (error) {
        throw new Error(
          'Could not resume Web Audio. Try enabling sound again from the sound control.',
          { cause: error },
        );
      }
    }
    if (this.disposed) throw new Error('Audio director was disposed while sound was starting');
    if (context.state !== 'running') throw new Error('Web Audio did not enter the running state');
    this.applyLevels(0.15);
    if (!this.hidden && this.scheduler === null) {
      this.nextStep = context.currentTime + 0.08;
      this.scheduler = window.setInterval(() => this.scheduleMusic(), 25);
    }
  }

  setLevels(music: number, city: number): void {
    this.musicLevel = clamp01(music);
    this.cityLevel = clamp01(city);
    this.applyLevels(0.09);
  }

  setPaused(paused: boolean): void {
    this.paused = paused;
    this.updateEmitterLevels(0.18);
  }

  setHidden(hidden: boolean): void {
    this.hidden = hidden;
    if (this.hideTimer !== null) window.clearTimeout(this.hideTimer);
    this.hideTimer = null;
    const context = this.context;
    if (!context) return;

    if (hidden) {
      this.setParam(this.master?.gain, 0, 0.2);
      this.hideTimer = window.setTimeout(() => {
        this.hideTimer = null;
        if (this.hidden && context.state === 'running') void context.suspend();
      }, 260);
    } else {
      void this.enable().catch(() => {
        // A visibility-triggered resume is best-effort; keep the graph silent
        // until a later user gesture successfully resumes it.
        this.setParam(this.master?.gain, 0, 0.08);
      });
    }
  }

  update(world: AudioWorld, _dt: number): void {
    if (this.disposed) return;
    this.currentHour = ((world.hour % 24) + 24) % 24;
    this.rain = clamp01(world.rain);
    if (!this.context) return;

    this.setParam(this.rainGain?.gain, this.rain * 0.22, 0.45);
    // Camera height softly blends a broad city wash into neighborhood detail.
    const zoom = clamp((world.camera.y - 8) / 70, 0, 1);
    this.setParam(this.cityNoiseGain?.gain, 0.055 + zoom * 0.045, 0.8);
    this.setParam(this.windGain?.gain, 0.018 + zoom * 0.012, 1.1);
    const pedestrianDensity = Math.sqrt(clamp(world.pedestrians ?? 0, 0, 64) / 64);
    this.setParam(this.pedestrianGain?.gain, 0.022 * pedestrianDensity, 0.9);
    const riverDistance = Number.isFinite(world.riverDistance)
      ? Math.max(0, world.riverDistance!)
      : Infinity;
    const riverDistance3d = Math.hypot(riverDistance, Math.max(0, world.camera.y));
    const waterAttenuation = Number.isFinite(riverDistance3d)
      ? 1 / (1 + riverDistance3d * 0.025 + riverDistance3d * riverDistance3d * 0.00016)
      : 0;
    this.setParam(this.waterGain?.gain, 0.038 * waterAttenuation, 1.15);
    this.updateEventCues(world);

    const camX = world.camera.x;
    const camZ = world.camera.z;
    const camY = world.camera.y;
    const forward = world.camera.forward;
    const forwardLength = forward ? Math.hypot(forward.x, forward.z) : 0;
    const forwardX = forwardLength > 1e-4 ? forward!.x / forwardLength : 0;
    const forwardZ = forwardLength > 1e-4 ? forward!.z / forwardLength : -1;
    const rightX = -forwardZ;
    const rightZ = forwardX;
    const selected: Array<{
      id: number;
      x: number;
      y: number;
      z: number;
      speed: number;
      kind: EmitterKind;
      distanceSq: number;
    }> = [];
    const rangeSq = 190 * 190;
    for (const e of this.emitters) e.seen = false;

    for (const train of world.trains) {
      const dx = train.x - camX;
      const dz = train.z - camZ;
      const dy = 2.75 - camY;
      const distanceSq = dx * dx + dy * dy + dz * dz;
      if (distanceSq > rangeSq) continue;
      selected.push({
        id: train.id,
        x: train.x,
        y: 2.75,
        z: train.z,
        speed: train.speed ?? 0,
        kind: 'train',
        distanceSq,
      });
    }
    for (const vehicle of world.traffic ?? []) {
      const dx = vehicle.x - camX;
      const dz = vehicle.z - camZ;
      const distanceSq = dx * dx + camY * camY + dz * dz;
      if (distanceSq > rangeSq) continue;
      selected.push({
        id: vehicle.id,
        x: vehicle.x,
        y: 0,
        z: vehicle.z,
        speed: vehicle.speed ?? 0,
        kind: 'traffic',
        distanceSq,
      });
    }
    for (const bus of world.buses ?? []) {
      const dx = bus.x - camX;
      const dz = bus.z - camZ;
      const distanceSq = dx * dx + camY * camY + dz * dz;
      if (distanceSq > rangeSq) continue;
      selected.push({
        id: bus.id,
        x: bus.x,
        y: 0,
        z: bus.z,
        speed: bus.speed ?? 0,
        kind: 'bus',
        distanceSq,
      });
    }
    for (const site of world.construction) {
      const dx = site.x - camX;
      const dz = site.z - camZ;
      const dy = -camY;
      const distanceSq = dx * dx + dy * dy + dz * dz;
      if (distanceSq > rangeSq) continue;
      selected.push({
        id: site.id,
        x: site.x,
        y: 0,
        z: site.z,
        speed: 0,
        kind: 'construction',
        distanceSq,
      });
    }
    selected.sort((a, b) => a.distanceSq - b.distanceSq);

    const limit = Math.min(MAX_EMITTERS, selected.length);
    for (let i = 0; i < limit; i++) {
      const target = selected[i];
      let emitter = this.emitters.find(
        (item) => item.kind === target.kind && item.id === target.id,
      );
      if (!emitter) emitter = this.emitters.find((item) => !item.seen);
      if (!emitter) continue;
      const isNew = emitter.kind !== target.kind || emitter.id !== target.id;
      emitter.id = target.id;
      emitter.kind = target.kind;
      emitter.x = target.x;
      emitter.z = target.z;
      emitter.speed = target.speed;
      emitter.distanceSq = target.distanceSq;
      const sourceLevel =
        target.kind === 'train'
          ? 0.105
          : target.kind === 'traffic'
            ? 0.038
            : target.kind === 'bus'
              ? 0.052
              : 0.055;
      emitter.level =
        (1 / (1 + Math.sqrt(target.distanceSq) * 0.075 + target.distanceSq * 0.0011)) * sourceLevel;
      emitter.seen = true;
      if (isNew) this.configureEmitter(emitter);

      const attenuation =
        1 / (1 + Math.sqrt(target.distanceSq) * 0.075 + target.distanceSq * 0.0011);
      const pan = clamp(((target.x - camX) * rightX + (target.z - camZ) * rightZ) / 32, -1, 1);
      this.setParam(emitter.pan.pan, pan, 0.8);
      if (target.kind === 'train') {
        const speedPitch = clamp(Math.abs(target.speed), 0, 1) * 11;
        this.setParam(emitter.hum.frequency, 58 + speedPitch, 0.55);
        this.setParam(emitter.secondary.frequency, 88 + speedPitch * 1.24, 0.55);
        this.setParam(emitter.filter.frequency, 310 + attenuation * 530, 0.6);
      } else if (target.kind === 'traffic') {
        const speedPitch = clamp(Math.abs(target.speed), 0, 1) * 18;
        this.setParam(emitter.hum.frequency, 62 + speedPitch, 0.75);
        this.setParam(emitter.secondary.frequency, 94 + speedPitch * 1.35, 0.75);
        this.setParam(emitter.filter.frequency, 250 + attenuation * 380, 0.8);
      } else if (target.kind === 'bus') {
        // Low, slow twin engine harmonics and a soft filtered tire wash keep
        // buses distinct from the brighter, lighter traffic voice.
        const speedPitch = clamp(Math.abs(target.speed), 0, 1) * 11;
        this.setParam(emitter.hum.frequency, 43 + speedPitch, 0.8);
        this.setParam(emitter.secondary.frequency, 69 + speedPitch * 1.15, 0.8);
        this.setParam(emitter.filter.frequency, 205 + attenuation * 250, 0.9);
        emitter.noiseGain.gain.value = 0.28 + clamp(Math.abs(target.speed), 0, 1) * 0.08;
      } else {
        this.setParam(emitter.hum.frequency, 46, 0.7);
        this.setParam(emitter.secondary.frequency, 173, 0.7);
        this.setParam(emitter.filter.frequency, 380 + attenuation * 460, 0.7);
      }
    }

    for (const emitter of this.emitters) {
      if (!emitter.seen) {
        emitter.level = 0;
        this.setParam(emitter.gain.gain, 0, 0.5);
      }
    }
    this.updateEmitterLevels(0.18);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    if (this.scheduler !== null) window.clearInterval(this.scheduler);
    this.scheduler = null;
    if (this.hideTimer !== null) window.clearTimeout(this.hideTimer);
    this.hideTimer = null;
    const context = this.context;
    if (context) {
      for (const emitter of this.emitters) {
        try {
          emitter.noise.stop();
        } catch {
          /* already stopped */
        }
        try {
          emitter.hum.stop();
        } catch {
          /* already stopped */
        }
        try {
          emitter.secondary.stop();
        } catch {
          /* already stopped */
        }
      }
      for (const node of this.musicNodes) {
        if (node instanceof OscillatorNode || node instanceof AudioBufferSourceNode) {
          try {
            node.stop();
          } catch {
            /* already stopped */
          }
        }
      }
      void context.close();
    }
    this.context = null;
  }

  private createGraph(): void {
    const AudioContextCtor = window.AudioContext;
    if (!AudioContextCtor) return;
    const context = new AudioContextCtor();
    this.context = context;
    this.noiseBuffer = this.createNoiseBuffer(context, 4);

    const master = context.createGain();
    const musicBus = context.createGain();
    const cityBus = context.createGain();
    const ambienceBus = context.createGain();
    master.gain.value = 0;
    musicBus.gain.value = this.musicLevel;
    cityBus.gain.value = this.cityLevel;
    ambienceBus.gain.value = 1;
    musicBus.connect(master);
    cityBus.connect(master);
    ambienceBus.connect(cityBus);
    const limiter = context.createDynamicsCompressor();
    limiter.threshold.value = -8;
    limiter.knee.value = 0;
    limiter.ratio.value = 12;
    limiter.attack.value = 0.003;
    limiter.release.value = 0.18;
    master.connect(limiter).connect(context.destination);
    this.master = master;
    this.limiter = limiter;
    this.musicBus = musicBus;
    this.cityBus = cityBus;
    this.ambienceBus = ambienceBus;

    this.createAmbience(context, ambienceBus);
    this.createMusic(context, musicBus);
    this.createEmitters(context, cityBus);
  }

  private createAmbience(context: AudioContext, bus: AudioNode): void {
    const bedNoise = context.createBufferSource();
    bedNoise.buffer = this.noiseBuffer;
    bedNoise.loop = true;
    const bedFilter = context.createBiquadFilter();
    bedFilter.type = 'lowpass';
    bedFilter.frequency.value = 550;
    const bedGain = context.createGain();
    bedGain.gain.value = 0.07;
    bedNoise.connect(bedFilter).connect(bedGain).connect(bus);
    bedNoise.start();
    this.musicNodes.push(bedNoise);
    this.cityNoiseGain = bedGain;

    // Reuse the existing generated city noise as a short, spatial activity
    // texture when a street or building opens; events allocate no sources.
    const openingFilter = context.createBiquadFilter();
    openingFilter.type = 'bandpass';
    openingFilter.frequency.value = 620;
    openingFilter.Q.value = 0.48;
    const openingGain = context.createGain();
    openingGain.gain.value = 0;
    bedNoise.connect(openingFilter).connect(openingGain);
    this.openingGain = openingGain;

    const rainSource = context.createBufferSource();
    rainSource.buffer = this.noiseBuffer;
    rainSource.loop = true;
    const rainFilter = context.createBiquadFilter();
    rainFilter.type = 'highpass';
    rainFilter.frequency.value = 950;
    const rainTone = context.createBiquadFilter();
    rainTone.type = 'lowpass';
    rainTone.frequency.value = 7200;
    const rainGain = context.createGain();
    rainGain.gain.value = 0;
    rainSource.connect(rainFilter).connect(rainTone).connect(rainGain).connect(bus);
    rainSource.start();
    this.musicNodes.push(rainSource);
    this.rainGain = rainGain;

    const wind = context.createOscillator();
    wind.type = 'sine';
    wind.frequency.value = 0.08;
    const windGain = context.createGain();
    windGain.gain.value = 0.025;
    const windTone = context.createBiquadFilter();
    windTone.type = 'lowpass';
    windTone.frequency.value = 180;
    wind.connect(windGain).connect(windTone).connect(bus);
    wind.start();
    this.musicNodes.push(wind);
    this.windGain = windGain;

    // A separate, low, filtered pass through the same locally generated noise
    // bed gives the waterfront a soft wash without adding event-driven sources.
    const water = context.createBufferSource();
    water.buffer = this.noiseBuffer;
    water.loop = true;
    const waterFilter = context.createBiquadFilter();
    waterFilter.type = 'lowpass';
    waterFilter.frequency.value = 360;
    waterFilter.Q.value = 0.35;
    const waterGain = context.createGain();
    waterGain.gain.value = 0;
    water.connect(waterFilter).connect(waterGain).connect(bus);
    water.start();
    this.musicNodes.push(water);
    this.waterGain = waterGain;

    const pedestrianSource = context.createBufferSource();
    pedestrianSource.buffer = this.noiseBuffer;
    pedestrianSource.loop = true;
    const pedestrianFilter = context.createBiquadFilter();
    pedestrianFilter.type = 'bandpass';
    pedestrianFilter.frequency.value = 420;
    pedestrianFilter.Q.value = 0.42;
    const pedestrianGain = context.createGain();
    pedestrianGain.gain.value = 0;
    pedestrianSource.connect(pedestrianFilter).connect(pedestrianGain).connect(bus);
    pedestrianSource.start();
    this.musicNodes.push(pedestrianSource);
    this.pedestrianGain = pedestrianGain;

    // Fixed sine voices provide a quiet nonverbal station cue without creating
    // one-shot sources in response to an event stream.
    const cueFilter = context.createBiquadFilter();
    cueFilter.type = 'lowpass';
    cueFilter.frequency.value = 1_450;
    const cueGain = context.createGain();
    cueGain.gain.value = 0;
    cueFilter.connect(cueGain).connect(bus);
    for (const frequency of [659.25, 987.77]) {
      const cue = context.createOscillator();
      cue.type = 'sine';
      cue.frequency.value = frequency;
      cue.connect(cueFilter);
      cue.start();
      this.musicNodes.push(cue);
      this.cueOscillators.push(cue);
    }
    this.cueGain = cueGain;
  }

  private createMusic(context: AudioContext, bus: AudioNode): void {
    // D major / B minor palette, played slowly through soft triangle and sine voices.
    const chordVoices = [
      { osc: 146.83, gain: 0.025 },
      { osc: 185, gain: 0.019 },
      { osc: 220, gain: 0.017 },
      { osc: 293.66, gain: 0.009 },
    ];
    this.musicVoiceOscs = [];
    this.musicVoiceGains = [];
    for (let index = 0; index < chordVoices.length; index++) {
      const oscillator = context.createOscillator();
      oscillator.type = index === 3 ? 'sine' : 'triangle';
      oscillator.frequency.value = chordVoices[index].osc;
      const filter = context.createBiquadFilter();
      filter.type = 'lowpass';
      filter.frequency.value = 900;
      const gain = context.createGain();
      gain.gain.value = 0;
      oscillator.connect(filter).connect(gain).connect(bus);
      oscillator.start();
      this.musicVoiceOscs.push(oscillator);
      this.musicVoiceGains.push(gain);
      this.musicNodes.push(oscillator);
    }

    const bass = context.createOscillator();
    bass.type = 'triangle';
    bass.frequency.value = 73.42;
    const bassFilter = context.createBiquadFilter();
    bassFilter.type = 'lowpass';
    bassFilter.frequency.value = 190;
    const bassGain = context.createGain();
    bassGain.gain.value = 0;
    bass.connect(bassFilter).connect(bassGain).connect(bus);
    bass.start();
    this.musicNodes.push(bass);
    this.bassOsc = bass;
    this.bassGain = bassGain;

    const lead = context.createOscillator();
    lead.type = 'sine';
    lead.frequency.value = 440;
    const leadFilter = context.createBiquadFilter();
    leadFilter.type = 'lowpass';
    leadFilter.frequency.value = 1200;
    const leadGain = context.createGain();
    leadGain.gain.value = 0;
    lead.connect(leadFilter).connect(leadGain).connect(bus);
    lead.start();
    this.musicNodes.push(lead);
    this.leadOsc = lead;
    this.leadGain = leadGain;

    const kick = context.createOscillator();
    kick.type = 'sine';
    kick.frequency.value = 52;
    const kickGain = context.createGain();
    kickGain.gain.value = 0;
    kick.connect(kickGain).connect(bus);
    kick.start();
    this.musicNodes.push(kick);
    this.kickOsc = kick;
    this.kickGain = kickGain;

    const drumNoise = context.createBufferSource();
    drumNoise.buffer = this.noiseBuffer;
    drumNoise.loop = true;
    const snareFilter = context.createBiquadFilter();
    snareFilter.type = 'bandpass';
    snareFilter.frequency.value = 1900;
    snareFilter.Q.value = 0.7;
    const snareGain = context.createGain();
    snareGain.gain.value = 0;
    drumNoise.connect(snareFilter).connect(snareGain).connect(bus);

    const hatFilter = context.createBiquadFilter();
    hatFilter.type = 'highpass';
    hatFilter.frequency.value = 6500;
    const hatGain = context.createGain();
    hatGain.gain.value = 0;
    drumNoise.connect(hatFilter).connect(hatGain).connect(bus);
    drumNoise.start();
    this.musicNodes.push(drumNoise);
    this.snareGain = snareGain;
    this.hatGain = hatGain;
  }

  private createEmitters(context: AudioContext, bus: AudioNode): void {
    const buffer = this.noiseBuffer;
    if (!buffer) return;
    for (let i = 0; i < MAX_EMITTERS; i++) {
      const input = context.createGain();
      const pan = context.createStereoPanner();
      const outputGain = context.createGain();
      outputGain.gain.value = 0;
      const filter = context.createBiquadFilter();
      filter.type = 'lowpass';
      filter.frequency.value = 480;
      const noiseGain = context.createGain();
      noiseGain.gain.value = 0.28;
      const humGain = context.createGain();
      humGain.gain.value = 0.64;
      const secondaryGain = context.createGain();
      secondaryGain.gain.value = 0.15;
      const hum = context.createOscillator();
      hum.type = 'triangle';
      hum.frequency.value = 58;
      const secondary = context.createOscillator();
      secondary.type = 'sine';
      secondary.frequency.value = 90;
      const noise = context.createBufferSource();
      noise.buffer = buffer;
      noise.loop = true;
      const filteredNoise = context.createBiquadFilter();
      filteredNoise.type = 'bandpass';
      filteredNoise.frequency.value = 310;
      filteredNoise.Q.value = 0.7;

      noise.connect(filteredNoise).connect(noiseGain).connect(input);
      hum.connect(humGain).connect(input);
      secondary.connect(secondaryGain).connect(input);
      input.connect(filter).connect(pan).connect(outputGain).connect(bus);
      noise.start();
      hum.start();
      secondary.start();
      this.emitters.push({
        id: -1,
        kind: null,
        x: 0,
        z: 0,
        speed: 0,
        seen: false,
        distanceSq: Infinity,
        level: 0,
        pan,
        gain: outputGain,
        filter,
        noiseGain,
        humGain,
        secondaryGain,
        hum,
        secondary,
        noise,
      });
    }
  }

  private configureEmitter(emitter: Emitter): void {
    if (emitter.kind === 'train') {
      emitter.noiseGain.gain.value = 0.22;
      emitter.humGain.gain.value = 0.65;
      emitter.secondaryGain.gain.value = 0.16;
    } else if (emitter.kind === 'traffic') {
      emitter.noiseGain.gain.value = 0.26;
      emitter.humGain.gain.value = 0.32;
      emitter.secondaryGain.gain.value = 0.09;
    } else if (emitter.kind === 'bus') {
      emitter.noiseGain.gain.value = 0.32;
      emitter.humGain.gain.value = 0.48;
      emitter.secondaryGain.gain.value = 0.2;
    } else {
      emitter.noiseGain.gain.value = 0.33;
      emitter.humGain.gain.value = 0.13;
      emitter.secondaryGain.gain.value = 0.24;
    }
  }

  private scheduleMusic(): void {
    const context = this.context;
    if (!context || context.state !== 'running' || this.hidden) return;
    const bpm = 82;
    const stepLength = 60 / bpm / 4;
    const horizon = context.currentTime + 0.14;
    while (this.nextStep < horizon) {
      this.scheduleStep(this.nextStep, this.currentStep + 1);
      this.nextStep += stepLength;
      this.currentStep++;
    }
  }

  private scheduleStep(time: number, step: number): void {
    const context = this.context;
    if (!context) return;
    const stepInBar = step % 16;
    const bar = Math.floor(step / 16);
    if (stepInBar === 0) {
      const progression = [
        [D4, Fs4, A4, D5],
        [B3, D4, Fs4, A4],
        [G3, B3, D4, G4],
        [A3, Cs4, E4, A4],
      ];
      const chord = progression[Math.floor(bar / 2) % progression.length];
      const night = this.currentHour >= 22 || this.currentHour < 5;
      const dawn = this.currentHour >= 5 && this.currentHour < 9;
      for (let i = 0; i < this.musicVoiceOscs.length; i++) {
        const oscillator = this.musicVoiceOscs[i];
        const gain = this.musicVoiceGains[i].gain;
        oscillator.frequency.setTargetAtTime(chord[i], time, 0.09);
        gain.setTargetAtTime(
          (night ? 0.011 : dawn ? 0.021 : 0.026) * (i === 3 ? 0.48 : 1),
          time,
          0.25,
        );
      }
      this.setAudioParam(this.bassOsc?.frequency, chord[0] / 2, time, 0.12);
    }

    // Sparse lofi backbeat: soft kick and brushed noise, with open space between hits.
    if (stepInBar === 0 || stepInBar === 8)
      this.pulse(this.kickGain?.gain, time, stepInBar === 0 ? 0.11 : 0.075, 0.17);
    if (stepInBar === 4 || stepInBar === 12) this.pulse(this.snareGain?.gain, time, 0.035, 0.11);
    if (stepInBar === 2 || stepInBar === 6 || stepInBar === 10 || stepInBar === 14) {
      this.pulse(this.hatGain?.gain, time, 0.012, 0.035);
    }

    if (stepInBar === 0) this.pulse(this.bassGain?.gain, time, 0.035, 0.19);
    if (stepInBar === 8) this.pulse(this.bassGain?.gain, time, 0.027, 0.16);

    // A restrained, changing four-note answer gives the loop a little human shape.
    if (stepInBar === 6 || stepInBar === 14) {
      const motif = [A4, Fs4, E4, D4, Fs4, A4, B4, A4];
      const note = motif[(bar * 2 + (stepInBar === 14 ? 1 : 0)) % motif.length];
      this.setAudioParam(this.leadOsc?.frequency, note, time, 0.04);
      this.pulse(
        this.leadGain?.gain,
        time,
        this.currentHour > 21 || this.currentHour < 6 ? 0.006 : 0.011,
        0.28,
      );
    }
  }

  private pulse(param: AudioParam | undefined, time: number, peak: number, release: number): void {
    if (!param) return;
    param.cancelScheduledValues(time);
    param.setValueAtTime(Math.max(0.0001, param.value * 0.25), time);
    param.linearRampToValueAtTime(peak, time + 0.012);
    param.exponentialRampToValueAtTime(0.0001, time + release);
  }

  private applyLevels(seconds: number): void {
    if (!this.context) return;
    this.setParam(this.musicBus?.gain, this.musicLevel, seconds);
    this.setParam(this.cityBus?.gain, this.cityLevel, seconds);
    this.setParam(this.master?.gain, this.hidden ? 0 : 1, seconds);
  }

  private updateEmitterLevels(seconds: number): void {
    const pausedScale = this.paused || this.hidden ? 0 : 1;
    let activeMix = 0;
    for (const emitter of this.emitters) if (emitter.seen) activeMix += emitter.level;
    const mixScale = activeMix > MAX_EMITTER_MIX ? MAX_EMITTER_MIX / activeMix : 1;
    for (const emitter of this.emitters) {
      const target = emitter.seen ? emitter.level * mixScale * pausedScale : 0;
      this.setParam(emitter.gain.gain, target, seconds);
    }
  }

  private updateEventCues(world: AudioWorld): void {
    const tickRewound =
      Number.isFinite(world.tick) &&
      this.lastWorldTick !== null &&
      world.tick! < this.lastWorldTick;
    const changedWorld = !this.hasWorldKey || world.worldKey !== this.worldKey || tickRewound;
    if (changedWorld) {
      this.hasWorldKey = true;
      this.worldKey = world.worldKey;
      this.lastWorldTick = Number.isFinite(world.tick) ? world.tick! : null;
      this.seenEventIds.clear();
      this.eventIdOrder.length = 0;
      this.lastBusCueTimes.clear();
      for (const event of world.events ?? []) this.rememberEvent(event.id);
      return; // A loaded or newly-created world's existing event list is history.
    }

    const tick = world.tick;
    if (!Number.isFinite(tick)) return;
    const previousTick = this.lastWorldTick;
    this.lastWorldTick = tick!;
    if (previousTick === null) return;

    const canCue =
      !this.paused &&
      !this.hidden &&
      (world.developmentRate ?? 1) <= 1 &&
      (world.playbackRate ?? 1) <= 1;
    const now = this.context?.currentTime ?? 0;
    let selected: CueEvent | null = null;
    for (const event of world.events ?? []) {
      if (this.seenEventIds.has(event.id)) continue;
      this.rememberEvent(event.id);
      // Event publication can lag behind movement snapshots. Keep a short
      // two-second tick window while still rejecting history and future ticks.
      if (
        selected ||
        event.tick < tick! - 40 ||
        event.tick > tick! ||
        !isCueEvent(event) ||
        !canCue
      )
        continue;
      if (now - this.lastCueTime < 2.2) continue;
      if (event.kind === 'BusStop') {
        const lastBusCue = this.lastBusCueTimes.get(event.entity_id);
        if (lastBusCue !== undefined && now - lastBusCue < BUS_CUE_COOLDOWN_SECONDS) continue;
      } else if (isOpeningEvent(event) && !this.isOpeningCueInRange(world, event)) {
        continue;
      }
      selected = event;
    }
    if (!selected) return;
    if (selected.kind === 'BusStop') {
      this.playBusCue(now);
      this.lastBusCueTimes.delete(selected.entity_id);
      this.lastBusCueTimes.set(selected.entity_id, now);
      if (this.lastBusCueTimes.size > 64)
        this.lastBusCueTimes.delete(this.lastBusCueTimes.keys().next().value!);
    } else if (selected.kind === 'TrainStationStop') {
      this.playStationCue(now);
    } else if (isOpeningEvent(selected)) {
      this.playOpeningCue(world, selected, now);
    }
    this.lastCueTime = now;
  }

  private isOpeningCueInRange(world: AudioWorld, event: OpeningEvent): boolean {
    if (!Number.isFinite(event.x) || !Number.isFinite(event.z)) return false;
    const dx = event.x! - world.camera.x;
    const dz = event.z! - world.camera.z;
    const dy = Math.max(0, world.camera.y);
    return Math.hypot(dx, dy, dz) <= 190;
  }

  private playOpeningCue(world: AudioWorld, event: OpeningEvent, time: number): void {
    const gain = this.openingGain?.gain;
    const panner = this.getOpeningPanner()?.pan;
    if (!gain || !panner || !Number.isFinite(event.x) || !Number.isFinite(event.z)) return;
    const dx = event.x! - world.camera.x;
    const dz = event.z! - world.camera.z;
    const dy = Math.max(0, world.camera.y);
    const distance = Math.hypot(dx, dy, dz);
    const attenuation = 1 / (1 + distance * 0.075 + distance * distance * 0.0011);
    const forward = world.camera.forward;
    const forwardLength = forward ? Math.hypot(forward.x, forward.z) : 0;
    const forwardX = forwardLength > 1e-4 ? forward!.x / forwardLength : 0;
    const forwardZ = forwardLength > 1e-4 ? forward!.z / forwardLength : -1;
    const pan = clamp((dx * -forwardZ + dz * forwardX) / 32, -1, 1);

    panner.setTargetAtTime(pan, time, 0.08);
    gain.cancelScheduledValues(time);
    gain.setValueAtTime(0.00001, time);
    // A muted, filtered city-texture swell reads as nearby activity, not a
    // success jingle or a new sound source for every event.
    gain.linearRampToValueAtTime(0.0035 * attenuation, time + 0.055);
    gain.exponentialRampToValueAtTime(0.00001, time + 0.46);
  }

  private getOpeningPanner(): StereoPannerNode | null {
    if (this.openingPanner) return this.openingPanner;
    const context = this.context;
    if (!context || !this.openingGain || !this.ambienceBus) return null;
    // One lazily created node per enabled graph keeps the idle graph's source
    // inventory unchanged; all later openings reuse this same spatial path.
    const panner = context.createStereoPanner();
    this.openingGain.connect(panner).connect(this.ambienceBus);
    this.openingPanner = panner;
    return panner;
  }

  private rememberEvent(id: number): void {
    if (this.seenEventIds.has(id)) return;
    this.seenEventIds.add(id);
    this.eventIdOrder.push(id);
    if (this.eventIdOrder.length > 128) {
      const expired = this.eventIdOrder.shift();
      if (expired !== undefined) this.seenEventIds.delete(expired);
    }
  }

  private playStationCue(time: number): void {
    const gain = this.cueGain?.gain;
    if (!gain) return;
    this.setAudioParam(this.cueOscillators[0]?.frequency, 659.25, time, 0.05);
    this.setAudioParam(this.cueOscillators[1]?.frequency, 987.77, time, 0.05);
    gain.cancelScheduledValues(time);
    gain.setValueAtTime(0.0001, time);
    gain.linearRampToValueAtTime(0.008, time + 0.035);
    gain.exponentialRampToValueAtTime(0.0001, time + 0.62);
  }

  private playBusCue(time: number): void {
    const gain = this.cueGain?.gain;
    if (!gain) return;
    // A low, muted door chime shares the preallocated cue voices; no event can
    // create more audio sources or queue a backlog of one-shots.
    this.setAudioParam(this.cueOscillators[0]?.frequency, 392, time, 0.06);
    this.setAudioParam(this.cueOscillators[1]?.frequency, 523.25, time, 0.06);
    gain.cancelScheduledValues(time);
    gain.setValueAtTime(0.0001, time);
    gain.linearRampToValueAtTime(0.0055, time + 0.04);
    gain.exponentialRampToValueAtTime(0.0001, time + 0.72);
  }

  private setParam(param: AudioParam | undefined, value: number, seconds: number): void {
    if (!param || !this.context) return;
    param.setTargetAtTime(value, this.context.currentTime, Math.max(0.01, seconds));
  }

  private setAudioParam(
    param: AudioParam | undefined,
    value: number,
    time: number,
    smoothing: number,
  ): void {
    if (!param) return;
    param.setTargetAtTime(value, time, smoothing);
  }

  private createNoiseBuffer(context: AudioContext, duration: number): AudioBuffer {
    const buffer = context.createBuffer(
      1,
      Math.ceil(context.sampleRate * duration),
      context.sampleRate,
    );
    const samples = buffer.getChannelData(0);
    let last = 0;
    // Slightly brown the noise so the city bed feels soft under the music.
    for (let i = 0; i < samples.length; i++) {
      const white = Math.random() * 2 - 1;
      last = (last + 0.035 * white) / 1.035;
      samples[i] = last * 3.2;
    }
    return buffer;
  }
}

const D4 = 293.66;
const Fs4 = 369.99;
const A4 = 440;
const D5 = 587.33;
const B3 = 246.94;
const G3 = 196;
const G4 = 392;
const A3 = 220;
const Cs4 = 277.18;
const E4 = 329.63;
const B4 = 493.88;
