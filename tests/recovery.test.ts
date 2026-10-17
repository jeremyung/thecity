import { describe, expect, it, vi } from 'vitest';
import type { LoadedWorld } from '../web/src/storage/saves';
import { resolveStartupWorld } from '../web/src/storage/recovery';

function engineCheckpoint(seed: number, tick: number) {
  return JSON.stringify({
    schema_version: 1,
    engine_version: '0.1.0',
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
    nodes: [],
    roads: [],
    parcels: [],
    buildings: [],
    projects: [],
    travelers: [],
    trains: [],
    events: [],
    population: 0,
    demand: 0,
    growth_cursor: 0,
    station_online: false,
    weather_day: 0,
    weather_kind: 'clear',
    weather_intensity: 0,
    last_departure_day: 0,
    last_renewal_day: 0,
  });
}

function loaded(data: string, seed: number): LoadedWorld {
  return { data, seed, savedAt: 1234 };
}

describe('graphics recovery startup', () => {
  it('restores the exact saved engine checkpoint and its seed instead of the URL seed', async () => {
    const data = engineCheckpoint(87_231, 4_321);
    const loadSaved = vi.fn(async () => loaded(data, 87_231));

    await expect(
      resolveStartupWorld('?recover=1&seed=19&webgl=1', 87231, loadSaved),
    ).resolves.toEqual({
      kind: 'recovered',
      seed: 87_231,
      data,
    });
    expect(loadSaved).toHaveBeenCalledOnce();
  });

  it('preserves normal startup seed behavior without reading saved worlds', async () => {
    const loadSaved = vi.fn(async () => null);
    await expect(resolveStartupWorld('?seed=19', 87231, loadSaved)).resolves.toEqual({
      kind: 'new',
      seed: 19,
    });
    expect(loadSaved).not.toHaveBeenCalled();
    await expect(resolveStartupWorld('', 87_231, loadSaved)).resolves.toEqual({
      kind: 'new',
      seed: 87_231,
    });
  });

  it('blocks recovery startup when the latest save is missing or invalid', async () => {
    await expect(resolveStartupWorld('?recover=1', 87231, async () => null)).rejects.toThrow(
      'No valid saved city is available',
    );
    await expect(
      resolveStartupWorld('?recover=1', 87231, async () => loaded('{"seed":7}', 7)),
    ).rejects.toThrow('saved city for graphics recovery is invalid');
  });

  it('blocks recovery when IndexedDB cannot be read or the stored seed disagrees', async () => {
    await expect(
      resolveStartupWorld('?recover=1', 87231, async () => {
        throw new Error('quota denied');
      }),
    ).rejects.toThrow('Could not read the saved city');
    await expect(
      resolveStartupWorld('?recover=1', 87231, async () => loaded(engineCheckpoint(8, 2), 7)),
    ).rejects.toThrow('seed does not match');
  });
});
