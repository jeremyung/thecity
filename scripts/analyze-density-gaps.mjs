#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';

export function extent(items) {
  if (items.length === 0) return null;
  return {
    minX: Math.min(...items.map((item) => item.x)),
    maxX: Math.max(...items.map((item) => item.x)),
    minZ: Math.min(...items.map((item) => item.z)),
    maxZ: Math.max(...items.map((item) => item.z)),
  };
}

export function polygonFor(building) {
  const angle = building.rotation ?? 0;
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const halfWidth = building.width / 2;
  const halfDepth = building.depth / 2;
  return [
    [-halfWidth, -halfDepth],
    [halfWidth, -halfDepth],
    [halfWidth, halfDepth],
    [-halfWidth, halfDepth],
  ].map(([x, z]) => [
    // Match THREE.Object3D.rotation.y: x' = cos*x + sin*z, z' = -sin*x + cos*z.
    building.x + x * cos + z * sin,
    building.z - x * sin + z * cos,
  ]);
}

function pointSegmentDistance(point, a, b) {
  const dx = b[0] - a[0];
  const dz = b[1] - a[1];
  const lengthSquared = dx * dx + dz * dz;
  const t = lengthSquared === 0
    ? 0
    : Math.max(0, Math.min(1, ((point[0] - a[0]) * dx + (point[1] - a[1]) * dz) / lengthSquared));
  return Math.hypot(point[0] - a[0] - t * dx, point[1] - a[1] - t * dz);
}

function cross(a, b, c) {
  return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
}

function onSegment(a, b, point) {
  const epsilon = 1e-9;
  return point[0] >= Math.min(a[0], b[0]) - epsilon
    && point[0] <= Math.max(a[0], b[0]) + epsilon
    && point[1] >= Math.min(a[1], b[1]) - epsilon
    && point[1] <= Math.max(a[1], b[1]) + epsilon;
}

function segmentDistance(a, b, c, d) {
  const abC = cross(a, b, c);
  const abD = cross(a, b, d);
  const cdA = cross(c, d, a);
  const cdB = cross(c, d, b);
  if ((abC * abD < 0 && cdA * cdB < 0)
      || (Math.abs(abC) < 1e-9 && onSegment(a, b, c))
      || (Math.abs(abD) < 1e-9 && onSegment(a, b, d))
      || (Math.abs(cdA) < 1e-9 && onSegment(c, d, a))
      || (Math.abs(cdB) < 1e-9 && onSegment(c, d, b))) {
    return 0;
  }
  return Math.min(
    pointSegmentDistance(a, c, d),
    pointSegmentDistance(b, c, d),
    pointSegmentDistance(c, a, b),
    pointSegmentDistance(d, a, b),
  );
}

export function polygonDistance(a, b) {
  if (pointInPolygon(a[0], b) || pointInPolygon(b[0], a)) return 0;
  let minimum = Infinity;
  for (let i = 0; i < a.length; i += 1) {
    for (let j = 0; j < b.length; j += 1) {
      minimum = Math.min(minimum, segmentDistance(
        a[i], a[(i + 1) % a.length], b[j], b[(j + 1) % b.length],
      ));
    }
  }
  return minimum;
}

export function runGeometryChecks() {
  const rotated = polygonFor({ x: 0, z: 0, width: 4, depth: 2, rotation: Math.PI / 2 });
  assert.deepEqual(rotated.map(([x]) => Math.round(x)).sort((a, b) => a - b), [-1, -1, 1, 1]);
  assert.deepEqual(rotated.map(([, z]) => Math.round(z)).sort((a, b) => a - b), [-2, -2, 2, 2]);

  const enclosing = polygonFor({ x: 0, z: 0, width: 10, depth: 10, rotation: 0 });
  const contained = polygonFor({ x: 0, z: 0, width: 2, depth: 2, rotation: Math.PI / 4 });
  assert.equal(polygonDistance(enclosing, contained), 0, 'a contained parcel has zero envelope gap');

  const touching = polygonFor({ x: 6, z: 0, width: 2, depth: 2, rotation: 0 });
  assert.equal(polygonDistance(enclosing, touching), 0, 'touching parcel edges have zero gap');

  const separated = polygonFor({ x: 7, z: 0, width: 2, depth: 2, rotation: 0 });
  assert.equal(polygonDistance(enclosing, separated), 1, 'separated axis-aligned parcel edges report their gap');
  return 'oriented rectangle geometry checks passed';
}

function pointInPolygon(point, polygon) {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i, i += 1) {
    const [xi, zi] = polygon[i];
    const [xj, zj] = polygon[j];
    if ((zi > point[1]) !== (zj > point[1])
        && point[0] < ((xj - xi) * (point[1] - zi)) / (zj - zi) + xi) {
      inside = !inside;
    }
  }
  return inside;
}

function quantiles(values) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const quantile = (fraction) => sorted[Math.floor((sorted.length - 1) * fraction)];
  return {
    count: sorted.length,
    min: sorted[0],
    p10: quantile(0.1),
    median: quantile(0.5),
    p90: quantile(0.9),
    max: sorted.at(-1),
    over10m: sorted.filter((value) => value > 10).length,
    over25m: sorted.filter((value) => value > 25).length,
    over50m: sorted.filter((value) => value > 50).length,
  };
}

export function analyzeSave(path) {
const world = JSON.parse(readFileSync(path, 'utf8'));
const buildings = world.buildings ?? [];
const openBuildings = buildings.filter((building) => building.stage === 'open');
const roads = world.roads ?? [];
const openRoads = roads.filter((road) => road.stage === 'open');
const ordinaryRoads = openRoads.filter((road) => !['rail', 'bridge'].includes(road.kind));
const nodesById = new Map((world.nodes ?? []).map((node) => [node.id, node]));
const parcels = world.parcels ?? [];
const parcelIdsWithBuilding = new Set(buildings.map((building) => building.parcel_id));
const frontages = world.parcel_frontages ?? [];

const parcelArea = openBuildings.reduce((sum, building) => sum + building.width * building.depth, 0);
const buildingBounds = extent(openBuildings);
const boundsArea = buildingBounds
  ? (buildingBounds.maxX - buildingBounds.minX) * (buildingBounds.maxZ - buildingBounds.minZ)
  : 0;
const roadPoints = openRoads.flatMap((road) => [nodesById.get(road.from), nodesById.get(road.to)]).filter(Boolean);
const roadLengths = openRoads.reduce((sum, road) => sum + road.length, 0);
const ordinaryLength = ordinaryRoads.reduce((sum, road) => sum + road.length, 0);
const frontageCounts = new Map();
for (const frontage of frontages) {
  frontageCounts.set(frontage.road_id, (frontageCounts.get(frontage.road_id) ?? 0) + 1);
}
const ordinaryFrontageDistribution = {
  zeroOrOne: ordinaryRoads.filter((road) => (frontageCounts.get(road.id) ?? 0) <= 1).length,
  atMostTwo: ordinaryRoads.filter((road) => (frontageCounts.get(road.id) ?? 0) <= 2).length,
  threeOrMore: ordinaryRoads.filter((road) => (frontageCounts.get(road.id) ?? 0) >= 3).length,
};

const polygons = openBuildings.map(polygonFor);
const nearestEnvelopeGaps = [];
for (let i = 0; i < polygons.length; i += 1) {
  let nearest = Infinity;
  for (let j = 0; j < polygons.length; j += 1) {
    if (i !== j) nearest = Math.min(nearest, polygonDistance(polygons[i], polygons[j]));
  }
  if (Number.isFinite(nearest)) nearestEnvelopeGaps.push(nearest);
}

const domainMin = -400;
const domainMax = 400;
const tiles = [];
for (let z = domainMin; z < domainMax; z += 100) {
  for (let x = domainMin; x < domainMax; x += 100) {
    tiles.push({
      minX: x,
      minZ: z,
      buildingCenters: openBuildings.filter((building) =>
        building.x >= x && building.x < x + 100 && building.z >= z && building.z < z + 100).length,
    });
  }
}

const plan = world.neighborhood_plan ?? null;
const planRoads = plan
  ? plan.segments.map(([from, to]) => openRoads.find((road) =>
    (road.from === from && road.to === to) || (road.from === to && road.to === from))).filter(Boolean)
  : [];
const planRoadIds = new Set(planRoads.map((road) => road.id));
const planFrontages = frontages.filter((frontage) => planRoadIds.has(frontage.road_id));

const round = (value) => Math.round(value * 100) / 100;
const report = {
  input: path,
  simulation: {
    seed: world.seed,
    tick: world.tick,
    developmentRate: world.development_rate,
    population: world.population,
    buildings: buildings.length,
    openBuildings: openBuildings.length,
    parcels: parcels.length,
    unbuiltParcels: parcels.filter((parcel) => !parcelIdsWithBuilding.has(parcel.id)).length,
  },
  footprint: {
    parcelEnvelopeAreaM2: round(parcelArea),
    grossDomainAreaM2: (domainMax - domainMin) ** 2,
    grossParcelEnvelopeCoveragePercent: round(parcelArea / 6400),
    buildingCenterBoundsM: buildingBounds,
    buildingCenterBoundsAreaM2: round(boundsArea),
    parcelEnvelopeCoverageWithinCenterBoundsPercent: round(boundsArea ? parcelArea / boundsArea * 100 : 0),
    developableLandGeometryAvailable: false,
  },
  nearestParcelEnvelopeEdgeGapM: quantiles(nearestEnvelopeGaps),
  roads: {
    allOpenCount: openRoads.length,
    ordinaryCount: ordinaryRoads.length,
    ordinaryKinds: Object.fromEntries(
      Object.entries(Object.groupBy(ordinaryRoads, (road) => road.kind)).map(([kind, items]) => [kind, items.length]),
    ),
    bridgeCount: openRoads.filter((road) => road.kind === 'bridge').length,
    railCount: openRoads.filter((road) => road.kind === 'rail').length,
    allOpenCenterlineLengthM: round(roadLengths),
    ordinaryCenterlineLengthM: round(ordinaryLength),
    ordinaryMeanSegmentLengthM: ordinaryRoads.length ? round(ordinaryLength / ordinaryRoads.length) : null,
    openRoadEndpointBoundsM: extent(roadPoints),
    ordinaryFrontageRecords: ordinaryRoads.reduce((sum, road) => sum + (frontageCounts.get(road.id) ?? 0), 0),
    ordinaryFrontagesPerRoad: ordinaryRoads.length
      ? round(ordinaryRoads.reduce((sum, road) => sum + (frontageCounts.get(road.id) ?? 0), 0) / ordinaryRoads.length)
      : null,
    ordinaryFrontageDistribution,
  },
  domain100mTiles: {
    total: tiles.length,
    withNoBuildingCenters: tiles.filter((tile) => tile.buildingCenters === 0).length,
    minimumBuildingsInOccupiedTile: Math.min(...tiles.filter((tile) => tile.buildingCenters > 0).map((tile) => tile.buildingCenters)),
    maximumBuildingsInTile: Math.max(...tiles.map((tile) => tile.buildingCenters)),
    rowsSouthToNorth: Array.from({ length: 8 }, (_, row) => tiles.slice(row * 8, row * 8 + 8).map((tile) => tile.buildingCenters)),
  },
  neighborhoodPlan: plan ? {
    anchorNode: plan.anchor_node,
    segments: plan.segments.length,
    completedSegments: plan.next_segment,
    complete: plan.next_segment >= plan.segments.length,
    openPlanRoads: planRoads.length,
    frontageCount: planFrontages.length,
    frontageCountsByRoad: planRoads.map((road) => ({ roadId: road.id, lengthM: road.length, frontages: frontageCounts.get(road.id) ?? 0 })),
  } : null,
  method: {
    nearestGap: 'For each open building, construct its width×depth oriented parcel rectangle from center and rotation, compute distance between all polygon edge pairs (zero for intersecting/touching polygons), then retain the minimum distance to another envelope.',
    caveat: 'Parcel envelopes approximate land allocation, not visible building-wall area. The save does not contain developable-land geometry or rejected parcel-candidate history.',
  },
};

return report;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const inputPath = process.argv[2];
  if (inputPath === '--self-test') {
    console.log(runGeometryChecks());
    process.exit(0);
  }
  if (!inputPath) {
    console.error('Usage: node scripts/analyze-density-gaps.mjs <world-save.json>');
    process.exit(2);
  }
  console.log(JSON.stringify(analyzeSave(inputPath), null, 2));
}
