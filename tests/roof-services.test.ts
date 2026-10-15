import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { renderRoofServices } from '../web/src/render/roof-services';

type Primitive =
  | { kind: 'box'; x: number; y: number; z: number; w: number; h: number; d: number; material: THREE.Material }
  | { kind: 'cylinder'; x: number; y: number; z: number; r: number; h: number; material: THREE.Material };

function specimen(w: number, d: number, seed = 17) {
  const materials = {
    concrete: new THREE.MeshStandardMaterial(),
    iron: new THREE.MeshStandardMaterial(),
    paleConcrete: new THREE.MeshStandardMaterial(),
    dark: new THREE.MeshStandardMaterial(),
  };
  const primitives: Primitive[] = [];
  const kit = {
    box(x: number, y: number, z: number, width: number, h: number, depth: number, material: THREE.Material) {
      primitives.push({ kind: 'box', x, y, z, w: width, h, d: depth, material });
    },
    cylinder(x: number, y: number, z: number, r: number, h: number, material: THREE.Material) {
      primitives.push({ kind: 'cylinder', x, y, z, r, h, material });
    },
  };
  renderRoofServices(kit, { w, d, top: 6, seed, ...materials });
  return { primitives, materials };
}

function bounds(p: Primitive) {
  const rx = p.kind === 'box' ? p.w / 2 : p.r;
  const rz = p.kind === 'box' ? p.d / 2 : p.r;
  const ry = p.kind === 'box' ? p.h / 2 : p.h / 2;
  return { minX: p.x - rx, maxX: p.x + rx, minY: p.y - ry, maxY: p.y + ry, minZ: p.z - rz, maxZ: p.z + rz };
}

describe('roof service detail', () => {
  it('mirrors a complete arrangement without changing dimensions or clearances', () => {
    const original = specimen(8, 7, 0).primitives;
    const mirrored = specimen(8, 7, 4).primitives;
    expect(mirrored).toHaveLength(original.length);
    original.forEach((part, index) => {
      const { material: _a, ...a } = part;
      const { material: _b, ...b } = mirrored[index];
      expect(b).toEqual({ ...a, x: -a.x });
    });
  });
  it('is deterministic, bounded to the roof, and within the primitive budget', () => {
    for (const [w, d] of [[3.2, 3.2], [3.2, 8], [4.6, 5.2], [6, 5], [12, 10]] as const) {
      const first = specimen(w, d, 51).primitives;
      const second = specimen(w, d, 51).primitives;
      const layout = (items: Primitive[]) => items.map(({ material: _material, ...item }) => item);
      expect(layout(first)).toEqual(layout(second));
      expect(first.length).toBeLessThanOrEqual(45);
      for (const primitive of first) {
        const b = bounds(primitive);
        expect(b.minX).toBeGreaterThanOrEqual(-w / 2 - 1e-6);
        expect(b.maxX).toBeLessThanOrEqual(w / 2 + 1e-6);
        expect(b.minZ).toBeGreaterThanOrEqual(-d / 2 - 1e-6);
        expect(b.maxZ).toBeLessThanOrEqual(d / 2 + 1e-6);
        expect(b.maxY).toBeLessThan(9); // roof top 6; service silhouette stays below top + 3
        expect(Object.values(b).every(Number.isFinite)).toBe(true);
      }
    }
  });

  it('supports the tank from the roof through legs and a pad', () => {
    const { primitives } = specimen(8, 7);
    const roofY = 6.53;
    const legs = primitives.filter((p): p is Extract<Primitive, { kind: 'box' }> =>
      p.kind === 'box' && Math.abs(p.w - .075) < 1e-8 && Math.abs(p.h - .28) < 1e-8 && Math.abs(p.d - .075) < 1e-8,
    );
    expect(legs).toHaveLength(4);
    expect(legs.every(leg => leg.y - leg.h / 2 === roofY)).toBe(true);

    const pad = primitives.find((p): p is Extract<Primitive, { kind: 'box' }> =>
      p.kind === 'box' && Math.abs(p.h - .08) < 1e-8 && p.material === primitives[0].material,
    );
    expect(pad).toBeDefined();
    const tank = primitives.find((p): p is Extract<Primitive, { kind: 'cylinder' }> =>
      p.kind === 'cylinder' && p.h > .5,
    );
    expect(tank).toBeDefined();
    expect(pad!.y + pad!.h / 2).toBeCloseTo(tank!.y - tank!.h / 2, 8);
    expect(legs.every(leg => Math.abs(leg.x - pad!.x) < pad!.w / 2 && Math.abs(leg.z - pad!.z) < pad!.d / 2)).toBe(true);
  });

  it('supports the HVAC casing and separates the main equipment footprints', () => {
    const { primitives, materials } = specimen(4.6, 5.2);
    const roofY = 6.53;
    const rails = primitives.filter((p): p is Extract<Primitive, { kind: 'box' }> =>
      p.kind === 'box' && p.material === materials.iron && Math.abs(p.w - .12) < 1e-8 && Math.abs(p.h - .12) < 1e-8,
    );
    expect(rails).toHaveLength(2);
    expect(rails.every(rail => Math.abs(rail.y - rail.h / 2 - roofY) < 1e-8)).toBe(true);

    const hvac = primitives.find((p): p is Extract<Primitive, { kind: 'box' }> =>
      p.kind === 'box' && p.material === materials.paleConcrete && Math.abs(p.h - .43) < 1e-8,
    );
    expect(hvac).toBeDefined();
    expect(rails.every(rail => Math.abs(rail.y + rail.h / 2 - (hvac!.y - hvac!.h / 2)) < 1e-8)).toBe(true);
    expect(rails.every(rail => Math.abs(rail.x - hvac!.x) < hvac!.w / 2)).toBe(true);

    const tank = primitives.find((p): p is Extract<Primitive, { kind: 'cylinder' }> =>
      p.kind === 'cylinder' && p.r >= .7 && p.h > .5,
    );
    const headhouse = primitives.find((p): p is Extract<Primitive, { kind: 'box' }> =>
      p.kind === 'box' && p.material === materials.concrete && Math.abs(p.h - 2.1) < 1e-8,
    );
    expect(tank).toBeDefined();
    expect(headhouse).toBeDefined();
    expect(headhouse!.y + headhouse!.h / 2).toBeLessThan(9);
    const rects = [
      { x: tank!.x, z: tank!.z, w: tank!.r * 2, d: tank!.r * 2 },
      { x: hvac!.x, z: hvac!.z, w: hvac!.w, d: hvac!.d },
      // Include the coping extension and front door projection in the headhouse envelope.
      { x: headhouse!.x, z: headhouse!.z + .018, w: headhouse!.w + .12, d: headhouse!.d + .12 + .035 },
    ];
    for (let i = 0; i < rects.length; i++) for (let j = i + 1; j < rects.length; j++) {
      const a = rects[i], b = rects[j];
      const separatedX = Math.abs(a.x - b.x) >= (a.w + b.w) / 2;
      const separatedZ = Math.abs(a.z - b.z) >= (a.d + b.d) / 2;
      const gapX = Math.abs(a.x - b.x) - (a.w + b.w) / 2;
      const gapZ = Math.abs(a.z - b.z) - (a.d + b.d) / 2;
      expect(separatedX || separatedZ).toBe(true);
      expect(Math.max(gapX, gapZ)).toBeGreaterThanOrEqual(.1 - 1e-8);
    }

    const risers = primitives.filter((p): p is Extract<Primitive, { kind: 'cylinder' }> =>
      p.kind === 'cylinder' && (Math.abs(p.r - .035) < 1e-8 || Math.abs(p.r - .055) < 1e-8),
    );
    expect(risers).toHaveLength(6);
    expect(risers.every(pipe => Math.abs(pipe.x - headhouse!.x) - (headhouse!.w / 2 + pipe.r) >= .1 - 1e-8)).toBe(true);
  });

  it('fits the complete cluster with clearance on the smallest supported roof', () => {
    const { primitives, materials } = specimen(4.6, 4.6);
    const hvac = primitives.find((p): p is Extract<Primitive, { kind: 'box' }> =>
      p.kind === 'box' && p.material === materials.paleConcrete && Math.abs(p.h - .43) < 1e-8,
    );
    const headhouse = primitives.find((p): p is Extract<Primitive, { kind: 'box' }> =>
      p.kind === 'box' && p.material === materials.concrete && Math.abs(p.h - 2.1) < 1e-8,
    );
    const tank = primitives.find((p): p is Extract<Primitive, { kind: 'cylinder' }> =>
      p.kind === 'cylinder' && p.h > .5,
    );
    expect(hvac).toBeDefined();
    expect(headhouse).toBeDefined();
    expect(tank).toBeDefined();
    const pairGap = (a: { x: number; z: number; w: number; d: number }, b: { x: number; z: number; w: number; d: number }) =>
      Math.max(Math.abs(a.x - b.x) - (a.w + b.w) / 2, Math.abs(a.z - b.z) - (a.d + b.d) / 2);
    const tankPlan = { x: tank!.x, z: tank!.z, w: tank!.r * 2, d: tank!.r * 2 };
    const hvacPlan = { x: hvac!.x, z: hvac!.z, w: hvac!.w, d: hvac!.d };
    const headhousePlan = { x: headhouse!.x, z: headhouse!.z, w: headhouse!.w + .12, d: headhouse!.d + .155 };
    expect(pairGap(tankPlan, hvacPlan)).toBeGreaterThanOrEqual(.1);
    expect(pairGap(tankPlan, headhousePlan)).toBeGreaterThanOrEqual(.1);
    expect(pairGap(hvacPlan, headhousePlan)).toBeGreaterThanOrEqual(.1);
  });

  it('omits service detail when the roof is too narrow to clear the tank from parapets', () => {
    const { primitives } = specimen(3.1, 4.2);
    expect(primitives).toHaveLength(0);
  });
});
