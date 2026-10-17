import { instanceCapacity, reuseInstances } from './reuse-instances';
import * as THREE from 'three/webgpu';
import {
  attribute,
  cameraPosition,
  cameraProjectionMatrix,
  clamp,
  color,
  cross,
  float,
  int,
  max,
  normalize,
  positionLocal,
  screenSize,
  smoothstep,
  varying,
  vec3,
  vec4,
} from 'three/tsl';
import type { ArtMaterials } from './materials';
import type { PlantedTree } from './parks';
import { openSpaces } from './parks';
import { CURRENT_EXPRESSWAY_PROFILE } from './expressway-profile';
import { sampledRail } from './rail-path';
import { bezierRoad } from './road-curve';
import { EXPRESSWAY_DECK, roadWidth } from './road-elevation';
import { overlapsStation } from './station-clearance';
import { streetLightPlacements } from './street-lighting';
import { distanceToSegment, SpatialCells } from './tree-occlusion';
import type { Point, Road, World, WorldNode } from '../types';

/**
 * Utility poles and street wiring. The rail catenary belongs to the rail
 * corridor (`rail-corridor.ts`), which follows the curved alignment.
 *
 * Street poles follow whole streets, not engine segments: master-plan roads
 * split at every junction, so collinear segments are first chained into
 * streets and poles are spaced along each chain. Local streets and lanes are
 * shared surfaces whose facades meet the carriageway edge, so poles stand just
 * inside that edge and every conductor hangs on the street side, clear of
 * balconies. Arterials carry poles on the sidewalk, and none within the
 * station district, where cables run underground.
 */

export type OverheadWorld = Pick<
  World,
  'nodes' | 'roads' | 'building_access' | 'bus_lines' | 'parcels' | 'buildings'
> &
  Partial<Pick<World, 'rail_alignments' | 'road_decks' | 'rail_platforms' | 'expressway_profile'>>;

export interface UtilityPole {
  x: number;
  z: number;
  /** Unit vector from the pole toward the street centre; every crossarm points this way. */
  ix: number;
  iz: number;
  height: number;
  transformer: boolean;
  /** Street chain index; spans along a chain join consecutive poles. */
  street: number;
}
export interface WireSpan {
  from: number;
  to: number;
  crossing: boolean;
}
export interface StreetWiring {
  poles: UtilityPole[];
  spans: WireSpan[];
}

export const POLE_SPACING = 30;
const MIN_POLE_SPACING = 26,
  MAX_POLE_SPACING = 36;
/** Poles keep this far from a street's end node; dead ends keep less. */
const STREET_END_MARGIN = 5,
  DEAD_END_MARGIN = 2;
/** Longest span carried between two poles; a wider gap breaks the run. */
export const MAX_SPAN = 45;
/** Longest span strung across a junction between two streets. */
export const MAX_CROSSING_SPAN = 30;
/** How far past the carriageway edge a crossing span may pass: shared-surface streets, arterials (over the sidewalk). */
export const CORNER_ALLOWANCE = [2, 3] as const;
export const MAX_POLES = 1200;
/** Poles closer than this to an existing pole are dropped; the crossing spans connect the gap. */
export const POLE_MIN_GAP = 8;
/** Along-street nudges tried when a pole's ideal spot is taken, nearest first. */
const SLIDE_STEPS = [0, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 6, -6];
export const POLE_LAMP_CLEARANCE = 2,
  POLE_TREE_CLEARANCE = 1.5,
  POLE_DOOR_CLEARANCE = 1,
  POLE_STOP_CLEARANCE = 4,
  POLE_OPEN_SPACE_MARGIN = 1;
/** Arterials inside this radius of the station have their cables underground. */
export const STATION_UNDERGROUND_RADIUS = 220;
/** Collinear street segments continue one chain through a node within this bend. */
const CHAIN_BEND = Math.cos(THREE.MathUtils.degToRad(25));
/** Shared-surface poles stand this far inside the carriageway edge; arterial poles this far onto the sidewalk. */
export const SHARED_POLE_INSET = 0.35,
  SIDEWALK_POLE_OFFSET = 0.45;
export const TALL_POLE_MIN = 10,
  TALL_POLE_MAX = 12;
/** Poles near the expressway are cut down so the wiring clears its deck: at most this tall, and 2.5 m below the deck. */
export const SHORT_POLE = 8.5;
const DECK_CLEARANCE = 2.5,
  MIN_SHORT_POLE = 6;
/** Poles this far beyond the expressway's edge still stand under its deck overhang and piers. */
const EXPRESSWAY_REACH = 4;
/** Curved decks are sampled into straight obstacle pieces this long. */
const CURVE_STEP = 4;
const OBSTACLE_CELL = 24;
/** Half-width of the rail viaduct's pier caps, the widest part of the deck. */
const VIADUCT_HALF_WIDTH = (roadWidth('rail') + 1.2) / 2;
const WIRED_KINDS = new Set(['local', 'lane', 'arterial']);

/** Conductors on a pole: lateral offset toward the street centre, drop below the pole top, radius, and sag scale. */
export const CONDUCTORS = [
  { lateral: 0.6, drop: 0.22, radius: 0.016, sag: 1 },
  { lateral: 1.2, drop: 0.22, radius: 0.016, sag: 1 },
  { lateral: 0.9, drop: 1.37, radius: 0.02, sag: 1.1 },
  // The telecom bundle hangs lowest and heaviest; its height is capped below.
  { lateral: 0.35, drop: 0, radius: 0.045, sag: 1.3 },
] as const;
const TELECOM_TOP = 6.4,
  TELECOM_BELOW_TOP = 3.4;
export function conductorHeight(pole: Pick<UtilityPole, 'height'>, index: number) {
  return index === 3
    ? Math.min(TELECOM_TOP, pole.height - TELECOM_BELOW_TOP)
    : pole.height - CONDUCTORS[index].drop;
}
export function conductorPoint(pole: UtilityPole, index: number): THREE.Vector3 {
  const c = CONDUCTORS[index];
  return new THREE.Vector3(
    pole.x + pole.ix * c.lateral,
    conductorHeight(pole, index),
    pole.z + pole.iz * c.lateral,
  );
}
/** Mid-span sag: about 1.4% of the span, kept within a believable range. */
export function spanSag(length: number, scale = 1) {
  return THREE.MathUtils.clamp(length * 0.014, 0.12, 0.7) * scale;
}

function hash(x: number, z: number, salt = 0) {
  let h =
    Math.imul(Math.round(x * 10) ^ 0x9e3779b1, 0x85ebca6b) ^
    Math.imul(Math.round(z * 10) ^ salt, 0xc2b2ae35);
  h ^= h >>> 16;
  h = Math.imul(h, 0x7feb352d);
  h ^= h >>> 15;
  return (h >>> 0) / 4294967296;
}
const finite = (...points: Point[]) =>
  points.every((p) => Number.isFinite(p.x) && Number.isFinite(p.z));

interface Segment {
  road: Road;
  a: Point;
  b: Point;
  length: number;
  /** Expressway pieces: half the deck width and the deck height above ground. */
  halfWidth?: number;
  deck?: number;
}
export interface Chain {
  kind: string;
  points: Point[];
  segments: Segment[];
  endDegrees: [number, number];
}

/** Segments with both nodes, in road id order: open ones by default, or every stage past survey. */
function roadSegments(world: Pick<World, 'nodes' | 'roads'>, openOnly = true) {
  const nodes = new Map(world.nodes.map((node) => [node.id, node]));
  const result: Segment[] = [];
  for (const road of [...world.roads].sort((x, y) => x.id - y.id)) {
    if (openOnly ? road.stage !== 'open' : road.stage === 'survey') continue;
    const a = nodes.get(road.from),
      b = nodes.get(road.to);
    if (!a || !b || !finite(a, b)) continue;
    const length = Math.hypot(b.x - a.x, b.z - a.z);
    if (length < 0.5) continue;
    result.push({ road, a, b, length });
  }
  return result;
}

/**
 * Join segments of one kind into chains through nodes where they continue
 * within `bend` (cosine). Each node pairs its most collinear segments first.
 * Chains are ordered by their lowest road id, so placement is deterministic.
 */
export function chainSegments(segments: Segment[], bend = CHAIN_BEND): Chain[] {
  const incident = new Map<number, Segment[]>();
  for (const segment of segments)
    for (const id of [segment.road.from, segment.road.to]) {
      const list = incident.get(id);
      if (list) list.push(segment);
      else incident.set(id, [segment]);
    }
  const outward = (segment: Segment, node: number) => {
    const [from, to] = segment.road.from === node ? [segment.a, segment.b] : [segment.b, segment.a];
    return { x: (to.x - from.x) / segment.length, z: (to.z - from.z) / segment.length };
  };
  const partner = new Map<string, Segment>();
  for (const [node, list] of incident) {
    const pairs: Array<{ dot: number; x: Segment; y: Segment }> = [];
    for (let i = 0; i < list.length; i++)
      for (let j = i + 1; j < list.length; j++) {
        if (list[i].road.kind !== list[j].road.kind) continue;
        const u = outward(list[i], node),
          v = outward(list[j], node),
          dot = u.x * v.x + u.z * v.z;
        if (dot <= -bend) pairs.push({ dot, x: list[i], y: list[j] });
      }
    pairs.sort((p, q) => p.dot - q.dot || p.x.road.id - q.x.road.id || p.y.road.id - q.y.road.id);
    for (const { x, y } of pairs) {
      if (partner.has(`${node}:${x.road.id}`) || partner.has(`${node}:${y.road.id}`)) continue;
      partner.set(`${node}:${x.road.id}`, y);
      partner.set(`${node}:${y.road.id}`, x);
    }
  }
  const visited = new Set<number>(),
    chains: Chain[] = [];
  const other = (segment: Segment, node: number) =>
    segment.road.from === node ? segment.road.to : segment.road.from;
  const nodeOf = (segment: Segment, id: number) =>
    segment.road.from === id ? segment.a : segment.b;
  for (const start of segments) {
    if (visited.has(start.road.id)) continue;
    visited.add(start.road.id);
    // Walk backward from `from`, then forward from `to`.
    const back: Segment[] = [];
    let node = start.road.from,
      current = start;
    for (;;) {
      const next = partner.get(`${node}:${current.road.id}`);
      if (!next || visited.has(next.road.id)) break;
      visited.add(next.road.id);
      back.push(next);
      node = other(next, node);
      current = next;
    }
    const tail = node;
    const forward: Segment[] = [];
    node = start.road.to;
    current = start;
    for (;;) {
      const next = partner.get(`${node}:${current.road.id}`);
      if (!next || visited.has(next.road.id)) break;
      visited.add(next.road.id);
      forward.push(next);
      node = other(next, node);
      current = next;
    }
    const head = node;
    const ordered = [...back.reverse(), start, ...forward];
    // Node sequence from the tail end to the head end.
    const points: Point[] = [];
    let at = tail;
    const firstNode = ordered[0].road.from === at ? ordered[0].a : ordered[0].b;
    points.push({ x: firstNode.x, z: firstNode.z });
    for (const segment of ordered) {
      const next = other(segment, at);
      const p = nodeOf(segment, next);
      points.push({ x: p.x, z: p.z });
      at = next;
    }
    const degree = (id: number) => incident.get(id)?.length ?? 0;
    chains.push({
      kind: start.road.kind,
      points,
      segments: ordered,
      endDegrees: [degree(tail), degree(head)],
    });
  }
  return chains;
}

/** Point and unit direction at arc length `s` along a polyline. */
function along(points: Point[], cumulative: number[], s: number) {
  let i = 1;
  while (i < points.length - 1 && cumulative[i] < s) i++;
  const a = points[i - 1],
    b = points[i],
    length = cumulative[i] - cumulative[i - 1] || 1,
    t = THREE.MathUtils.clamp((s - cumulative[i - 1]) / length, 0, 1);
  return {
    x: a.x + (b.x - a.x) * t,
    z: a.z + (b.z - a.z) * t,
    ux: (b.x - a.x) / length,
    uz: (b.z - a.z) / length,
  };
}
function cumulativeLengths(points: Point[]) {
  const c = [0];
  for (let i = 1; i < points.length; i++)
    c.push(c[i - 1] + Math.hypot(points[i].x - points[i - 1].x, points[i].z - points[i - 1].z));
  return c;
}

/** Shortest distance between segments pq and rs in plan. */
function segmentDistance(p: Point, q: Point, r: Point, s: Point) {
  const cross2 = (a: Point, b: Point, c: Point) =>
    (b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x);
  const d1 = cross2(p, q, r),
    d2 = cross2(p, q, s),
    d3 = cross2(r, s, p),
    d4 = cross2(r, s, q);
  if (((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0)))
    return 0;
  return Math.min(
    distanceToSegment(r, p, q),
    distanceToSegment(s, p, q),
    distanceToSegment(p, r, s),
    distanceToSegment(q, r, s),
  );
}

/**
 * Obstacle pieces for every road past survey. Rail follows its engine-owned
 * alignment and expressways their deck curve, so keep-outs track the real
 * curved corridors rather than the straight node-to-node chord.
 */
function obstacleSegments(world: OverheadWorld): Segment[] {
  const straight = roadSegments(world, false);
  const alignments = new Map(
    (world.rail_alignments ?? []).map((alignment) => [alignment.road_id, alignment]),
  );
  const decks = new Map((world.road_decks ?? []).map((deck) => [deck.road_id, deck]));
  const halfWidth = (world.expressway_profile ?? CURRENT_EXPRESSWAY_PROFILE).width / 2;
  const result: Segment[] = [];
  const polyline = (road: Road, points: Point[], extra: Partial<Segment> = {}) => {
    for (let i = 1; i < points.length; i++) {
      const a = points[i - 1],
        b = points[i],
        length = Math.hypot(b.x - a.x, b.z - a.z);
      if (length > 1e-3) result.push({ road, a, b, length, ...extra });
    }
  };
  for (const segment of straight) {
    const { road } = segment,
      alignment = road.kind === 'rail' ? alignments.get(road.id) : undefined,
      deck = road.kind === 'expressway' ? decks.get(road.id) : undefined;
    if (alignment && alignment.samples.length > 1) {
      const path = sampledRail(alignment),
        steps = Math.max(1, Math.ceil(path.length / CURVE_STEP));
      polyline(
        road,
        Array.from({ length: steps + 1 }, (_, i) => path.at((path.length * i) / steps)),
      );
    } else if (road.kind === 'expressway') {
      const extra = { halfWidth, deck: deck?.deck_height ?? EXPRESSWAY_DECK };
      if (deck) {
        const [c, d] = deck.controls,
          curve = bezierRoad(segment.a, { x: c[0], z: c[1] }, { x: d[0], z: d[1] }, segment.b),
          steps = Math.max(1, Math.ceil(curve.length / CURVE_STEP));
        polyline(
          road,
          Array.from({ length: steps + 1 }, (_, i) => curve.at((curve.length * i) / steps)),
          extra,
        );
      } else result.push({ ...segment, ...extra });
    } else result.push(segment);
  }
  return result;
}
/** Pole height that keeps every conductor below an expressway deck overhead. */
const underDeck = (deck: number) =>
  Math.max(MIN_SHORT_POLE, Math.min(SHORT_POLE, deck - DECK_CLEARANCE));

/**
 * Deterministic street poles and the spans they carry. `trees` are the
 * street trees already planted, which poles stand clear of.
 */
export function streetWiring(
  world: OverheadWorld,
  trees: readonly PlantedTree[] = [],
): StreetWiring {
  // Poles keep clear of roads still being built, so a later opening never lands on one.
  const segments = roadSegments(world, false),
    obstacles = obstacleSegments(world);
  const station = world.nodes.find((node) => node.kind === 'station');
  const platforms = world.rail_platforms ?? [];
  const inStation = (p: Point, margin: number) =>
    platforms.some((platform) => overlapsStation(p, margin, platform));
  const lamps = new SpatialCells<Point>(OBSTACLE_CELL),
    treeCells = new SpatialCells<Point>(OBSTACLE_CELL),
    doors = new SpatialCells<readonly Point[]>(OBSTACLE_CELL);
  const stops = new SpatialCells<Point>(OBSTACLE_CELL),
    roads = new SpatialCells<Segment>(OBSTACLE_CELL),
    poleCells = new SpatialCells<number>(OBSTACLE_CELL);
  const lots = new SpatialCells<{
    x: number;
    z: number;
    cos: number;
    sin: number;
    hw: number;
    hd: number;
  }>(OBSTACLE_CELL);
  for (const lamp of streetLightPlacements(world))
    lamps.insert(lamp, lamp.x, lamp.z, POLE_LAMP_CLEARANCE);
  for (const tree of trees) treeCells.insert(tree, tree.x, tree.z, POLE_TREE_CLEARANCE);
  // Entry paths run from the door to the street, possibly with several legs.
  for (const access of world.building_access ?? []) {
    const path = access.points;
    if (path.length < 2 || !finite(...path)) continue;
    const r = POLE_DOOR_CLEARANCE;
    doors.insertBox(
      path,
      Math.min(...path.map((p) => p.x)) - r,
      Math.min(...path.map((p) => p.z)) - r,
      Math.max(...path.map((p) => p.x)) + r,
      Math.max(...path.map((p) => p.z)) + r,
    );
  }
  const nearEntry = (p: Point) =>
    doors
      .at(p.x, p.z)
      .some((path) =>
        path.slice(1).some((b, i) => distanceToSegment(p, path[i], b) < POLE_DOOR_CLEARANCE),
      );
  for (const stop of (world.bus_lines ?? []).flatMap((line) => line.stop_positions ?? []))
    if (finite(stop)) stops.insert(stop, stop.x, stop.z, POLE_STOP_CLEARANCE);
  for (const segment of obstacles) {
    const r = keepOut(segment.road.kind) + (segment.halfWidth ?? 0) + EXPRESSWAY_REACH;
    roads.insertBox(
      segment,
      Math.min(segment.a.x, segment.b.x) - r,
      Math.min(segment.a.z, segment.b.z) - r,
      Math.max(segment.a.x, segment.b.x) + r,
      Math.max(segment.a.z, segment.b.z) + r,
    );
  }
  for (const lot of openSpaces(world)) {
    const rotation = lot.rotation ?? 0,
      hw = lot.width / 2 + POLE_OPEN_SPACE_MARGIN,
      hd = lot.depth / 2 + POLE_OPEN_SPACE_MARGIN,
      r = Math.hypot(hw, hd);
    lots.insert(
      { x: lot.x, z: lot.z, cos: Math.cos(rotation), sin: Math.sin(rotation), hw, hd },
      lot.x,
      lot.z,
      r,
    );
  }
  const insideLot = (p: Point) =>
    lots.at(p.x, p.z).some((lot) => {
      const dx = p.x - lot.x,
        dz = p.z - lot.z;
      const lx = dx * lot.cos - dz * lot.sin,
        lz = dx * lot.sin + dz * lot.cos;
      return Math.abs(lx) < lot.hw && Math.abs(lz) < lot.hd;
    });
  /** The lowest expressway deck a point stands under (including its edge reach), or Infinity. */
  const deckOver = (p: Point) =>
    roads
      .at(p.x, p.z)
      .reduce(
        (low, other) =>
          other.deck !== undefined &&
          distanceToSegment(p, other.a, other.b) < (other.halfWidth ?? 0) + EXPRESSWAY_REACH
            ? Math.min(low, other.deck)
            : low,
        Infinity,
      );

  const poles: UtilityPole[] = [],
    spans: WireSpan[] = [];
  const chains = chainSegments(
    segments.filter(
      (segment) =>
        segment.road.stage === 'open' &&
        segment.road.connected &&
        WIRED_KINDS.has(segment.road.kind),
    ),
  );
  const chainPoles: number[][] = [];
  for (let street = 0; street < chains.length && poles.length < MAX_POLES; street++) {
    const chain = chains[street],
      own = new Set(chain.segments.map((segment) => segment.road.id));
    const width = roadWidth(chain.kind),
      arterial = chain.kind === 'arterial';
    const offset = arterial ? width / 2 + SIDEWALK_POLE_OFFSET : width / 2 - SHARED_POLE_INSET;
    const firstId = Math.min(...own),
      side = hash(firstId, chain.points.length, firstId) < 0.5 ? -1 : 1;
    const cumulative = cumulativeLengths(chain.points),
      total = cumulative[cumulative.length - 1];
    const startMargin = chain.endDegrees[0] > 1 ? STREET_END_MARGIN : DEAD_END_MARGIN,
      endMargin = chain.endDegrees[1] > 1 ? STREET_END_MARGIN : DEAD_END_MARGIN;
    const available = total - startMargin - endMargin;
    const targets: number[] = [];
    if (available < MIN_POLE_SPACING * 0.5) {
      if (total >= 6) targets.push(total / 2);
    } else {
      let intervals = Math.max(1, Math.round(available / POLE_SPACING));
      if (available / intervals > MAX_POLE_SPACING) intervals++;
      const spacing = available / intervals;
      for (let k = 0; k <= intervals; k++) targets.push(startMargin + k * spacing);
    }
    const placed: number[] = [];
    for (const target of targets) {
      if (poles.length >= MAX_POLES) break;
      for (const slide of SLIDE_STEPS) {
        const s = THREE.MathUtils.clamp(target + slide, 1, total - 1);
        const at = along(chain.points, cumulative, s);
        // Normal to the left of travel, flipped to the chain's side.
        const nx = at.uz * side,
          nz = -at.ux * side;
        const p = { x: at.x + nx * offset, z: at.z + nz * offset };
        if (
          arterial &&
          station &&
          Math.hypot(p.x - station.x, p.z - station.z) < STATION_UNDERGROUND_RADIUS
        )
          break;
        if (
          lamps
            .at(p.x, p.z)
            .some((lamp) => Math.hypot(lamp.x - p.x, lamp.z - p.z) < POLE_LAMP_CLEARANCE)
        )
          continue;
        if (
          treeCells
            .at(p.x, p.z)
            .some((tree) => Math.hypot(tree.x - p.x, tree.z - p.z) < POLE_TREE_CLEARANCE)
        )
          continue;
        if (nearEntry(p)) continue;
        if (
          stops
            .at(p.x, p.z)
            .some((stop) => Math.hypot(stop.x - p.x, stop.z - p.z) < POLE_STOP_CLEARANCE)
        )
          continue;
        if (insideLot(p) || inStation(p, CONDUCTORS[1].lateral + 0.6)) continue;
        if (
          roads
            .at(p.x, p.z)
            .some(
              (other) =>
                !own.has(other.road.id) &&
                distanceToSegment(p, other.a, other.b) < keepOut(other.road.kind),
            )
        )
          continue;
        if (
          poleCells
            .at(p.x, p.z)
            .some((index) => Math.hypot(poles[index].x - p.x, poles[index].z - p.z) < POLE_MIN_GAP)
        )
          continue;
        const r = hash(p.x, p.z, firstId);
        const index = poles.length;
        const tall = TALL_POLE_MIN + r * (TALL_POLE_MAX - TALL_POLE_MIN),
          deck = deckOver(p);
        poles.push({
          x: p.x,
          z: p.z,
          ix: -nx,
          iz: -nz,
          height: Number.isFinite(deck) ? Math.min(tall, underDeck(deck)) : tall,
          transformer: hash(p.z, p.x, index) < 0.25,
          street,
        });
        poleCells.insert(index, p.x, p.z, POLE_MIN_GAP);
        placed.push(index);
        break;
      }
    }
    chainPoles.push(placed);
  }

  const blocked = (a: Point, b: Point) => {
    // Rail viaducts and bridge decks sit in the way of a span.
    const mid = { x: (a.x + b.x) / 2, z: (a.z + b.z) / 2 };
    for (let k = 0; k <= 8; k++)
      if (inStation({ x: a.x + ((b.x - a.x) * k) / 8, z: a.z + ((b.z - a.z) * k) / 8 }, 0.5))
        return true;
    const nearby = new Set<Segment>();
    for (const p of [a, mid, b]) for (const segment of roads.at(p.x, p.z)) nearby.add(segment);
    for (const segment of nearby) {
      if (segment.road.kind !== 'rail' && segment.road.kind !== 'bridge') continue;
      const half = segment.road.kind === 'rail' ? VIADUCT_HALF_WIDTH : roadWidth('bridge') / 2;
      if (segmentDistance(a, b, segment.a, segment.b) < half + 0.5) return true;
    }
    return false;
  };
  const shorten = (a: number, b: number) => {
    const p = poles[a],
      q = poles[b];
    // A span may pass under a deck between poles in different cells; check every cell it crosses.
    const near = new Set<Segment>();
    for (let k = 0; k <= 4; k++)
      for (const segment of roads.at(p.x + ((q.x - p.x) * k) / 4, p.z + ((q.z - p.z) * k) / 4))
        near.add(segment);
    for (const segment of near) {
      if (
        segment.deck !== undefined &&
        segmentDistance(p, q, segment.a, segment.b) < (segment.halfWidth ?? 0) + 1
      ) {
        const h = underDeck(segment.deck);
        p.height = Math.min(p.height, h);
        q.height = Math.min(q.height, h);
      }
    }
  };
  const seen = new Set<string>();
  const link = (a: number, b: number, crossing: boolean) => {
    const key = a < b ? `${a}:${b}` : `${b}:${a}`;
    if (seen.has(key)) return false;
    if (blocked(poles[a], poles[b])) return false;
    seen.add(key);
    spans.push({ from: a, to: b, crossing });
    shorten(a, b);
    return true;
  };
  for (const placed of chainPoles)
    for (let i = 1; i < placed.length; i++) {
      const a = poles[placed[i - 1]],
        b = poles[placed[i]];
      if (Math.hypot(b.x - a.x, b.z - a.z) <= MAX_SPAN) link(placed[i - 1], placed[i], false);
    }

  // Crossing spans: each street's end pole joins the nearest pole of another
  // street across the junction, if the span stays over road.
  // Wires at 6 m and up may cut a corner chamfer, but not a block.
  const onRoad = (p: Point) =>
    roads
      .at(p.x, p.z)
      .some(
        (segment) =>
          WIRED_KINDS.has(segment.road.kind) &&
          distanceToSegment(p, segment.a, segment.b) <=
            roadWidth(segment.road.kind) / 2 +
              CORNER_ALLOWANCE[segment.road.kind === 'arterial' ? 1 : 0],
      );
  const overRoad = (a: Point, b: Point) => {
    for (let k = 1; k < 6; k++) {
      const t = k / 6;
      if (!onRoad({ x: a.x + (b.x - a.x) * t, z: a.z + (b.z - a.z) * t })) return false;
    }
    return true;
  };
  for (let street = 0; street < chainPoles.length; street++) {
    const placed = chainPoles[street];
    if (!placed.length) continue;
    for (const end of new Set([placed[0], placed[placed.length - 1]])) {
      const p = poles[end];
      const candidates = new Set<number>(),
        reach = Math.ceil(MAX_CROSSING_SPAN / OBSTACLE_CELL);
      for (let dx = -reach; dx <= reach; dx++)
        for (let dz = -reach; dz <= reach; dz++)
          for (const index of poleCells.at(p.x + dx * OBSTACLE_CELL, p.z + dz * OBSTACLE_CELL))
            candidates.add(index);
      let best = -1,
        bestDistance = MAX_CROSSING_SPAN;
      for (const index of [...candidates].sort((x, y) => x - y)) {
        const q = poles[index];
        if (q.street === street) continue;
        const distance = Math.hypot(q.x - p.x, q.z - p.z);
        if (distance < bestDistance && overRoad(p, q) && !blocked(p, q)) {
          best = index;
          bestDistance = distance;
        }
      }
      if (best >= 0) link(end, best, true);
    }
  }
  return { poles, spans };
}

/**
 * Distance from another road's centreline that a pole must keep: its
 * carriageway and a margin, the expressway pier line, or the rail viaduct
 * plus the crossarm's reach, so neither pole nor arm meets the deck.
 */
function keepOut(kind: string) {
  return kind === 'expressway'
    ? 3
    : kind === 'rail'
      ? VIADUCT_HALF_WIDTH + CONDUCTORS[1].lateral + 0.6
      : roadWidth(kind) / 2 + 0.8;
}

/** Ribbon geometry that the wire material widens toward the camera. */
class WireBuilder {
  private positions: number[] = [];
  private tangents: number[] = [];
  private sides: number[] = [];
  private radii: number[] = [];
  private indices: number[] = [];
  polyline(points: readonly THREE.Vector3[], radius: number) {
    if (points.length < 2) return;
    const base = this.positions.length / 3;
    for (let i = 0; i < points.length; i++) {
      const p = points[i],
        a = points[Math.max(0, i - 1)],
        b = points[Math.min(points.length - 1, i + 1)];
      const t = new THREE.Vector3().subVectors(b, a).normalize();
      for (const side of [-1, 1]) {
        this.positions.push(p.x, p.y, p.z);
        this.tangents.push(t.x, t.y, t.z);
        this.sides.push(side);
        this.radii.push(radius);
      }
    }
    for (let i = 0; i < points.length - 1; i++) {
      const k = base + i * 2;
      this.indices.push(k, k + 1, k + 2, k + 1, k + 3, k + 2);
    }
  }
  span(a: THREE.Vector3, b: THREE.Vector3, sag: number, radius: number) {
    const length = a.distanceTo(b),
      steps = length < 20 ? 8 : 10,
      points: THREE.Vector3[] = [];
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      points.push(
        new THREE.Vector3()
          .lerpVectors(a, b, t)
          .setY(a.y + (b.y - a.y) * t - 4 * sag * t * (1 - t)),
      );
    }
    this.polyline(points, radius);
  }
  get empty() {
    return this.indices.length === 0;
  }
  finish() {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(this.positions, 3));
    g.setAttribute('wireTangent', new THREE.Float32BufferAttribute(this.tangents, 3));
    g.setAttribute('wireSide', new THREE.Float32BufferAttribute(this.sides, 1));
    g.setAttribute('wireRadius', new THREE.Float32BufferAttribute(this.radii, 1));
    g.setIndex(this.indices);
    g.computeBoundingSphere();
    // The vertex stage widens ribbons by up to about a pixel; pad the culling bounds.
    if (g.boundingSphere) g.boundingSphere.radius += 1;
    return g;
  }
}

/** A wire is never drawn thinner than this on screen; thinner wires fade instead. */
export const WIRE_MIN_PIXELS = 1;
/** Street wiring fades out between these camera distances; poles stay. */
export const WIRE_FADE = [330, 450] as const;
/** Wires are batched per square cell of this size so distant cells can be culled. */
export const WIRE_CELL = 420;

function wireMaterial(art: ArtMaterials, key: string, fade: readonly [number, number]) {
  const make = (hex: string, scale: number) => {
    const material = new THREE.MeshBasicNodeMaterial({
      transparent: true,
      depthWrite: false,
      side: THREE.DoubleSide,
    });
    const center = positionLocal,
      tangent = vec3(attribute('wireTangent', 'vec3')),
      side = float(attribute('wireSide', 'float')),
      radius = float(attribute('wireRadius', 'float')).mul(scale);
    const toCamera = cameraPosition.sub(center),
      distance = toCamera.length();
    const across = normalize(cross(tangent, toCamera).add(vec3(0, 1e-6, 0)));
    // World size of one pixel at this distance: 2d / (P[1][1] * viewport height).
    const focal = vec4(cameraProjectionMatrix.element(int(1))).y;
    const pixel = distance.mul(2).div(focal.mul(screenSize.y));
    const minHalf = pixel.mul(WIRE_MIN_PIXELS * 0.5);
    material.positionNode = center.add(across.mul(max(radius, minHalf).mul(side)));
    const coverage = clamp(radius.div(minHalf), 0, 1).mul(smoothstep(fade[1], fade[0], distance));
    material.opacityNode = varying(coverage);
    material.colorNode = color(hex).mul(art.night.mul(-0.35).add(1));
    return material;
  };
  // Anime keeps hairline ink; Miniature reads as slightly heavier model cord.
  return art.shared(key, () => ({
    anime: make('#2c3236', 1) as unknown as THREE.MeshStandardNodeMaterial,
    miniature: make('#24272a', 1.4) as unknown as THREE.MeshStandardNodeMaterial,
  }));
}

function instanced(
  name: string,
  geometry: THREE.BufferGeometry,
  material: THREE.Material,
  count: number,
  castShadow = false,
) {
  const mesh = new THREE.InstancedMesh(geometry, material, instanceCapacity(count));
  mesh.name = name;
  mesh.count = count;
  mesh.castShadow = castShadow;
  mesh.receiveShadow = true;
  return mesh;
}

/** Build street wiring. Reusable pole meshes are adopted from `previous`;
 * the caller retires the leftover group. Rail catenary belongs to its corridor. */
export function buildOverheadLines(
  world: OverheadWorld,
  art: ArtMaterials,
  trees: readonly PlantedTree[] = [],
  previous?: THREE.Group,
): THREE.Group {
  const root = new THREE.Group();
  root.name = 'overhead-lines';
  const { poles, spans } = streetWiring(world, trees);
  const dummy = new THREE.Object3D();
  const yawOf = (ix: number, iz: number) => Math.atan2(-iz, ix);

  if (poles.length) {
    const concrete = art.get('#aba99f', 0.92),
      steel = art.get('#5d6466', 0.6),
      porcelain = art.get('#d9d4c8', 0.35),
      can = art.get('#8f979a', 0.55);
    const shaftGeometry = new THREE.CylinderGeometry(0.11, 0.17, 1, 8);
    shaftGeometry.translate(0, 0.5, 0);
    const shafts = instanced('utility-pole-shafts', shaftGeometry, concrete, poles.length, true);
    const topArm = new THREE.BoxGeometry(1.5, 0.09, 0.09);
    topArm.translate(0.6, 0, 0);
    const lowArm = new THREE.BoxGeometry(1.2, 0.08, 0.08);
    lowArm.translate(0.5, 0, 0);
    const topArms = instanced('utility-pole-top-arms', topArm, steel, poles.length);
    const lowArms = instanced('utility-pole-low-arms', lowArm, steel, poles.length);
    const insulators = instanced(
      'utility-pole-insulators',
      new THREE.CylinderGeometry(0.045, 0.055, 0.16, 6),
      porcelain,
      poles.length * 3,
    );
    const withTransformer = poles.filter((pole) => pole.transformer);
    const cans = instanced(
      'utility-pole-transformers',
      new THREE.CylinderGeometry(0.3, 0.3, 1.05, 10),
      can,
      withTransformer.length,
    );
    let insulator = 0,
      transformer = 0;
    for (let i = 0; i < poles.length; i++) {
      const pole = poles[i],
        yaw = yawOf(pole.ix, pole.iz);
      dummy.rotation.set(0, yaw, 0);
      dummy.position.set(pole.x, 0, pole.z);
      dummy.scale.set(1, pole.height, 1);
      dummy.updateMatrix();
      shafts.setMatrixAt(i, dummy.matrix);
      dummy.scale.setScalar(1);
      dummy.position.set(pole.x, pole.height - 0.35, pole.z);
      dummy.updateMatrix();
      topArms.setMatrixAt(i, dummy.matrix);
      dummy.position.set(pole.x, pole.height - 1.5, pole.z);
      dummy.updateMatrix();
      lowArms.setMatrixAt(i, dummy.matrix);
      for (const index of [0, 1, 2]) {
        const p = conductorPoint(pole, index);
        dummy.position.set(p.x, p.y - 0.08, p.z);
        dummy.updateMatrix();
        insulators.setMatrixAt(insulator++, dummy.matrix);
      }
      if (pole.transformer) {
        dummy.position.set(pole.x + pole.ix * 0.42, pole.height - 2.7, pole.z + pole.iz * 0.42);
        dummy.updateMatrix();
        cans.setMatrixAt(transformer++, dummy.matrix);
      }
    }
    for (const mesh of [shafts, topArms, lowArms, insulators, cans]) {
      mesh.instanceMatrix.needsUpdate = true;
      const old = previous?.getObjectByName(mesh.name);
      const retained = reuseInstances(old instanceof THREE.InstancedMesh ? old : undefined, mesh);
      // Pole primitives are fixed by role, so their old geometry is identical.
      if (retained !== mesh) mesh.geometry.dispose();
      retained.computeBoundingSphere();
      root.add(retained);
    }
  }

  const cells = new Map<string, WireBuilder>();
  const cell = (p: Point) => {
    const key = `${Math.floor(p.x / WIRE_CELL)}:${Math.floor(p.z / WIRE_CELL)}`;
    let builder = cells.get(key);
    if (!builder) {
      builder = new WireBuilder();
      cells.set(key, builder);
    }
    return builder;
  };
  for (const span of spans) {
    const a = poles[span.from],
      b = poles[span.to],
      builder = cell({ x: (a.x + b.x) / 2, z: (a.z + b.z) / 2 });
    for (let index = 0; index < CONDUCTORS.length; index++) {
      const p = conductorPoint(a, index),
        q = conductorPoint(b, index);
      builder.span(p, q, spanSag(p.distanceTo(q), CONDUCTORS[index].sag), CONDUCTORS[index].radius);
    }
  }
  const wire = wireMaterial(art, 'overhead-wire', WIRE_FADE);
  for (const [key, builder] of [...cells].sort(([x], [y]) => x.localeCompare(y))) {
    if (builder.empty) continue;
    const mesh = new THREE.Mesh(builder.finish(), wire);
    mesh.name = `overhead-wires-${key}`;
    mesh.renderOrder = 1;
    root.add(mesh);
  }
  return root;
}
