import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import Controls, { RenderTimingNote, activeBusToFollow, busLineLabel, ContrastToggle, LensControl, RecentEvents, type ControlsProps } from '../web/src/ui/Controls';
import type { World } from '../web/src/types';

const world: World = {
  schema_version: 1,
  engine_version: 'test',
  seed: 87231,
  tick: 0,
  day: 3,
  minute_of_day: 360.0600000000002,
  development_rate: 4,
  revision: 0,
  stage: 'TransitCity',
  bounds: { min_x: -10, max_x: 10, min_z: -10, max_z: 10 },
  terrain: { river: [], protected: [], gateways: [] },
  weather: { kind: 'clear', intensity: 0 },
  nodes: [], roads: [], parcels: [], buildings: [], projects: [], trains: [], events: [],
  stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: 0 },
};

function renderControls(overrides: Partial<ControlsProps> = {}) {
  const props: ControlsProps = {
    world,
    ready: true,
    error: null,
    message: null,
    paused: false,
    speed: 1,
    development: 1,
    style: 'anime',
    mode: 'watch',
    sceneLabel: 'Along the river',
    preview: null,
    sound: false,
    musicLevel: 0.38,
    cityLevel: 0.55,
    focus: true,
    contrast: 'standard',
    lens: 'environment',
    selected: null,
    subject: null,
    quality: 'high',
    metrics: null,
    onPause: vi.fn(),
    onGraphicsRetry: vi.fn(),
    onGraphicsFallback: vi.fn(),
    onSpeed: vi.fn(),
    onDevelopment: vi.fn(),
    onStyle: vi.fn(),
    onMode: vi.fn(),
    onPreview: vi.fn(),
    onSound: vi.fn(),
    onLevels: vi.fn(),
    onFocus: vi.fn(),
    onContrast: vi.fn(),
    onLens: vi.fn(),
    onSave: vi.fn(),
    onResume: vi.fn(),
    onNewWorld: vi.fn(),
    onCloseInspector: vi.fn(),
    onFollow: vi.fn(),
    onQuality: vi.fn(),
    onDismissError: vi.fn(),
    ...overrides,
  };
  return renderToStaticMarkup(React.createElement(Controls, props));
}

describe('city controls', () => {
  it('renders fractional simulation minutes as a stable clock and expands stage names', () => {
    const html = renderControls();

    expect(html).toContain('06:00');
    expect(html).toContain('Transit City');
    expect(html).not.toContain('0.060000');
  });

  it('shows the selected preview hour in the city clock', () => {
    const html = renderControls({ preview: 18 });

    expect(html).toContain('LIGHT PREVIEW');
    expect(html).toContain('18:00');
    expect(html).not.toContain('06:00');
  });

  it('switches masthead ink for night and derives contrast from the preview hour', () => {
    const liveNight = renderControls({ world: { ...world, minute_of_day: 21 * 60 } });
    expect(liveNight).toContain('data-masthead-tone="night"');

    const previewNight = renderControls({ preview: 21 });
    expect(previewNight).toContain('data-masthead-tone="night"');

    const previewDay = renderControls({ preview: 12 });
    expect(previewDay).toContain('data-masthead-tone="day"');
  });

  it('keeps high-contrast controls independent of the night masthead treatment', () => {
    const html = renderControls({ world: { ...world, minute_of_day: 21 * 60 }, contrast: 'high' });
    expect(html).toContain('data-contrast="high" data-masthead-tone="night"');
    expect(html).not.toContain('data-contrast="standard"');
  });

  it('makes the paused state visible and announces status messages', () => {
    const paused = renderControls({ paused: true });
    expect(paused).toContain('SIMULATION PAUSED');
    expect(paused).toContain('role="status" aria-live="polite"');

    const notice = renderControls({ message: 'Your city is saved on this device.' });
    expect(notice).toContain('role="status"');
    expect(notice).toContain('Your city is saved on this device.');
  });

  it('announces style preparation while keeping the applied style selected', () => {
    const html = renderControls({ style: 'anime', preparingStyle: 'miniature' });

    expect(html).toContain('role="status" aria-live="polite" aria-atomic="true">Preparing Miniature…</span>');
    expect(html).toMatch(/aria-pressed="true"[^>]*>Anime<\/button>/);
    expect(html).toMatch(/aria-pressed="false"[^>]*>Miniature<\/button>/);

    const animeReady = renderControls({ style: 'miniature', preparingStyle: 'anime' });
    expect(animeReady).toContain('Preparing Anime…');
    expect(animeReady).toMatch(/aria-pressed="true"[^>]*>Miniature<\/button>/);
  });

  it('stacks readable event labels and humanizes event reasons', () => {
    const html = renderToStaticMarkup(React.createElement(RecentEvents, { events: [
      { id: 1, tick: 12, entity_id: 1, kind: 'ResidentTripArrived', reason: 'A resident is walking_to_bus toward Riverside Station' },
      { id: 2, tick: 11, entity_id: 1, kind: 'NewUnrecognizedLongEventKind', reason: 'walking_to_bus' },
    ] }));

    expect(html).toContain('class="recent-events"');
    expect(html).toContain('class="event-kind"');
    expect(html).toContain('<span class="event-kind">Resident arrived</span><span class="event-reason">A resident is walking to bus toward Riverside Station</span>');
    expect(html).toContain('<span class="event-kind">New Unrecognized Long Event Kind</span><span class="event-reason">Walking to bus</span>');
  });

  it('explains when the simulation cannot sustain the requested fast-forward speed', () => {
    const html = renderControls({ speed: 4, achievableSpeed: 2.6 });
    expect(html).toContain('RUNNING 2.6× · REQUESTED 4×');
    expect(html).toContain('role="status" aria-live="polite"');
  });

  it('exposes the selected control contrast mode', () => {
    const standard = renderControls();
    expect(standard).toContain('data-contrast="standard"');

    const high = renderControls({ contrast: 'high' });
    expect(high).toContain('data-contrast="high"');
    const checkbox = renderToStaticMarkup(React.createElement(ContrastToggle, { contrast: 'high', onContrast: vi.fn() }));
    expect(checkbox).toContain('type="checkbox" checked=""');
    expect(checkbox).toContain('High contrast controls');
  });

  it('offers the three focal-length presets in a compact accessible camera setting', () => {
    const onLens = vi.fn();
    const html = renderToStaticMarkup(React.createElement(LensControl, { lens: 'neighborhood', onLens }));
    expect(html).toContain('Camera lens');
    expect(html).toContain('aria-label="Camera lens"');
    expect(html).toContain('35 mm');
    expect(html).toContain('50 mm');
    expect(html).toContain('85 mm');
    expect(html).toContain('value="neighborhood"');
  });

  it('keeps camera controls available and offers retry and WebGL fallback for graphics errors', () => {
    const html = renderControls({ error: 'Graphics could not start. Try the WebGL fallback.' });

    expect(html).toContain('aria-label="Camera mode"');
    expect(html).toContain('aria-label="Simulation controls"');
    expect(html).toContain('>Retry</button>');
    expect(html).toContain('Use WebGL fallback');
    expect(html).toContain('type="button"');
  });

  it('shows the followed subject and an accessible follow action', () => {
    const html = renderControls({ subject: { id: 91, name: 'River crossing', kind: 'train', activity: 'Evening service', destination: 'Station' } });

    expect(html).toContain('aria-label="Following subject"');
    expect(html).toContain('River crossing');
    expect(html).toContain('Follow this train');
    expect(html).toContain('aria-label="Stop following"');
  });

  it('keeps bus follow hidden for legacy or inactive service and exposes an active bus', () => {
    expect(activeBusToFollow(world)).toBeNull();
    const busWorld: World = {
      ...world,
      buses: [{ id: 60, line_id: 8, progress: 18, direction: 1, dwelling: false, passengers: 4, capacity: 18, next_stop: 2, waiting: 3 }],
      bus_lines: [{ id: 8, stops: [1, 2], route: [1, 2], active: false }],
    };
    expect(activeBusToFollow(busWorld)).toBeNull();
    busWorld.bus_lines![0].active = true;
    expect(activeBusToFollow(busWorld)?.id).toBe(60);
  });

  it('uses a friendly bus route label instead of exposing the engine line ID', () => {
    const busWorld: World = {
      ...world,
      bus_lines: [{ id: 591, stops: [1, 2], route: [1, 2], active: true }],
    };

    expect(busLineLabel(busWorld, 591)).toBe('Neighborhood line 1');
    expect(busLineLabel(busWorld, 591, '591')).toBe('Neighborhood line 1');
    expect(busLineLabel({ ...busWorld, bus_lines: [{ ...busWorld.bus_lines![0], name: 'Riverside' }] }, 591)).toBe('Riverside');
  });

  it('shows bus line, next stop, load, dwell and waiting details in the followed subject panel', () => {
    const html = renderControls({ subject: {
      id: 60, name: 'Neighborhood Bus', kind: 'bus', activity: 'in_service',
      line: 'Neighborhood line 1', nextStop: 'Kanda Station', passengers: 4, capacity: 18, dwelling: true, waiting: 3,
    } });
    expect(html).toContain('Follow this bus');
    expect(html).toContain('Neighborhood line 1');
    expect(html).toContain('Next stop · Kanda Station');
    expect(html).toContain('Load 4 of 18');
    expect(html).toContain('Dwelling at stop');
    expect(html).toContain('Waiting 3');
  });

  it('humanizes followed activity and bounds recent subject history to three lines', () => {
    const html = renderControls({ subject: {
      id: 92,
      name: 'Yuki Tanaka',
      kind: 'train',
      activity: 'at_station',
      destination_name: 'Kanda Station',
      history: ['oldest_entry', 'boards the evening service', 'leaves the station', 'returns to the platform'],
      passengers: 3,
      capacity: 8,
      station_waiting: 5,
      gateway_waiting: 2,
    } });

    expect(html).toContain('Yuki Tanaka');
    expect(html).toContain('At Station · Kanda Station');
    expect(html).toContain('Passengers 3 of 8');
    expect(html).toContain('Station waiting 5');
    expect(html).toContain('Gateway waiting 2');
    expect(html).toContain('Returns to the platform');
    expect(html).toContain('Leaves the station');
    expect(html).toContain('Boards the evening service');
    expect(html).not.toContain('Oldest entry');
  });

  it('allows the error message to be dismissed when a handler is supplied', () => {
    const html = renderControls({ error: 'Graphics could not start.', onDismissError: vi.fn() });

    expect(html).toContain('aria-label="Dismiss error message"');
  });
});

it('labels render sampling and reports frame and CPU percentiles separately',()=>{
  const metrics={fps:0,frameMs:3,calls:1,triangles:12,textures:1,geometries:1,backend:'WebGPUBackend',timing:{samples:0,phase:'ready' as const,frameP95Ms:0,cpuP95Ms:0,maxFrameMs:0}};
  expect(renderToStaticMarkup(React.createElement(RenderTimingNote,{metrics}))).toContain('Collecting frame timings');
  const sampled=renderToStaticMarkup(React.createElement(RenderTimingNote,{metrics:{...metrics,fps:20,timing:{samples:120,phase:'ready',frameP95Ms:100,cpuP95Ms:8,maxFrameMs:300}}}));
  expect(sampled).toContain('frame p95');
  expect(sampled).toContain('CPU p95');
  expect(sampled).toContain('longest frame');
  expect(sampled).not.toContain('Collecting frame timings');
});
