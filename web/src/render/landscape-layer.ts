import type { CityRenderer } from './city';
import * as THREE from 'three/webgpu';
import { instanceCapacity, reuseInstances } from './reuse-instances';
import { riverbankTreePoints } from './riverbank-planting';
import { overlapsStation } from './station-clearance';
import { treeClearanceSignature } from './tree-clearance-signature';
import { createTreeClearanceGrid } from './tree-clearance-grid';
import {
  color,
  mix,
  positionLocal,
  smoothstep,
  mx_noise_float,
  add,
  vec3,
  floor,
  fract,
  step,
  vec2,
} from 'three/tsl';
import { backdropForestMatrices } from './backdrop-forest';
import { Kit } from './kit';
import { buildOverheadLines } from './overhead-lines';
import { RiverWater } from './water';
import { createRiverbankGeometry } from './riverbank';
import { sampledRail } from './rail-path';
import { createFoliageGeometry, createTreeTrunkGeometry } from './foliage';
import { seasonAt, seasonKey, treeColor, type SeasonState } from './seasons';
import {
  addNeighborhoodShrine,
  addPocketPark,
  addScrambleCrossing,
  addShrineGrove,
  addStationPlaza,
  openSpaceKind,
  openSpaces,
  streetTreePlacements,
  type PlantedTree,
} from './parks';
import { createBackdropGeometry } from './backdrop';
import { createSkylineGeometry } from './skyline';
import { createPavedGroundTest } from './paved-ground';
import { roadWidth } from './road-elevation';
import { type World, type Point } from '../types';
import { distanceToSegment, dummy, random, v, type TreeBatch, type TreeLook } from './city';

export class LandscapeLayer {
  constructor(private readonly host: CityRenderer) {}

  buildTerrain(world: World) {
    const river = world.terrain.river;
    const halfWidth = (this.host.riverHalfWidth = world.terrain.river_half_width ?? 32);
    // Natural land is the underlay for near-coplanar block/parcel paving.
    // Bias only its depth away from the camera: moving the actual surface would
    // leave tree roots and banks floating, while biasing paving can hide details.
    const land = new THREE.MeshStandardNodeMaterial({
      roughness: 1,
      polygonOffset: true,
      polygonOffsetFactor: 2,
      polygonOffsetUnits: 2,
    });
    const meadow = mx_noise_float(positionLocal.mul(0.024)).mul(0.5).add(0.5);
    const scrub = mx_noise_float(positionLocal.mul(0.007)).mul(0.5).add(0.5);
    const landBase = mix(
      mix(color('#577d61'), color('#94ab6d'), meadow),
      color('#c2bc8c'),
      smoothstep(0.59, 0.79, scrub).mul(0.55),
    ).mul(mx_noise_float(positionLocal.mul(0.6)).mul(0.075).add(0.965));
    land.colorNode = landBase.mul(this.host.art.cloudShade());
    for (const side of [-1, 1]) {
      const shape = new THREE.Shape();
      const edge = side < 0 ? -420 : 420;
      shape.moveTo(edge, -420);
      const pts = side < 0 ? river : [...river].reverse();
      if (side < 0) {
        for (const p of pts) shape.lineTo(p.x - halfWidth, p.z);
        shape.lineTo(edge, 420);
      } else {
        shape.lineTo(edge, 420);
        for (const p of pts) shape.lineTo(p.x + halfWidth, p.z);
      }
      shape.closePath();
      const geo = new THREE.ShapeGeometry(shape);
      geo.rotateX(Math.PI / 2);
      // Shape XY -> XZ with positive z, normals need both sides for this winding.
      land.side = THREE.DoubleSide;
      land.userData.ownedMaterial = true;
      const mesh = new THREE.Mesh(geo, land);
      mesh.userData.ownedMaterial = true;
      mesh.receiveShadow = true;
      this.terrain.add(mesh);
    }
    const visibleRiver = [{ x: river[0].x, z: -1400 }, ...river, { x: river.at(-1)!.x, z: 1400 }];
    this.host.water = new RiverWater(visibleRiver, halfWidth);
    this.host.scene.add(this.host.water.mesh);
    this.host.applyQuality();
    const backdropMaterial = new THREE.MeshStandardNodeMaterial({ roughness: 1 });
    backdropMaterial.colorNode = mix(
      landBase,
      this.outerLandscapeColor(),
      smoothstep(420, 520, positionLocal.x.abs().max(positionLocal.z.abs())),
    ).mul(this.host.art.cloudShade());
    const backdrop = new THREE.Mesh(
      createBackdropGeometry(world.seed, river, halfWidth),
      backdropMaterial,
    );
    backdrop.userData.ownedMaterial = true;
    backdrop.receiveShadow = true;
    this.terrain.add(backdrop);
    // Low-poly crowns give the wooded hills a broken silhouette; haze softens
    // them with distance. One instanced draw, no shadows (outside the frustum).
    const forest = backdropForestMatrices(backdrop.geometry, world.seed);
    if (forest.length) {
      const crowns = new THREE.InstancedMesh(
        new THREE.IcosahedronGeometry(1, 0),
        this.host.art.get('#e9eee5', 0.95),
        forest.length,
      );
      const tones = ['#4a6347', '#58704c', '#3f5843', '#66784f'].map((c) => new THREE.Color(c));
      forest.forEach((matrix, i) => {
        crowns.setMatrixAt(i, matrix);
        crowns.setColorAt(i, tones[i % tones.length]);
      });
      crowns.instanceMatrix.needsUpdate = true;
      crowns.computeBoundingSphere();
      this.terrain.add(crowns);
    }
    const bankKit = new Kit();
    // Sweep the same X-offset boundary used by land and water. Shared bend
    // stations keep the wall and path continuous without overlapping boxes.
    for (const side of [-1, 1] as const) {
      bankKit.add(
        createRiverbankGeometry(river, halfWidth, side, 0, 1.4, -1.3, 0.04),
        this.host.art.get('#9b9e8b'),
      );
      bankKit.add(
        createRiverbankGeometry(river, halfWidth, side, 0, 1.6, 0.04, 0.18),
        this.host.art.get('#b0b2a0'),
      );
      bankKit.add(
        createRiverbankGeometry(river, halfWidth, side, 5.5, 10.5, -0.02, 0.1),
        this.host.art.get('#c2c2a6'),
      );
    }
    this.terrain.add(bankKit.finish(false));
    const rng = random(world.seed + 937);
    const positions: Array<{ x: number; z: number; s: number; h?: number }> = [];
    // Groves leave clear meadows between them; the city gradually clears real
    // development footprints, while riverbank vegetation stays recognizable.
    for (let grove = 0; grove < 26; grove++) {
      const cx = rng() * 740 - 370,
        cz = rng() * 740 - 370,
        radius = 24 + rng() * 40;
      for (let tree = 0; tree < 24; tree++) {
        const a = rng() * Math.PI * 2,
          r = Math.sqrt(rng()) * radius,
          x = cx + Math.cos(a) * r,
          z = cz + Math.sin(a) * r;
        if (
          Math.abs(x) > 397 ||
          Math.abs(z) > 397 ||
          Math.abs(x - this.riverX(river, z)) < halfWidth + 17
        )
          continue;
        positions.push({ x, z, s: 1.5 + rng() * 2.8 });
      }
    }
    // A line of blossoms along the river creates a recognizable foreground rhythm.
    for (let z = -330; z < 350; z += 24)
      positions.push({ x: this.riverX(river, z) + halfWidth + 17, z, s: 2.6 });
    for (let i = 0; i < 360; i++) {
      const z = rng() * 800 - 400,
        side = rng() > 0.5 ? 1 : -1;
      positions.push({
        x: this.riverX(river, z) + side * (halfWidth + 21 + rng() * 28),
        z,
        s: 1.8 + rng() * 2.5,
      });
    }
    positions.push(...riverbankTreePoints(river, halfWidth));
    this.treePoints = positions;
    this.treeClearanceKey = '';
    // Small overlapping leaf masses give a broken canopy silhouette at both
    // observation distances, rather than a row of identical smooth spheres.
    const foliage = createFoliageGeometry('near');
    this.treeNearGeometry = foliage;
    this.treeFarGeometry = createFoliageGeometry('far');
    this.treeTrunkGeometry = createTreeTrunkGeometry();
    this.groveLook = positions.map((p, i) => {
      const blossom = Math.abs(p.x - this.riverX(river, p.z) - halfWidth - 17) < 0.01,
        variation = ((i * 2654435761) >>> 0) / 4294967296;
      return blossom
        ? { species: 'blossom', color: '#e5bdba', variation }
        : {
            species: 'broadleaf',
            color: ['#758a67', '#899b70', '#607e67', '#a0aa7a'][i % 4],
            variation,
          };
    });
    this.treeBatches = this.createTreeBatches(positions, (_, local, i) =>
      treeColor(
        this.groveLook[i].species,
        this.groveLook[i].color,
        this.season,
        this.groveLook[i].variation,
      ),
    );
    for (const batch of this.treeBatches) this.terrain.add(batch.trees, batch.trunks);
    this.updateTrees(world);
    this.buildMountains(rng);
    this.buildSkyline(world.seed, river);
    const detail = new Kit();
    // Park benches along the riverside walk.
    for (let z = -150; z < 300; z += 80) {
      const x = this.riverX(river, z) + halfWidth + 12;
      detail.box(x, 0.9, z, 1.2, 0.15, 3.1, this.host.art.get('#7a6754'));
      detail.box(x + 0.5, 1.3, z, 0.12, 0.8, 3.1, this.host.art.get('#7a6754'));
    }
    this.terrain.add(detail.finish(false));
  }

  private buildMountains(rng: () => number) {
    for (let ring = 0; ring < 3; ring++) {
      const vertices: number[] = [],
        indices: number[] = [];
      const phase = rng() * Math.PI * 2;
      for (let i = 0; i <= 96; i++) {
        const a = (i / 96) * Math.PI * 2,
          r = 2500 + ring * 250,
          h =
            160 +
            ring * 32 +
            Math.sin(a * 5 + phase) * 38 +
            Math.sin(a * 11 + phase * 0.3) * 17 +
            Math.sin(a * 23 + phase) * 7;
        vertices.push(Math.cos(a) * r, -8, Math.sin(a) * r, Math.cos(a) * r, h, Math.sin(a) * r);
        if (i < 96) {
          const n = i * 2;
          indices.push(n, n + 2, n + 1, n + 1, n + 2, n + 3);
        }
      }
      const g = new THREE.BufferGeometry();
      g.setAttribute('position', new THREE.Float32BufferAttribute(vertices, 3));
      g.setIndex(indices);
      g.computeVertexNormals();
      const m = new THREE.MeshBasicNodeMaterial({ side: THREE.DoubleSide });
      m.colorNode = mix(
        this.host.hillColor,
        mix(this.host.skyLow, this.host.skyHigh, 0.55),
        0.2 + ring * 0.24,
      ).mul(mx_noise_float(positionLocal.mul(0.025)).mul(0.055).add(0.97));
      const mountain = new THREE.Mesh(g, m);
      mountain.userData.ownedMaterial = true;
      this.terrain.add(mountain);
    }
  }

  private buildSkyline(seed: number, river: Point[]) {
    const material = new THREE.MeshBasicNodeMaterial({ side: THREE.DoubleSide });
    // World-space window cells: sparse warm lights after dusk, flat haze by day.
    const cell = floor(
      vec3(positionLocal.x.add(positionLocal.z).mul(0.31), positionLocal.y.mul(0.29), 0),
    );
    const lit = step(0.48, mx_noise_float(cell.mul(0.73))).mul(step(6, positionLocal.y));
    material.colorNode = mix(
      this.host.skylineColor,
      color('#e8b878'),
      lit.mul(this.host.skylineNight).mul(0.55),
    ).mul(mx_noise_float(positionLocal.mul(0.02)).mul(0.06).add(0.97));
    const skyline = new THREE.Mesh(createSkylineGeometry(seed, river), material);
    skyline.userData.ownedMaterial = true;
    this.terrain.add(skyline);
  }

  /** Instanced trees in 200 m cells so each cell can switch foliage detail by camera distance. */
  private createTreeBatches<T extends { x: number; z: number }>(
    points: readonly T[],
    colorOf: (p: T, local: number, index: number) => THREE.Color,
  ): TreeBatch[] {
    if (!this.treeNearGeometry || !this.treeTrunkGeometry) return [];
    const cells = new Map<string, number[]>();
    for (let i = 0; i < points.length; i++) {
      const p = points[i],
        key = `${Math.floor((p.x + 400) / 200)}:${Math.floor((p.z + 400) / 200)}`;
      const list = cells.get(key) ?? [];
      list.push(i);
      cells.set(key, list);
    }
    const batches: TreeBatch[] = [];
    for (const [key, indices] of cells) {
      const trunks = new THREE.InstancedMesh(
          this.treeTrunkGeometry,
          this.host.art.get('#746b5c'),
          instanceCapacity(indices.length),
        ),
        trees = new THREE.InstancedMesh(
          this.treeNearGeometry,
          this.host.art.get('#fffffe', 0.96),
          instanceCapacity(indices.length),
        ),
        center = new THREE.Vector3();
      trunks.name = `tree-trunks-${key}`;
      trees.name = `tree-crowns-${key}`;
      trunks.count = trees.count = indices.length;
      trunks.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      trees.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
      trees.castShadow = true;
      trunks.castShadow = true;
      for (let local = 0; local < indices.length; local++) {
        const p = points[indices[local]];
        center.x += p.x;
        center.z += p.z;
        trees.setColorAt(local, colorOf(p, local, indices[local]));
      }
      center.divideScalar(indices.length);
      batches.push({ indices, trees, trunks, center, far: false });
    }
    return batches;
  }

  updateTrees(world: World) {
    const clearanceKey = treeClearanceSignature(world);
    if (clearanceKey === this.treeClearanceKey) return;
    const nodes = new Map(world.nodes.map((n) => [n.id, n]));
    const onPaving = createPavedGroundTest(world.ground ?? []);
    const maxCanopy = Math.max(0, ...this.treePoints.map((p) => p.s * 1.7 + 2));
    const candidatesAt = createTreeClearanceGrid(
      world.buildings,
      world.roads,
      (world.rear_extensions ?? []).slice(0, 2),
      world.nodes,
      maxCanopy,
      (kind) =>
        roadWidth(kind) / 2 + (kind === 'bridge' ? 2.5 + maxCanopy : kind === 'lane' ? 1.5 : 3.5),
      world.rail_alignments,
    );
    const railPaths = new Map(
      (world.rail_alignments ?? []).map((a) => [a.road_id, sampledRail(a)]),
    );
    for (const batch of this.treeBatches) {
      for (let local = 0; local < batch.indices.length; local++) {
        const i = batch.indices[local];
        const p = this.treePoints[i];
        const candidates = candidatesAt(p);
        let hidden =
          onPaving(p) ||
          candidates.buildings.some((b) => {
            const dx = p.x - b.x,
              dz = p.z - b.z,
              c = Math.cos(b.rotation),
              s = Math.sin(b.rotation),
              localX = c * dx - s * dz,
              localZ = s * dx + c * dz,
              canopy = p.s * 1.7 + 2;
            return (
              Math.abs(localX) < b.width / 2 + canopy &&
              localZ > -b.depth / 2 - canopy &&
              localZ < b.depth / 2 + canopy + (b.anchor ? 4 : 0)
            );
          });
        if (!hidden)
          hidden = candidates.roads.some((r) => {
            const a = nodes.get(r.from),
              b = nodes.get(r.to);
            const path = railPaths.get(r.id),
              nearest = path?.at(path.distanceAt(p));
            return (
              a &&
              b &&
              (nearest
                ? Math.hypot(p.x - nearest.x, p.z - nearest.z)
                : distanceToSegment(p, a, b)) <
                roadWidth(r.kind) / 2 +
                  (r.kind === 'bridge' ? 2.5 + (2.38 * p.s) / 1.8 : r.kind === 'lane' ? 1.5 : 3.5)
            );
          });
        if (!hidden)
          hidden = (world.rail_platforms ?? []).some((platform) =>
            overlapsStation(p, p.s * 1.7 + 2, platform),
          );
        if (!hidden)
          hidden = candidates.rearExtensions.some((e) => {
            const dx = p.x - e.x,
              dz = p.z - e.z,
              c = Math.cos(e.rotation),
              s = Math.sin(e.rotation),
              canopy = p.s * 1.7 + 2;
            return (
              Math.abs(c * dx - s * dz) < e.width / 2 + canopy &&
              Math.abs(s * dx + c * dz) < e.depth / 2 + canopy
            );
          });
        dummy.position.set(p.x, 0, p.z);
        dummy.rotation.set(0, i * 2.39996, 0);
        dummy.scale.set(
          hidden ? 0 : p.s / 1.8,
          hidden ? 0 : (p.s / 1.8) * (p.h ?? 1),
          hidden ? 0 : p.s / 1.8,
        );
        dummy.updateMatrix();
        batch.trunks.setMatrixAt(local, dummy.matrix);
        batch.trees.setMatrixAt(local, dummy.matrix);
      }
      batch.trees.instanceMatrix.needsUpdate = true;
      batch.trunks.instanceMatrix.needsUpdate = true;
      batch.trees.computeBoundingSphere();
      batch.trunks.computeBoundingSphere();
    }
    this.treeClearanceKey = clearanceKey;
  }

  updateTreeDetail() {
    if (!this.treeNearGeometry || !this.treeFarGeometry) return;
    for (const batch of [...this.treeBatches, ...this.parkBatches]) {
      const distance = this.host.camera.position.distanceTo(batch.center),
        far = batch.far ? distance > 280 : distance > 330;
      if (far !== batch.far) {
        batch.far = far;
        batch.trees.geometry = far ? this.treeFarGeometry : this.treeNearGeometry;
        batch.trees.computeBoundingSphere();
      }
    }
  }

  /** Repaint every crown for the current season; called only when its quantized key changes. */
  paintSeason() {
    this.paintedSeason = seasonKey(this.season);
    const paint = (batches: TreeBatch[], looks: TreeLook[]) => {
      for (const batch of batches) {
        for (let local = 0; local < batch.indices.length; local++) {
          const look = looks[batch.indices[local]];
          if (look)
            batch.trees.setColorAt(
              local,
              treeColor(look.species, look.color, this.season, look.variation),
            );
        }
        if (batch.trees.instanceColor) batch.trees.instanceColor.needsUpdate = true;
      }
    };
    paint(this.treeBatches, this.groveLook);
    paint(this.parkBatches, this.parkLook);
  }

  clearParks() {
    // Instanced trees share the terrain's foliage and trunk geometry; detach
    // them so retiring the park group disposes only park-owned geometry.
    for (const batch of this.parkBatches) {
      batch.trees.removeFromParent();
      batch.trunks.removeFromParent();
      batch.trees.dispose();
      batch.trunks.dispose();
    }
    this.parkBatches = [];
    this.host.retireGroup(this.parks);
    this.parks = new THREE.Group();
    this.parks.name = 'parks';
    this.host.scene.add(this.parks);
  }

  rebuildParks(world: World) {
    // Retain instance identity across nearby development; fresh InstancedMesh
    // objects otherwise rebuild hundreds of node programs on a world update.
    const previousBatches = this.parkBatches;
    const previousTrees = new Map(previousBatches.map((b) => [b.trees.name, b]));
    for (const batch of previousBatches) {
      batch.trees.removeFromParent();
      batch.trunks.removeFromParent();
    }
    const previousWires = this.parks.getObjectByName('overhead-lines') as THREE.Group | undefined;
    previousWires?.removeFromParent();
    this.parkBatches = [];
    this.clearParks();
    const streetTrees = streetTreePlacements(world),
      kit = new Kit(),
      planted: PlantedTree[] = [...streetTrees];
    for (const parcel of openSpaces(world)) {
      const kind = openSpaceKind(parcel);
      planted.push(
        ...(kind === 'plaza'
          ? addStationPlaza
          : kind === 'shrine'
            ? addNeighborhoodShrine
            : addPocketPark)(kit, parcel, this.host.art),
      );
    }
    addScrambleCrossing(kit, world, this.host.art);
    for (const polygon of world.terrain.protected)
      planted.push(...addShrineGrove(kit, polygon, this.host.art));
    this.parks.add(
      kit.finish(),
      buildOverheadLines(world, this.host.art, streetTrees, previousWires),
    );
    this.parkLook = planted;
    this.parkBatches = this.createTreeBatches(planted, (p) =>
      treeColor(p.species, p.color, this.season, p.variation),
    );
    for (const batch of this.parkBatches) {
      for (let local = 0; local < batch.indices.length; local++) {
        const i = batch.indices[local],
          p = planted[i];
        dummy.position.set(p.x, 0, p.z);
        dummy.rotation.set(0, i * 2.39996, 0);
        dummy.scale.setScalar(p.s / 1.8);
        dummy.updateMatrix();
        batch.trees.setMatrixAt(local, dummy.matrix);
        batch.trunks.setMatrixAt(local, dummy.matrix);
      }
      const previous = previousTrees.get(batch.trees.name);
      const nearGeometry = batch.trees.geometry;
      batch.trees = reuseInstances(previous?.trees, batch.trees);
      batch.trunks = reuseInstances(previous?.trunks, batch.trunks);
      batch.trees.geometry = nearGeometry;
      batch.trees.instanceMatrix.needsUpdate = true;
      batch.trunks.instanceMatrix.needsUpdate = true;
      batch.trees.computeBoundingSphere();
      batch.trunks.computeBoundingSphere();
      this.parks.add(batch.trees, batch.trunks);
    }
    const retained = new Set(this.parkBatches.flatMap((b) => [b.trees, b.trunks]));
    for (const batch of previousBatches)
      for (const mesh of [batch.trees, batch.trunks])
        if (!retained.has(mesh)) this.host.retire(() => mesh.dispose());
    if (previousWires) this.host.retireGroup(previousWires);
  }

  /** Distant land as a working landscape rather than one mottled lawn: a
   * suburban roofscape continues the city past its edge, farmland fills the
   * valley floor and forest climbs the hills. All of it sits behind range haze. */
  private outerLandscapeColor() {
    const p = positionLocal;
    // Field parcels on a slightly rotated grid, each with its own crop tone.
    const u = p.x.mul(0.92).add(p.z.mul(0.39)).div(74),
      v = p.z.mul(0.92).sub(p.x.mul(0.39)).div(46);
    const parcel = mx_noise_float(vec3(floor(u), floor(v), 3.1).mul(0.61))
      .mul(0.5)
      .add(0.5);
    const crops = mix(
      mix(color('#7d985c'), color('#a7a272'), smoothstep(0.3, 0.62, parcel)),
      color('#5d7a4e'),
      smoothstep(0.7, 0.86, parcel),
    );
    const fu = fract(u),
      fv = fract(v);
    const hedge = smoothstep(0.0, 0.05, fu.min(fu.oneMinus()).min(fv.min(fv.oneMinus())));
    const fields = mix(color('#566b47'), crops, hedge.mul(0.35).add(0.65));
    // Forest on higher ground and in irregular woods on the plain. A warped,
    // multi-octave mask gives ragged woodland edges rather than smooth ovals.
    const warp = vec2(
      mx_noise_float(vec2(p.x, p.z).mul(0.02)),
      mx_noise_float(vec2(p.z, p.x).mul(0.02).add(7.3)),
    ).mul(24);
    const wp = vec2(p.x, p.z).add(warp);
    const woods = mx_noise_float(wp.mul(0.005))
      .mul(0.7)
      .add(mx_noise_float(wp.mul(0.021)).mul(0.3))
      .mul(0.5)
      .add(0.5);
    const forestMask = smoothstep(26, 58, p.y).max(smoothstep(0.58, 0.7, woods));
    const crowns = mx_noise_float(p.mul(0.21)).mul(0.5).add(0.5);
    const canopy = mix(color('#4a6248'), color('#6c8053'), crowns.mul(crowns));
    // Distant buildings are silhouettes/geometry, not a translucent roof grid
    // painted over fields. That grid produced ghost rectangles and moire at
    // grazing angles, even with shadows and AO disabled.
    return mix(fields, canopy, forestMask);
  }

  riverX(points: Point[], z: number) {
    for (let i = 0; i < points.length - 1; i++)
      if (z <= points[i + 1].z) {
        const t = (z - points[i].z) / (points[i + 1].z - points[i].z);
        return THREE.MathUtils.lerp(points[i].x, points[i + 1].x, t);
      }
    return points.at(-1)!.x;
  }

  // Crown look per tree (species, summer color, variation) for seasonal repainting.
  private groveLook: TreeLook[] = [];

  private parkBatches: TreeBatch[] = [];

  private parkLook: TreeLook[] = [];

  // Street trees, pocket parks and the shrine grove follow roads and parcels,
  // so they rebuild separately from the seeded terrain groves.
  parks = new THREE.Group();

  paintedSeason = '';

  season: SeasonState = seasonAt(0);

  terrain = new THREE.Group();

  treeBatches: TreeBatch[] = [];

  private treeClearanceKey = '';

  treeFarGeometry: THREE.BufferGeometry | null = null;

  treeNearGeometry: THREE.BufferGeometry | null = null;

  treePoints: Array<{ x: number; z: number; s: number; h?: number }> = [];

  treeTrunkGeometry: THREE.BufferGeometry | null = null;
}
