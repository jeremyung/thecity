#!/usr/bin/env node

// Offline, approximate 5 m sample of potential 14x12 m roadside parcel centers.
// This is deliberately not an engine admission oracle: it classifies grid
// centers against current geometry and asks whether an ordinary connected road
// lies about one frontage offset away.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { polygonFor } from '../../audit/analyze-density-gaps.mjs';

const MIN = -400;
const MAX = 400;
const STEP = 5;
const WIDTH = 14;
const DEPTH = 12;
const RIVER_HALF_WIDTH = 32;
const RIVER_BANK_BUFFER = 6;
const GRID_RADIUS = Math.hypot(WIDTH / 2, DEPTH / 2);

function pointSegmentDistance(point, a, b) {
  const dx = b[0] - a[0];
  const dz = b[1] - a[1];
  const lengthSquared = dx * dx + dz * dz;
  const t =
    lengthSquared === 0
      ? 0
      : Math.max(0, Math.min(1, ((point[0] - a[0]) * dx + (point[1] - a[1]) * dz) / lengthSquared));
  return Math.hypot(point[0] - a[0] - t * dx, point[1] - a[1] - t * dz);
}

function cross(a, b, c) {
  return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
}

function onSegment(a, b, p) {
  return (
    Math.min(a[0], b[0]) <= p[0] + 1e-8 &&
    p[0] <= Math.max(a[0], b[0]) + 1e-8 &&
    Math.min(a[1], b[1]) <= p[1] + 1e-8 &&
    p[1] <= Math.max(a[1], b[1]) + 1e-8
  );
}

function segmentsIntersect(a, b, c, d) {
  const abC = cross(a, b, c);
  const abD = cross(a, b, d);
  const cdA = cross(c, d, a);
  const cdB = cross(c, d, b);
  return (
    (abC * abD < 0 && cdA * cdB < 0) ||
    (Math.abs(abC) < 1e-8 && onSegment(a, b, c)) ||
    (Math.abs(abD) < 1e-8 && onSegment(a, b, d)) ||
    (Math.abs(cdA) < 1e-8 && onSegment(c, d, a)) ||
    (Math.abs(cdB) < 1e-8 && onSegment(c, d, b))
  );
}

function segmentDistance(a, b, c, d) {
  if (segmentsIntersect(a, b, c, d)) return 0;
  return Math.min(
    pointSegmentDistance(a, c, d),
    pointSegmentDistance(b, c, d),
    pointSegmentDistance(c, a, b),
    pointSegmentDistance(d, a, b),
  );
}

function pointInPolygon(point, polygon) {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i, i += 1) {
    const [xi, zi] = polygon[i];
    const [xj, zj] = polygon[j];
    if (
      zi > point[1] !== zj > point[1] &&
      point[0] < ((xj - xi) * (point[1] - zi)) / (zj - zi) + xi
    )
      inside = !inside;
  }
  return inside;
}

function polygonSegmentDistance(poly, a, b) {
  if (pointInPolygon(a, poly) || pointInPolygon(b, poly)) return 0;
  let result = Infinity;
  for (let i = 0; i < poly.length; i += 1) {
    result = Math.min(result, segmentDistance(a, b, poly[i], poly[(i + 1) % poly.length]));
  }
  return result;
}

function rectBounds(rect) {
  const c = Math.cos(rect.rotation);
  const s = Math.sin(rect.rotation);
  const halfX = (Math.abs(c) * rect.width + Math.abs(s) * rect.depth) / 2;
  const halfZ = (Math.abs(s) * rect.width + Math.abs(c) * rect.depth) / 2;
  return [rect.x - halfX, rect.z - halfZ, rect.x + halfX, rect.z + halfZ];
}

function broadphaseMayOverlap(aBounds, bBounds, margin = 4) {
  return !(
    aBounds[2] + margin < bBounds[0] ||
    bBounds[2] + margin < aBounds[0] ||
    aBounds[3] + margin < bBounds[1] ||
    bBounds[3] + margin < aBounds[1]
  );
}

function rectsOverlap(a, b, gap = 0) {
  const [aminX, aminZ, amaxX, amaxZ] = rectBounds(a);
  const [bminX, bminZ, bmaxX, bmaxZ] = rectBounds(b);
  if (amaxX + gap < bminX || bmaxX + gap < aminX || amaxZ + gap < bminZ || bmaxZ + gap < aminZ)
    return false;
  const axes = [
    [Math.cos(a.rotation), -Math.sin(a.rotation)],
    [Math.sin(a.rotation), Math.cos(a.rotation)],
    [Math.cos(b.rotation), -Math.sin(b.rotation)],
    [Math.sin(b.rotation), Math.cos(b.rotation)],
  ];
  const dx = b.x - a.x;
  const dz = b.z - a.z;
  for (const [ux, uz] of axes) {
    const separation = Math.abs(dx * ux + dz * uz);
    const ax = [Math.cos(a.rotation), -Math.sin(a.rotation)];
    const az = [Math.sin(a.rotation), Math.cos(a.rotation)];
    const bx = [Math.cos(b.rotation), -Math.sin(b.rotation)];
    const bz = [Math.sin(b.rotation), Math.cos(b.rotation)];
    const ra =
      (a.width / 2 + gap / 2) * Math.abs(ax[0] * ux + ax[1] * uz) +
      (a.depth / 2 + gap / 2) * Math.abs(az[0] * ux + az[1] * uz);
    const rb =
      (b.width / 2 + gap / 2) * Math.abs(bx[0] * ux + bx[1] * uz) +
      (b.depth / 2 + gap / 2) * Math.abs(bz[0] * ux + bz[1] * uz);
    if (separation >= ra + rb) return false;
  }
  return true;
}

function nearestRoad(point, roads, nodes) {
  let best = null;
  for (const road of roads) {
    const a = nodes.get(road.from);
    const b = nodes.get(road.to);
    if (!a || !b) continue;
    const p = [point[0], point[1]];
    const aa = [a.x, a.z];
    const bb = [b.x, b.z];
    const dx = bb[0] - aa[0];
    const dz = bb[1] - aa[1];
    const denominator = dx * dx + dz * dz;
    const t =
      denominator === 0
        ? 0
        : Math.max(0, Math.min(1, ((p[0] - aa[0]) * dx + (p[1] - aa[1]) * dz) / denominator));
    const closest = [aa[0] + dx * t, aa[1] + dz * t];
    const distance = Math.hypot(p[0] - closest[0], p[1] - closest[1]);
    if (!best || distance < best.distance) best = { road, a, b, t, closest, distance };
  }
  return best;
}

export function analyzeOpportunity(path) {
  const world = JSON.parse(readFileSync(path, 'utf8'));
  const seed = world.seed;
  const bend = (seed % 5) * 9 - 18;
  const river = [
    [-112 + bend, -400],
    [-98 + bend, -190],
    [-112 + bend, 20],
    [-94 + bend, 210],
    [-109 + bend, 400],
  ];
  const nodes = new Map(world.nodes.map((node) => [node.id, node]));
  const openRoads = world.roads.filter((road) => road.stage === 'open');
  const ordinaryRoads = openRoads.filter((road) => !['rail', 'bridge'].includes(road.kind));
  const connectedOrdinary = ordinaryRoads.filter((road) => road.connected);
  const buildings = new Map(world.buildings.map((building) => [building.parcel_id, building]));
  const frontages = new Map(
    world.parcel_frontages.map((frontage) => [frontage.parcel_id, frontage]),
  );
  const parcels = world.parcels.map((parcel) => {
    const building = buildings.get(parcel.id);
    const frontage = frontages.get(parcel.id);
    let rotation = parcel.rotation;
    if (rotation == null) {
      rotation = building?.rotation;
      if (rotation == null && frontage) {
        const road = world.roads.find((item) => item.id === frontage.road_id);
        const a = road && nodes.get(road.from);
        const b = road && nodes.get(road.to);
        if (a && b)
          rotation = Math.atan2(frontage.side * (b.z - a.z), -frontage.side * (b.x - a.x));
      }
    }
    const actual = { ...parcel, rotation: rotation ?? 0 };
    const legacy = parcel.rotation == null ? { ...parcel, rotation: 0 } : null;
    return {
      ...actual,
      legacy,
      bounds: rectBounds(actual),
      legacyBounds: legacy && rectBounds(legacy),
    };
  });
  const riverSegments = river.slice(1).map((b, i) => [river[i], b]);
  const counts = Object.fromEntries(
    [
      'samples',
      'boundary',
      'protected',
      'river_bank',
      'existing_oriented_envelope',
      'existing_legacy_envelope',
      'legacy_parcel_only',
      'oriented_parcel_only',
      'exact_road_footprint_clearance',
      'legacy_road_center_radius',
      'legacy_center_only',
      'footprint_only',
      'open_road_clearance',
      'no_near_frontage',
      'near_connected_frontage',
      'near_open_unconnected_frontage',
      'exclusive_terrain',
      'exclusive_parcel',
      'exclusive_road',
      'exclusive_access',
      'clear_connected_frontage_center',
      'clear_unconnected_frontage_center',
    ].map((key) => [key, 0]),
  );
  const cells = Array.from({ length: 8 }, () =>
    Array.from({ length: 8 }, () => ({ samples: 0, spare_near_frontage: 0 })),
  );

  for (let iz = 0; iz < 160; iz += 1) {
    const z = MIN + (iz + 0.5) * STEP;
    for (let ix = 0; ix < 160; ix += 1) {
      const x = MIN + (ix + 0.5) * STEP;
      counts.samples += 1;
      const cell = cells[Math.floor(iz / 20)][Math.floor(ix / 20)];
      cell.samples += 1;
      const point = [x, z];
      const nearest = nearestRoad(point, ordinaryRoads, nodes);
      const dxToRoad = nearest ? nearest.closest[0] - x : 0;
      const dzToRoad = nearest ? nearest.closest[1] - z : 1;
      const candidate = {
        x,
        z,
        width: WIDTH,
        depth: DEPTH,
        rotation: Math.atan2(dxToRoad, dzToRoad),
      };
      const candidatePoly = polygonFor(candidate);
      const corners = candidatePoly;
      const boundaryBlocked = corners.some(
        ([cx, cz]) => cx < MIN + 4 || cx > MAX - 4 || cz < MIN + 4 || cz > MAX - 4,
      );
      const protectedBlocked = rectsOverlap(candidate, {
        x: -305,
        z: -310,
        width: 90,
        depth: 80,
        rotation: 0,
      });
      const terrainBlocked = boundaryBlocked || protectedBlocked;
      if (boundaryBlocked) counts.boundary += 1;
      if (protectedBlocked) counts.protected += 1;
      const riverDistance = Math.min(
        ...riverSegments.map(([a, b]) => pointSegmentDistance(point, a, b)),
      );
      const riverBlocked = riverDistance < RIVER_HALF_WIDTH + RIVER_BANK_BUFFER + GRID_RADIUS;
      if (riverBlocked) counts.river_bank += 1;

      const candidateBounds = rectBounds(candidate);
      let actualParcelBlocked = false;
      let legacyEnvelopeBlocked = false;
      for (const parcel of parcels) {
        if (
          !actualParcelBlocked &&
          broadphaseMayOverlap(candidateBounds, parcel.bounds) &&
          rectsOverlap(candidate, parcel, 2)
        )
          actualParcelBlocked = true;
        if (
          !legacyEnvelopeBlocked &&
          parcel.legacy &&
          broadphaseMayOverlap(candidateBounds, parcel.legacyBounds) &&
          rectsOverlap(candidate, parcel.legacy, 2)
        )
          legacyEnvelopeBlocked = true;
        if (actualParcelBlocked && legacyEnvelopeBlocked) break;
      }
      if (actualParcelBlocked) counts.existing_oriented_envelope += 1;
      if (legacyEnvelopeBlocked) counts.existing_legacy_envelope += 1;
      if (legacyEnvelopeBlocked && !actualParcelBlocked) counts.legacy_parcel_only += 1;
      if (actualParcelBlocked && !legacyEnvelopeBlocked) counts.oriented_parcel_only += 1;

      let exactRoadBlocked = false;
      let legacyCenterBlocked = false;
      for (const road of openRoads) {
        const a = nodes.get(road.from);
        const b = nodes.get(road.to);
        if (!a || !b) continue;
        const centerDistance = pointSegmentDistance(point, [a.x, a.z], [b.x, b.z]);
        if (centerDistance < Math.max(WIDTH, DEPTH) / 2 + 4) legacyCenterBlocked = true;
        if (
          centerDistance < 20 &&
          polygonSegmentDistance(candidatePoly, [a.x, a.z], [b.x, b.z]) < 5
        )
          exactRoadBlocked = true;
        if (exactRoadBlocked && legacyCenterBlocked) break;
      }
      const blockedRoad = exactRoadBlocked || legacyCenterBlocked;
      const nearAccess =
        nearest &&
        nearest.distance >= 12.5 &&
        nearest.distance <= 17.5 &&
        nearest.t >= 0.03 &&
        nearest.t <= 0.97;
      if (blockedRoad) counts.open_road_clearance += 1;
      if (exactRoadBlocked) counts.exact_road_footprint_clearance += 1;
      if (legacyCenterBlocked) counts.legacy_road_center_radius += 1;
      if (legacyCenterBlocked && !exactRoadBlocked) counts.legacy_center_only += 1;
      if (exactRoadBlocked && !legacyCenterBlocked) counts.footprint_only += 1;
      if (!nearAccess) counts.no_near_frontage += 1;
      else if (nearest.road.connected) {
        counts.near_connected_frontage += 1;
      } else {
        counts.near_open_unconnected_frontage += 1;
      }
      if (terrainBlocked || riverBlocked) counts.exclusive_terrain += 1;
      else if (actualParcelBlocked || legacyEnvelopeBlocked) counts.exclusive_parcel += 1;
      else if (blockedRoad) counts.exclusive_road += 1;
      else if (!nearAccess || !nearest.road.connected) counts.exclusive_access += 1;
      else {
        counts.clear_connected_frontage_center += 1;
        cell.spare_near_frontage += 1;
      }
    }
  }
  const floors = world.buildings.map((building) => building.floors).sort((a, b) => a - b);
  const quantile = (q) => floors[Math.floor((floors.length - 1) * q)];
  const dimensionHistogram = {};
  for (const parcel of world.parcels) {
    const key = `${parcel.width.toFixed(2)}x${parcel.depth.toFixed(2)}`;
    dimensionHistogram[key] = (dimensionHistogram[key] ?? 0) + 1;
  }
  return {
    input: path,
    caveat:
      'Approximate grid-center classification, not the Rust parcel admission oracle or a buildable-land estimate. Grid tests one 14x12m footprint facing the nearest ordinary road; frontage is counted only when the candidate center is 12.5–17.5m from that centerline and projects away from its endpoints. Clearance and access are mirrored for diagnosis only. Each sample represents a 25m² center bin, not land area.',
    simulation: {
      seed,
      tick: world.tick,
      demand: world.demand,
      buildings: world.buildings.length,
      parcels: world.parcels.length,
      openRoads: openRoads.length,
      ordinaryRoads: ordinaryRoads.length,
      connectedOrdinaryRoads: connectedOrdinary.length,
      openUnbuiltParcels: world.parcels.filter((p) => !buildings.has(p.id) && p.reservation == null)
        .length,
    },
    sample: {
      stepM: STEP,
      candidateFootprintM: [WIDTH, DEPTH],
      counts,
      cellSpareNearFrontage: cells,
    },
    fabric: {
      floors: { median: quantile(0.5), p90: quantile(0.9), max: floors.at(-1) },
      parcelDimensions: dimensionHistogram,
    },
  };
}

export function runGeometryChecks() {
  const orientations = [0, Math.PI / 8, Math.PI / 4, Math.PI / 2, (3 * Math.PI) / 4];
  for (const aRotation of orientations) {
    for (const bRotation of orientations) {
      for (let dx = -20; dx <= 20; dx += 2) {
        for (let dz = -20; dz <= 20; dz += 2) {
          const a = { x: 0, z: 0, width: 14, depth: 12, rotation: aRotation };
          const b = { x: dx, z: dz, width: 13, depth: 16, rotation: bRotation };
          if (rectsOverlap(a, b, 2)) {
            assert.ok(
              broadphaseMayOverlap(rectBounds(a), rectBounds(b)),
              'broadphase must retain every SAT overlap',
            );
          }
        }
      }
    }
  }
  assert.equal(
    rectsOverlap(
      { x: 0, z: 0, width: 14, depth: 12, rotation: Math.PI / 4 },
      { x: 0, z: 0, width: 14, depth: 12, rotation: 0 },
      2,
    ),
    true,
  );
  return 'opportunity geometry sanity checks passed';
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv[2] === '--self-test') {
    console.log(runGeometryChecks());
    process.exit(0);
  }
  if (!process.argv[2]) {
    console.error(
      'Usage: node scripts/diagnose/diagnose-density-opportunity.mjs <world-save.json> [world-save.json ...]',
    );
    process.exit(2);
  }
  for (const path of process.argv.slice(2))
    console.log(JSON.stringify(analyzeOpportunity(path), null, 2));
}
