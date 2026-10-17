import type { CityRenderer } from './city';
import * as THREE from 'three/webgpu';
import { MovementPresentation } from './movement-presentation';
import { type MovementSupport } from '../movement-support';
import { MOVEMENT_STRIDE, PEDESTRIAN_GROUND_SURFACE, pedestrianSurface } from '../movement';
import { add, step, sample } from 'three/tsl';
import { CityBuses } from './buses';
import { isRoadVehicle, ROAD_SURFACE_Y, VehicleFleet } from './vehicles';
import { TrainFleet } from './trains';
import { RAIL_HEAD_Y } from './rail-profile';
import { bridgeLift, bridgeVehiclePose, vehicleSurfacePose } from './road-elevation';
import { dummy, personColors } from './city';

export class ActorsLayer {
  readonly buses: CityBuses;
  readonly fleet: VehicleFleet;
  readonly trains: TrainFleet;

  constructor(private readonly host: CityRenderer) {
    this.buses = new CityBuses(host.art, (release) => host.retire(release));
    this.fleet = new VehicleFleet(host.art);
    this.trains = new TrainFleet(host.art);
  }

  updateActors(now: number) {
    const t = Math.min(1, (now - this.received) / this.interval);
    let ped = 0,
      train = 0,
      bus = 0;
    this.buses.begin(now);
    this.trains.begin();
    this.fleet.begin();
    const pedIds: number[] = [];
    this.previous = this.current;
    this.current = this.movementPresentation.sample(now, this.host.roadsLayer.infrastructure());
    this.motionDelta = this.actorSampleAt
      ? Math.max(0.001, (now - this.actorSampleAt) / 1000)
      : 0.05;
    this.actorSampleAt = now;
    const cameraPositions: number[] = [];
    for (const [id, current] of this.current) {
      const old = this.previous.get(id) ?? current,
        x = current.x,
        z = current.z,
        h = Math.PI / 2 - current.h;
      const lift =
        current.k === 0
          ? pedestrianSurface(current.y, bridgeLift(current, this.host.roadsLayer.bridgeSpans)) -
            PEDESTRIAN_GROUND_SURFACE
          : current.k === 2
            ? 0
            : bridgeLift({ x, z }, this.host.roadsLayer.bridgeSpans);
      let surfaceY = current.y;

      if (current.k === 0 && ped < 2000) {
        surfaceY = lift + PEDESTRIAN_GROUND_SURFACE;
        const offset = ped * MOVEMENT_STRIDE;
        this.umbrellaTuples[offset] = id;
        this.umbrellaTuples[offset + 1] = x;
        this.umbrellaTuples[offset + 2] = z;
        this.umbrellaTuples[offset + 3] = Math.PI / 2 - h;
        this.umbrellaTuples[offset + 4] = 0;
        this.umbrellaTuples[offset + 5] = lift;
        const moving = Math.hypot(current.x - old.x, current.z - old.z) > 0.01;
        const step = moving ? Math.sin(this.motionTick * 0.31 + id) * 0.045 : 0;
        dummy.position.set(x, 1.05 + step + lift, z);
        dummy.rotation.set(0, h, moving ? step * 0.4 : 0);
        dummy.scale.setScalar(1);
        dummy.updateMatrix();
        this.pedestrians.setMatrixAt(ped, dummy.matrix);
        this.pedestrians.setColorAt(ped, personColors[id % personColors.length]);
        dummy.position.y = 1.95 + step + lift;
        dummy.updateMatrix();
        this.heads.setMatrixAt(ped, dummy.matrix);
        pedIds[ped] = id;
        ped++;
      } else if (isRoadVehicle(current.k)) {
        const vehicle = vehicleSurfacePose(
          { x, z },
          h,
          2.7,
          this.host.roadsLayer.bridgeSpans,
          current.y,
        );
        surfaceY = vehicle.lift + ROAD_SURFACE_Y;
        this.fleet.add(id, current.k, x, z, h, vehicle.lift, vehicle.pitch, this.motionDelta);
      } else if (current.k === 3 && bus < 2) {
        const vehicle = bridgeVehiclePose({ x, z }, h, 5.1, this.host.roadsLayer.bridgeSpans);
        surfaceY = vehicle.lift + ROAD_SURFACE_Y;
        this.buses.set(
          bus++,
          id,
          x,
          z,
          h,
          this.host.worldState,
          vehicle.lift + ROAD_SURFACE_Y,
          vehicle.pitch,
        );
      } else if (current.k === 2 && train < 8) {
        this.trains.add(id, x, z, h, RAIL_HEAD_Y, 0, this.motionDelta, t);
        train++;
      } else {
        this.current.delete(id);
        continue;
      }
      cameraPositions.push(id, x, z, current.h, current.k, surfaceY);
    }
    this.host.rig.setPositions(
      new Float32Array(cameraPositions),
      this.movementPresentation.withheld(this.current),
    );
    this.umbrellaCount = ped;
    this.trains.finish(train);
    for (const { mesh, ids } of this.trains.pickable) this.actorIds.set(mesh, ids);
    this.buses.finish(bus);
    for (const mesh of this.buses.meshes) this.actorIds.set(mesh, this.buses.ids);
    this.actorIds.set(this.pedestrians, pedIds);
    this.actorIds.set(this.heads, pedIds);
    this.fleet.finish();
    for (const { mesh, ids } of this.fleet.pickable) this.actorIds.set(mesh, ids);
    for (const mesh of [this.pedestrians, this.heads]) {
      mesh.count = ped;
      mesh.instanceMatrix.needsUpdate = true;
    }
    if (this.pedestrians.instanceColor) this.pedestrians.instanceColor.needsUpdate = true;
  }

  setMovement(
    data: Float32Array,
    tick: number,
    minute: number,
    trainDoors: Float32Array,
    support: ReadonlyMap<number, MovementSupport>,
  ) {
    this.trains.setMovement(trainDoors);
    const now = performance.now();
    this.interval = Math.max(20, Math.min(250, now - this.received));
    this.received = now;
    this.motionTick = tick;
    this.host.hour = minute / 60;
    this.movementPresentation.receive(data, support, this.interval);
  }

  actorIds = new Map<THREE.Object3D, number[]>();

  actorSampleAt = 0;

  current = new Map<number, { x: number; z: number; h: number; k: number; y: number }>();

  heads!: THREE.InstancedMesh;

  interval = 50;

  motionDelta = 0.05;

  motionTick = 0;

  movementPresentation = new MovementPresentation();

  pedestrians!: THREE.InstancedMesh;

  previous = new Map<number, { x: number; z: number; h: number; k: number; y: number }>();

  received = performance.now();

  umbrellaCount = 0;

  umbrellaTuples = new Float32Array(2000 * MOVEMENT_STRIDE);
}
