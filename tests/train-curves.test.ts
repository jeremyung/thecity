import * as THREE from 'three/webgpu';
import { describe, expect, it } from 'vitest';
import fixtureText from './fixtures/rail-alignment-native.json?raw';
import { trainWorldWithInfrastructure } from '../web/src/render/trains';
import { sampledRail } from '../web/src/render/rail-path';
import { roadBatchSignatures } from '../web/src/render/road-batch-signature';
import { COACH_SPACING } from '../web/src/render/rail-profile';
import type { RailAlignment, World } from '../web/src/types';

const alignment = (JSON.parse(fixtureText).cases as RailAlignment[])[0];
const path = sampledRail(alignment);
const art = { get: (color: string) => new THREE.MeshStandardMaterial({ color }) };
function world(): World {
  const a = alignment.samples[0], b = alignment.samples.at(-1)!;
  return { trains: [{ id: 42, route: [1, 2], progress: 0 }],
    nodes: [{ id: 1, x: a[1], z: a[2], kind: 'station' }, { id: 2, x: b[1], z: b[2], kind: 'rail_gateway' }],
    roads: [{ id: alignment.road_id, from: 1, to: 2, kind: 'rail', stage: 'open', connected: true, lanes: 1, length: path.length, progress: 1 }],
    rail_alignments: [alignment], projects: [], terrain: { river: [] },
  } as unknown as World;
}
function matrix(mesh: THREE.InstancedMesh, index = 0) {
  const m = new THREE.Matrix4(); mesh.getMatrixAt(index, m); return m;
}

describe('curved train presentation', () => {
  // Four-car tangent/leaf transform and reversal coverage lives in
  // train-fleet-curves.test.ts and train-door-state.test.ts after the asset merge.
  it('uses committed paths and invalidates the infrastructure batch when alignment changes', () => {
    const old = world(), current = world();
    current.rail_alignments = [{ ...alignment, samples: alignment.samples.map(s => [s[0],s[1],s[2]+4,s[3],s[4]]) }];
    const held = trainWorldWithInfrastructure(current, old);
    expect(held.trains).toBe(current.trains);
    expect(held.rail_alignments).toBe(old.rail_alignments);
    expect(held.roads).toBe(old.roads);
    expect(trainWorldWithInfrastructure(current, null).rail_alignments).toEqual([]);
    expect(roadBatchSignatures(old).completed).not.toBe(roadBatchSignatures(current).completed);
  });
});
