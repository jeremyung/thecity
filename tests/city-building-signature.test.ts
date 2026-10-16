import { describe, expect, it } from 'vitest';
import { buildingRenderSignature } from '../web/src/render/city';
import { windowLived } from '../web/src/render/building-surfaces';
import type { Building } from '../web/src/types';

const building = (overrides: Partial<Building> = {}): Building => ({
  id: 1,
  parcel_id: 1,
  x: 0,
  z: 0,
  rotation: 0,
  width: 8,
  depth: 8,
  floors: 2,
  kind: 'home',
  stage: 'open',
  progress: 1,
  capacity: 12,
  occupancy: 0,
  business_active: false,
  built_day: 1,
  name: 'Lane House',
  appearance_seed: 41,
  history: [],
  anchor: false,
  vacant_days: 0,
  ...overrides,
});

describe('building render signatures', () => {
  it('rebuilds when a saved original footprint changes the retained building form', () => {
    const current = building({ depth: 20, z: -4 });
    const original = { building_id: current.id, x: 0, z: 0, width: 8, depth: 12, rotation: 0 };
    expect(buildingRenderSignature(current, original)).not.toBe(buildingRenderSignature(current));
    expect(buildingRenderSignature(current, { ...original, depth: 14, z: -1 })).not.toBe(
      buildingRenderSignature(current, original),
    );
    expect(buildingRenderSignature(current, { ...original })).toBe(
      buildingRenderSignature(current, original),
    );
  });
  it('rebuilds an extended building even when its identity and open stage stay unchanged', () => {
    const before = building({ occupancy: 3, depth: 12 });
    const completed = building({ ...before, z: -4, depth: 20, capacity: 20 });
    expect(buildingRenderSignature(completed)).not.toBe(buildingRenderSignature(before));
    for (const geometry of [{ x: 1 }, { z: 1 }, { width: 9 }, { depth: 9 }, { rotation: 0.5 }]) {
      expect(buildingRenderSignature(building(geometry))).not.toBe(
        buildingRenderSignature(building()),
      );
    }
  });
  it('rebuilds residential windows only when occupancy crosses the vacant boundary', () => {
    // Window panes bake whether anyone lives behind them, for homes and apartment blocks alike.
    for (const kind of ['home', 'apartment']) {
      const vacant = building({ kind, occupancy: 0 });
      const occupied = building({ kind, occupancy: 1 });
      const fuller = building({ kind, occupancy: 2 });
      expect(buildingRenderSignature(vacant), kind).not.toBe(buildingRenderSignature(occupied));
      expect(buildingRenderSignature(occupied), kind).toBe(buildingRenderSignature(fuller));
      expect(buildingRenderSignature(fuller), kind).not.toBe(
        buildingRenderSignature(building({ kind, occupancy: 0 })),
      );
    }
  });

  it('keeps every signature in step with the window occupancy the kit bakes', () => {
    for (const kind of ['home', 'apartment', 'shop', 'cafe', 'office'])
      for (const stage of ['open', 'construction', 'renovation'])
        for (const occupancy of [0, 4])
          for (const business_active of [true, false]) {
            const a = building({ kind, stage, occupancy, business_active });
            for (const b of [
              building({ kind, stage, occupancy: occupancy ? 0 : 4, business_active }),
              building({ kind, stage, occupancy, business_active: !business_active }),
            ]) {
              if (windowLived(a) !== windowLived(b))
                expect(
                  buildingRenderSignature(a),
                  `${kind} ${stage} ${occupancy} ${business_active}`,
                ).not.toBe(buildingRenderSignature(b));
            }
          }
  });

  it('does not rebuild businesses for occupancy changes', () => {
    for (const kind of ['shop', 'cafe', 'office']) {
      const empty = building({ kind, occupancy: 0 });
      const occupied = building({ kind, occupancy: 11 });
      expect(buildingRenderSignature(empty), kind).toBe(buildingRenderSignature(occupied));
    }
  });

  it('updates occupied annex apartment glazing across the empty boundary only', () => {
    const original = { building_id: 1, x: 0, z: 0, width: 8, depth: 8, rotation: 0 };
    const empty = building({ kind: 'apartment', depth: 16, z: -4, occupancy: 0 });
    expect(buildingRenderSignature(empty, original)).not.toBe(
      buildingRenderSignature({ ...empty, occupancy: 1 }, original),
    );
    expect(buildingRenderSignature({ ...empty, occupancy: 1 }, original)).toBe(
      buildingRenderSignature({ ...empty, occupancy: 2 }, original),
    );
  });
});
