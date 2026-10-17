import {
  DEFAULT_TILT_SHIFT,
  normalizeTiltShift,
  type TiltShiftSettings,
} from './optics/tilt-shift';
import React, { Suspense, lazy, useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import Controls, { busLineLabel } from './ui/Controls';
import './ui/style.css';
import './base.css';
import { CameraDirector } from './camera/director';
import { audioSceneContext } from './app/audio-scene';
import { CityRenderer, type RenderMetrics } from './render/city';
import { SimulationClient } from './worker/client';
import { AudioDirector } from './audio/director';
import {
  saveWorld,
  loadWorld,
  exportWorld,
  loadViewerPreferences,
  saveViewerPreferences,
  validateWorldSave,
} from './storage/saves';
import { resolveStartupWorld } from './storage/recovery';
import { createGraphicsStartupWatchdog } from './graphics-startup';
import type { LensPresetId } from './optics/lens';
import type { World, Style, Metrics } from './types';

type Subject = {
  id: number;
  name: string;
  kind: string;
  activity: string;
  destination?: string;
  history?: string[];
  passengers?: number;
  capacity?: number;
  station_waiting?: number;
  gateway_waiting?: number;
  line?: string;
  nextStop?: string;
  dwelling?: boolean;
  waiting?: number;
};
function inspectedSubject(id: number, data: unknown, world: World | null): Subject | null {
  const info = data as {
    entityType?: string;
    name?: string;
    activity?: string;
    kind?: string;
    destination?: number;
    history?: string[];
    passengers?: number;
    capacity?: number;
    station_waiting?: number;
    gateway_waiting?: number;
    line?: string | number;
    line_id?: number;
    next_stop?: number | null;
    dwelling?: boolean;
    waiting?: number;
  };
  if (info.entityType === 'unknown') return null;
  const bus = info.entityType === 'bus' || info.kind === 'bus';
  const snapshotBus = bus ? world?.buses?.find((entity) => entity.id === id) : undefined;
  const lineId =
    info.line_id ?? snapshotBus?.line_id ?? (typeof info.line === 'number' ? info.line : undefined);
  const nextStopId = info.next_stop ?? snapshotBus?.next_stop ?? undefined;
  const destinationId = bus ? nextStopId : info.destination;
  const node = world?.nodes.find((n) => n.id === destinationId);
  const lineLabel = busLineLabel(
    world,
    lineId,
    typeof info.line === 'string' ? info.line : undefined,
  );
  const dwelling = info.dwelling ?? snapshotBus?.dwelling;
  return {
    id,
    name: info.name ?? (bus ? 'Neighborhood Bus' : 'A city traveler'),
    kind: bus ? 'bus' : info.kind === 'rail' ? 'train' : info.kind === 'car' ? 'vehicle' : 'person',
    activity: (
      info.activity ?? (bus ? (dwelling ? 'At stop' : 'In service') : 'On a journey')
    ).replaceAll('_', ' '),
    destination: bus
      ? undefined
      : (world?.buildings.find((b) => b.id === destinationId)?.name ??
        node?.kind.replaceAll('_', ' ')),
    history: info.history?.slice(-3),
    passengers: info.passengers ?? snapshotBus?.passengers,
    capacity: info.capacity ?? snapshotBus?.capacity,
    station_waiting: info.station_waiting,
    gateway_waiting: info.gateway_waiting,
    line: bus ? lineLabel : undefined,
    nextStop: node?.kind.replaceAll('_', ' ') ?? undefined,
    dwelling: bus ? dwelling : undefined,
    waiting: bus ? (info.waiting ?? snapshotBus?.waiting) : undefined,
  };
}

// Development-only review tooling loads on demand and never ships in production builds.
const ArchitectureReviewRoute = import.meta.env.DEV
  ? lazy(() => import('./dev/architecture-review'))
  : () => null;

function App() {
  if (
    import.meta.env.DEV &&
    new URLSearchParams(window.location.search).get('review') === 'architecture'
  )
    return (
      <Suspense fallback={null}>
        <ArchitectureReviewRoute />
      </Suspense>
    );
  return <CityExperience />;
}

function CityExperience() {
  const mount = useRef<HTMLDivElement>(null),
    runtime = useRef<{
      renderer: CityRenderer;
      director: CameraDirector;
      client: SimulationClient;
      audio: AudioDirector;
    } | null>(null);
  const [world, setWorld] = useState<World | null>(null),
    [ready, setReady] = useState(false),
    [error, setError] = useState<string | null>(null),
    [message, setMessage] = useState<string | null>(null);
  const [preferences, setPreferences] = useState(loadViewerPreferences);
  const [paused, setPaused] = useState(false),
    [speed, setSpeed] = useState(1),
    [style, setStyle] = useState<Style>(preferences.style),
    [mode, setMode] = useState<'watch' | 'explore' | 'overview'>('watch');
  const [preparingStyle, setPreparingStyle] = useState<Style | null>(null);
  const [fixedReviewQuality, setFixedReviewQuality] = useState(false);
  const [tiltShift, setTiltShift] = useState<TiltShiftSettings>({ ...DEFAULT_TILT_SHIFT });
  const [preview, setPreview] = useState<number | null>(null),
    [sound, setSound] = useState(false),
    [musicLevel, setMusicLevel] = useState(preferences.musicLevel),
    [cityLevel, setCityLevel] = useState(preferences.cityLevel),
    [focus, setFocus] = useState(preferences.focus),
    [development, setDevelopment] = useState(1),
    [quality, setQuality] = useState(preferences.quality),
    [contrast, setContrast] = useState(preferences.contrast),
    [lens, setLens] = useState<LensPresetId>(preferences.lens);
  const [subject, setSubject] = useState<Subject | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null),
    [metrics, setMetrics] = useState<RenderMetrics | null>(null),
    [sceneLabel, setSceneLabel] = useState('Along the river');
  const [simulationMetrics, setSimulationMetrics] = useState<Metrics | null>(null);
  const preferencesRef = useRef(preferences);
  const styleSwitch = useRef(0);
  const startupOverridden = useRef(false);
  const currentWorld = useRef<World | null>(null),
    saveBusy = useRef(false),
    subjectRef = useRef<number | null>(null);
  subjectRef.current = subject?.id ?? null;
  function notice(text: string) {
    setMessage(text);
    window.setTimeout(() => setMessage(null), 5000);
  }
  function storePreferences(patch: Partial<typeof preferences>) {
    const next = { ...preferencesRef.current, ...patch };
    preferencesRef.current = next;
    setPreferences(next);
    saveViewerPreferences(next);
  }
  useEffect(() => {
    if (!mount.current) return;
    let renderer: CityRenderer;
    let director!: CameraDirector;
    try {
      // Composition root: the renderer gets a camera rig, never the camera module itself.
      renderer = new CityRenderer(
        mount.current,
        (camera, element) => (director = new CameraDirector(camera, element)),
        new URLSearchParams(location.search).has('webgl'),
      );
    } catch (e) {
      setError(
        `Graphics could not start: ${e instanceof Error ? e.message : String(e)}. Try the WebGL fallback.`,
      );
      return;
    }
    const client = new SimulationClient(),
      audio = new AudioDirector();
    runtime.current = { renderer, director, client, audio };
    const recoveryRequested = new URLSearchParams(location.search).get('recover') === '1';
    renderer.setLens(lens, false);
    renderer.setTiltShift(tiltShift);
    renderer.setFocus(focus);
    renderer.setQuality(recoveryRequested ? 'balanced' : quality);
    audio.setLevels(musicLevel, cityLevel);
    if (recoveryRequested) setQuality('balanced');
    let disposed = false,
      graphicsReady = false,
      simReady = false,
      latestMetrics: Metrics | null = null,
      audioEpoch = 0,
      recoveryInitPending = false;
    const graphicsStartup = createGraphicsStartupWatchdog(setError);
    const isReady = () => {
      if (graphicsReady && simReady && !disposed) setReady(true);
    };
    renderer.onPick = (id, kind) => {
      if (kind === 'building') {
        setSelectedId(id);
        setSubject(null);
        renderer.setSelected(id);
      } else {
        setSelectedId(null);
        renderer.setSelected(null);
        void client
          .inspect(id)
          .then((data) => {
            if (!disposed) setSubject(inspectedSubject(id, data, currentWorld.current));
          })
          .catch(() => {});
      }
    };
    director.onModeChange = setMode;
    renderer.onMetrics = setMetrics;
    renderer.onError = (error) => {
      if (disposed) return;
      graphicsStartup.complete();
      graphicsReady = false;
      setReady(false);
      styleSwitch.current++;
      setPreparingStyle(null);
      setError(`Graphics stopped: ${error.message}. Try Retry or the WebGL fallback.`);
    };
    client.onError((e) => {
      if (!disposed) setError(e.message);
    });
    client.subscribeWorld((w) => {
      if (disposed) return;
      const replaced = audioEpoch !== client.worldEpoch;
      if (replaced) {
        audioEpoch = client.worldEpoch;
        setDevelopment(w.development_rate);
      }
      currentWorld.current = w;
      renderer.setWorld(w, replaced);
      setWorld(w);
      simReady = true;
      if (recoveryInitPending) {
        const url = new URL(window.location.href);
        url.searchParams.delete('recover');
        window.history.replaceState(null, '', `${url.pathname}${url.search}${url.hash}`);
        recoveryInitPending = false;
      }
      isReady();
    });
    client.subscribeMovement((data, tick, minute, day, trainDoors, support) => {
      renderer.actors.setMovement(data, tick, minute, trainDoors, support);
      if (currentWorld.current) {
        currentWorld.current.tick = tick;
        currentWorld.current.minute_of_day = minute;
        currentWorld.current.day = day;
      }
    });
    client.subscribeMetrics((m) => {
      latestMetrics = m;
      if (!disposed) setSimulationMetrics(m);
    });
    renderer
      .start(style)
      .then(() => {
        if (disposed) return;
        graphicsStartup.complete();
        graphicsReady = true;
        isReady();
      })
      .catch((e) => {
        graphicsStartup.complete();
        if (!disposed) {
          styleSwitch.current++;
          setPreparingStyle(null);
          setError(
            `Graphics could not start: ${e instanceof Error ? e.message : String(e)}. Try the WebGL fallback.`,
          );
        }
      });
    void resolveStartupWorld(location.search, 87231, loadWorld)
      .then((startup) => {
        if (disposed || startupOverridden.current) return;
        if (startup.kind === 'recovered') {
          setQuality('balanced');
          recoveryInitPending = true;
          client.init(startup.seed, startup.data);
        } else client.init(startup.seed);
        client.controls({ hidden: document.hidden });
      })
      .catch((e) => {
        if (!disposed && !startupOverridden.current)
          setError(
            `Could not restore the saved city after graphics failure: ${e instanceof Error ? e.message : String(e)}`,
          );
      });
    let lastAudio = performance.now(),
      lastInspect = 0;
    const pulse = window.setInterval(() => {
      const now = performance.now();
      audio.update(
        {
          ...audioSceneContext(renderer.presentationView()),
          worldKey: audioEpoch,
          playbackRate: latestMetrics?.achievableSpeed ?? 1,
        },
        Math.min(0.25, (now - lastAudio) / 1000),
      );
      lastAudio = now;
      setSceneLabel(director.label);
      if (currentWorld.current) setWorld({ ...currentWorld.current });
      const id = subjectRef.current;
      if (id !== null && now - lastInspect > 5000) {
        lastInspect = now;
        void client
          .inspect(id)
          .then((data) => {
            if (!disposed)
              setSubject((previous) =>
                previous?.id === id ? inspectedSubject(id, data, currentWorld.current) : previous,
              );
          })
          .catch(() => {});
      }
    }, 250);
    const checkpoint = async () => {
      if (!simReady || saveBusy.current) return;
      saveBusy.current = true;
      try {
        const data = await client.save();
        const seed = currentWorld.current?.seed;
        if (seed !== undefined) await saveWorld(data, seed);
      } catch (e) {
        if (!disposed) setError(`Could not save: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        saveBusy.current = false;
      }
    };
    const autosave = window.setInterval(() => void checkpoint(), 60000);
    const visibility = () => {
      client.controls({ hidden: document.hidden });
      audio.setHidden(document.hidden);
      if (document.hidden) void checkpoint();
    };
    document.addEventListener('visibilitychange', visibility);
    if (import.meta.env.DEV) {
      Object.assign(window, {
        __city: {
          renderer,
          client,
          get world() {
            return currentWorld.current;
          },
          get simMetrics() {
            return latestMetrics;
          },
          save: () => client.save(),
          export: async () => exportWorld(await client.save()),
        },
      });
    }
    return () => {
      disposed = true;
      styleSwitch.current++;
      setPreparingStyle(null);
      graphicsStartup.cleanup();
      clearInterval(pulse);
      clearInterval(autosave);
      document.removeEventListener('visibilitychange', visibility);
      client.dispose();
      audio.dispose();
      renderer.dispose();
      runtime.current = null;
    };
  }, []);
  async function save() {
    try {
      const r = runtime.current;
      if (!r || !currentWorld.current) return;
      const data = await r.client.save();
      await saveWorld(data, currentWorld.current.seed);
      notice('Your city is saved on this device.');
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    }
  }
  async function recoverGraphics(useWebGL: boolean) {
    const r = runtime.current,
      current = currentWorld.current;
    if (!r) {
      // The renderer constructor can fail before a worker exists; in that
      // case there is no live engine state to checkpoint. Keep the seed and
      // any pending recovery marker while switching/retrying backends.
      const url = new URL(window.location.href);
      if (useWebGL) url.searchParams.set('webgl', '1');
      else url.searchParams.delete('webgl');
      window.location.assign(url.toString());
      return;
    }
    if (!current) {
      setError(
        'Graphics recovery cannot checkpoint a city that has not finished opening. The latest saved city is unchanged.',
      );
      return;
    }
    try {
      const data = await r.client.save();
      await saveWorld(data, current.seed);
      const url = new URL(window.location.href);
      url.searchParams.set('recover', '1');
      if (useWebGL) url.searchParams.set('webgl', '1');
      else url.searchParams.delete('webgl');
      window.location.assign(url.toString());
    } catch (e) {
      setError(
        `Graphics recovery could not checkpoint this city. The latest saved city is unchanged. ${e instanceof Error ? e.message : String(e)}`,
      );
    }
  }
  async function exportCity() {
    try {
      const r = runtime.current;
      if (!r) return;
      exportWorld(await r.client.save());
      notice('Your city checkpoint is ready to keep.');
    } catch (e) {
      setError(`Could not export: ${String(e)}`);
    }
  }
  function resetWorldSelection(r: { renderer: CityRenderer; director: CameraDirector }) {
    r.director.setMode('watch');
    r.renderer.setSelected(null);
    setMode('watch');
    setSelectedId(null);
    setSubject(null);
  }
  async function importCity(file: File) {
    try {
      if (file.size > 16 * 1024 * 1024) throw new Error('Checkpoint exceeds the 16 MiB limit.');
      const data = await file.text();
      const checkpoint = validateWorldSave(data);
      const r = runtime.current;
      if (!r) throw new Error('The city is not ready to import a checkpoint.');
      if ((await save()) !== true) return;
      resetWorldSelection(r);
      r.client.init(checkpoint.seed, checkpoint.data);
      notice('Your current city is saved while the imported checkpoint opens.');
    } catch (e) {
      setError(`Could not import city: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  async function reviewMature() {
    try {
      const response = await fetch(
        '/tests/fixtures/validation/early-reservation-wasm-87231-save.json',
        {
          cache: 'no-store',
        },
      );
      if (!response.ok) throw new Error('The local review fixture is unavailable.');
      await importCity(
        new File([await response.text()], 'early-reservation-wasm-87231-save.json', {
          type: 'application/json',
        }),
      );
    } catch (e) {
      setError(`Could not open the review city: ${String(e)}`);
    }
  }
  async function resume() {
    try {
      const saved = await loadWorld();
      if (!saved) {
        notice('There is no saved city on this device yet.');
        return;
      }
      const r = runtime.current;
      if (r) {
        startupOverridden.current = true;
        resetWorldSelection(r);
        r.client.init(saved.seed, saved.data);
      }
      notice('Your city continues where you left it.');
    } catch (e) {
      setError(String(e));
    }
  }
  async function newWorld(seed: number) {
    try {
      if (currentWorld.current && !(await save())) return;
      const r = runtime.current;
      if (!r)
        throw new Error(
          'The graphics renderer is not available yet. Try the graphics recovery option.',
        );
      startupOverridden.current = true;
      const url = new URL(window.location.href);
      url.searchParams.delete('recover');
      window.history.replaceState(null, '', `${url.pathname}${url.search}${url.hash}`);
      resetWorldSelection(r);
      setError(null);
      r.client.init(seed);
      notice('A new beginning, beside a different river.');
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <main className="city-app">
      <div
        className="city-canvas"
        ref={mount}
        aria-label="Living three-dimensional Tokyo city. Drag to explore; use arrow keys to pan and plus or minus to zoom."
      />
      <Controls
        achievableSpeed={simulationMetrics?.achievableSpeed}
        world={world}
        ready={ready}
        error={error}
        message={message}
        paused={paused}
        speed={speed}
        style={style}
        preparingStyle={preparingStyle}
        mode={mode}
        sceneLabel={sceneLabel}
        preview={preview}
        sound={sound}
        musicLevel={musicLevel}
        cityLevel={cityLevel}
        focus={focus}
        tiltShift={tiltShift}
        onTiltShift={(patch) => {
          const value = normalizeTiltShift({ ...tiltShift, ...patch });
          setTiltShift(value);
          runtime.current?.renderer.setTiltShift(value);
          if (patch.enabled === true && !focus) {
            setFocus(true);
            storePreferences({ focus: true });
            runtime.current?.renderer.setFocus(true);
          }
        }}
        contrast={contrast}
        lens={lens}
        selected={world?.buildings.find((b) => b.id === selectedId) ?? null}
        metrics={metrics}
        subject={subject}
        development={development}
        quality={quality}
        onDismissError={() => setError(null)}
        onDevelopment={(value) => {
          setDevelopment(value);
          runtime.current?.client.controls({ development: value });
        }}
        onQuality={(value) => {
          setQuality(value);
          storePreferences({ quality: value });
          runtime.current?.renderer.setQuality(value);
        }}
        fixedReviewQuality={fixedReviewQuality}
        onFixedReviewQuality={
          import.meta.env.DEV
            ? (value) => {
                runtime.current?.renderer.setFixedReviewQuality(value);
                setFixedReviewQuality(value);
              }
            : undefined
        }
        onPause={() => {
          const value = !paused;
          setPaused(value);
          runtime.current?.client.controls({ paused: value });
          runtime.current?.audio.setPaused(value);
        }}
        onSpeed={(value) => {
          setSpeed(value);
          runtime.current?.client.controls({ speed: value });
        }}
        onStyle={(value) => {
          const request = ++styleSwitch.current;
          const r = runtime.current;
          if (!r) {
            setStyle(value);
            setPreparingStyle(null);
            storePreferences({ style: value });
            return;
          }
          setPreparingStyle(value);
          void Promise.resolve()
            .then(() => r.renderer.setStyle(value))
            .then(() => {
              if (request !== styleSwitch.current) return;
              setStyle(r.renderer.style);
              setPreparingStyle(null);
              storePreferences({ style: r.renderer.style });
            })
            .catch((e) => {
              if (request !== styleSwitch.current) return;
              setPreparingStyle(null);
              setStyle(r.renderer.style);
              setError(`Could not prepare this rendering style: ${String(e)}`);
            });
        }}
        onMode={(value) => {
          setMode(value);
          runtime.current?.director.setMode(value);
        }}
        onPreview={(value) => {
          setPreview(value);
          runtime.current?.renderer.setPreview(value);
        }}
        onSound={() => {
          const r = runtime.current;
          if (!r) return;
          if (!sound) {
            r.audio
              .enable()
              .then(() => {
                r.audio.setLevels(musicLevel, cityLevel);
                setSound(true);
              })
              .catch((e) => setError(String(e)));
          } else {
            r.audio.setLevels(0, 0);
            setSound(false);
          }
        }}
        onLevels={(m, c) => {
          setMusicLevel(m);
          setCityLevel(c);
          storePreferences({ musicLevel: m, cityLevel: c });
          if (sound) runtime.current?.audio.setLevels(m, c);
        }}
        onFocus={() => {
          const value = !focus;
          setFocus(value);
          storePreferences({ focus: value });
          runtime.current?.renderer.setFocus(value);
        }}
        onContrast={(value) => {
          setContrast(value);
          storePreferences({ contrast: value });
        }}
        onLens={(value) => {
          setLens(value);
          storePreferences({ lens: value });
          runtime.current?.renderer.setLens(value, true);
        }}
        onGraphicsRetry={() =>
          void recoverGraphics(new URLSearchParams(window.location.search).has('webgl'))
        }
        onGraphicsFallback={() =>
          void recoverGraphics(!new URLSearchParams(window.location.search).has('webgl'))
        }
        onReviewMature={import.meta.env.DEV ? () => void reviewMature() : undefined}
        onSave={() => void save()}
        onExport={() => void exportCity()}
        onImport={(file) => void importCity(file)}
        onResume={() => void resume()}
        onNewWorld={(seed) => void newWorld(seed)}
        onReviewSkyline={
          import.meta.env.DEV
            ? () => {
                if (!runtime.current?.director.reviewSkyline())
                  notice('This city has no completed tall-building scene yet.');
              }
            : undefined
        }
        onReviewBuilding={
          import.meta.env.DEV
            ? (id) => {
                if (!runtime.current?.director.reviewBuilding(id))
                  notice('This building is not available to frame.');
              }
            : undefined
        }
        onReviewStation={
          import.meta.env.DEV
            ? () => {
                const station = currentWorld.current?.nodes.find((n) => n.kind === 'station');
                if (!station || !runtime.current?.director.reviewStation(station.id))
                  notice('This station is not available to frame.');
              }
            : undefined
        }
        onReviewWaterReflection={
          import.meta.env.DEV
            ? () => runtime.current?.renderer.toggleWaterReflectionForReview() ?? false
            : undefined
        }
        onReviewStreetLights={
          import.meta.env.DEV
            ? () => runtime.current?.renderer.toggleStreetLightsForReview() ?? false
            : undefined
        }
        onApplyCameraPose={
          import.meta.env.DEV
            ? (position, target) =>
                runtime.current?.director.reviewPose(
                  { x: position[0], y: position[1], z: position[2] },
                  { x: target[0], y: target[1], z: target[2] },
                ) ?? false
            : undefined
        }
        onReadCameraPose={
          import.meta.env.DEV
            ? () => {
                const director = runtime.current?.director;
                if (!director) return 'Camera unavailable';
                const bounds = director.element.getBoundingClientRect();
                return JSON.stringify(
                  {
                    seed: currentWorld.current?.seed,
                    tick: currentWorld.current?.tick,
                    position: director.camera.position.toArray(),
                    target: director.target.toArray(),
                    lens: director.lens,
                    fov: director.camera.fov,
                    aspect: director.camera.aspect,
                    viewport: [bounds.width, bounds.height],
                    style,
                    previewHour: preview,
                    renderQuality: runtime.current?.renderer.reviewQualitySnapshot(),
                  },
                  null,
                  2,
                );
              }
            : undefined
        }
        onReviewWaterfront={
          import.meta.env.DEV
            ? (preset = 'overhead') => {
                if (!runtime.current?.director.reviewWaterfront(preset))
                  notice('This city has no river view yet.');
              }
            : undefined
        }
        onCloseInspector={() => {
          setSelectedId(null);
          setSubject(null);
          runtime.current?.renderer.setSelected(null);
          runtime.current?.director.follow(null);
        }}
        onFollow={(id) => {
          const r = runtime.current;
          if (!r) return;
          r.director.follow(id);
          if (!currentWorld.current?.buildings.some((b) => b.id === id)) {
            void r.client
              .inspect(id)
              .then((data) => {
                setSubject(inspectedSubject(id, data, currentWorld.current));
                setSelectedId(null);
              })
              .catch(() => {});
          }
        }}
      />
    </main>
  );
}
createRoot(document.getElementById('root')!).render(<App />);
