import { afterEach, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import {
  bodyForKind,
  isRoadVehicle,
  ROAD_SURFACE_Y,
  VEHICLE_KINDS,
  vehicleBody,
  VehicleFleet,
  vehicleLook,
  VehicleMotion,
} from '../web/src/render/vehicles';
import { LIVERY_INDEX } from '../web/src/render/vehicle-materials';

afterEach(() => vi.unstubAllGlobals());

describe('road vehicle kinds', () => {
  it('maps engine movement kinds to bodies and leaves people, trains and buses alone', () => {
    expect([0, 2, 3].map(isRoadVehicle)).toEqual([false, false, false]);
    expect(Object.values(VEHICLE_KINDS).every(isRoadVehicle)).toBe(true);
    expect(bodyForKind(VEHICLE_KINDS.mail)).toBe('kei');
    expect(bodyForKind(VEHICLE_KINDS.taxi)).toBe('taxi');
  });

  it('gives each vehicle a stable look: fleets carry liveries, private cars do not', () => {
    expect(vehicleLook(1, 42)).toEqual(vehicleLook(1, 42));
    expect(vehicleLook(1, 42).livery).toBe(-1);
    expect(vehicleLook(VEHICLE_KINDS.mail, 7).livery).toBe(LIVERY_INDEX['まち郵便']);
    const taxis = new Set(
      Array.from({ length: 200 }, (_, id) => vehicleLook(VEHICLE_KINDS.taxi, id).livery),
    );
    expect(taxis).toEqual(
      new Set(
        ['まちタクシー', '青葉交通', 'みどり交通', 'さくら無線'].map(
          (name) => LIVERY_INDEX[name as keyof typeof LIVERY_INDEX],
        ),
      ),
    );
    const trucks = new Set(
      Array.from({ length: 200 }, (_, id) => vehicleLook(VEHICLE_KINDS.truck, id).livery),
    );
    expect(trucks.size).toBe(4);
  });

  it('spreads each class across its bodies by weight and keeps builds within a few percent', () => {
    const share = (kind: number, body: string) =>
      Array.from({ length: 2000 }, (_, id) => vehicleBody(kind, id)).filter((b) => b === body)
        .length / 2000;
    expect(share(VEHICLE_KINDS.compact, 'compact')).toBeCloseTo(0.45, 1);
    expect(share(VEHICLE_KINDS.compact, 'compact-minivan')).toBeCloseTo(0.25, 1);
    expect(share(VEHICLE_KINDS.kei, 'kei-truck')).toBeCloseTo(0.15, 1);
    expect(share(VEHICLE_KINDS.mail, 'kei')).toBe(1);
    for (let id = 0; id < 200; id++) {
      const look = vehicleLook(VEHICLE_KINDS.compact, id);
      expect(look.width).toBeGreaterThanOrEqual(0.97);
      expect(look.width).toBeLessThanOrEqual(1.03);
    }
  });
});

describe('vehicle motion', () => {
  it('accumulates distance, ignores teleports, steers into turns and brakes when stopped', () => {
    const motion = new VehicleMotion();
    const start = motion.update(5, 0, 0, 0, 2.5, 1 / 60).odo;
    let s = motion.update(5, 0, 0.2, 0, 2.5, 1 / 60);
    expect(s.odo - start).toBeCloseTo(0.2, 5);
    s = motion.update(5, 0, 40, 0, 2.5, 1 / 60);
    expect(s.odo - start).toBeCloseTo(0.2, 5);
    // Steady left turn: positive yaw rate gives positive steer.
    let yaw = 0;
    for (let i = 0; i < 60; i++) {
      yaw += 0.01;
      s = motion.update(5, Math.sin(yaw) * 20, 40 + i * 0.2, yaw, 2.5, 1 / 60);
    }
    expect(s.steer).toBeGreaterThan(0.02);
    for (let i = 0; i < 60; i++)
      s = motion.update(5, Math.sin(yaw) * 20, 40 + 59 * 0.2, yaw, 2.5, 1 / 60);
    expect(s.brake).toBeGreaterThan(0.9);
  });
});

describe('vehicle fleet', () => {
  it('places each kind on its body mesh at road height with picking ids', () => {
    vi.stubGlobal('document', {
      createElement: () => ({
        width: 0,
        height: 0,
        getContext: () => ({ fillRect() {}, fillText() {}, strokeRect() {} }),
      }),
    });
    const art = new ArtMaterials(),
      fleet = new VehicleFleet(art, 4);
    fleet.begin();
    expect(fleet.add(11, VEHICLE_KINDS.compact, 10, 20, Math.PI / 2, 0, 0, 1 / 60)).toBe(true);
    expect(fleet.add(12, VEHICLE_KINDS.taxi, 30, 20, 0, 1.5, 0.1, 1 / 60)).toBe(true);
    expect(fleet.add(13, 3, 0, 0, 0, 0, 0, 1 / 60)).toBe(false);
    fleet.finish();
    const counts = fleet.meshes.map((m) => m.count);
    expect(counts.reduce((a, b) => a + b, 0)).toBe(2);
    const pick = fleet.pickable.filter((p) => p.ids.length);
    expect(pick.flatMap((p) => p.ids).sort()).toEqual([11, 12, 12]);
    const taxi = fleet.meshes.find(
      (m) => m.count === 1 && m.geometry.getAttribute('vstate').getZ(0) === 0,
    )!;
    const staticPosition = taxi.geometry.getAttribute(
      'position',
    ) as THREE.InterleavedBufferAttribute;
    const state = taxi.geometry.getAttribute('vstate') as THREE.InstancedBufferAttribute;
    const tint = taxi.geometry.getAttribute('vtint') as THREE.InstancedBufferAttribute;
    expect(staticPosition.data.stride).toBe(15);
    expect(state).toBeInstanceOf(THREE.InstancedBufferAttribute);
    expect(tint).toBeInstanceOf(THREE.InstancedBufferAttribute);
    expect(state.usage).toBe(THREE.DynamicDrawUsage);
    expect(tint.usage).toBe(THREE.DynamicDrawUsage);
    expect(state.getZ(0)).toBe(0); // Instance updates stay in their own buffer.
    const matrix = new THREE.Matrix4();
    taxi.getMatrixAt(0, matrix);
    expect(new THREE.Vector3().setFromMatrixPosition(matrix).y).toBeCloseTo(ROAD_SURFACE_Y + 1.5);
    for (let i = 0; i < 8; i++) fleet.add(100 + i, VEHICLE_KINDS.kei, i, 0, 0, 0, 0, 1 / 60);
    fleet.finish();
    expect(Math.max(...fleet.meshes.map((m) => m.count))).toBeLessThanOrEqual(4);
    fleet.dispose();
    art.dispose();
  });
});
