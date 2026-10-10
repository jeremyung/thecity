import { validateWorldSave, type LoadedWorld } from './saves';

export type StartupWorld =
  | { kind: 'new'; seed: number }
  | { kind: 'recovered'; seed: number; data: string };

/** Resolve a requested graphics recovery before simulation startup. A normal
 * visit never reads local saves and keeps the URL-seed behavior unchanged. */
export async function resolveStartupWorld(
  search: string,
  defaultSeed: number,
  loadSaved: () => Promise<LoadedWorld | null>,
): Promise<StartupWorld> {
  const query = new URLSearchParams(search);
  const seed = (Number(query.get('seed') ?? defaultSeed) >>> 0);
  if (query.get('recover') !== '1') return { kind: 'new', seed };

  let saved: LoadedWorld | null;
  try {
    saved = await loadSaved();
  } catch (error) {
    throw new Error(`Could not read the saved city for graphics recovery: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (!saved) throw new Error('No valid saved city is available for graphics recovery. Open World and saves to begin a new city.');

  let checkpoint: { data: string; seed: number };
  try {
    checkpoint = validateWorldSave(saved.data);
  } catch (error) {
    throw new Error(`The saved city for graphics recovery is invalid: ${error instanceof Error ? error.message : String(error)}`);
  }
  if ((saved.seed >>> 0) !== checkpoint.seed) throw new Error('The saved city seed does not match its checkpoint; graphics recovery stopped safely.');
  return { kind: 'recovered', seed: checkpoint.seed, data: checkpoint.data };
}
