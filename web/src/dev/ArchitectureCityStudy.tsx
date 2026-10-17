import { CameraDirector } from '../camera/director';
import { useEffect, useRef, useState } from 'react';
import { CityRenderer, type RenderMetrics } from '../render/city';
import { architectureCityStudy } from '../render/architecture/city-study';
import { type ApartmentEra } from '../render/architecture/era-preference';
import type { Style } from '../types';
let matureSnapshot: Promise<import('../types').World> | null = null;
function frozenMatureWorld(): Promise<import('../types').World> {
  return (matureSnapshot ??= (async () => {
    const url = new URL('/wasm/city_engine.js', location.origin).href,
      wasm = await import(/* @vite-ignore */ url);
    await wasm.default();
    const response = await fetch('/tests/fixtures/access-joins/mature-world.json');
    if (!response.ok)
      throw Error('Run npm run setup:fixtures to load the pinned mature-city review.');
    const engine = wasm.CityEngine.restore(await response.text());
    try {
      return JSON.parse(engine.snapshot());
    } finally {
      engine.free();
    }
  })());
}
export function ArchitectureCityStudy() {
  const host = useRef<HTMLDivElement>(null),
    viewer = useRef<CityRenderer | null>(null);
  const [metrics, setMetrics] = useState<RenderMetrics | null>(null),
    [error, setError] = useState(''),
    [style, setStyle] = useState<Style>('miniature');
  const [mature, setMature] = useState(
    new URLSearchParams(location.search).get('family') === 'mature',
  );
  const [towers, setTowers] = useState(
    new URLSearchParams(location.search).get('family') === 'towers',
  );
  const [era, setEra] = useState<ApartmentEra>(towers ? 'contemporary' : 'mixed');
  const [hour, setHour] = useState(14),
    [rain, setRain] = useState(false);
  const [seed, setSeed] = useState(32),
    [stage, setStage] = useState<import('../types').Building['stage']>('open');
  useEffect(() => {
    if (!host.current) return;
    const renderer = new CityRenderer(
      host.current,
      (camera, element) => new CameraDirector(camera, element),
      new URLSearchParams(location.search).has('webgl'),
    );
    viewer.current = renderer;
    renderer.onMetrics = setMetrics;
    renderer.onError = (e) => setError(e.message);
    let stopped = false;
    void renderer
      .start('miniature')
      .then(async () => {
        const initial = mature ? await frozenMatureWorld() : architectureCityStudy(seed, towers);
        if (stopped) return;
        renderer.setApartmentEra(era);
        renderer.setWorld(initial);
        renderer.setPreview(14);
        renderer.setFocus(false);
        (renderer.rig as CameraDirector).reviewPose(
          towers ? { x: 240, y: 180, z: 260 } : { x: 105, y: 105, z: 115 },
          { x: 0, y: towers ? 40 : 12, z: 0 },
        );
      })
      .catch((e) => {
        if (!stopped) setError(String(e));
      });
    return () => {
      stopped = true;
      viewer.current = null;
      renderer.dispose();
    };
  }, []);
  useEffect(() => {
    const r = viewer.current;
    if (!r?.worldState) return;
    let cancelled = false;
    void (mature ? frozenMatureWorld() : Promise.resolve(architectureCityStudy(seed, towers)))
      .then((source) => {
        if (cancelled) return;
        const world = {
          ...source,
          buildings: source.buildings.map((b) =>
            stage === 'open' ? b : { ...b, stage, progress: 0.76 },
          ),
          minute_of_day: hour * 60,
          weather: {
            kind: rain ? ('rain' as const) : ('clear' as const),
            intensity: rain ? 0.7 : 0,
          },
        };
        r.setPreview(hour);
        r.setWorld(world);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [seed, stage, towers, mature, hour, rain]);
  const pose = (far: boolean) =>
    (viewer.current?.rig as CameraDirector | undefined)?.reviewPose(
      towers
        ? far
          ? { x: 280, y: 190, z: 290 }
          : { x: 220, y: 110, z: 220 }
        : far
          ? { x: 350, y: 230, z: 350 }
          : { x: 36, y: 11, z: 3 },
      towers ? { x: 0, y: 55, z: -24 } : far ? { x: 0, y: 12, z: 0 } : { x: 0, y: 8, z: 0 },
    );
  return (
    <main style={{ position: 'fixed', inset: 0 }}>
      <div
        ref={host}
        style={{ position: 'absolute', inset: 0 }}
        aria-label="City architecture integration study"
      />
      <section
        style={{
          position: 'absolute',
          top: 16,
          left: 16,
          width: 270,
          padding: 16,
          background: '#f5f1e9ed',
          fontFamily: 'system-ui',
          color: '#223b40',
        }}
      >
        <h2>City integration study</h2>
        <p>
          {mature ? 'Pinned mature city' : towers ? '5-tower' : '24-building'} fixture · actual city
          renderer and chunk scheduler · no saved world changes
        </p>
        <p>
          <label>
            Fixture{' '}
            <select
              aria-label="Study fixture"
              value={mature ? 'mature' : towers ? 'towers' : 'midrise'}
              onChange={(e) => {
                setMature(e.target.value === 'mature');
                setTowers(e.target.value === 'towers');
              }}
            >
              <option value="midrise">Mid-rise block</option>
              <option value="towers">Five towers</option>
              <option value="mature">Pinned mature city</option>
            </select>
          </label>
        </p>
        <p>
          <label>
            Apartment architecture{' '}
            <select
              aria-label="Apartment architecture"
              value={era}
              onChange={(e) => {
                const next = e.target.value as ApartmentEra;
                setEra(next);
                viewer.current?.setApartmentEra(next);
              }}
            >
              <option value="mixed">Mixed eras</option>
              <option value="contemporary">Contemporary</option>
              <option value="older">Older styles</option>
              <option value="legacy">Existing renderer</option>
            </select>
          </label>
        </p>
        <label>
          Seed{' '}
          <input
            disabled={mature}
            aria-label="Study seed"
            type="number"
            value={seed}
            onChange={(e) => setSeed(Number(e.target.value))}
          />
        </label>
        <p>
          <label>
            Stage{' '}
            <select
              aria-label="Study stage"
              value={stage}
              onChange={(e) => setStage(e.target.value as import('../types').Building['stage'])}
            >
              {['open', 'structure', 'surface', 'finishing', 'renovation', 'demolition'].map(
                (s) => (
                  <option key={s}>{s}</option>
                ),
              )}
            </select>
          </label>
        </p>
        <p>
          <label>
            Light{' '}
            <select
              aria-label="Study light"
              value={hour}
              onChange={(e) => setHour(Number(e.target.value))}
            >
              <option value="6">Dawn</option>
              <option value="14">Day</option>
              <option value="18.5">Dusk</option>
              <option value="21">Night</option>
            </select>
          </label>{' '}
          <label>
            <input
              aria-label="Rain"
              type="checkbox"
              checked={rain}
              onChange={(e) => setRain(e.target.checked)}
            />
            Rain
          </label>
        </p>
        <p>
          <button onClick={() => pose(false)}>Street</button>{' '}
          <button onClick={() => pose(true)}>Distant</button>{' '}
          <button
            onClick={() => {
              const next = style === 'anime' ? 'miniature' : 'anime';
              setStyle(next);
              void viewer.current?.setStyle(next).catch((e) => setError(String(e)));
            }}
          >
            Style: {style}
          </button>
        </p>
        <output>
          {metrics
            ? `${metrics.backend} · ${metrics.calls} draw calls · ${metrics.triangles.toLocaleString()} triangles · ${metrics.fps.toFixed(0)} FPS · ${metrics.pendingBuildings} buildings / ${metrics.pendingChunks} chunks pending`
            : 'Starting renderer'}
        </output>
        {error && <p role="alert">{error}</p>}
      </section>
    </main>
  );
}
