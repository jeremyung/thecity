#!/usr/bin/env node

// Read-only feasibility probe for the largest void in a matching saved world.
// It retraces planar ordinary-road faces, rasterizes free face cells, then
// searches for a clear access corridor and lots. This is diagnostic only.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const savePath = resolve(root, 'validation/early-reservation-wasm-87231-save.json');
const outputPath = resolve(root, 'validation/face-access-corridor-diagnostic.json');
const mapPath = resolve(root, 'validation/face-access-corridor-map.svg');
const STEP = 2;
const PARCEL_CLEARANCE = 5;
const LOT_GAP = 2;
const ROAD_SEPARATION = 8;
const LOT_W = 8;
const LOT_D = 8;
const SITE_OFFSET = 12;

const saveText = await readFile(savePath, 'utf8');
const world = JSON.parse(saveText);
const saveSha256 = createHash('sha256').update(saveText).digest('hex');
const nodes = new Map(world.nodes.map((n) => [n.id, n]));
const allRoads = world.roads.filter((r) => r.stage === 'open' && r.connected && !['rail', 'bridge'].includes(r.kind))
  .map((road) => ({ ...road, a: nodes.get(road.from), b: nodes.get(road.to) }))
  .filter((r) => r.a && r.b);
const buildings = new Map(world.buildings.map((b) => [b.parcel_id, b]));
const frontages = new Map(world.parcel_frontages.map((f) => [f.parcel_id, f]));

const dist = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
function pointSegmentDistance(p, a, b) {
  const dx = b.x - a.x, dz = b.z - a.z;
  const den = dx * dx + dz * dz;
  const t = den === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.z - a.z) * dz) / den));
  return Math.hypot(p.x - (a.x + dx * t), p.z - (a.z + dz * t));
}
function cross(a, b, c) { return (b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x); }
function onSegment(a, b, p) {
  return p.x >= Math.min(a.x, b.x) - 1e-8 && p.x <= Math.max(a.x, b.x) + 1e-8
    && p.z >= Math.min(a.z, b.z) - 1e-8 && p.z <= Math.max(a.z, b.z) + 1e-8;
}
function segmentsIntersect(a, b, c, d) {
  const abC = cross(a, b, c), abD = cross(a, b, d), cdA = cross(c, d, a), cdB = cross(c, d, b);
  return (abC * abD < 0 && cdA * cdB < 0)
    || (Math.abs(abC) < 1e-8 && onSegment(a, b, c)) || (Math.abs(abD) < 1e-8 && onSegment(a, b, d))
    || (Math.abs(cdA) < 1e-8 && onSegment(c, d, a)) || (Math.abs(cdB) < 1e-8 && onSegment(c, d, b));
}
function segmentDistance(a, b, c, d) {
  if (segmentsIntersect(a, b, c, d)) return 0;
  return Math.min(pointSegmentDistance(a, c, d), pointSegmentDistance(b, c, d),
    pointSegmentDistance(c, a, b), pointSegmentDistance(d, a, b));
}
function rectCorners(rect) {
  const c = Math.cos(rect.rotation), s = Math.sin(rect.rotation);
  return [[-rect.width / 2, -rect.depth / 2], [rect.width / 2, -rect.depth / 2],
    [rect.width / 2, rect.depth / 2], [-rect.width / 2, rect.depth / 2]].map(([x, z]) => ({
    x: rect.x + x * c + z * s,
    z: rect.z - x * s + z * c,
  }));
}
function pointInPolygon(p, poly) {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const a = poly[i], b = poly[j];
    if ((a.z > p.z) !== (b.z > p.z) && p.x < ((b.x - a.x) * (p.z - a.z)) / (b.z - a.z) + a.x) inside = !inside;
  }
  return inside;
}
function pointRectDistance(p, rect) {
  const poly = rectCorners(rect);
  if (pointInPolygon(p, poly)) return 0;
  let best = Infinity;
  for (let i = 0; i < 4; i++) best = Math.min(best, pointSegmentDistance(p, poly[i], poly[(i + 1) % 4]));
  return best;
}
function rectSegmentDistance(rect, a, b) {
  const poly = rectCorners(rect);
  if (pointInPolygon(a, poly) || pointInPolygon(b, poly)) return 0;
  let best = Infinity;
  for (let i = 0; i < 4; i++) best = Math.min(best, segmentDistance(a, b, poly[i], poly[(i + 1) % 4]));
  return best;
}
function rectsOverlap(a, b, gap = 0) {
  const ap = rectCorners(a), bp = rectCorners(b);
  const axes = [];
  for (const poly of [ap, bp]) for (let i = 0; i < 2; i++) {
    const p = poly[i], q = poly[i + 1];
    const dx = q.x - p.x, dz = q.z - p.z, len = Math.hypot(dx, dz) || 1;
    axes.push({ x: -dz / len, z: dx / len });
  }
  for (const axis of axes) {
    const project = (poly) => poly.map((p) => p.x * axis.x + p.z * axis.z);
    const aa = project(ap), bb = project(bp);
    if (Math.max(...aa) + gap <= Math.min(...bb) || Math.max(...bb) + gap <= Math.min(...aa)) return false;
  }
  return true;
}
function rotationForFrontage(a, b, side) {
  return Math.atan2(side * (b.z - a.z), -side * (b.x - a.x));
}
function getParcelFootprints() {
  return world.parcels.map((parcel) => {
    const building = buildings.get(parcel.id);
    const frontage = frontages.get(parcel.id);
    const road = frontage && world.roads.find((r) => r.id === frontage.road_id);
    const a = road && nodes.get(road.from), b = road && nodes.get(road.to);
    const renderRotation = parcel.rotation ?? building?.rotation
      ?? (frontage && a && b ? rotationForFrontage(a, b, frontage.side) : 0);
    const actual = { x: parcel.x, z: parcel.z, width: parcel.width, depth: parcel.depth, rotation: renderRotation };
    const footprints = [actual];
    if (parcel.rotation == null) footprints.push({ ...actual, rotation: 0 });
    const bounds = footprints.flatMap(rectCorners);
    return { id: parcel.id, footprints, bounds };
  });
}
const parcelFootprints = getParcelFootprints();
const clearCellMemo = new Map();

// Mirror the bounded planar half-edge face walk from engine/src/road_faces.rs
// for this offline diagnostic. The mature checkpoint is small enough for the
// same 512-node/edge envelope; fail closed on crossings or invalid faces.
function traceFaces() {
  const usedNodeIds = new Set(allRoads.flatMap((road) => [road.from, road.to]));
  const graphNodes = [...usedNodeIds].map((id) => world.nodes.find((n) => n.id === id)).filter(Boolean)
    .sort((a, b) => a.id - b.id);
  if (graphNodes.length > 512 || allRoads.length > 512) throw new Error('ordinary road graph exceeds face tracer cap');
  const index = new Map(graphNodes.map((n, i) => [n.id, i]));
  const edgeData = allRoads.map((road) => ({ id: road.id, from: index.get(road.from), to: index.get(road.to), road }));
  for (let i = 0; i < edgeData.length; i++) for (let j = i + 1; j < edgeData.length; j++) {
    const a = edgeData[i], b = edgeData[j];
    if ([a.from, a.to].some((n) => n === b.from || n === b.to)) continue;
    const d = segmentDistance(graphNodes[a.from], graphNodes[a.to], graphNodes[b.from], graphNodes[b.to]);
    if (d <= 1e-5) throw new Error(`ordinary road graph is nonplanar: roads ${a.id} and ${b.id}, separation ${d}`);
  }
  const adjacency = Array.from({ length: graphNodes.length }, () => []);
  edgeData.forEach((e, ei) => { adjacency[e.from].push([e.to, ei]); adjacency[e.to].push([e.from, ei]); });
  const entered = new Array(graphNodes.length).fill(0), low = new Array(graphNodes.length).fill(0), bridges = new Set();
  let time = 0;
  function visit(v, parentEdge) {
    entered[v] = low[v] = ++time;
    for (const [next, edgeIndex] of adjacency[v].slice().sort((a, b) => a[0] - b[0] || edgeData[a[1]].id - edgeData[b[1]].id)) {
      if (edgeIndex === parentEdge) continue;
      if (!entered[next]) {
        visit(next, edgeIndex); low[v] = Math.min(low[v], low[next]);
        if (low[next] > entered[v]) bridges.add(edgeIndex);
      } else low[v] = Math.min(low[v], entered[next]);
    }
  }
  for (let i = 0; i < graphNodes.length; i++) if (!entered[i]) visit(i, -1);
  const outgoing = Array.from({ length: graphNodes.length }, () => []);
  const halves = [], halfIndex = new Map();
  edgeData.forEach((e, ei) => {
    if (bridges.has(ei)) return;
    const a = graphNodes[e.from], b = graphNodes[e.to], f = halves.length;
    const forward = { edge: ei, from: e.from, to: e.to, angle: Math.atan2(b.z - a.z, b.x - a.x) };
    const reverse = { edge: ei, from: e.to, to: e.from, angle: Math.atan2(a.z - b.z, a.x - b.x) };
    halves.push(forward, reverse); outgoing[e.from].push(forward); outgoing[e.to].push(reverse); halfIndex.set(ei, [f, f + 1]);
  });
  for (const list of outgoing) list.sort((a, b) => a.angle - b.angle || graphNodes[a.to].id - graphNodes[b.to].id || edgeData[a.edge].id - edgeData[b.edge].id);
  const visited = new Set(), faces = [];
  for (let start = 0; start < halves.length; start++) {
    if (visited.has(start)) continue;
    let current = start; const vertices = [], edgeIndices = [];
    for (let guard = 0; guard < halves.length + 1; guard++) {
      if (visited.has(current)) { if (current !== start) throw new Error('invalid half-edge embedding'); break; }
      visited.add(current);
      const half = halves[current]; vertices.push(half.from); edgeIndices.push(half.edge);
      const list = outgoing[half.to];
      const reversePos = list.findIndex((h) => h.edge === half.edge && h.to === half.from);
      if (reversePos < 0) throw new Error('half-edge reverse missing');
      const next = list[(reversePos + list.length - 1) % list.length];
      const pair = halfIndex.get(next.edge);
      current = halves[pair[0]].from === next.from && halves[pair[0]].to === next.to ? pair[0] : pair[1];
    }
    if (vertices.length < 3 || new Set(vertices).size !== vertices.length) continue;
    let area2 = 0;
    for (let i = 0; i < vertices.length; i++) {
      const p = graphNodes[vertices[i]], q = graphNodes[vertices[(i + 1) % vertices.length]];
      area2 += p.x * q.z - q.x * p.z;
    }
    const area = area2 / 2;
    if (area <= 1e-3) continue;
    let first = 0;
    for (let i = 1; i < vertices.length; i++) if (graphNodes[vertices[i]].id < graphNodes[vertices[first]].id) first = i;
    const ids = vertices.map((v) => graphNodes[v].id);
    const orderedIds = ids.slice(first).concat(ids.slice(0, first));
    const polygon = orderedIds.map((id) => { const n = nodes.get(id); return { x: n.x, z: n.z }; });
    const boundaryRoadIds = edgeIndices.map((ei, i) => ({ from: graphNodes[vertices[i]].id, to: graphNodes[vertices[(i + 1) % vertices.length]].id, id: edgeData[ei].id }));
    faces.push({ area, polygon, boundaryRoadIds, identity: orderedIds });
  }
  return faces.sort((a, b) => a.identity.join(',').localeCompare(b.identity.join(',')));
}
function rasterFace(face) {
  const xs = face.polygon.map((p) => p.x), zs = face.polygon.map((p) => p.z);
  const ix0 = Math.floor(Math.min(...xs) / STEP), ix1 = Math.floor(Math.max(...xs) / STEP);
  const iz0 = Math.floor(Math.min(...zs) / STEP), iz1 = Math.floor(Math.max(...zs) / STEP);
  const poly = face.polygon;
  const bend = (world.seed % 5) * 9 - 18;
  const river = [[-112 + bend, -400], [-98 + bend, -190], [-112 + bend, 20], [-94 + bend, 210], [-109 + bend, 400]]
    .map(([x, z]) => ({ x, z }));
  const riverSegments = river.slice(1).map((p, i) => [river[i], p]);
  const protectedPoly = [{x:-350,z:-350},{x:-260,z:-350},{x:-260,z:-270},{x:-350,z:-270}];
  const roadBuffers = allRoads.map((road) => ({ road, clearance: road.kind === 'arterial' ? 7 : 4.5 }));
  const set = new Set();
  for (let iz = iz0; iz <= iz1; iz++) for (let ix = ix0; ix <= ix1; ix++) {
    const p = { x: (ix + 0.5) * STEP, z: (iz + 0.5) * STEP };
    if (!pointInPolygon(p, poly)) continue;
    if (parcelFootprints.some((parcel) => parcel.footprints.some((rect) => pointRectDistance(p, rect) <= 0.01))) continue;
    if (roadBuffers.some(({road,clearance}) => pointSegmentDistance(p, road.a, road.b) < clearance)) continue;
    if (pointInPolygon(p, protectedPoly)) continue;
    if (riverSegments.some(([a,b]) => pointSegmentDistance(p,a,b) < 38)) continue;
    set.add(`${ix},${iz}`);
  }
  const components = [];
  while (set.size) {
    const start = set.values().next().value, stack = [start], component = [];
    set.delete(start);
    while (stack.length) {
      const key = stack.pop(); component.push(key);
      const [ix, iz] = key.split(',').map(Number);
      for (const next of [`${ix+1},${iz}`,`${ix-1},${iz}`,`${ix},${iz+1}`,`${ix},${iz-1}`]) if (set.delete(next)) stack.push(next);
    }
    const points = component.map((key) => { const [ix, iz] = key.split(',').map(Number); return {ix,iz,x:(ix+.5)*STEP,z:(iz+.5)*STEP,key}; });
    components.push({ face, cells: points, area_m2: points.length * STEP * STEP,
      centroid: [points.reduce((s,p)=>s+p.x,0)/points.length, points.reduce((s,p)=>s+p.z,0)/points.length],
      bbox: [Math.min(...points.map(p=>p.x)), Math.min(...points.map(p=>p.z)), Math.max(...points.map(p=>p.x)), Math.max(...points.map(p=>p.z))] });
  }
  return components;
}
const tracedFaces = traceFaces();
const voidComponents = tracedFaces.flatMap(rasterFace).sort((a,b)=>b.area_m2-a.area_m2);
const selectedComponent = voidComponents[0];
const face = selectedComponent.face;
const cells = selectedComponent.cells;
const mask = new Set(cells.map((c) => c.key));
const boundaryRoadIds = new Set(face.boundaryRoadIds.map((e) => e.id));
const boundaryRoads = allRoads.filter((r) => boundaryRoadIds.has(r.id));
function cellForPoint(p) {
  const ix = Math.floor(p.x / STEP), iz = Math.floor(p.z / STEP);
  return mask.has(`${ix},${iz}`);
}
function segmentInsideMask(a, b, sampleStep = 1) {
  const n = Math.max(1, Math.ceil(dist(a, b) / sampleStep));
  for (let i = 0; i <= n; i++) {
    const t = i / n;
    if (!cellForPoint({ x: a.x + (b.x - a.x) * t, z: a.z + (b.z - a.z) * t })) return false;
  }
  return true;
}
function segmentEntersMask(a, b, sampleStep = 1) {
  const n = Math.max(1, Math.ceil(dist(a, b) / sampleStep));
  let entered = false;
  for (let i = 0; i <= n; i++) {
    const t = i / n;
    const inside = cellForPoint({ x: a.x + (b.x - a.x) * t, z: a.z + (b.z - a.z) * t });
    if (inside) entered = true;
    else if (entered) return false;
  }
  return entered;
}
function parcelClearPoint(p, clearance = PARCEL_CLEARANCE) {
  return parcelFootprints.every((parcel) => parcel.footprints.every((rect) => pointRectDistance(p, rect) >= clearance));
}
function parcelClearSegment(a, b, clearance = PARCEL_CLEARANCE) {
  return parcelFootprints.every((parcel) => parcel.footprints.every((rect) => rectSegmentDistance(rect, a, b) >= clearance));
}
function roadClearPoint(p, excludeIds = new Set(), clearance = ROAD_SEPARATION) {
  return allRoads.every((road) => excludeIds.has(road.id) || pointSegmentDistance(p, road.a, road.b) >= clearance);
}
function roadClearSegment(a, b, excludeIds = new Set(), clearance = ROAD_SEPARATION) {
  return allRoads.every((road) => excludeIds.has(road.id) || segmentDistance(a, b, road.a, road.b) >= clearance);
}
function nearestPointOnRoad(p, road) {
  const dx = road.b.x - road.a.x, dz = road.b.z - road.a.z;
  const den = dx * dx + dz * dz;
  const t = den === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - road.a.x) * dx + (p.z - road.a.z) * dz) / den));
  return { x: road.a.x + dx * t, z: road.a.z + dz * t, t };
}
function stubOnlyTouchesHost(a, b, host, attach) {
  const cut = Math.min(0.3, 4 / (dist(host.a, host.b) || 1));
  const left = { x: host.a.x + (host.b.x - host.a.x) * Math.max(0, attach.t - cut), z: host.a.z + (host.b.z - host.a.z) * Math.max(0, attach.t - cut) };
  const right = { x: host.a.x + (host.b.x - host.a.x) * Math.min(1, attach.t + cut), z: host.a.z + (host.b.z - host.a.z) * Math.min(1, attach.t + cut) };
  const residuals = [];
  if (attach.t - cut > 0) residuals.push([host.a, left]);
  if (attach.t + cut < 1) residuals.push([right, host.b]);
  return residuals.every(([c, d]) => segmentDistance(a, b, c, d) >= 2.5);
}

// A candidate access spur starts at an interior projection on a boundary road.
// Attachments are sampled every 2 m; grid cells are the saved 2 m void raster.
// Each candidate's initial connector must remain in the face mask, clear both
// parcel reservations by 5 m, and touch only the intended host road.
const sourceCandidates = [];
let portalRejected = { too_far: 0, parcel_clearance: 0, other_road: 0, outside_face: 0 };
for (const host of boundaryRoads) {
  const len = dist(host.a, host.b);
  const steps = Math.max(1, Math.floor(len / STEP));
  for (let i = 3; i < steps - 3; i++) {
    const t = i / steps;
    const attach = { x: host.a.x + (host.b.x - host.a.x) * t, z: host.a.z + (host.b.z - host.a.z) * t, t };
    let best = null;
    for (const cell of cells) {
      const d = dist(attach, cell);
      if (d < 10 || d > 18) continue;
      if (!cellForPoint(cell)) continue;
      if (!clearCell(cell)) continue;
      const stub = [attach, cell];
      // The connector deliberately crosses the host road's own masked
      // corridor before entering the unbuilt face; require one monotonic entry.
      if (!segmentEntersMask(...stub)) { portalRejected.outside_face++; continue; }
      if (!parcelClearSegment(...stub)) { portalRejected.parcel_clearance++; continue; }
      if (!stubOnlyTouchesHost(attach, cell, host, attach) || !roadClearSegment(...stub, new Set([host.id]), 3.0)) {
        portalRejected.other_road++; continue;
      }
      if (!best || d < best.distance) best = { host, attach, cell, distance: d };
    }
    if (best) sourceCandidates.push(best);
    else portalRejected.too_far++;
  }
}

// Narrow portal set to stable, spatially distinct candidates, then flood the
// face-cell graph once per portal.  Each cell/edge is rechecked against exact
// OBB parcel envelopes and open-road centerlines. This is a bounded offline
// search over 3,144 cells and 19 boundary roads; it is not runtime planner code.
const portals = [];
for (const c of sourceCandidates.sort((a, b) => a.distance - b.distance || a.host.id - b.host.id || a.attach.t - b.attach.t)) {
  if (portals.every((p) => p.host.id !== c.host.id || dist(p.attach, c.attach) > 8)) portals.push(c);
}
const offsets = [[1,0],[-1,0],[0,1],[0,-1],[1,1],[1,-1],[-1,1],[-1,-1]];
function clearCell(cell) {
  if (clearCellMemo.has(cell.key)) return clearCellMemo.get(cell.key);
  const ok = parcelClearPoint(cell) && roadClearPoint(cell);
  clearCellMemo.set(cell.key, ok);
  return ok;
}
function clearGridEdge(a, b) {
  return segmentInsideMask(a, b) && parcelClearSegment(a, b) && roadClearSegment(a, b);
}
const keyToCell = new Map(cells.map((c) => [c.key, c]));
let bestCorridor = null;
let bfsStats = { portal_count: portals.length, visited_cells: 0, accepted_edges: 0, rejected_edges: 0,
  rejected_by_mask: 0, rejected_by_parcel_clearance: 0, rejected_by_road_clearance: 0, rejected_by_point_clearance: 0, goal_pairs: 0 };
const directionVectors = [[1,0],[-1,0],[0,1],[0,-1]];
const queue = [];
const previous = new Map();
const portalForCell = new Map();
for (const portal of portals) {
  if (previous.has(portal.cell.key)) continue;
  previous.set(portal.cell.key, null);
  portalForCell.set(portal.cell.key, portal);
  queue.push(portal.cell);
}
for (let qi = 0; qi < queue.length; qi++) {
  const u = queue[qi];
  bfsStats.visited_cells++;
  for (const [dx, dz] of offsets) {
    const v = keyToCell.get(`${u.ix + dx},${u.iz + dz}`);
    if (!v || previous.has(v.key)) { bfsStats.rejected_edges++; continue; }
    if (!clearCell(v)) {
      bfsStats.rejected_edges++;
      if (!parcelClearPoint(v)) bfsStats.rejected_by_point_clearance++;
      else if (!roadClearPoint(v)) bfsStats.rejected_by_road_clearance++;
      continue;
    }
    if (!segmentInsideMask(u, v)) { bfsStats.rejected_edges++; bfsStats.rejected_by_mask++; continue; }
    if (!parcelClearSegment(u, v)) { bfsStats.rejected_edges++; bfsStats.rejected_by_parcel_clearance++; continue; }
    if (!roadClearSegment(u, v)) { bfsStats.rejected_edges++; bfsStats.rejected_by_road_clearance++; continue; }
    previous.set(v.key, u.key); portalForCell.set(v.key, portalForCell.get(u.key)); queue.push(v); bfsStats.accepted_edges++;
  }
}
for (const endpoint of queue) {
  const portal = portalForCell.get(endpoint.key);
  const routeKeys = [];
  for (let key = endpoint.key; key != null; key = previous.get(key)) routeKeys.push(key);
  routeKeys.reverse();
  const route = [portal.attach, ...routeKeys.map((key) => keyToCell.get(key))];
  if (route.length < 2) continue;
  for (const [dx, dz] of directionVectors) {
    const endCell = keyToCell.get(`${endpoint.ix + dx * 11},${endpoint.iz + dz * 11}`); // 22 m minimum alley length
    if (!endCell) continue;
    if (!clearCell(endCell) || !segmentInsideMask(endpoint, endCell) || !parcelClearSegment(endpoint, endCell) || !roadClearSegment(endpoint, endCell)) continue;
    const tangent = Math.atan2(endCell.z - endpoint.z, endCell.x - endpoint.x);
    const options = [];
    for (const side of [-1, 1]) {
      const normal = { x: -Math.sin(tangent) * side, z: Math.cos(tangent) * side };
      const pair = [];
      for (const along of [5, 15]) {
        const center = { x: endpoint.x + Math.cos(tangent) * along + normal.x * SITE_OFFSET,
          z: endpoint.z + Math.sin(tangent) * along + normal.z * SITE_OFFSET };
        const lot = { x: center.x, z: center.z, width: LOT_W, depth: LOT_D,
          rotation: Math.atan2(side * Math.sin(tangent), -side * Math.cos(tangent)) };
        const insideFace = rectCorners(lot).every(cellForPoint);
        const noParcelOverlap = parcelFootprints.every((parcel) => parcel.footprints.every((old) => !rectsOverlap(lot, old, LOT_GAP)));
        const noRoadOverlap = allRoads.every((road) => rectSegmentDistance(lot, road.a, road.b) >= PARCEL_CLEARANCE);
        const noPathOverlap = rectSegmentDistance(lot, endpoint, endCell) >= PARCEL_CLEARANCE;
        if (insideFace && noParcelOverlap && noRoadOverlap && noPathOverlap) pair.push(lot);
      }
      if (pair.length >= 2) options.push({ side, lots: pair });
    }
    if (!options.length) continue;
    bfsStats.goal_pairs++;
    const chosen = options[0];
    const candidate = { host_road_id: portal.host.id, host_attach: portal.attach, connector: route, alley_end: endCell,
      alley_length_m: dist(endpoint, endCell), lots: chosen.lots, side: chosen.side,
      total_length_m: route.slice(1).reduce((sum, p, i) => sum + dist(route[i], p), 0) + dist(endpoint, endCell),
      host_split_lengths_m: [portal.attach.t * portal.host.length, (1 - portal.attach.t) * portal.host.length],
      host_split_valid: portal.attach.t * portal.host.length >= 10 && (1 - portal.attach.t) * portal.host.length >= 10 };
    if (!candidate.host_split_valid) continue;
    if (!bestCorridor || candidate.total_length_m < bestCorridor.total_length_m
        || (candidate.total_length_m === bestCorridor.total_length_m && candidate.host_road_id < bestCorridor.host_road_id)) bestCorridor = candidate;
  }
}

function pointOnPath(point) {
  const [minX, minZ, maxX, maxZ] = mapBounds;
  return { x: 38 + (point.x - minX) / (maxX - minX) * 1120,
    y: 38 + (maxZ - point.z) / (maxZ - minZ) * 1120 };
}
const polygonXs = face.polygon.map((p) => p.x), polygonZs = face.polygon.map((p) => p.z);
const mapBounds = [Math.min(...polygonXs), Math.min(...polygonZs), Math.max(...polygonXs), Math.max(...polygonZs)];
function svgPolygon(points, bounds, fill, stroke, width = 1, opacity = 1) {
  const xy = points.map(pointOnPath).map((p) => `${p.x.toFixed(2)},${p.y.toFixed(2)}`).join(' ');
  return `<polygon points="${xy}" fill="${fill}" fill-opacity="${opacity}" stroke="${stroke}" stroke-width="${width}"/>`;
}
function svgLine(a, b, color, width, dash = '') {
  const p = pointOnPath(a), q = pointOnPath(b);
  return `<line x1="${p.x.toFixed(2)}" y1="${p.y.toFixed(2)}" x2="${q.x.toFixed(2)}" y2="${q.y.toFixed(2)}" stroke="${color}" stroke-width="${width}" stroke-linecap="round" ${dash ? `stroke-dasharray="${dash}"` : ''}/>`;
}
const svg = [];
svg.push(`<?xml version="1.0" encoding="UTF-8"?><svg xmlns="http://www.w3.org/2000/svg" width="1200" height="1240" viewBox="0 0 1200 1240">`);
svg.push(`<rect width="1200" height="1240" fill="#f2f0e7"/><text x="38" y="25" font-family="sans-serif" font-size="16" font-weight="bold">Largest mature void: free cells, access corridor search (diagnostic)</text>`);
svg.push(`<g transform="translate(0 8)">`);
for (const c of cells) { const p = pointOnPath(c); svg.push(`<rect x="${(p.x - 1.4).toFixed(1)}" y="${(p.y - 1.4).toFixed(1)}" width="2.8" height="2.8" fill="#93bd9a" fill-opacity=".62"/>`); }
for (const parcel of world.parcels) {
  const building = buildings.get(parcel.id);
  const front = frontages.get(parcel.id), road = front && world.roads.find((r) => r.id === front.road_id);
  const a = road && nodes.get(road.from), b = road && nodes.get(road.to);
  const rotation = parcel.rotation ?? building?.rotation ?? (front && a && b ? rotationForFrontage(a, b, front.side) : 0);
  const rect = { x: parcel.x, z: parcel.z, width: parcel.width, depth: parcel.depth, rotation };
  const bds = rectCorners(rect);
  if (bds.some((p) => p.x >= mapBounds[0] - 3 && p.x <= mapBounds[2] + 3 && p.z >= mapBounds[1] - 3 && p.z <= mapBounds[3] + 3)) svg.push(svgPolygon(bds, null, building ? '#c97957' : '#8b8e86', '#4f5754', .7, .8));
}
for (const road of allRoads) {
  if (!boundaryRoads.includes(road)) continue;
  svg.push(svgLine(road.a, road.b, '#576360', road.kind === 'arterial' ? 7 : 3.5));
}
if (bestCorridor) {
  const route = bestCorridor.connector;
  for (let i = 1; i < route.length; i++) svg.push(svgLine(route[i - 1], route[i], '#147ac0', 4));
  svg.push(svgLine(bestCorridor.connector.at(-1), bestCorridor.alley_end, '#147ac0', 6, '9 5'));
  for (const lot of bestCorridor.lots) svg.push(svgPolygon(rectCorners(lot), null, '#f3d45e', '#795f13', 2, .95));
  const start = pointOnPath(bestCorridor.host_attach);
  svg.push(`<circle cx="${start.x}" cy="${start.y}" r="6" fill="#f12d56" stroke="white" stroke-width="2"/>`);
}
svg.push(`</g><text x="38" y="1190" font-family="sans-serif" font-size="12" fill="#344">Green: unbuilt face raster · gray/red: existing parcel reservations · blue: best clear path · yellow: two candidate 8×8 m lots · red: host attachment</text>`);
svg.push(`</svg>`);

const result = {
  source: { save: savePath, sha256: saveSha256, seed: world.seed, tick: world.tick,
    face_selection: 'largest connected free component from faces traced directly from this same save; no older raster mask reused',
    traced_face_area_m2: face.area, free_component_area_m2: cells.length * STEP * STEP,
    free_component_centroid_xz: selectedComponent.centroid, free_component_bbox_xz: selectedComponent.bbox,
    face_cells_2m: cells.length, face_vertex_ids: face.identity, traced_face_count: tracedFaces.length,
    aggregate_free_area_all_traced_faces_m2: voidComponents.reduce((sum, c) => sum + c.area_m2, 0),
    largest_free_components: voidComponents.slice(0, 12).map((c) => ({ area_m2: c.area_m2, centroid_xz: c.centroid, bbox_xz: c.bbox, face_area_m2: c.face.area, face_vertex_ids: c.face.identity })) },
  geometry_assumptions: { grid_step_m: STEP, parcel_centerline_clearance_m: PARCEL_CLEARANCE,
    existing_road_separation_m: ROAD_SEPARATION, lot_gap_m: LOT_GAP, candidate_lot_m: [LOT_W, LOT_D],
    frontage_center_offset_m: SITE_OFFSET, road_kind: 'local', road_width_from_renderer_m: 2.1,
    face_mask: 'recomputed on this matching save: 2m face polygon raster, subtracting both parcel footprint envelopes and 4.5m local/7m arterial road centerline corridors plus protected and river buffers',
    parcel_footprints: 'Both saved rendered orientation and legacy axis-aligned envelope are obstacles when parcel.rotation is absent.' },
  search: { boundary_roads: boundaryRoads.map((r) => ({ id: r.id, kind: r.kind, length_m: r.length })),
    raw_portal_candidates: sourceCandidates.length, distinct_portals: portals.length, portal_rejections: portalRejected,
    portal_examples: portals.slice(0, 12).map((p) => ({ road_id: p.host.id, attachment: p.attach, first_cell: [p.cell.x, p.cell.z], connector_length_m: p.distance })),
    stats: bfsStats, approximate_safe_cell_count: [...clearCellMemo.values()].filter(Boolean).length },
  best_corridor: bestCorridor ? { host_road_id: bestCorridor.host_road_id, host_attachment: bestCorridor.host_attach,
    connector_points_xz: bestCorridor.connector.map((p) => [p.x, p.z]), alley_end_xz: [bestCorridor.alley_end.x, bestCorridor.alley_end.z],
    alley_length_m: bestCorridor.alley_length_m, total_path_length_m: bestCorridor.total_length_m,
    lots: bestCorridor.lots.map((l) => ({ x: l.x, z: l.z, width: l.width, depth: l.depth, rotation: l.rotation })),
    status: 'Approximate corridor candidate passes raster path, oriented + legacy parcel-clearance, existing-road separation, and lot envelope/gap tests. It is not yet passed through authoritative Engine road-split/project admission.' }
    : null,
  limitations: ['Void mask and face topology are approximate 2m raster geometry.',
    'Candidate centerlines use 5m parcel clearance and 8m separation from non-host existing roads, but the path is not submitted to Engine admission.',
    'Host attachment is an interior projection and requires a road split/junction; route endpoints/corners are sampled at 2m.',
    'Candidate lot checks use 8x8m geometry, 2m gap and 5m clearance to existing roads; lot access/path and complete frontage lifecycle remain to be validated.',
    'No terrain/river exclusion was restrictive here: the audited face has zero protected overlap and is over 300m from the river.']
};
await writeFile(outputPath, `${JSON.stringify(result, null, 2)}\n`);
await writeFile(mapPath, `${svg.join('')}\n`);
console.log(JSON.stringify({ output: outputPath, map: mapPath, portals: portals.length, best: result.best_corridor?.total_path_length_m ?? null, host: result.best_corridor?.host_road_id ?? null, stats: bfsStats }));
