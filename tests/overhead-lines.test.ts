import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { disposeGroup } from '../web/src/render/kit';
import {
  buildOverheadLines,
  conductorHeight,
  CORNER_ALLOWANCE,
  conductorPoint,
  MAX_CROSSING_SPAN,
  MAX_POLES,
  MAX_SPAN,
  POLE_DOOR_CLEARANCE,
  POLE_LAMP_CLEARANCE,
  POLE_MIN_GAP,
  POLE_STOP_CLEARANCE,
  POLE_TREE_CLEARANCE,
  SHARED_POLE_INSET,
  SHORT_POLE,
  SIDEWALK_POLE_OFFSET,
  STATION_UNDERGROUND_RADIUS,
  streetWiring,
  TALL_POLE_MAX,
  TALL_POLE_MIN,
  type OverheadWorld,
} from '../web/src/render/overhead-lines';
import { overlapsStation } from '../web/src/render/station-clearance';
import { streetTreePlacements } from '../web/src/render/parks';
import { EXPRESSWAY_DECK, roadWidth } from '../web/src/render/road-elevation';
import { streetLightPlacements } from '../web/src/render/street-lighting';
import { distanceToSegment } from '../web/src/render/tree-occlusion';
import type {
  Parcel,
  Point,
  RailAlignment,
  RailPlatform,
  RoadKind,
  Stage,
  NodeKind,
} from '../web/src/types';

/** Engine-style rail samples along a polyline: distance, x, z, unit tangent. */
function alignment(road_id: number, points: Point[]): RailAlignment {
  const samples: RailAlignment['samples'] = [];
  let distance = 0;
  for (let i = 0; i < points.length; i++) {
    if (i > 0) distance += Math.hypot(points[i].x - points[i - 1].x, points[i].z - points[i - 1].z);
    const a = points[Math.max(0, i - 1)],
      b = points[Math.min(points.length - 1, i + 1)],
      l = Math.hypot(b.x - a.x, b.z - a.z);
    samples.push([distance, points[i].x, points[i].z, (b.x - a.x) / l, (b.z - a.z) / l]);
  }
  return { road_id, samples };
}

type Segment = {
  id: number;
  from: number;
  to: number;
  kind?: RoadKind;
  stage?: Stage;
  connected?: boolean;
};
function world(
  segments: Segment[],
  points: Array<{ id: number; x: number; z: number; kind?: NodeKind }>,
  extra: Partial<OverheadWorld> = {},
): OverheadWorld {
  return {
    nodes: points.map((point) => ({ kind: 'junction' as const, ...point })),
    roads: segments.map((segment) => {
      const from = points.find((point) => point.id === segment.from)!,
        to = points.find((point) => point.id === segment.to)!;
      return {
        id: segment.id,
        from: segment.from,
        to: segment.to,
        kind: segment.kind ?? 'local',
        stage: segment.stage ?? 'open',
        progress: 1,
        connected: segment.connected ?? true,
        length: Math.hypot(to.x - from.x, to.z - from.z),
        lanes: 2,
      };
    }),
    parcels: [],
    buildings: [],
    ...extra,
  };
}
/** A straight local street split at every 40 m node, with side streets making T-junctions. */
function splitStreet(kind: RoadKind = 'local', length = 240, step = 40) {
  const points: Array<{ id: number; x: number; z: number; kind?: NodeKind }> = [],
    segments: Segment[] = [];
  for (let i = 0; i <= length / step; i++) points.push({ id: i + 1, x: i * step, z: 0 });
  for (let i = 0; i < length / step; i++)
    segments.push({ id: i + 1, from: i + 1, to: i + 2, kind });
  // Side streets on the north at every interior node.
  for (let i = 1; i < length / step; i++) {
    points.push({ id: 100 + i, x: i * step, z: 60 });
    segments.push({ id: 100 + i, from: i + 1, to: 100 + i, kind: 'local' as const });
  }
  return { points, segments };
}
const along = (p: Point) => p.x;

describe('street poles', () => {
  it('space poles along whole streets, not engine segments, deterministically', () => {
    const { points, segments } = splitStreet();
    const w = world(segments, points);
    const wiring = streetWiring(w);
    expect(wiring).toEqual(streetWiring(w));
    // The main street is one chain: its poles stand on one side, at the shared-surface inset.
    const main = wiring.poles.filter((pole) => Math.abs(pole.z) < 5);
    expect(main.length).toBeGreaterThanOrEqual(7);
    expect(main.length).toBeLessThanOrEqual(9);
    const sides = new Set(main.map((pole) => Math.sign(pole.z)));
    expect(sides.size).toBe(1);
    for (const pole of main) {
      expect(Math.abs(pole.z)).toBeCloseTo(roadWidth('local') / 2 - SHARED_POLE_INSET, 6);
      // Crossarms point toward the street centre.
      expect(pole.iz).toBeCloseTo(-Math.sign(pole.z), 6);
      expect(pole.height).toBeGreaterThanOrEqual(TALL_POLE_MIN);
      expect(pole.height).toBeLessThanOrEqual(TALL_POLE_MAX);
    }
    const xs = main.map(along).sort((a, b) => a - b);
    for (let i = 1; i < xs.length; i++) {
      expect(xs[i] - xs[i - 1]).toBeGreaterThanOrEqual(POLE_MIN_GAP - 1e-6);
      expect(xs[i] - xs[i - 1]).toBeLessThanOrEqual(MAX_SPAN);
    }
    // Run spans join consecutive main-street poles.
    const index = new Map(wiring.poles.map((pole, i) => [pole, i]));
    const runs = wiring.spans.filter(
      (span) =>
        !span.crossing &&
        main.includes(wiring.poles[span.from]) &&
        main.includes(wiring.poles[span.to]),
    );
    expect(runs.length).toBe(main.length - 1);
    expect(index.size).toBe(wiring.poles.length);
  });

  it('keep lamps, street trees, entries, bus stops, open space and cross streets clear', () => {
    const { points, segments } = splitStreet('arterial', 400, 80);
    const base = world(segments, points);
    const poles = streetWiring(base).poles;
    const target = poles.find((pole) => Math.abs(pole.z) < 10 && pole.x > 100)!;
    const side = Math.sign(target.z);
    const lot: Parcel = {
      id: 7,
      x: 300,
      z: side * 16,
      width: 12,
      depth: 12,
      rotation: 0,
      access_node: 1,
      suitability: 1,
      protected: true,
      reservation: null,
      open_space: 'park',
    };
    const blocked = world(segments, points, {
      building_access: [
        {
          building_id: 1,
          road_id: 2,
          points: [
            { x: target.x, z: target.z + side * 3 },
            { x: target.x, z: target.z + side * 0.2 },
          ],
        },
      ],
      bus_lines: [
        {
          id: 1,
          stops: [],
          route: [],
          active: true,
          stop_positions: [
            {
              node_id: 2,
              x: 200,
              z: side * (roadWidth('arterial') / 2 + SIDEWALK_POLE_OFFSET),
              heading: 0,
              direction: 1,
            },
          ],
        },
      ],
      parcels: [lot],
    });
    const trees = streetTreePlacements(blocked);
    const wiring = streetWiring(blocked, trees);
    const lamps = streetLightPlacements(blocked);
    for (const pole of wiring.poles) {
      for (const lamp of lamps)
        expect(Math.hypot(lamp.x - pole.x, lamp.z - pole.z)).toBeGreaterThanOrEqual(
          POLE_LAMP_CLEARANCE,
        );
      for (const tree of trees)
        expect(Math.hypot(tree.x - pole.x, tree.z - pole.z)).toBeGreaterThanOrEqual(
          POLE_TREE_CLEARANCE,
        );
      expect(
        distanceToSegment(
          pole,
          { x: target.x, z: target.z + side * 3 },
          { x: target.x, z: target.z + side * 0.2 },
        ),
      ).toBeGreaterThanOrEqual(POLE_DOOR_CLEARANCE);
      expect(
        Math.hypot(
          pole.x - 200,
          pole.z - side * (roadWidth('arterial') / 2 + SIDEWALK_POLE_OFFSET),
        ),
      ).toBeGreaterThanOrEqual(POLE_STOP_CLEARANCE);
      expect(
        Math.abs(pole.x - lot.x) < lot.width / 2 + 1 &&
          Math.abs(pole.z - lot.z) < lot.depth / 2 + 1,
      ).toBe(false);
      // No arterial pole stands in a side street's carriageway mouth.
      if (Math.abs(Math.abs(pole.z) - (roadWidth('arterial') / 2 + SIDEWALK_POLE_OFFSET)) < 1e-6)
        for (const segment of blocked.roads.filter((road) => road.id >= 100)) {
          const a = blocked.nodes.find((node) => node.id === segment.from)!,
            b = blocked.nodes.find((node) => node.id === segment.to)!;
          expect(distanceToSegment(pole, a, b)).toBeGreaterThanOrEqual(
            roadWidth('local') / 2 + 0.8 - 1e-6,
          );
        }
    }
    expect(trees.length).toBeGreaterThan(0);
    expect(wiring.poles.length).toBeGreaterThan(5);
  });

  it('wire only open, connected streets; keep off bridges, the viaduct and the underground station district', () => {
    const points = [
      { id: 1, x: 0, z: 0 },
      { id: 2, x: 200, z: 0 },
      { id: 3, x: 0, z: 100 },
      { id: 4, x: 200, z: 100 },
      { id: 5, x: 0, z: 200 },
      { id: 6, x: 200, z: 200 },
      { id: 7, x: 100, z: -60 },
      { id: 8, x: 100, z: 260 },
      { id: 9, x: 0, z: 600, kind: 'station' as const },
      { id: 10, x: 200, z: 600 },
      { id: 11, x: 0, z: 640 },
      { id: 12, x: 200, z: 640 },
    ];
    const w = world(
      [
        { id: 1, from: 1, to: 2, stage: 'foundation' as const },
        { id: 2, from: 3, to: 4, connected: false },
        { id: 3, from: 5, to: 6, kind: 'bridge' as const },
        { id: 4, from: 7, to: 8, kind: 'local' as const },
        { id: 5, from: 9, to: 10, kind: 'arterial' as const },
        { id: 6, from: 11, to: 12, kind: 'local' as const },
      ],
      points,
    );
    const { poles } = streetWiring(w);
    // Only the north-south local street, and the local street in the station district.
    for (const pole of poles) {
      const onLocal = Math.abs(pole.x - 100) < 5;
      const nearStation = Math.abs(pole.z - 640) < 5;
      expect(onLocal || nearStation).toBe(true);
      if (onLocal)
        for (const z of [0, 100])
          expect(Math.abs(pole.z - z)).toBeGreaterThanOrEqual(roadWidth('local') / 2 + 0.8 - 1e-6);
      // The bridge deck carries none.
      if (onLocal)
        expect(Math.abs(pole.z - 200)).toBeGreaterThanOrEqual(roadWidth('bridge') / 2 + 0.8 - 1e-6);
    }
    expect(poles.some((pole) => Math.abs(pole.z - 640) < 5)).toBe(true);
    expect(
      poles.some(
        (pole) =>
          Math.abs(pole.z - 600) < 10 &&
          Math.hypot(pole.x, pole.z - 600) < STATION_UNDERGROUND_RADIUS,
      ),
    ).toBe(false);

    // A rail viaduct across the street: poles clear the deck and arm reach, and no span passes beneath.
    const railed = world(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 3, to: 4, kind: 'rail' as const },
      ],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 300, z: 0 },
        { id: 3, x: 150, z: -100 },
        { id: 4, x: 150, z: 100 },
      ],
    );
    const wiring = streetWiring(railed);
    for (const pole of wiring.poles)
      expect(Math.abs(pole.x - 150)).toBeGreaterThan((roadWidth('rail') + 1.2) / 2 + 1.2);
    for (const span of wiring.spans) {
      const a = wiring.poles[span.from],
        b = wiring.poles[span.to];
      expect(Math.sign(a.x - 150)).toBe(Math.sign(b.x - 150));
    }
  });

  it('keep poles and spans clear of every engine-owned station platform', () => {
    const platform: RailPlatform = {
      node_id: 2,
      x: 0,
      z: 0,
      ux: -1,
      uz: 0,
      surface_y: 8,
      stair_x: 4.05,
      stair_bottom_z: -20,
      stair_top_z: -1,
      ground_y: 0.27,
      stair_steps: 44,
      ground_path: [
        { x: 0, z: 0 },
        { x: 7.5, z: -1.5 },
        { x: 7.5, z: -21.5 },
        { x: 4.05, z: -21.5 },
        { x: 4.05, z: -20 },
      ],
      ground_width: 1.6,
      construction_phase: 4,
    };
    const w = world(
      [
        { id: 1, from: 1, to: 2, kind: 'rail' as const },
        { id: 2, from: 2, to: 3, kind: 'rail' as const },
        { id: 3, from: 4, to: 5 },
        { id: 4, from: 6, to: 7 },
      ],
      [
        { id: 1, x: -300, z: 0 },
        { id: 2, x: 0, z: 0, kind: 'station' as const },
        { id: 3, x: 300, z: 0 },
        { id: 4, x: -200, z: -9 },
        { id: 5, x: 200, z: -9 },
        { id: 6, x: -40, z: -60 },
        { id: 7, x: -40, z: 60 },
      ],
      { rail_platforms: [platform] },
    );
    const { poles, spans } = streetWiring(w);
    for (const pole of poles)
      expect(overlapsStation(pole, 1.5, platform), `pole ${pole.x},${pole.z}`).toBe(false);
    for (const span of spans) {
      const a = poles[span.from],
        b = poles[span.to];
      for (let k = 0; k <= 8; k++)
        expect(
          overlapsStation(
            { x: a.x + ((b.x - a.x) * k) / 8, z: a.z + ((b.z - a.z) * k) / 8 },
            0,
            platform,
          ),
        ).toBe(false);
    }
    expect(poles.some((pole) => pole.x > 60 && Math.abs(pole.z + 9) < 5)).toBe(true);
  });

  it('follow the curved rail alignment, not the straight chord between its nodes', () => {
    // The rail bows 60 m north of its chord; the street runs under the bow, well clear of the chord.
    const bow: Point[] = [];
    for (let i = 0; i <= 40; i++) {
      const t = i / 40;
      bow.push({ x: -200 + 400 * t, z: 60 * Math.sin(Math.PI * t) });
    }
    const w = world(
      [
        { id: 1, from: 1, to: 2, kind: 'rail' as const },
        { id: 2, from: 3, to: 4 },
      ],
      [
        { id: 1, x: -200, z: 0 },
        { id: 2, x: 200, z: 0 },
        { id: 3, x: 0, z: -100 },
        { id: 4, x: 0, z: 150 },
      ],
      { rail_alignments: [alignment(1, bow)] },
    );
    const { poles, spans } = streetWiring(w);
    const toRail = (p: Point) =>
      Math.min(...bow.slice(1).map((b, i) => distanceToSegment(p, bow[i], b)));
    for (const pole of poles)
      expect(toRail(pole)).toBeGreaterThan((roadWidth('rail') + 1.2) / 2 + 1.2);
    // Spans never pass beneath the curved viaduct...
    for (const span of spans) {
      const a = poles[span.from],
        b = poles[span.to];
      expect(Math.sign(a.z - 60)).toBe(Math.sign(b.z - 60));
    }
    // ...while poles still stand inside the keep-out a straight viaduct along the chord would have had.
    const chordKeepOut = (roadWidth('rail') + 1.2) / 2 + 1.2 + 0.6;
    expect(poles.some((pole) => Math.abs(pole.z) < chordKeepOut)).toBe(true);
  });

  it('cut poles near the expressway so the wiring clears its deck', () => {
    const w = world(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 3, to: 4, kind: 'expressway' as const },
      ],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 300, z: 0 },
        { id: 3, x: 150, z: -200 },
        { id: 4, x: 150, z: 200 },
      ],
    );
    const { poles, spans } = streetWiring(w);
    for (const pole of poles) {
      expect(Math.abs(pole.x - 150)).toBeGreaterThanOrEqual(3);
      if (Math.abs(pole.x - 150) < 14) expect(pole.height).toBe(SHORT_POLE);
    }
    for (const span of spans) {
      const a = poles[span.from],
        b = poles[span.to];
      if (
        Math.sign(a.x - 150) !== Math.sign(b.x - 150) ||
        Math.min(Math.abs(a.x - 150), Math.abs(b.x - 150)) < roadWidth('expressway') / 2 + 1
      ) {
        expect(a.height).toBe(SHORT_POLE);
        expect(b.height).toBe(SHORT_POLE);
      }
    }
    // Every conductor of a short pole stays below the deck soffit.
    for (let index = 0; index < 4; index++)
      expect(conductorHeight({ height: SHORT_POLE }, index)).toBeLessThan(EXPRESSWAY_DECK - 0.6);
  });

  it('follow a curved, lower expressway deck and fit poles under its actual height', () => {
    // The deck curves 80 m east of its chord at x=0 and runs at 9 m; the street at z=0 passes under the bend.
    const w = world(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 3, to: 4, kind: 'expressway' as const },
      ],
      [
        { id: 1, x: -100, z: 0 },
        { id: 2, x: 300, z: 0 },
        { id: 3, x: 0, z: -200 },
        { id: 4, x: 0, z: 200 },
      ],
      {
        road_decks: [
          {
            road_id: 2,
            deck_height: 9,
            controls: [
              [107, -67],
              [107, 67],
            ],
          },
        ],
      },
    );
    const { poles } = streetWiring(w);
    // Under the bend (x ~ 80) poles are cut to clear the 9 m deck; under the chord (x ~ 0) nothing is overhead.
    const bent = poles.filter((pole) => Math.abs(pole.x - 80) < 16),
      chord = poles.filter((pole) => Math.abs(pole.x) < 12 && Math.abs(pole.x - 80) > 30);
    expect(bent.length).toBeGreaterThan(0);
    for (const pole of bent) {
      expect(pole.height).toBeLessThanOrEqual(9 - 2.5 + 1e-9);
      for (let index = 0; index < 4; index++)
        expect(conductorHeight(pole, index)).toBeLessThan(9 - 1);
    }
    for (const pole of chord) expect(pole.height).toBeGreaterThanOrEqual(TALL_POLE_MIN);
  });

  it('string crossing spans over the junction, never through a block', () => {
    // A T-junction: the branch street's end pole joins the through street.
    const w = world(
      [
        { id: 1, from: 1, to: 2 },
        { id: 2, from: 2, to: 3 },
        { id: 3, from: 2, to: 4 },
      ],
      [
        { id: 1, x: 0, z: 0 },
        { id: 2, x: 120, z: 0 },
        { id: 3, x: 240, z: 0 },
        { id: 4, x: 120, z: 120 },
      ],
    );
    const { poles, spans } = streetWiring(w);
    const crossings = spans.filter((span) => span.crossing);
    expect(crossings.length).toBeGreaterThan(0);
    for (const span of crossings) {
      const a = poles[span.from],
        b = poles[span.to];
      expect(a.street).not.toBe(b.street);
      expect(Math.hypot(a.x - b.x, a.z - b.z)).toBeLessThanOrEqual(MAX_CROSSING_SPAN);
      // Sampled points stay over one of the streets.
      for (let k = 1; k < 6; k++) {
        const t = k / 6,
          p = { x: a.x + (b.x - a.x) * t, z: a.z + (b.z - a.z) * t };
        expect(
          Math.abs(p.z) <= roadWidth('local') / 2 + CORNER_ALLOWANCE[0] ||
            Math.abs(p.x - 120) <= roadWidth('local') / 2 + CORNER_ALLOWANCE[0],
        ).toBe(true);
      }
    }
    for (const span of spans) expect(span.from).not.toBe(span.to);
  });

  it('respect the pole cap', () => {
    const points = [],
      segments: Segment[] = [];
    let id = 1;
    for (let i = 0; i < 60; i++) {
      points.push({ id: id, x: 0, z: i * 20 }, { id: id + 1, x: 1200, z: i * 20 });
      segments.push({ id: i + 1, from: id, to: id + 1 });
      id += 2;
    }
    expect(streetWiring(world(segments, points)).poles.length).toBe(MAX_POLES);
  });

  it('hang conductors on the street side of the pole with a believable sag', () => {
    const { points, segments } = splitStreet();
    const { poles } = streetWiring(world(segments, points));
    const pole = poles[0];
    for (let index = 0; index < 4; index++) {
      const p = conductorPoint(pole, index);
      // Toward the street centre, never over the facade line.
      expect(Math.abs(p.z)).toBeLessThan(Math.abs(pole.z));
      expect(p.y).toBeGreaterThan(5);
      expect(p.y).toBeLessThan(pole.height);
    }
  });
});

describe('overhead line layer', () => {
  it('builds instanced poles and batched wires, leaving the rail catenary to the corridor, and disposes cleanly', () => {
    const { points, segments } = splitStreet();
    points.push(
      { id: 500, x: -300, z: -80 },
      { id: 501, x: 100, z: -80, kind: 'station' as const },
      { id: 502, x: 500, z: -80 },
    );
    segments.push(
      { id: 500, from: 500, to: 501, kind: 'rail' as const },
      { id: 501, from: 501, to: 502, kind: 'rail' as const },
    );
    const art = new ArtMaterials();
    const group = buildOverheadLines(world(segments, points), art);
    const instanced = new Map<string, number>();
    const wires: THREE.Mesh[] = [];
    group.traverse((object) => {
      if (object instanceof THREE.InstancedMesh) instanced.set(object.name, object.count);
      else if (object instanceof THREE.Mesh) wires.push(object);
    });
    expect(instanced.get('utility-pole-shafts')).toBeGreaterThan(5);
    expect(instanced.get('utility-pole-insulators')).toBe(
      instanced.get('utility-pole-shafts')! * 3,
    );
    expect([...instanced.keys()].some((name) => name.startsWith('rail-'))).toBe(false);
    expect(wires.length).toBeGreaterThan(1);
    for (const wire of wires) {
      expect(wire.geometry.getAttribute('wireTangent').count).toBe(
        wire.geometry.getAttribute('position').count,
      );
      expect(wire.castShadow).toBe(false);
    }
    // Wire materials are shared art pairs that swap with the style.
    const anime = wires[0].material as THREE.Material;
    art.setStyle('miniature');
    expect(art.variant(anime)).not.toBe(anime);
    disposeGroup(group);
    art.dispose();
  });
});

it('retains pole buffers while producing exactly the fresh placement after a street moves', () => {
  const { points, segments } = splitStreet();
  const art = new ArtMaterials();
  const before = buildOverheadLines(world(segments, points), art);
  const shafts = before.getObjectByName('utility-pole-shafts') as THREE.InstancedMesh;
  const buffer = shafts.instanceMatrix;
  const moved = world(
    segments,
    points.map((p) => ({ ...p, z: p.z + 5 })),
  );
  const expected = buildOverheadLines(moved, art);
  const actual = buildOverheadLines(moved, art, [], before);
  expect(actual.getObjectByName(shafts.name)).toBe(shafts);
  expect(shafts.instanceMatrix).toBe(buffer);
  for (const wanted of expected.children) {
    const got = actual.getObjectByName(wanted.name) as THREE.Mesh;
    expect(got).toBeDefined();
    if (wanted instanceof THREE.InstancedMesh) {
      const mesh = got as THREE.InstancedMesh;
      expect(mesh.count).toBe(wanted.count);
      expect(Array.from(mesh.instanceMatrix.array.slice(0, mesh.count * 16))).toEqual(
        Array.from(wanted.instanceMatrix.array.slice(0, wanted.count * 16)),
      );
    }
    expect(Array.from(got.geometry.getAttribute('position').array)).toEqual(
      Array.from((wanted as THREE.Mesh).geometry.getAttribute('position').array),
    );
  }
  // Retiring the old group must not dispose adopted instances/geometry.
  let disposed = false;
  shafts.geometry.addEventListener('dispose', () => {
    disposed = true;
  });
  disposeGroup(before);
  expect(disposed).toBe(false);
  disposeGroup(actual);
  disposeGroup(expected);
  art.dispose();
});
