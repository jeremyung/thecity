import { describe, expect, it } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildingKit, disposeGroup } from '../web/src/render/kit';
import { buildingRenderSignature } from '../web/src/render/city';
import type { Building, Style } from '../web/src/types';

const floorHeight = 3.25,
  base = 0.58;

function tower(profile: number, kind: 'office' | 'apartment', progress: number): Building {
  return {
    id: profile + 1,
    parcel_id: profile + 1,
    x: 0,
    z: 0,
    rotation: 0,
    width: 24,
    depth: 20,
    floors: 22,
    kind,
    stage: progress < 0.6 ? 'structure' : 'surface',
    progress,
    capacity: 80,
    occupancy: 0,
    business_active: false,
    built_day: 0,
    name: `${kind} tower`,
    appearance_seed: profile,
    history: [],
    anchor: false,
    vacant_days: 0,
  };
}

type Box = { x: number; y: number; z: number; w: number; h: number; d: number };

function boxesFor(root: THREE.Object3D, material: THREE.Material): Box[] {
  const result: Box[] = [];
  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh) || object.material !== material) return;
    const p = object.geometry.getAttribute('position');
    for (let start = 0; start + 23 < p.count; start += 24) {
      let minX = Infinity,
        maxX = -Infinity,
        minY = Infinity,
        maxY = -Infinity;
      let minZ = Infinity,
        maxZ = -Infinity;
      for (let i = start; i < start + 24; i++) {
        minX = Math.min(minX, p.getX(i));
        maxX = Math.max(maxX, p.getX(i));
        minY = Math.min(minY, p.getY(i));
        maxY = Math.max(maxY, p.getY(i));
        minZ = Math.min(minZ, p.getZ(i));
        maxZ = Math.max(maxZ, p.getZ(i));
      }
      result.push({
        x: (minX + maxX) / 2,
        y: (minY + maxY) / 2,
        z: (minZ + maxZ) / 2,
        w: maxX - minX,
        h: maxY - minY,
        d: maxZ - minZ,
      });
    }
  });
  return result;
}

describe('tower construction mass geometry', () => {
  it('erects concrete floor plates on the podium, recessed shaft, and crown masses', () => {
    const art = new ArtMaterials();
    const bodyW = 24 * 0.84,
      bodyD = 20 * 0.84;
    for (const style of ['anime', 'miniature'] as Style[]) {
      art.setStyle(style);
      const slab = art.get('#aaa99e');
      for (const profile of [0, 1, 2])
        for (const kind of ['office', 'apartment'] as const) {
          const shaftW = bodyW * (profile === 2 ? 0.77 : 0.86);
          const shaftD = bodyD * (profile === 0 ? 0.79 : 0.87);
          const crownFloors = profile === 1 ? 3 : 2;
          const masses = [
            { x: 0, w: bodyW, d: bodyD, start: 0, count: 3 },
            {
              x: profile === 2 ? -bodyW * 0.07 : 0,
              w: shaftW,
              d: shaftD,
              start: 3,
              count: 22 - 3 - crownFloors,
            },
            {
              x: profile === 2 ? -bodyW * 0.07 : 0,
              w: shaftW * 0.83,
              d: shaftD * 0.82,
              start: 22 - crownFloors,
              count: crownFloors,
            },
          ];

          // At 40% only the structure is erected. At 72%, the first three
          // floors have received their authored envelope; those plates vanish.
          for (const progress of [0.4, 0.72]) {
            const building = tower(profile, kind, progress);
            const group = buildingKit(building, art);
            const builtFloors = Math.ceil(22 * Math.max(0, Math.min(1, (progress - 0.12) / 0.57)));
            const installedFloors = Math.min(
              builtFloors,
              Math.ceil(22 * Math.max(0, Math.min(1, (progress - 0.69) / 0.23))),
            );
            const slabs = boxesFor(group, slab).filter((box) => Math.abs(box.h - 0.16) < 0.002);
            expect(slabs).toHaveLength(builtFloors - installedFloors);
            for (const mass of masses) {
              const erected = Math.max(0, Math.min(mass.count, builtFloors - mass.start));
              for (let localFloor = 0; localFloor < erected; localFloor++) {
                const worldFloor = mass.start + localFloor;
                if (worldFloor < installedFloors) continue;
                const y = base + (worldFloor + 1) * floorHeight - 0.08;
                const plate = slabs.find(
                  (box) =>
                    Math.abs(box.y - y) < 0.002 &&
                    Math.abs(box.x - mass.x) < 0.002 &&
                    Math.abs(box.z) < 0.002 &&
                    Math.abs(box.w - mass.w) < 0.002 &&
                    Math.abs(box.d - mass.d) < 0.002,
                );
                expect(
                  plate,
                  `${style} ${kind} profile ${profile} ${progress} progress floor ${worldFloor + 1}`,
                ).toBeDefined();
              }
            }
            // A full-width generic frame would fail here: every upper plate must
            // use its mass footprint, and the crown is absent before its tier.
            expect(slabs.every((box) => box.w <= bodyW + 0.01 && box.d <= bodyD + 0.01)).toBe(true);
            expect(slabs.every((box) => box.y > base + installedFloors * floorHeight)).toBe(true);
            disposeGroup(group);
          }
        }
    }
    art.dispose();
  });

  it('matches the finished authored envelope at completion without construction emission', () => {
    const art = new ArtMaterials();
    for (const style of ['anime', 'miniature'] as Style[]) {
      art.setStyle(style);
      for (const profile of [0, 1, 2])
        for (const kind of ['office', 'apartment'] as const) {
          const staged = buildingKit(tower(profile, kind, 0.92), art);
          const openBuilding = tower(profile, kind, 1);
          openBuilding.stage = 'open';
          const open = buildingKit(openBuilding, art);
          const constructionEmissive: THREE.Material[] = [];
          staged.traverse((object) => {
            if (!(object instanceof THREE.Mesh)) return;
            const materials = Array.isArray(object.material) ? object.material : [object.material];
            for (const material of materials)
              if (
                (material.userData.artBuildingClass as { emissive?: boolean } | undefined)?.emissive
              )
                constructionEmissive.push(material);
          });
          expect(constructionEmissive, `${style} ${kind} profile ${profile}`).toHaveLength(0);

          // Compare the shared authored tower materials. Emission materials are
          // excluded because operation lighting is intentionally suppressed.
          const profileColors = [
            {
              stone: '#c4c3b8',
              frame: kind === 'apartment' ? '#87999c' : '#768b93',
              shadow: '#59666b',
              glass: '#5c7d8e',
            },
            {
              stone: '#aab6b8',
              frame: kind === 'apartment' ? '#829294' : '#647e8a',
              shadow: '#59666b',
              glass: '#6f8f9a',
            },
            {
              stone: '#d3cec0',
              frame: kind === 'apartment' ? '#99a5a3' : '#8e9da1',
              shadow: '#59666b',
              glass: '#536f83',
            },
          ][profile];
          const shellFrameRoof = new Set<THREE.Material>([
            art.get(profileColors.stone),
            art.get(profileColors.frame, 0.6),
            art.get(kind === 'apartment' ? profileColors.shadow : '#304752'),
            art.get('#64767d'),
            art.get('#becacd', 0.32),
          ]);
          const authored = (root: THREE.Object3D) => {
            const entries: string[] = [];
            root.traverse((object) => {
              if (
                !(object instanceof THREE.Mesh) ||
                !shellFrameRoof.has(object.material as THREE.Material)
              )
                return;
              const p = object.geometry.getAttribute('position');
              entries.push(
                `${(object.material as THREE.Material).uuid}:${Array.from(p.array).join(',')}`,
              );
            });
            return entries.sort();
          };
          const stagedShell = authored(staged),
            openShell = authored(open);
          expect(
            stagedShell.length,
            `${style} ${kind} profile ${profile} staged shell geometry`,
          ).toBeGreaterThan(0);
          expect(
            openShell.length,
            `${style} ${kind} profile ${profile} open shell geometry`,
          ).toBeGreaterThan(0);
          expect(stagedShell, `${style} ${kind} profile ${profile} shell/frame/roof`).toEqual(
            openShell,
          );

          const glass = art.window(kind === 'office' ? 'office' : 'home');
          const boxKey = (box: Box) =>
            [box.x, box.y, box.z, box.w, box.h, box.d].map((v) => Math.round(v * 1000)).join(':');
          const boxCounts = (root: THREE.Object3D) => {
            const counts = new Map<string, number>();
            for (const box of boxesFor(root, glass))
              counts.set(boxKey(box), (counts.get(boxKey(box)) ?? 0) + 1);
            return counts;
          };
          const stagedGlass = boxCounts(staged),
            openGlass = boxCounts(open);
          expect(
            openGlass.size,
            `${style} ${kind} profile ${profile} open glazing`,
          ).toBeGreaterThan(0);
          for (const [key, count] of openGlass) {
            expect(
              stagedGlass.get(key) ?? 0,
              `${style} ${kind} profile ${profile} glazing box ${key}`,
            ).toBeGreaterThanOrEqual(count);
          }

          const roof = art.get('#64767d');
          expect(boxesFor(staged, roof).length).toBeGreaterThan(0);
          disposeGroup(staged);
          disposeGroup(open);
        }
    }
    art.dispose();
  });

  it('omits roof equipment until every tower floor is installed', () => {
    const art = new ArtMaterials();
    const roof = art.get('#64767d');
    const partial = buildingKit(tower(0, 'office', 0.72), art);
    expect(boxesFor(partial, roof)).toHaveLength(0);
    const complete = buildingKit(tower(0, 'office', 0.92), art);
    expect(boxesFor(complete, roof).length).toBeGreaterThan(0);
    disposeGroup(partial);
    disposeGroup(complete);
    art.dispose();
  });

  it('changes the render signature when an erected or clad tower floor advances', () => {
    const baseBuilding = tower(0, 'office', 0.4);
    const sameFloors = { ...baseBuilding, progress: 0.401 };
    const nextErected = { ...baseBuilding, progress: 0.43 };
    expect(buildingRenderSignature(sameFloors)).toBe(buildingRenderSignature(baseBuilding));
    expect(buildingRenderSignature(nextErected)).not.toBe(buildingRenderSignature(baseBuilding));
    const claddingStart = { ...baseBuilding, progress: 0.7 };
    const nextClad = { ...baseBuilding, progress: 0.715 };
    expect(buildingRenderSignature(nextClad)).not.toBe(buildingRenderSignature(claddingStart));
  });
});
