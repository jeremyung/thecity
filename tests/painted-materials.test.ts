import { afterEach, describe, expect, it, vi } from "vitest";
import * as THREE from "three/webgpu";
import { ArtMaterials } from "../web/src/render/materials";

function userClass(material: THREE.Material): { key: string; roughness: number; emissive: boolean; painted: boolean; activity: string } {
  return material.userData.artBuildingClass;
}

function installCanvasForFoliageMask() {
  const context = new Proxy({} as Record<PropertyKey, unknown>, {
    get(target, key) {
      if (!(key in target)) target[key] = () => undefined;
      return target[key];
    },
    set(target, key, value) { target[key] = value; return true; },
  });
  vi.stubGlobal("document", {
    createElement: () => ({ width: 1, height: 1, getContext: () => context }),
  });
}

afterEach(() => vi.unstubAllGlobals());

describe("painted surface material classes", () => {
  it("keeps day-night glazing separate from equally colored lamps through batching and style switches", () => {
    const art = new ArtMaterials();
    try {
      for (const activity of ['home', 'office', 'shop'] as const) {
        const pane = art.window(activity);
        const lamp = art.get('#e9aa58', .42, true, activity);
        expect(pane).not.toBe(lamp);
        expect(pane.userData.windowPane).toBe(true);
        expect(lamp.userData.windowPane).toBeUndefined();
        const paneBatch = art.buildingBatch(pane);
        const lampBatch = art.buildingBatch(lamp);
        expect(paneBatch.material).not.toBe(lampBatch.material);
        expect(paneBatch.color).toBeUndefined();
        for (const style of ['anime', 'miniature'] as const) {
          const batch = art.variant(paneBatch.material, style) as THREE.MeshStandardNodeMaterial;
          expect(batch.userData.windowPane).toBe(true);
          // The shader reads room state from pane attributes; automatic
          // vertex-color multiplication would incorrectly tint daytime glass.
          expect(batch.vertexColors).toBe(false);
          expect(art.variant(lampBatch.material, style).userData.windowPane).toBeUndefined();
        }
      }
    } finally { art.dispose(); }
  });

  it("classifies matte paint while excluding glazing, emission, foliage, and white sentinels", () => {
    installCanvasForFoliageMask();
    const art = new ArtMaterials();
    try {
      const paintedWall = art.get("#c8b99f", 0.85, false);
      const glass = art.get("#607a82", 0.32, false);
      const homeEmission = art.get("#efbd77", 0.85, true, "home");
      const foliage = art.get("#fffffe", 0.96, false);
      const whiteSentinel = art.get("#ffffff", 0.85, false);

      expect(userClass(paintedWall)).toMatchObject({ roughness: 0.85, emissive: false, painted: true });
      expect(userClass(glass)).toMatchObject({ roughness: 0.32, emissive: false, painted: false });
      expect(userClass(homeEmission)).toMatchObject({ emissive: true, painted: false, activity: "home" });
      expect(userClass(foliage)).toMatchObject({ emissive: false, painted: false });
      expect(userClass(whiteSentinel)).toMatchObject({ emissive: false, painted: false });
    } finally {
      art.dispose();
    }
  });

  it("keeps painted and unpainted surfaces in separate batches at equal roughness", () => {
    const art = new ArtMaterials();
    try {
      const wall = art.get("#c8b99f", 0.85, false);
      const white = art.get("#ffffff", 0.85, false);
      const wallClass = userClass(wall);
      const whiteClass = userClass(white);

      expect(wallClass.roughness).toBe(whiteClass.roughness);
      expect(wallClass.painted).toBe(true);
      expect(whiteClass.painted).toBe(false);
      expect(wallClass.key).not.toBe(whiteClass.key);
      expect(art.buildingBatch(wall).material).not.toBe(art.buildingBatch(white).material);
    } finally {
      art.dispose();
    }
  });

  it("preserves paint and activity-specific emission classes in source and batch style variants", () => {
    const art = new ArtMaterials();
    try {
      const painted = art.get("#c8b99f", 0.85, false);
      const home = art.get("#efbd77", 0.85, true, "home");
      const office = art.get("#efbd77", 0.85, true, "office");
      const shop = art.get("#efbd77", 0.85, true, "shop");
      const sourceClasses = [painted, home, office, shop].map(userClass);
      const animeBatchMaterials = [painted, home, office, shop].map((material) => art.buildingBatch(material).material);

      expect(animeBatchMaterials.map(userClass)).toEqual(sourceClasses);
      expect(userClass(home).key).not.toBe(userClass(office).key);
      expect(userClass(office).key).not.toBe(userClass(shop).key);

      art.setStyle("miniature");
      const miniatureSources = [painted, home, office, shop].map((material) => art.variant(material, "miniature"));
      const miniatureBatches = animeBatchMaterials.map((material) => art.variant(material, "miniature"));
      expect(miniatureSources.map(userClass)).toEqual(sourceClasses);
      expect(miniatureBatches.map(userClass)).toEqual(sourceClasses);
      expect(miniatureSources.map((material) => material.type)).toEqual([
        "MeshStandardNodeMaterial", "MeshStandardNodeMaterial", "MeshStandardNodeMaterial", "MeshStandardNodeMaterial",
      ]);
      expect(miniatureBatches.map((material) => material.type)).toEqual(miniatureSources.map((material) => material.type));
    } finally {
      art.dispose();
    }
  });
});
