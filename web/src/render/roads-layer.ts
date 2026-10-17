import type { CityRenderer } from './city';
import { roadMarkingTrims } from './road-markings';
import { junctionFootprint, type JunctionArm } from './road-junction';
import { buildStationPlatform } from './station-kit';
import { color } from 'three/tsl';
import { signTexture } from './materials';
import { sampledRail } from './rail-path';
import { prepareStreetLighting } from './street-lighting';
import { bezierRoad } from './road-curve';
import { distanceToSegment } from './city';
import * as THREE from 'three/webgpu';
import { groundRoadFootprint, groundRoadSurfaceGeometry } from './ground-road-footprint';
import { RoadSurfacePartition } from './road-surface-partition';
import { type MarkingTrim } from './road-markings';
import { NearbyStreetLights } from './nearby-street-lights';
import { VisibleInfrastructure, type InfrastructureCommit } from './visible-infrastructure';
import { add, step } from 'three/tsl';
import { Kit, disposeGroup } from './kit';
import { StagedKit } from './staged-kit';
import { trainWorldWithInfrastructure } from './trains';
import { addRailCorridor } from './rail-corridor';
import { type RailPath } from './rail-path';
import { RAIL_CENTER_OFFSET, RAIL_HEAD_WIDTH } from './rail-profile';
import { PersistentStreetLighting, type StreetLightingData } from './street-lighting';
import { sweepRoad, type RoadCurve } from './road-curve';
import { CURRENT_EXPRESSWAY_PROFILE } from './expressway-profile';
import { addExpresswayFurniture } from './expressway-kit';
import { roadBatchSignatures } from './road-batch-signature';
import {
  ARTERIAL_SIDEWALK,
  bridgeDeckHeight,
  EXPRESSWAY_DECK,
  RAIL_DECK,
  roadWidth,
  expresswayPierClear,
  type CrossingDeck,
  type BridgeSpan,
} from './road-elevation';
import { type World, type Point, type Road } from '../types';
import { v } from './city';

export class RoadsLayer {
  constructor(private readonly host: CityRenderer) {}

  rebuildRoads(world: World, completedSignature = roadBatchSignatures(world).completed) {
    const copy = structuredClone({
      nodes: world.nodes,
      roads: world.roads,
      projects: world.projects,
      expressway_profile: world.expressway_profile,
      road_decks: world.road_decks,
      rail_platforms: world.rail_platforms,
      terrain: world.terrain,
    });
    world = { ...world, ...copy };
    if (this.roadBuild || this.queuedRoadWorld) {
      // Keep one active coherent build and one latest input. Restarting an
      // active job on every snapshot can prevent it from ever becoming visible.
      this.queuedRoadWorld = { world, signature: completedSignature };
      return;
    }
    this.prepareRoads(world, completedSignature);
  }

  cancelRoadBuild() {
    if (this.roadBuild) {
      const job = this.roadBuild;
      job.preparation?.return();
      job.kit.dispose();
      disposeGroup(job.construction);
      disposeGroup(job.station);
    }
    this.roadBuild = null;
    this.queuedRoadWorld = null;
  }

  updateRoads() {
    if (!this.roadBuild) {
      if (this.queuedRoadWorld) {
        const next = this.queuedRoadWorld;
        this.queuedRoadWorld = null;
        try {
          this.prepareRoads(next.world, next.signature);
        } catch (error) {
          this.roadSignature = '';
          throw error;
        }
      }
      return;
    }
    const job = this.roadBuild;
    let built: THREE.Group | null = null;
    try {
      if (job.preparation) {
        const deadline = performance.now() + 2;
        for (let units = 0; units < 128; units++) {
          if (job.preparation.next().done) {
            job.preparation = null;
            break;
          }
          if (performance.now() >= deadline) break;
        }
        return;
      }
      const deadline = performance.now() + 2;
      let complete = false;
      // Allow cheap slices to share a frame, but never drain a whole job here.
      // A single primitive/merge slice may exceed this soft time budget.
      for (let slices = 0; slices < 8; slices++) {
        if (job.kit.step()) {
          complete = true;
          break;
        }
        if (performance.now() >= deadline) break;
      }
      if (!complete) return;
      built = job.kit.takeGroup()!;
      const roads = new THREE.Group();
      roads.add(built);
      // Detached candidates miss scene-wide style traversal during preparation.
      for (const group of [roads, job.construction, job.station])
        group.traverse((o) => {
          if (o instanceof THREE.Mesh)
            o.material = Array.isArray(o.material)
              ? o.material.map((m) => this.host.art.variant(m))
              : this.host.art.variant(o.material);
        });
      this.host.retireGroup(this.roads);
      this.host.retireGroup(this.constructionRoads);
      this.host.retireGroup(this.station);
      this.roads = roads;
      this.constructionRoads = job.construction;
      this.station = job.station;
      this.host.scene.add(this.roads, this.constructionRoads, this.station);
      if (job.lighting.data) {
        if (!this.streetLighting) {
          this.streetLighting = new PersistentStreetLighting(this.host.art);
          this.host.scene.add(this.streetLighting.root);
        }
        this.streetLighting.commit(job.lighting.data);
        this.nearbyStreetLights?.setSources(job.lighting.data.sources);
      }
      this.infrastructure().commit(job.identities);
      const bridgeNodes = new Map(job.world.nodes.map((n) => [n.id, n]));
      this.bridgeSpans = job.world.roads
        .filter((r) => r.kind === 'bridge' && r.stage === 'open')
        .flatMap((r) => {
          const a = bridgeNodes.get(r.from),
            b = bridgeNodes.get(r.to);
          return a && b ? [{ a, b }] : [];
        });
      this.committedRoadWorld = job.world;
      this.host.actors.trains.setWorld(
        trainWorldWithInfrastructure(this.host.worldState ?? job.world, job.world),
      );
      this.completedRoadSignature = job.signature;
      this.completedRoadSurfaces = job.surfaces;
      this.roadBuild = null;
    } catch (error) {
      if (built) disposeGroup(built);
      this.cancelRoadBuild();
      this.roadSignature = '';
      throw error;
    }
  }

  private prepareRoads(world: World, completedSignature: string) {
    const kit = new StagedKit(),
      constructionKit = new Kit();
    const construction = new THREE.Group(),
      station = new THREE.Group(),
      lighting: { data: StreetLightingData | null } = { data: null };
    const rebuildCompleted = completedSignature !== this.completedRoadSignature;
    const surfaces = rebuildCompleted
      ? new RoadSurfacePartition()
      : (this.completedRoadSurfaces ?? new RoadSurfacePartition());
    const preparation = this.populateRoads(
      world,
      rebuildCompleted,
      kit,
      constructionKit,
      construction,
      station,
      lighting,
      surfaces,
    );
    if (rebuildCompleted) {
      this.roadBuild = {
        kit,
        construction,
        station,
        lighting,
        signature: completedSignature,
        surfaces,
        identities: this.infrastructure().capture(world),
        world,
        preparation,
      };
    } else {
      try {
        // Construction-only refreshes contain no preparation yields.
        preparation.next();
        kit.dispose();
        this.host.retireGroup(this.constructionRoads);
        this.constructionRoads = construction;
        this.host.scene.add(construction);
      } catch (error) {
        kit.dispose();
        disposeGroup(construction);
        disposeGroup(station);
        throw error;
      }
    }
  }

  addGroundRoadSurface(kit: Kit, pieces: Point[][], material: THREE.Material) {
    for (const points of pieces) {
      kit.add(groundRoadSurfaceGeometry(points), material);
    }
  }

  roadKit(
    kit: Kit,
    a: Point & { kind?: string },
    b: Point & { kind?: string },
    r: Road,
    deckHeight = EXPRESSWAY_DECK,
    crossings: readonly CrossingDeck[] = [],
    curve?: RoadCurve,
    platforms = this.host.worldState?.rail_platforms,
    markingTrim: MarkingTrim = { start: 0, end: 0 },
    surfaces?: RoadSurfacePartition,
    expresswayProfile = CURRENT_EXPRESSWAY_PROFILE,
  ) {
    if (r.kind === 'rail' && curve) {
      const open = r.stage === 'open',
        end = curve.length * (open ? 1 : THREE.MathUtils.clamp((r.progress - 0.12) / 0.88, 0, 1));
      const y = 1 + RAIL_DECK,
        width = roadWidth('rail'),
        concrete = this.host.art.get('#9a9d93');
      if (!open)
        for (let d = 0; d < curve.length; d += 18) {
          const p = curve.at(d);
          for (const side of [-1, 1])
            kit.box(
              p.x + p.uz * side * 5,
              0.7,
              p.z - p.ux * side * 5,
              0.12,
              1.4,
              0.12,
              this.host.art.get('#b79b73'),
            );
        }
      if (end < 0.01) return;
      sweepRoad(kit, curve, 0, end, 0, y - 0.35, width + 0.6, 0.5, this.host.art.get('#a3a69c'));
      sweepRoad(
        kit,
        curve,
        0,
        end,
        0,
        y,
        width,
        0.3,
        this.host.art.get(open ? '#747f7b' : '#ac9a78'),
      );
      for (let d = 11; d < end; d += 22) {
        const p = curve.at(d),
          yaw = Math.atan2(p.ux, p.uz);
        for (const side of [-1, 1])
          kit.box(
            p.x + p.uz * side * 2.4,
            y / 2,
            p.z - p.ux * side * 2.4,
            1.1,
            y,
            1.1,
            concrete,
            yaw,
          );
        kit.box(p.x, y - 0.55, p.z, width + 1.2, 0.7, 1.4, concrete, yaw);
      }
      for (const side of [-1, 1])
        sweepRoad(
          kit,
          curve,
          0,
          end,
          side * RAIL_CENTER_OFFSET,
          y + 0.45,
          RAIL_HEAD_WIDTH,
          0.17,
          this.host.art.get('#a3aca7', 0.4),
        );
      for (let d = 0; d < end; d += 0.75) {
        const p = curve.at(d);
        kit.box(
          p.x,
          y + 0.29,
          p.z,
          2.2,
          0.15,
          0.24,
          this.host.art.get('#6d6457'),
          Math.atan2(p.ux, p.uz),
        );
      }
      if (open) addRailCorridor(kit, a, b, this.host.art, platforms, curve as RailPath);
      return;
    }
    if (r.kind === 'expressway' && curve) {
      const open = r.stage === 'open',
        end = curve.length * (open ? 1 : THREE.MathUtils.clamp((r.progress - 0.12) / 0.88, 0, 1));
      if (!open)
        for (let distance = 0; distance < curve.length; distance += 18) {
          const p = curve.at(distance);
          for (const side of [-1, 1])
            kit.box(
              p.x + p.uz * side * 5,
              0.7,
              p.z - p.ux * side * 5,
              0.12,
              1.4,
              0.12,
              this.host.art.get('#b79b73'),
            );
        }
      if (end < 0.01) return;
      const width = expresswayProfile.width,
        y = deckHeight,
        concrete = this.host.art.get('#9a9d93');
      sweepRoad(kit, curve, 0, end, 0, y - 0.35, width + 0.6, 0.5, this.host.art.get('#a3a69c'));
      sweepRoad(
        kit,
        curve,
        0,
        end,
        0,
        y,
        width,
        0.3,
        open ? this.host.art.asphalt() : this.host.art.get('#ac9a78'),
      );
      for (const side of [-1, 1])
        sweepRoad(
          kit,
          curve,
          0,
          end,
          side * (width / 2 + 0.25),
          y + 0.75,
          0.3,
          1.3,
          this.host.art.get('#b8bbb2'),
        );
      for (let distance = 14; distance < end; distance += 28) {
        const p = curve.at(distance),
          angle = Math.atan2(p.ux, p.uz);
        if (
          !expresswayPierClear(
            p,
            { x: p.x - p.ux, z: p.z - p.uz },
            { x: p.x + p.ux, z: p.z + p.uz },
            y,
            crossings,
          )
        )
          continue;
        kit.box(p.x, y / 2, p.z, 2.2, y, 2.2, concrete, angle);
        kit.box(p.x, y - 0.55, p.z, width + 1.2, 0.7, 1.4, concrete, angle);
      }
      if (open)
        addExpresswayFurniture(
          kit,
          a,
          b,
          {
            paint: this.host.art.get('#dedaca'),
            concrete,
            metal: this.host.art.get('#53696a'),
            lamp: this.host.art.get('#ead5a3', 0.5, true),
          },
          y,
          curve,
          expresswayProfile,
        );
      return;
    }
    if (r.stage !== 'open') {
      const fullDx = b.x - a.x,
        fullDz = b.z - a.z,
        fullLength = Math.hypot(fullDx, fullDz),
        bearing = Math.atan2(fullDx, fullDz);
      // Survey stakes show the reserved corridor before any usable deck exists.
      for (let distance = 0; distance < fullLength; distance += 18) {
        const t = distance / fullLength;
        for (const side of [-1, 1])
          kit.box(
            a.x + fullDx * t + Math.cos(bearing) * side * 5,
            0.7,
            a.z + fullDz * t - Math.sin(bearing) * side * 5,
            0.12,
            1.4,
            0.12,
            this.host.art.get('#b79b73'),
          );
      }
      const fraction = THREE.MathUtils.clamp((r.progress - 0.12) / 0.88, 0, 1);
      if (fraction < 0.015) return;
      b = { x: a.x + fullDx * fraction, z: a.z + fullDz * fraction };
    }
    const dx = b.x - a.x,
      dz = b.z - a.z,
      len = Math.hypot(dx, dz),
      angle = Math.atan2(dx, dz),
      mid = { x: (a.x + b.x) / 2, z: (a.z + b.z) / 2 },
      rail = r.kind === 'rail',
      open = r.stage === 'open',
      bridge = r.kind === 'bridge';
    const expressway = r.kind === 'expressway',
      lane = r.kind === 'lane';
    const width = expressway ? expresswayProfile.width : roadWidth(r.kind),
      y = bridge ? 1.4 : rail ? 1 + RAIL_DECK : expressway ? deckHeight : 0.15;
    if (rail || expressway) {
      // Viaduct piers: single columns under the expressway, paired under rail.
      const spacing = expressway ? 28 : 22,
        pierColor = this.host.art.get('#9a9d93');
      for (let distance = spacing / 2; distance < len; distance += spacing) {
        const t = distance / len,
          px = a.x + dx * t,
          pz = a.z + dz * t;
        if (expressway && !expresswayPierClear({ x: px, z: pz }, a, b, y, crossings)) continue;
        if (expressway) kit.box(px, y / 2, pz, 2.2, y, 2.2, pierColor, angle);
        else
          for (const side of [-1, 1])
            kit.box(
              px + Math.cos(angle) * side * 2.4,
              y / 2,
              pz - Math.sin(angle) * side * 2.4,
              1.1,
              y,
              1.1,
              pierColor,
              angle,
            );
        kit.box(px, y - 0.55, pz, width + 1.2, 0.7, 1.4, pierColor, angle);
      }
      kit.box(mid.x, y - 0.35, mid.z, width + 0.6, 0.5, len, this.host.art.get('#a3a69c'), angle);
      if (expressway)
        for (const side of [-1, 1])
          kit.box(
            mid.x + Math.cos(angle) * side * (width / 2 + 0.25),
            y + 0.75,
            mid.z - Math.sin(angle) * side * (width / 2 + 0.25),
            0.3,
            1.3,
            len,
            this.host.art.get('#b8bbb2'),
            angle,
          );
    }
    if (bridge) {
      const ramp = Math.min(12, len * 0.22),
        shape = new THREE.Shape();
      shape.moveTo(-len / 2, 0.3);
      shape.lineTo(-len / 2 + ramp, 2);
      shape.lineTo(len / 2 - ramp, 2);
      shape.lineTo(len / 2, 0.3);
      shape.lineTo(len / 2, -0.15);
      shape.lineTo(len / 2 - ramp, 0.8);
      shape.lineTo(-len / 2 + ramp, 0.8);
      shape.lineTo(-len / 2, -0.15);
      shape.closePath();
      const deck = new THREE.ExtrudeGeometry(shape, {
        depth: width + 2.8,
        bevelEnabled: false,
        steps: 1,
      });
      deck.setIndex(Array.from({ length: deck.getAttribute('position').count }, (_, i) => i));
      deck.rotateY(-Math.PI / 2);
      deck.translate((width + 2.8) / 2, 0, 0);
      deck.rotateY(angle);
      deck.translate(mid.x, 0, mid.z);
      kit.add(deck, open ? this.host.art.asphalt() : this.host.art.get('#ac9a78'));
    } else if (surfaces && !rail && !expressway) {
      this.addGroundRoadSurface(
        kit,
        surfaces.claim(groundRoadFootprint(a, b, width)),
        open ? this.host.art.asphalt() : this.host.art.get('#ac9a78'),
      );
    } else
      kit.box(
        mid.x,
        y,
        mid.z,
        width,
        0.3,
        len,
        open ? this.host.art.asphalt() : this.host.art.get('#ac9a78'),
        angle,
      );
    if (expressway) {
      if (open)
        addExpresswayFurniture(
          kit,
          a,
          b,
          {
            paint: this.host.art.get('#dedaca'),
            concrete: this.host.art.get('#9a9d93'),
            metal: this.host.art.get('#53696a'),
            lamp: this.host.art.get('#ead5a3', 0.5, true),
          },
          y,
          undefined,
          expresswayProfile,
        );
      return;
    }
    if (rail) {
      if (open) addRailCorridor(kit, a, b, this.host.art, platforms);
      for (const side of [-1, 1])
        kit.box(
          mid.x + Math.cos(angle) * side * RAIL_CENTER_OFFSET,
          y + 0.45,
          mid.z - Math.sin(angle) * side * RAIL_CENTER_OFFSET,
          RAIL_HEAD_WIDTH,
          0.17,
          len,
          this.host.art.get('#a3aca7', 0.4),
          angle,
        );
      for (let distance = 0; distance < len; distance += 0.75) {
        const t = distance / len;
        kit.box(
          a.x + dx * t,
          y + 0.29,
          a.z + dz * t,
          2.2,
          0.15,
          0.24,
          this.host.art.get('#6d6457'),
          angle,
        );
      }
    } else if (lane) {
      // Lanes are shared-surface roji: no center line and no raised sidewalk.
    } else {
      if (open) {
        if (bridge || r.kind === 'arterial')
          for (let distance = 8; distance < len - 4; distance += 12) {
            if (
              !bridge &&
              (distance - 2 < markingTrim.start || distance + 2 > len - markingTrim.end)
            )
              continue;
            if (bridge) {
              // Split painted marks at ramp creases so neither end floats or sinks.
              const ramp = Math.min(12, len * 0.22),
                cuts = [
                  distance - 2,
                  ...[ramp, len - ramp].filter((v) => v > distance - 2 && v < distance + 2),
                  distance + 2,
                ];
              for (let i = 0; i < cuts.length - 1; i++) {
                const start = cuts[i],
                  end = cuts[i + 1],
                  fromY = bridgeDeckHeight(start, len),
                  toY = bridgeDeckHeight(end, len),
                  along = (start + end) / 2;
                const mark = new THREE.BoxGeometry(
                  0.18,
                  0.035,
                  Math.hypot(end - start, toY - fromY),
                );
                mark.rotateX(-Math.atan2(toY - fromY, end - start));
                mark.rotateY(angle);
                mark.translate(
                  a.x + (dx * along) / len,
                  (fromY + toY) / 2 + 0.035,
                  a.z + (dz * along) / len,
                );
                kit.add(mark, this.host.art.get('#dedaca'));
              }
            } else {
              const t = distance / len;
              kit.box(
                a.x + dx * t,
                y + 0.19,
                a.z + dz * t,
                0.18,
                0.035,
                4,
                this.host.art.get('#dedaca'),
                angle,
              );
              // Four-lane arterials: dashed lane dividers between the two lanes each way.
              if (r.kind === 'arterial')
                for (const side of [-1, 1])
                  kit.box(
                    a.x + dx * t + (Math.cos(angle) * side * width) / 4,
                    y + 0.19,
                    a.z + dz * t - (Math.sin(angle) * side * width) / 4,
                    0.14,
                    0.03,
                    2.4,
                    this.host.art.get('#dedaca'),
                    angle,
                  );
            }
          }
        // Open arterial sidewalks are committed separately by CitySidewalks.
        // Local streets retain painted edges on their shared surface.
        if (!bridge && r.kind === 'arterial' && !open)
          for (const side of [-1, 1])
            kit.box(
              mid.x + Math.cos(angle) * side * (width / 2 + ARTERIAL_SIDEWALK / 2),
              y + 0.15,
              mid.z - Math.sin(angle) * side * (width / 2 + ARTERIAL_SIDEWALK / 2),
              ARTERIAL_SIDEWALK,
              0.35,
              Math.max(1, len - width),
              this.host.art.get('#b4b8a6'),
              angle,
            );
        else if (!bridge && open && r.kind !== 'arterial') {
          const start = Math.max(width / 2, markingTrim.start),
            end = Math.min(len - width / 2, len - markingTrim.end);
          if (end > start)
            for (const side of [-1, 1]) {
              const t = (start + end) / (2 * len);
              kit.box(
                a.x + dx * t + Math.cos(angle) * side * (width / 2 - 0.45),
                y + 0.16,
                a.z + dz * t - Math.sin(angle) * side * (width / 2 - 0.45),
                0.14,
                0.03,
                end - start,
                this.host.art.get('#dedaca'),
                angle,
              );
            }
        }
      } else {
        for (let distance = 0; distance < len; distance += 12) {
          const t = distance / len,
            x = a.x + dx * t,
            z = a.z + dz * t;
          kit.cylinder(x + width / 2, y + 0.55, z, 0.25, 0.8, this.host.art.get('#c17d4f'), 5);
        }
      }
      if (bridge) {
        const ramp = Math.min(12, len * 0.22),
          distances = [0, ramp, len - ramp, len];
        for (const side of [-1, 1])
          for (let i = 0; i < 3; i++) {
            const start = distances[i],
              end = distances[i + 1],
              fromY = bridgeDeckHeight(start, len),
              toY = bridgeDeckHeight(end, len),
              along = (start + end) / 2;
            const railing = new THREE.BoxGeometry(0.14, 1.0, Math.hypot(end - start, toY - fromY));
            railing.rotateX(-Math.atan2(toY - fromY, end - start));
            railing.rotateY(angle);
            railing.translate(
              a.x + (dx * along) / len + Math.cos(angle) * side * (width / 2 + 1.28),
              (fromY + toY) / 2 + 0.5,
              a.z + (dz * along) / len - Math.sin(angle) * side * (width / 2 + 1.28),
            );
            kit.add(railing, this.host.art.get('#668084'));
          }
        for (const t of [0.28, 0.55, 0.76])
          kit.box(a.x + dx * t, -1, a.z + dz * t, width * 0.8, 5, 2, this.host.art.get('#a2aa9e'));
      }
      // Ordinary streets use buildStreetLighting's shared fixtures and pools.
      // Retain this separate bridge kit until its sloped deck has a light model.
      if (open && bridge && len > 25) {
        for (let distance = 12; distance < len - 8; distance += 44) {
          const t = distance / len,
            x = a.x + dx * t + Math.cos(angle) * (width / 2 + 1.5),
            z = a.z + dz * t - Math.sin(angle) * (width / 2 + 1.5);
          const baseY = bridgeDeckHeight(distance, len);
          kit.cylinder(x, baseY + 3.5, z, 0.1, 7, this.host.art.get('#53696a'), 6);
          kit.box(x - 0.7, baseY + 7, z, 1.8, 0.2, 0.5, this.host.art.get('#ead5a3', 0.5, true));
        }
      }
    }
  }

  get pendingRoads() {
    return Number(Boolean(this.roadBuild || this.queuedRoadWorld));
  }

  infrastructure() {
    return (this.visibleInfrastructure ??= new VisibleInfrastructure());
  }

  bridgeSpans: BridgeSpan[] = [];

  committedRoadWorld: World | null = null;

  completedRoadSignature = '';

  completedRoadSurfaces: RoadSurfacePartition | null = null;

  constructionRoads = new THREE.Group();

  nearbyStreetLights = new NearbyStreetLights();

  private queuedRoadWorld: { world: World; signature: string } | null = null;

  private roadBuild: {
    kit: StagedKit;
    construction: THREE.Group;
    station: THREE.Group;
    lighting: { data: StreetLightingData | null };
    signature: string;
    surfaces: RoadSurfacePartition;
    identities: InfrastructureCommit;
    world: World;
    preparation: Generator<void, void, unknown> | null;
  } | null = null;

  roadSignature = '';

  roads = new THREE.Group();

  station = new THREE.Group();

  streetLighting: PersistentStreetLighting | undefined;

  private visibleInfrastructure?: VisibleInfrastructure;

  private *populateRoads(
    world: World,
    rebuildCompleted: boolean,
    kit: StagedKit,
    constructionKit: Kit,
    construction: THREE.Group,
    station: THREE.Group,
    lighting: { data: StreetLightingData | null },
    surfaces: RoadSurfacePartition,
  ): Generator<void, void, unknown> {
    const nodes = new Map(world.nodes.map((n) => [n.id, n]));
    try {
      if (rebuildCompleted) {
        // Continuous pedestrian-scale river railings. Reserve openings for real
        // bridges, including construction, and rebuild when infrastructure changes.
        const bankMetal = this.host.art.get('#73827b', 0.66);
        for (const side of [-1, 1])
          for (let z = -390; z < 390; z += 6) {
            const a = {
              x:
                this.host.landscape.riverX(world.terrain.river, z) +
                side * (this.host.riverHalfWidth + 5.4),
              z,
            };
            const b = {
              x:
                this.host.landscape.riverX(world.terrain.river, z + 6) +
                side * (this.host.riverHalfWidth + 5.4),
              z: z + 6,
            };
            const middle = { x: (a.x + b.x) / 2, z: z + 3 };
            const opening = world.roads.some((road) => {
              if (road.kind !== 'bridge') return false;
              const from = nodes.get(road.from),
                to = nodes.get(road.to);
              return from && to && distanceToSegment(middle, from, to) < 10;
            });
            if (opening) continue;
            const length = Math.hypot(b.x - a.x, 6),
              angle = Math.atan2(b.x - a.x, 6);
            kit.box(a.x, 0.73, a.z, 0.08, 1.46, 0.08, bankMetal);
            for (const y of [0.64, 1.28])
              kit.box(middle.x, y, middle.z, 0.055, 0.065, length, bankMetal, angle);
            yield;
          }
      }
      const deckHeights = new Map((world.road_decks ?? []).map((d) => [d.road_id, d.deck_height]));
      const railAlignments = new Map((world.rail_alignments ?? []).map((a) => [a.road_id, a]));
      const crossings: CrossingDeck[] = world.roads.flatMap((road) => {
        const alignment = railAlignments.get(road.id);
        if (alignment)
          return alignment.samples.slice(1).map((s, i) => ({
            a: { x: alignment.samples[i][1], z: alignment.samples[i][2] },
            b: { x: s[1], z: s[2] },
            width: roadWidth(road.kind),
            height: 1 + RAIL_DECK,
          }));
        const a = nodes.get(road.from),
          b = nodes.get(road.to);
        return a && b
          ? [
              {
                a,
                b,
                width:
                  road.kind === 'expressway'
                    ? (world.expressway_profile ?? CURRENT_EXPRESSWAY_PROFILE).width
                    : roadWidth(road.kind),
                height:
                  road.kind === 'expressway'
                    ? (deckHeights.get(road.id) ?? EXPRESSWAY_DECK)
                    : road.kind === 'rail'
                      ? 1 + RAIL_DECK
                      : 0,
              },
            ]
          : [];
      });
      const curves = new Map<number, RoadCurve>();
      for (const alignment of world.rail_alignments ?? [])
        curves.set(alignment.road_id, sampledRail(alignment));
      const deckById = new Map((world.road_decks ?? []).map((d) => [d.road_id, d]));
      for (const road of world.roads) {
        const a = nodes.get(road.from),
          b = nodes.get(road.to),
          deck = deckById.get(road.id);
        if (a && b && deck) {
          const [c, d] = deck.controls;
          curves.set(road.id, bezierRoad(a, { x: c[0], z: c[1] }, { x: d[0], z: d[1] }, b));
        }
      }
      const markingTrims = rebuildCompleted
        ? roadMarkingTrims(world)
        : new Map<number, MarkingTrim>();
      // Existing pavement owns its footprint before construction claims the remainder.
      // Stable ordering also removes coplanar overlap between neighboring open roads.
      const orderedRoads = [...world.roads].sort((a, b) => a.id - b.id);
      if (rebuildCompleted)
        for (const road of orderedRoads) {
          if (road.stage !== 'open') continue;
          const a = nodes.get(road.from),
            b = nodes.get(road.to);
          if (!a || !b) continue;
          this.roadKit(
            kit,
            a,
            b,
            road,
            deckHeights.get(road.id),
            crossings,
            curves.get(road.id),
            world.rail_platforms,
            markingTrims.get(road.id),
            surfaces,
            world.expressway_profile,
          );
          yield;
        }
      if (rebuildCompleted) {
        const junctions = new Map<number, JunctionArm[]>();
        for (const road of world.roads) {
          if (
            road.stage !== 'open' ||
            road.kind === 'rail' ||
            road.kind === 'bridge' ||
            road.kind === 'expressway'
          )
            continue;
          const a = nodes.get(road.from),
            b = nodes.get(road.to);
          if (!a || !b) continue;
          for (const [node, end] of [
            [a, b],
            [b, a],
          ]) {
            const arms = junctions.get(node.id) ?? [];
            arms.push({ end, width: roadWidth(road.kind) });
            junctions.set(node.id, arms);
          }
        }
        for (const [id, arms] of junctions) {
          const node = nodes.get(id)!;
          const points = junctionFootprint(node, arms);
          const pieces = surfaces.claim(points);
          this.addGroundRoadSurface(kit, pieces, this.host.art.asphalt());
          yield;
        }
      }
      const constructionSurfaces = surfaces.fork();
      for (const road of orderedRoads) {
        if (road.stage === 'open') continue;
        const a = nodes.get(road.from),
          b = nodes.get(road.to);
        if (a && b)
          this.roadKit(
            constructionKit,
            a,
            b,
            road,
            deckHeights.get(road.id),
            crossings,
            curves.get(road.id),
            world.rail_platforms,
            undefined,
            constructionSurfaces,
            world.expressway_profile,
          );
        if (rebuildCompleted) yield;
      }
      for (const project of world.projects.filter((p) => p.kind === 'road' && p.progress < 1)) {
        const a = nodes.get(project.from!),
          b = nodes.get(project.to!);
        if (!a || !b) continue;
        if (world.roads.some((r) => r.from === a.id && r.to === b.id && r.stage !== 'open'))
          continue;
        this.roadKit(
          constructionKit,
          a,
          b,
          {
            id: project.id,
            from: a.id,
            to: b.id,
            kind:
              world.roads.find(
                (r) => (r.from === a.id && r.to === b.id) || (r.from === b.id && r.to === a.id),
              )?.kind ?? (Math.abs(a.x - b.x) > 100 ? 'bridge' : 'local'),
            stage: project.stage,
            progress: project.progress,
            connected: false,
            length: Math.hypot(a.x - b.x, a.z - b.z),
            lanes: 1,
          },
          undefined,
          [],
          undefined,
          world.rail_platforms,
          undefined,
          constructionSurfaces,
          world.expressway_profile,
        );
      }
      construction.add(constructionKit.finish());
      if (rebuildCompleted) yield;
      if (rebuildCompleted) {
        for (const node of world.nodes.filter((n) => n.kind === 'market' || n.kind === 'station')) {
          for (let stripe = -3; stripe <= 3; stripe++)
            kit.box(
              node.x + stripe * 1.1,
              0.37,
              node.z,
              0.65,
              0.025,
              6,
              this.host.art.get('#dedaca'),
            );
        }
        lighting.data = prepareStreetLighting(world);
        yield;
        for (const layout of world.rail_platforms ?? []) {
          const platform = buildStationPlatform(layout, this.host.art);
          if (layout.construction_phase === 4) {
            const signMaterial = new THREE.MeshBasicNodeMaterial({
              map: signTexture('青葉川  AOBAGAWA'),
              color: '#fff4d9',
            });
            const board = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.72, 3.4), signMaterial);
            board.userData.ownedMaterial = true;
            board.userData.ownedTexture = signMaterial.map;
            board.position.set(3.8, layout.surface_y + 2.6, 24);
            platform.add(board);
          }
          station.add(platform);
          yield;
        }
      }
    } finally {
      constructionKit.dispose();
    }
  }
}
