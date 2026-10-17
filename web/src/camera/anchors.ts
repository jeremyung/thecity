import * as THREE from 'three';
import { retainWatchSubjects } from './watch-selection';
import { SkylineClusterCache } from './skyline-cluster';
import { type CameraObstacleBox, type CameraPoint3 } from './clearance';
import { type Point, type Project, type World } from '../types';
import { type Anchor, type Extent, MAX_ANCHORS, type StreetSubject, UP } from './shot';

export function buildAnchors(skylineCache: SkylineClusterCache, world: World): Anchor[] {
  const anchors: Anchor[] = [];
  const nodes = new Map(world.nodes.map((node) => [node.id, node]));
  const add = (anchor: Anchor) => {
    if (anchors.some((candidate) => candidate.id === anchor.id)) return;
    anchors.push(anchor);
  };
  const buildings = world.buildings;

  for (const building of buildings) {
    if (building.anchor) {
      const river = closestRiverPoint(world.terrain.river, building);
      const targetX = river ? building.x * 0.6 + river.x * 0.4 : building.x;
      const targetZ = river ? building.z * 0.6 + river.z * 0.4 : building.z;
      // Keep the founder as the opening subject; a small riverward offset
      // lets the water read as context without using its length to set scale.
      add({
        id: `founder:${building.id}`,
        label: building.name || 'Founding street',
        x: building.x * 0.92 + targetX * 0.08,
        z: building.z * 0.92 + targetZ * 0.08,
        frameRadius: founderFrameRadius(building),
        importance: 10,
        activity: building.business_active === true ? 4 : building.occupancy,
        suitability: 'all',
      });
    }
    if (/station/i.test(building.kind)) {
      add({
        id: `station:${building.id}`,
        label: 'Station street',
        x: building.x,
        z: building.z,
        importance: 8,
        activity: building.occupancy + 3,
        suitability: 'night',
      });
    }
  }

  for (const road of world.roads) {
    if (road.stage !== 'open' || !road.connected) continue;
    if (road.kind === 'bridge') {
      const a = nodes.get(road.from);
      const b = nodes.get(road.to);
      if (a && b)
        add({
          id: `crossing:${road.id}`,
          label: 'River crossing',
          x: (a.x + b.x) / 2,
          z: (a.z + b.z) / 2,
          importance: 8,
          activity: road.lanes,
          suitability: 'rain',
        });
    }
  }

  const connectedNodes = new Set<number>();
  const parcelAccess = new Map(world.parcels.map((parcel) => [parcel.id, parcel.access_node]));
  for (const road of world.roads) {
    if (road.stage === 'open' && road.connected) {
      connectedNodes.add(road.from);
      connectedNodes.add(road.to);
    }
  }
  for (const project of world.projects) {
    if (project.progress >= 1) continue;
    const point = projectPoint(project, world, nodes);
    if (point) {
      const station = project.kind === 'station';
      add({
        id: `${station ? 'station-project' : 'construction'}:${project.id}`,
        label: station ? 'Station' : 'Construction',
        x: point.x,
        z: point.z,
        importance: station ? 8.5 : 6.5,
        activity: 5,
        suitability: station ? 'night' : 'rain',
      });
    }
  }

  const neighborhoodBins = new Map<
    string,
    {
      x: number;
      z: number;
      count: number;
      activity: number;
      minX: number;
      maxX: number;
      minZ: number;
      maxZ: number;
    }
  >();
  for (const building of buildings) {
    const access = parcelAccess.get(building.parcel_id);
    if (
      building.stage !== 'open' ||
      (connectedNodes.size > 0 && (access === undefined || !connectedNodes.has(access)))
    )
      continue;
    const gx = Math.floor(building.x / 120);
    const gz = Math.floor(building.z / 120);
    const id = `${gx}:${gz}`;
    const bin = neighborhoodBins.get(id) ?? {
      x: 0,
      z: 0,
      count: 0,
      activity: 0,
      minX: building.x,
      maxX: building.x,
      minZ: building.z,
      maxZ: building.z,
    };
    bin.x += building.x;
    bin.z += building.z;
    bin.count++;
    bin.activity += building.business_active === true ? 4 : building.occupancy;
    bin.minX = Math.min(bin.minX, building.x);
    bin.maxX = Math.max(bin.maxX, building.x);
    bin.minZ = Math.min(bin.minZ, building.z);
    bin.maxZ = Math.max(bin.maxZ, building.z);
    neighborhoodBins.set(id, bin);
  }
  // A street shot needs an actual open street, not the centroid of building roofs.
  const streetSegments = world.roads.flatMap((road) => {
    if (road.stage !== 'open' || !road.connected || !['local', 'arterial'].includes(road.kind))
      return [];
    const a = nodes.get(road.from),
      b = nodes.get(road.to);
    if (!a || !b || Math.hypot(b.x - a.x, b.z - a.z) < 24) return [];
    return [{ a, b, roadId: road.id }];
  });
  for (const [id, bin] of neighborhoodBins) {
    if (
      !anchors.some(
        (anchor) =>
          anchor.id.startsWith('founder:') &&
          Math.hypot(anchor.x - bin.x / bin.count, anchor.z - bin.z / bin.count) < 45,
      )
    ) {
      const cx = bin.x / bin.count,
        cz = bin.z / bin.count;
      const candidates: Array<StreetSubject & { distance: number }> = [];
      for (const { a, b, roadId } of streetSegments) {
        const dx = b.x - a.x,
          dz = b.z - a.z,
          length = Math.hypot(dx, dz);
        const t = THREE.MathUtils.clamp(
          ((cx - a.x) * dx + (cz - a.z) * dz) / (length * length),
          12 / length,
          1 - 12 / length,
        );
        const x = a.x + dx * t,
          z = a.z + dz * t,
          distance = Math.hypot(x - cx, z - cz);
        if (distance <= 70)
          candidates.push({
            x,
            z,
            roadId,
            distance,
            forward: { x: dx / length, z: dz / length },
          });
      }
      candidates.sort((a, b) => a.distance - b.distance || a.roadId - b.roadId);
      const streetSubjects = candidates.slice(0, 4),
        street = streetSubjects[0];
      add({
        id: `district:${id}`,
        label: 'Neighborhood street',
        x: street?.x ?? cx,
        z: street?.z ?? cz,
        targetY: street ? 1 : 0,
        streetForward: street?.forward,
        streetRoadId: street?.roadId,
        streetSubjects,
        frameRadius: Math.max(bin.maxX - bin.minX, bin.maxZ - bin.minZ) / 2 + 12,
        importance: 5.5,
        activity: bin.activity,
        suitability: 'all',
      });
    }
  }

  const accessibleNodes = world.nodes.filter((node) => connectedNodes.has(node.id));
  const riverPoint = nearbyRiverAccess(world.terrain.river, accessibleNodes);
  if (riverPoint)
    add({
      id: 'waterfront',
      label: 'Waterfront',
      x: riverPoint.x,
      z: riverPoint.z,
      importance: 4.8,
      activity: world.stats.active_travelers * 0.02,
      suitability: 'rain',
    });

  const skyline = buildSkylineAnchor(skylineCache, world, connectedNodes, parcelAccess);
  if (skyline) add(skyline);

  return retainWatchSubjects(anchors, MAX_ANCHORS);
}

/** Aim from a real nearby street or river edge toward an existing tall-building cluster. */
export function buildSkylineAnchor(
  skylineCache: SkylineClusterCache,
  world: World,
  connectedNodes: Set<number>,
  parcelAccess: Map<number, number>,
): Anchor | null {
  const buildings = world.buildings.filter((building) => {
    if (building.stage !== 'open') return false;
    const access = parcelAccess.get(building.parcel_id);
    return connectedNodes.size === 0 || (access !== undefined && connectedNodes.has(access));
  });
  if (buildings.length === 0) return null;

  const best = skylineCache.get(buildings);
  if (!best) return null;

  // Only the selected local tall-building cluster is framed; distant
  // neighborhoods never inflate the shot.

  let cameraSide: Point | null = null;
  let sideDistance = Infinity;
  for (const point of world.terrain.river) {
    const distance = Math.hypot(point.x - best.x, point.z - best.z);
    if (distance < sideDistance) {
      sideDistance = distance;
      cameraSide = point;
    }
  }
  if (sideDistance > 190) cameraSide = null;

  if (!cameraSide) {
    const nearbyStreet = buildings
      .filter((building) => building.floors < best!.floors * 0.65)
      .map((building) => ({
        building,
        distance: Math.hypot(building.x - best!.x, building.z - best!.z),
      }))
      .filter(({ distance }) => distance >= 16 && distance <= 150)
      .sort((a, b) => a.distance - b.distance)[0];
    if (nearbyStreet) cameraSide = nearbyStreet.building;
  }
  if (!cameraSide) {
    const neighboringBuildings = buildings.filter(
      (building) => Math.hypot(building.x - best!.x, building.z - best!.z) > 35,
    );
    if (neighboringBuildings.length > 0) {
      cameraSide = {
        x:
          neighboringBuildings.reduce((sum, building) => sum + building.x, 0) /
          neighboringBuildings.length,
        z:
          neighboringBuildings.reduce((sum, building) => sum + building.z, 0) /
          neighboringBuildings.length,
      };
    }
  }
  if (!cameraSide || Math.hypot(cameraSide.x - best.x, cameraSide.z - best.z) < 8) return null;

  const horizontal = new THREE.Vector2(cameraSide.x - best.x, cameraSide.z - best.z).normalize();
  const direction = new THREE.Vector3(horizontal.x, 0.38, horizontal.y).normalize();
  const framePoints = skylineBounds(best.frameBuildings);
  const view = direction.clone().negate().normalize();
  const right = new THREE.Vector3().crossVectors(view, UP).normalize();
  const cameraUp = new THREE.Vector3().crossVectors(right, view).normalize();
  let minRight = Infinity,
    maxRight = -Infinity;
  let minUp = Infinity,
    maxUp = -Infinity;
  let minDepth = Infinity,
    maxDepth = -Infinity;
  for (const point of framePoints) {
    const r = point.dot(right),
      u = point.dot(cameraUp),
      d = point.dot(direction);
    minRight = Math.min(minRight, r);
    maxRight = Math.max(maxRight, r);
    minUp = Math.min(minUp, u);
    maxUp = Math.max(maxUp, u);
    minDepth = Math.min(minDepth, d);
    maxDepth = Math.max(maxDepth, d);
  }
  const center = right
    .multiplyScalar((minRight + maxRight) / 2)
    .addScaledVector(cameraUp, (minUp + maxUp) / 2)
    .addScaledVector(direction, (minDepth + maxDepth) / 2);
  const frameRadius = Math.max(...framePoints.map((point) => point.distanceTo(center)));
  const station = world.nodes
    .filter(
      (node) =>
        node.kind === 'station' && (connectedNodes.size === 0 || connectedNodes.has(node.id)),
    )
    .sort(
      (a, b) => Math.hypot(a.x - best!.x, a.z - best!.z) - Math.hypot(b.x - best!.x, b.z - best!.z),
    )[0];
  const stationNearby = station && Math.hypot(station.x - best.x, station.z - best.z) <= 150;
  return {
    id: stationNearby ? `skyline:station:${station!.id}` : 'skyline:district',
    label: stationNearby ? 'Station skyline' : 'Neighborhood skyline',
    x: center.x,
    z: center.z,
    targetY: center.y,
    cameraDirection: direction,
    frameRadius,
    framePoints,
    importance: stationNearby ? 7.4 : 6.7,
    activity: Math.min(10, world.stats.active_travelers * 0.02),
    suitability: 'night',
  };
}

/** Match the visible footprint and rooftop equipment produced by buildingKit. */
export function skylineBounds(buildings: World['buildings']): THREE.Vector3[] {
  const points: THREE.Vector3[] = [];
  for (const building of buildings) {
    const tower =
      building.floors >= 12 && (building.kind === 'office' || building.kind === 'apartment');
    const width =
      Math.max(6, building.width * (building.anchor ? 0.66 : 0.84)) + (tower ? 0.65 : 1.2);
    // Tower entry canopies project about 2m beyond the street facade.
    const depth = Math.max(6, building.depth * 0.84) + (tower ? 4.1 : 1.2);
    const cos = Math.cos(building.rotation),
      sin = Math.sin(building.rotation);
    const base = 0.58 + building.floors * 3.25;
    // Flat roofs carry tanks/screens; 12+ floor towers add a screened crown
    // and a slim aerial reaching just under four meters above their floor count.
    const roofTop = base + (tower ? 4.05 : 2.75);
    for (const localX of [-width / 2, width / 2]) {
      for (const localZ of [-depth / 2, depth / 2]) {
        const x = building.x + cos * localX + sin * localZ;
        const z = building.z - sin * localX + cos * localZ;
        points.push(new THREE.Vector3(x, 0, z), new THREE.Vector3(x, roofTop, z));
      }
    }
  }
  return points;
}

export function projectPoint(
  project: Project,
  world: World,
  nodes: Map<number, Point>,
): Point | null {
  if (project.parcel_id !== null) {
    const parcel = world.parcels.find((candidate) => candidate.id === project.parcel_id);
    if (parcel) return parcel;
  }
  if (project.from !== null && project.to !== null) {
    const a = nodes.get(project.from);
    const b = nodes.get(project.to);
    if (a && b) return { x: (a.x + b.x) / 2, z: (a.z + b.z) / 2 };
  }
  return null;
}

export function nearbyRiverAccess(river: Point[], nodes: Point[]): Point | null {
  if (!river.length || !nodes.length) return null;
  const mid = river[Math.floor(river.length / 2)];
  let best: Point | null = null;
  let bestDistance = Infinity;
  for (const node of nodes) {
    const distance = Math.hypot(node.x - mid.x, node.z - mid.z);
    if (distance < bestDistance) {
      bestDistance = distance;
      best = node;
    }
  }
  return bestDistance <= 180 ? best : null;
}

export function closestRiverPoint(river: Point[], point: Point): Point | null {
  if (river.length === 0) return null;
  let closestX = river[0].x;
  let closestZ = river[0].z;
  let closestDistance = Infinity;
  for (let index = 0; index < river.length - 1; index++) {
    const a = river[index];
    const b = river[index + 1];
    const dx = b.x - a.x;
    const dz = b.z - a.z;
    const lengthSquared = dx * dx + dz * dz;
    const t =
      lengthSquared > 0
        ? THREE.MathUtils.clamp(((point.x - a.x) * dx + (point.z - a.z) * dz) / lengthSquared, 0, 1)
        : 0;
    const x = a.x + dx * t;
    const z = a.z + dz * t;
    const distance = Math.hypot(point.x - x, point.z - z);
    if (distance < closestDistance) {
      closestDistance = distance;
      closestX = x;
      closestZ = z;
    }
  }
  if (river.length === 1) {
    closestX = river[0].x;
    closestZ = river[0].z;
  }
  return { x: closestX, z: closestZ };
}

export function founderFrameRadius(building: World['buildings'][number]): number {
  const footprintRadius = Math.hypot(building.width, building.depth) / 2;
  // Frame the shop, its frontage, and a little sidewalk. The river is a
  // background cue; its geographic distance must not shrink the focal shop.
  return Math.max(24, footprintRadius + 10);
}

export function developedExtent(world: World): Extent {
  const nodes = new Map(world.nodes.map((node) => [node.id, node]));
  const connected = new Set<number>();
  const roadDegree = new Map<number, number>();
  const parcelAccess = new Map(world.parcels.map((parcel) => [parcel.id, parcel.access_node]));
  for (const road of world.roads) {
    if (road.stage === 'open' && road.connected) {
      connected.add(road.from);
      connected.add(road.to);
      roadDegree.set(road.from, (roadDegree.get(road.from) ?? 0) + 1);
      roadDegree.set(road.to, (roadDegree.get(road.to) ?? 0) + 1);
    }
  }
  const developedAccess = new Set<number>();
  const points: Point[] = [];
  for (const building of world.buildings) {
    const access = parcelAccess.get(building.parcel_id);
    if (
      building.anchor ||
      (building.stage === 'open' &&
        (connected.size === 0 || (access !== undefined && connected.has(access))))
    ) {
      points.push(building);
      if (access !== undefined) developedAccess.add(access);
    }
  }
  for (const id of connected) {
    const node = nodes.get(id);
    if (node && (developedAccess.has(id) || (roadDegree.get(id) ?? 0) > 1)) points.push(node);
  }
  if (points.length === 0) return { x: 50, z: 20, span: 120 };
  let minX = Infinity;
  let maxX = -Infinity;
  let minZ = Infinity;
  let maxZ = -Infinity;
  let sumX = 0;
  let sumZ = 0;
  for (const point of points) {
    minX = Math.min(minX, point.x);
    maxX = Math.max(maxX, point.x);
    minZ = Math.min(minZ, point.z);
    maxZ = Math.max(maxZ, point.z);
    sumX += point.x;
    sumZ += point.z;
  }
  return {
    x: sumX / points.length,
    z: sumZ / points.length,
    span: Math.max(120, maxX - minX, maxZ - minZ),
  };
}

export function hash(value: string): number {
  let result = 2166136261;
  for (let i = 0; i < value.length; i++) result = Math.imul(result ^ value.charCodeAt(i), 16777619);
  return result >>> 0;
}

export function cameraObstacleEnvelopes(world: World): CameraObstacleBox[] {
  return world.buildings.map((building) => {
    const tower =
      building.floors >= 12 && (building.kind === 'office' || building.kind === 'apartment');
    const floors = Math.max(1, building.floors);
    const base = 0.58;
    const roofAllowance = building.stage !== 'open' ? 5 : tower ? 4.05 : 2.75;
    // Use full authoritative parcel dimensions, then include facade, canopy,
    // and construction scaffold allowances. The envelope is intentionally
    // conservative; it is a camera obstacle, not a rendered footprint.
    return {
      center: { x: building.x, z: building.z },
      rotation: building.rotation,
      width: building.width + 2.4,
      depth: building.depth + 5.2,
      minY: 0,
      maxY: base + floors * 3.25 + roofAllowance,
    };
  });
}

export function cameraPoint(position: THREE.Vector3): CameraPoint3 {
  return { x: position.x, y: position.y, z: position.z };
}
