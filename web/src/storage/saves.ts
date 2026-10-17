import { parseApartmentEra, type ApartmentEra } from '../architecture-preference';
import type { LensPresetId } from '../optics/lens';

type SaveRecord = {
  slot: number;
  data: string;
  seed: number;
  savedAt: number;
};

export type LoadedWorld = { data: string; seed: number; savedAt: number };
export type ViewerPreferences = {
  apartmentEra: ApartmentEra;
  musicLevel: number;
  cityLevel: number;
  style: 'anime' | 'miniature';
  focus: boolean;
  quality: 'high' | 'balanced';
  contrast: 'standard' | 'high';
  lens: LensPresetId;
};

const DATABASE = 'thecity-worlds';
const DATABASE_VERSION = 1;
const STORE = 'saves';
const SAVE_SLOTS = 2;
const PREFERENCES_KEY = 'thecity-viewer-preferences';
const DEFAULT_PREFERENCES: ViewerPreferences = {
  apartmentEra: 'mixed',
  musicLevel: 0.38,
  cityLevel: 0.55,
  style: 'anime',
  focus: true,
  quality: 'high',
  contrast: 'standard',
  lens: 'environment',
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function isFiniteInteger(value: unknown, min = 0): value is number {
  return (
    typeof value === 'number' && Number.isFinite(value) && Number.isInteger(value) && value >= min
  );
}

function isEngineSave(data: string, expectedSeed?: number): boolean {
  try {
    const value: unknown = JSON.parse(data);
    if (!isRecord(value)) return false;
    const state = value;
    if (
      state.schema_version !== 1 ||
      typeof state.engine_version !== 'string' ||
      state.engine_version.length === 0
    )
      return false;
    if (
      !isFiniteInteger(state.seed) ||
      state.seed > 0xffff_ffff ||
      !isFiniteInteger(state.tick) ||
      !isFiniteInteger(state.revision)
    )
      return false;
    if (!isFiniteInteger(state.development_rate) || state.development_rate > 60) return false;
    if (
      ![
        'rng_growth',
        'rng_schedule',
        'rng_weather',
        'development_accum',
        'next_id',
        'next_event_id',
        'population',
        'growth_cursor',
        'weather_day',
        'last_departure_day',
        'last_renewal_day',
      ].every((key) => isFiniteInteger(state[key]))
    )
      return false;
    if (
      typeof state.station_online !== 'boolean' ||
      typeof state.weather_kind !== 'string' ||
      !Number.isFinite(state.weather_intensity) ||
      !Number.isFinite(state.demand)
    )
      return false;
    if (Number(state.weather_intensity) < 0 || Number(state.demand) < 0) return false;
    if (expectedSeed !== undefined && Number(state.seed) >>> 0 !== expectedSeed >>> 0) return false;
    const arrays = [
      'nodes',
      'roads',
      'parcels',
      'buildings',
      'projects',
      'travelers',
      'trains',
      'events',
    ];
    if (!arrays.every((key) => Array.isArray(state[key]))) return false;
    return arrays.every((key) => (state[key] as unknown[]).every(isRecord));
  } catch {
    return false;
  }
}

/** Validate a portable engine checkpoint before changing the running world. */
export function validateWorldSave(data: string): { data: string; seed: number } {
  if (!isEngineSave(data))
    throw new Error('World save has an unsupported or invalid engine schema');
  const state = JSON.parse(data) as { seed: number };
  return { data, seed: state.seed >>> 0 };
}

export function parseViewerPreferences(raw: string | null): ViewerPreferences {
  if (raw === null) return { ...DEFAULT_PREFERENCES };
  try {
    const value: unknown = JSON.parse(raw);
    if (!isRecord(value) || value.version !== 1) return { ...DEFAULT_PREFERENCES };
    const level = (entry: unknown, fallback: number) =>
      typeof entry === 'number' && Number.isFinite(entry)
        ? Math.max(0, Math.min(1, entry))
        : fallback;
    return {
      apartmentEra: parseApartmentEra(value.apartmentEra),
      musicLevel: level(value.musicLevel, DEFAULT_PREFERENCES.musicLevel),
      cityLevel: level(value.cityLevel, DEFAULT_PREFERENCES.cityLevel),
      style:
        value.style === 'anime' || value.style === 'miniature'
          ? value.style
          : DEFAULT_PREFERENCES.style,
      focus: typeof value.focus === 'boolean' ? value.focus : DEFAULT_PREFERENCES.focus,
      quality:
        value.quality === 'high' || value.quality === 'balanced'
          ? value.quality
          : DEFAULT_PREFERENCES.quality,
      contrast:
        value.contrast === 'high' || value.contrast === 'standard'
          ? value.contrast
          : DEFAULT_PREFERENCES.contrast,
      lens:
        value.lens === 'environment' || value.lens === 'neighborhood' || value.lens === 'detail'
          ? value.lens
          : DEFAULT_PREFERENCES.lens,
    };
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

/** Viewer preferences deliberately exclude sound-enabled state and light preview. */
export function loadViewerPreferences(): ViewerPreferences {
  try {
    return parseViewerPreferences(
      typeof localStorage === 'undefined' ? null : localStorage.getItem(PREFERENCES_KEY),
    );
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

export function saveViewerPreferences(preferences: ViewerPreferences): void {
  try {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(PREFERENCES_KEY, JSON.stringify({ version: 1, ...preferences }));
  } catch {
    // A preference write must not prevent observation when browser storage is unavailable.
  }
}

function openDatabase(): Promise<IDBDatabase> {
  if (typeof indexedDB === 'undefined')
    return Promise.reject(new Error('IndexedDB is unavailable in this browser'));
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DATABASE, DATABASE_VERSION);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(STORE))
        database.createObjectStore(STORE, { keyPath: 'slot' });
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('Could not open saved worlds'));
    request.onblocked = () => reject(new Error('Saved worlds database is blocked by another tab'));
  });
}

function transactionDone(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onabort = () =>
      reject(transaction.error ?? new Error('Save transaction was aborted'));
    transaction.onerror = () => reject(transaction.error ?? new Error('Save transaction failed'));
  });
}

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
  });
}

/** Store the latest checkpoint, rotating through two slots in one transaction. */
export async function saveWorld(data: string, seed: number): Promise<void> {
  if (!isEngineSave(data, seed))
    throw new Error('World save has an unsupported or invalid engine schema');
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE, 'readwrite');
    const done = transactionDone(transaction);
    const store = transaction.objectStore(STORE);
    let records: SaveRecord[];
    try {
      records = await requestResult(store.getAll() as IDBRequest<SaveRecord[]>);
    } catch (error) {
      await done.catch(() => undefined);
      throw error;
    }
    const occupied = new Set(records.map((record) => record.slot));
    let slot = -1;
    for (let candidate = 0; candidate < SAVE_SLOTS; candidate++) {
      if (!occupied.has(candidate)) {
        slot = candidate;
        break;
      }
    }
    if (slot < 0) {
      const oldest = records.reduce((a, b) => (a.savedAt <= b.savedAt ? a : b));
      slot = oldest.slot;
    }
    const savedAt = Math.max(Date.now(), ...records.map((record) => record.savedAt + 1));
    const record: SaveRecord = { slot, data, seed: seed >>> 0, savedAt };
    store.put(record);
    await done;
  } catch (error) {
    throw error instanceof Error ? error : new Error(String(error));
  } finally {
    database.close();
  }
}

/** Read the newest checkpoint from the two-slot rotation. */
export async function loadWorld(): Promise<LoadedWorld | null> {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE, 'readonly');
    const done = transactionDone(transaction);
    const [records] = await Promise.all([
      requestResult(transaction.objectStore(STORE).getAll() as IDBRequest<SaveRecord[]>),
      done,
    ]);
    const valid = records.filter((record) => {
      if (
        !record ||
        typeof record.data !== 'string' ||
        !Number.isFinite(record.seed) ||
        !Number.isFinite(record.savedAt)
      )
        return false;
      return isEngineSave(record.data, record.seed);
    });
    if (valid.length === 0) return null;
    const newest = valid.reduce((a, b) => (a.savedAt >= b.savedAt ? a : b));
    return { data: newest.data, seed: newest.seed, savedAt: newest.savedAt };
  } finally {
    database.close();
  }
}

/** Download a checkpoint as a portable JSON file. */
export function exportWorld(data: string): void {
  if (typeof document === 'undefined' || typeof URL === 'undefined') {
    throw new Error('World export requires a browser document');
  }
  const blob = new Blob([data], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `thecity-world-${new Date().toISOString().slice(0, 10)}.json`;
  link.hidden = true;
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}
