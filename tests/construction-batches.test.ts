import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ConstructionBatches } from '../web/src/render/construction-batches';

function source(x: number) {
  const geometry = new THREE.BoxGeometry(2, 3, 4),
    material = new THREE.MeshBasicMaterial(),
    group = new THREE.Group();
  group.position.x = x;
  group.add(new THREE.Mesh(geometry, material));
  const descriptor = {
    createGroup: () => group,
    disposeGroup: (value: THREE.Group) => {
      value.removeFromParent();
      geometry.dispose();
    },
  };
  return { group, descriptor, geometry, material };
}
function visibleTriangles(batches: ConstructionBatches, id: number) {
  const found: number[] = [];
  batches.root.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    let visible = true;
    for (let parent: THREE.Object3D | null = object; parent; parent = parent.parent)
      visible &&= parent.visible;
    if (!visible) return;
    const spans = object.userData.buildingSpans as
      Array<{ first: number; end: number; id: number }> | undefined;
    for (const span of spans ?? []) if (span.id === id) found.push(span.end - span.first);
  });
  return found.reduce((a, b) => a + b, 0);
}
function visibleMaterials(batches: ConstructionBatches, id: number) {
  const materials: THREE.Material[] = [];
  batches.root.traverse((object) => {
    if (!(object instanceof THREE.Mesh)) return;
    let visible = true;
    for (let parent: THREE.Object3D | null = object; parent; parent = parent.parent)
      visible &&= parent.visible;
    if (!visible) return;
    const spans = object.userData.buildingSpans as
      Array<{ first: number; end: number; id: number }> | undefined;
    if (spans?.some((span) => span.id === id)) materials.push(object.material as THREE.Material);
  });
  return materials;
}
function updateUntil(batches: ConstructionBatches, predicate: () => boolean) {
  for (let i = 0; i < 2000 && !predicate(); i++) batches.update();
  expect(predicate()).toBe(true);
}

describe('ConstructionBatches', () => {
  it('keeps the prior triangles until a changing source is ready, then shows only the latest descriptor', () => {
    const batches = new ConstructionBatches(
        (m) => m,
        (dispose) => dispose(),
      ),
      first = source(20),
      work = source(21),
      latest = source(22);
    batches.set(1, first.group, first.descriptor);
    updateUntil(batches, () => batches.isSourceVisible(1, first.descriptor));
    expect(visibleTriangles(batches, 1)).toBe(12);
    batches.set(1, work.group, work.descriptor, true);
    expect(visibleTriangles(batches, 1)).toBe(12);
    for (let i = 0; i < 2000 && batches.pendingCount; i++) {
      batches.update();
      expect(visibleTriangles(batches, 1)).toBe(12);
    }
    expect(batches.isSourceVisible(1, work.descriptor)).toBe(true);
    batches.set(1, latest.group, latest.descriptor, true);
    for (let i = 0; i < 2000 && !batches.isSourceVisible(1, latest.descriptor); i++) {
      batches.update();
      expect(visibleTriangles(batches, 1)).toBe(12);
    }
    expect(batches.isSourceVisible(1, latest.descriptor)).toBe(true);
    // Old membership can remain in a spatial batch until its neighborhood rebuild commits;
    // the visible triangle ownership must already have transferred atomically.
    expect(visibleTriangles(batches, 1)).toBe(12);
    batches.clear();
    for (const s of [first, work, latest]) s.material.dispose();
  });
  it('handles return to static, removal and clear without leaving visible ownership', () => {
    const batches = new ConstructionBatches(
        (m) => m,
        (dispose) => dispose(),
      ),
      initial = source(30),
      working = source(31),
      finished = source(32);
    batches.set(4, initial.group, initial.descriptor);
    updateUntil(batches, () => batches.isSourceVisible(4, initial.descriptor));
    batches.set(4, working.group, working.descriptor, true);
    updateUntil(batches, () => batches.isSourceVisible(4, working.descriptor));
    batches.set(4, finished.group, finished.descriptor, false);
    updateUntil(batches, () => batches.isSourceVisible(4, finished.descriptor));
    expect(visibleTriangles(batches, 4)).toBe(12);
    batches.remove(4);
    updateUntil(batches, () => !batches.hasVisibleSource(4));
    expect(visibleTriangles(batches, 4)).toBe(0);
    batches.clear();
    for (const s of [initial, working, finished]) s.material.dispose();
  });
  it('keeps a static peer and its contours alive while one building migrates', () => {
    const ink = new THREE.LineBasicNodeMaterial(),
      batches = new ConstructionBatches(
        (m) => m,
        (dispose) => dispose(),
        ink,
      ),
      moving = source(35),
      peer = source(36),
      work = source(37);
    batches.set(5, moving.group, moving.descriptor);
    batches.set(6, peer.group, peer.descriptor);
    updateUntil(
      batches,
      () =>
        batches.isSourceVisible(5, moving.descriptor) &&
        batches.isSourceVisible(6, peer.descriptor),
    );
    const peerTriangles = () => visibleTriangles(batches, 6);
    expect(peerTriangles()).toBe(12);
    batches.set(5, work.group, work.descriptor, true);
    for (let i = 0; i < 2000 && batches.isSourceVisible(5, moving.descriptor); i++) {
      batches.update();
      expect(peerTriangles()).toBe(12);
    }
    updateUntil(batches, () => batches.isSourceVisible(5, work.descriptor));
    expect(peerTriangles()).toBe(12);
    const contourLines: THREE.LineSegments[] = [];
    batches.root.traverse((o) => {
      if (o instanceof THREE.LineSegments && o.visible) contourLines.push(o);
    });
    expect(contourLines.length).toBeGreaterThan(0);
    batches.clear();
    for (const s of [moving, peer, work]) s.material.dispose();
    ink.dispose();
  });
  it('coalesces a rapid reversal while each committed frame retains one visible owner', () => {
    const batches = new ConstructionBatches(
        (m) => m,
        (dispose) => dispose(),
      ),
      initial = source(40),
      work = source(41),
      latest = source(42);
    batches.set(8, initial.group, initial.descriptor);
    updateUntil(batches, () => batches.isSourceVisible(8, initial.descriptor));
    batches.set(8, work.group, work.descriptor, true);
    batches.set(8, latest.group, latest.descriptor, false);
    for (let i = 0; i < 2000 && !batches.isSourceVisible(8, latest.descriptor); i++) {
      batches.update();
      expect(visibleTriangles(batches, 8)).toBe(12);
    }
    expect(batches.isSourceVisible(8, latest.descriptor)).toBe(true);
    expect(batches.isSourceVisible(8, work.descriptor)).toBe(false);
    batches.clear();
    for (const s of [initial, work, latest]) s.material.dispose();
  });
});

// Exercise overlapping builds with reusable factories, rather than a single
// finished update per transition. Every public update is a possible render.
it('preserves one visible owner through overlapping construction and reopening updates', () => {
  const material = new THREE.MeshBasicMaterial();
  const batches = new ConstructionBatches(
    (m) => m,
    (dispose) => dispose(),
  );
  const make = () => {
    const createGroup = () => {
      const group = new THREE.Group();
      group.position.x = 20;
      group.add(new THREE.Mesh(new THREE.BoxGeometry(2, 3, 4), material));
      return group;
    };
    return {
      createGroup,
      disposeGroup: (g: THREE.Group) =>
        g.traverse((o) => {
          if (o instanceof THREE.Mesh) o.geometry.dispose();
        }),
    };
  };
  const initial = make();
  batches.set(1, initial.createGroup(), initial);
  for (let id = 2; id < 40; id++) {
    const peer = make();
    batches.set(id, peer.createGroup(), peer);
  }
  updateUntil(batches, () => batches.pendingCount === 0);
  let latest = initial;
  for (let frame = 0; frame < 180; frame++) {
    if (frame % 7 === 0) {
      latest = make();
      batches.set(1, latest.createGroup(), latest, frame % 21 !== 0);
    }
    batches.update();
    expect(visibleTriangles(batches, 1), `target frame ${frame}`).toBe(12);
    expect(visibleTriangles(batches, 2), `peer frame ${frame}`).toBe(12);
  }
  updateUntil(batches, () => batches.pendingCount === 0);
  expect(batches.isSourceVisible(1, latest)).toBe(true);
  batches.clear();
  expect(batches.root.children.every((group) => group.children.length === 0)).toBe(true);
  material.dispose();
});

it('resolves the latest material style when a detached handoff build commits', () => {
  const authored = new THREE.MeshBasicMaterial(),
    oldStyle = new THREE.MeshBasicMaterial(),
    newStyle = new THREE.MeshBasicMaterial();
  let style = oldStyle;
  const batches = new ConstructionBatches(
    () => style,
    (dispose) => dispose(),
  );
  const make = (width = 2, meshCount = 1) => ({
    createGroup: () => {
      const group = new THREE.Group();
      group.position.x = 55;
      for (let i = 0; i < meshCount; i++)
        group.add(new THREE.Mesh(new THREE.BoxGeometry(width, 3, 4), authored));
      return group;
    },
    disposeGroup: (group: THREE.Group) =>
      group.traverse((object) => {
        if (object instanceof THREE.Mesh) object.geometry.dispose();
      }),
  });
  const staticSource = make(2, 30),
    dynamicSource = make(2, 30),
    returnedStatic = make(2, 30);
  batches.set(12, staticSource.createGroup(), staticSource, false);
  updateUntil(batches, () => batches.isSourceVisible(12, staticSource));
  expect(visibleMaterials(batches, 12).every((material) => material === oldStyle)).toBe(true);

  batches.set(12, dynamicSource.createGroup(), dynamicSource, true);
  // The scheduler first advances static removal, then starts preparing the
  // isolated dynamic batch. Change the scene style while that batch is detached.
  batches.update();
  batches.update();
  expect(batches.isSourceVisible(12, dynamicSource)).toBe(false);
  expect(visibleMaterials(batches, 12).every((material) => material === oldStyle)).toBe(true);
  style = newStyle;
  for (let frame = 0; frame < 2000 && !batches.isSourceVisible(12, dynamicSource); frame++) {
    batches.update();
    expect(visibleTriangles(batches, 12)).toBe(360);
  }
  expect(batches.isSourceVisible(12, staticSource)).toBe(false);
  expect(visibleTriangles(batches, 12)).toBe(360);
  expect(visibleMaterials(batches, 12).every((material) => material === newStyle)).toBe(true);

  // Switching back also keeps the current style on the batch committed from
  // the detached static build.
  batches.set(12, returnedStatic.createGroup(), returnedStatic, false);
  for (let frame = 0; frame < 2000 && !batches.isSourceVisible(12, returnedStatic); frame++) {
    batches.update();
    expect(visibleTriangles(batches, 12)).toBe(360);
  }
  expect(batches.isSourceVisible(12, dynamicSource)).toBe(false);
  expect(visibleTriangles(batches, 12)).toBe(360);
  expect(visibleMaterials(batches, 12).every((material) => material === newStyle)).toBe(true);
  batches.clear();
  authored.dispose();
  oldStyle.dispose();
  newStyle.dispose();
});

it('disposes batch-owned and pending source materials once when cleared during handoff', () => {
  const disposalCounts = new Map<THREE.Material, number>(),
    ownedMaterials: THREE.Material[] = [];
  const batches = new ConstructionBatches(
    (m) => m,
    (dispose) => dispose(),
  );
  const make = () => {
    const owned = new THREE.MeshBasicMaterial();
    ownedMaterials.push(owned);
    owned.addEventListener('dispose', () =>
      disposalCounts.set(owned, (disposalCounts.get(owned) ?? 0) + 1),
    );
    return {
      createGroup: () => {
        const group = new THREE.Group();
        group.position.x = 70;
        const mesh = new THREE.Mesh(new THREE.BoxGeometry(2, 3, 4), owned);
        mesh.userData.ownedMaterial = true;
        group.add(mesh);
        return group;
      },
      disposeGroup: (group: THREE.Group) =>
        group.traverse((object) => {
          if (!(object instanceof THREE.Mesh)) return;
          object.geometry.dispose();
          if (object.userData.ownedMaterial) {
            for (const material of Array.isArray(object.material)
              ? object.material
              : [object.material])
              material.dispose();
            delete object.userData.ownedMaterial;
          }
        }),
      owned,
    };
  };
  const initial = make(),
    incoming = make();
  batches.set(13, initial.createGroup(), initial, false);
  updateUntil(batches, () => batches.isSourceVisible(13, initial));
  batches.set(13, incoming.createGroup(), incoming, true);
  batches.update(); // the static removal is staged; incoming source remains pending
  batches.clear();
  batches.clear();
  expect(disposalCounts.get(initial.owned)).toBe(1);
  expect(disposalCounts.get(incoming.owned)).toBe(1);
  expect(ownedMaterials).toHaveLength(2);
  expect(batches.root.children.every((group) => group.children.length === 0)).toBe(true);
});

it('cancels a staged static migration cleanly when construction resumes', () => {
  const batches = new ConstructionBatches(
      (m) => m,
      (dispose) => dispose(),
    ),
    material = new THREE.MeshBasicMaterial();
  const disposalCounts = new Map<THREE.Group, number>();
  const make = () => {
    const groups: THREE.Group[] = [];
    const descriptor = {
      createGroup: () => {
        const group = new THREE.Group();
        group.position.x = 90;
        group.add(new THREE.Mesh(new THREE.BoxGeometry(2, 3, 4), material));
        groups.push(group);
        return group;
      },
      disposeGroup: (group: THREE.Group) => {
        disposalCounts.set(group, (disposalCounts.get(group) ?? 0) + 1);
        group.traverse((object) => {
          if (object instanceof THREE.Mesh) object.geometry.dispose();
        });
      },
    };
    return { groups, descriptor };
  };
  const original = make(),
    working = make(),
    returned = make(),
    resumed = make();
  batches.set(21, original.descriptor.createGroup(), original.descriptor);
  updateUntil(batches, () => batches.isSourceVisible(21, original.descriptor));
  batches.set(21, working.descriptor.createGroup(), working.descriptor, true);
  updateUntil(batches, () => batches.isSourceVisible(21, working.descriptor));

  batches.set(21, returned.descriptor.createGroup(), returned.descriptor, false);
  updateUntil(batches, () => returned.groups.length === 2); // second group is the staged static migration
  const stagedMigration = returned.groups[1];
  expect(batches.isSourceVisible(21, returned.descriptor)).toBe(true);
  expect(batches.pendingCount).toBeGreaterThan(0);
  expect(disposalCounts.get(stagedMigration) ?? 0).toBe(0);
  batches.set(21, resumed.descriptor.createGroup(), resumed.descriptor, true);
  expect(disposalCounts.get(stagedMigration)).toBe(1);
  for (let frame = 0; frame < 2000 && !batches.isSourceVisible(21, resumed.descriptor); frame++) {
    batches.update();
    expect(visibleTriangles(batches, 21)).toBe(12);
  }
  updateUntil(batches, () => batches.pendingCount === 0);
  expect(batches.isSourceVisible(21, resumed.descriptor)).toBe(true);
  expect(batches.isSourceVisible(21, returned.descriptor)).toBe(false);
  expect(visibleTriangles(batches, 21)).toBe(12);
  batches.clear();
  material.dispose();
});

it('publishes reopening before peer regeneration and drains its isolated batch under the factory quota', () => {
  const material = new THREE.MeshBasicMaterial();
  let calls = 0,
    peerCalls = 0;
  const batches = new ConstructionBatches(
    (m) => m,
    (dispose) => dispose(),
  );
  const make = (peer = false) => ({
    createGroup: () => {
      calls++;
      if (peer) peerCalls++;
      const group = new THREE.Group();
      group.position.x = 20;
      group.add(new THREE.Mesh(new THREE.BoxGeometry(2, 3, 4), material));
      return group;
    },
    disposeGroup: (group: THREE.Group) =>
      group.traverse((o) => {
        if (o instanceof THREE.Mesh) o.geometry.dispose();
      }),
  });
  const work = make();
  batches.set(1, work.createGroup(), work, true);
  for (let id = 2; id < 40; id++) {
    const peer = make(true);
    batches.set(id, peer.createGroup(), peer);
  }
  updateUntil(batches, () => batches.pendingCount === 0);
  const opened = make();
  batches.set(1, opened.createGroup(), opened, false);
  peerCalls = 0;
  batches.update();
  expect(batches.isSourceVisible(1, opened)).toBe(true);
  expect(peerCalls).toBe(0);
  for (let frame = 0; frame < 2000 && batches.pendingCount; frame++) {
    const before = calls;
    batches.update();
    expect(calls - before).toBeLessThanOrEqual(1);
    expect(batches.isSourceVisible(1, opened)).toBe(true);
    expect(visibleTriangles(batches, 1)).toBe(12);
    expect(visibleTriangles(batches, 2)).toBe(12);
  }
  expect(batches.pendingCount).toBe(0);
  expect(batches.root.children[1].children).toHaveLength(0);
  expect(peerCalls).toBe(38);
  batches.clear();
  material.dispose();
});
