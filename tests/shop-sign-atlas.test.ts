import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as THREE from "three/webgpu";
import { ArtMaterials, SHOP_SIGN_LABELS, SIGN_ATLAS_HEIGHT, shopSignCell, shopSignLabel } from "../web/src/render/materials";
import { buildingKit, disposeGroup } from "../web/src/render/kit";
import { BuildingChunks } from "../web/src/render/building-chunks";
import type { Building } from "../web/src/types";

type Rect = { canvas: { width: number; height: number }; op: string; x: number; y: number; w: number; h: number };
let rects: Rect[] = [];

beforeEach(() => {
  rects = [];
  vi.stubGlobal("document", {
    createElement: () => {
      const canvas = { width: 0, height: 0, getContext: () => context };
      const record = (op: string) => (x: number, y: number, w: number, h: number) => { rects.push({ canvas, op, x, y, w, h }); };
      const context = new Proxy({ fillRect: record("fill"), strokeRect: record("stroke") } as Record<PropertyKey, unknown>, {
        get(target, key) { if (!(key in target)) target[key] = () => undefined; return target[key]; },
        set(target, key, value) { target[key] = value; return true; },
      });
      return canvas;
    },
  });
});
afterEach(() => vi.unstubAllGlobals());

function building(kind: "shop" | "cafe", overrides: Partial<Building> = {}): Building {
  return {
    id: 45, parcel_id: 72, x: 12, z: -8, rotation: 0, width: 10, depth: 9, floors: 2, kind, stage: "open", progress: 1,
    capacity: 16, occupancy: 8, built_day: 4, name: `${kind} sample`, appearance_seed: 451, history: [], anchor: false, vacant_days: 0,
    ...overrides,
  };
}

function signMeshes(root: THREE.Object3D, material: THREE.Material) {
  const meshes: THREE.Mesh[] = [];
  root.traverse((object) => { if (object instanceof THREE.Mesh && object.material === material) meshes.push(object); });
  return meshes;
}

function vRange(geometry: THREE.BufferGeometry) {
  const uv = geometry.getAttribute("uv");
  let min = Infinity, max = -Infinity;
  for (let i = 0; i < uv.count; i++) { min = Math.min(min, uv.getY(i)); max = Math.max(max, uv.getY(i)); }
  return { min, max };
}

describe("shop sign atlas", () => {
  it("varies storefront labels by use and seed while the anchor keeps its name", () => {
    const shops = new Set(Array.from({ length: 64 }, (_, i) => shopSignLabel(building("shop", { appearance_seed: i << 11 }))));
    const cafes = new Set(Array.from({ length: 64 }, (_, i) => shopSignLabel(building("cafe", { appearance_seed: i << 11 }))));
    expect(shops.size).toBe(7);
    expect(cafes.size).toBe(3);
    expect([...shops, ...cafes].every((label) => SHOP_SIGN_LABELS.includes(label))).toBe(true);
    expect([...shops].some((label) => cafes.has(label))).toBe(false);
    expect(shopSignLabel(building("cafe", { anchor: true }))).toBe("喫茶 青葉");
  });

  it("packs every cell inside the atlas with at least 8px of paper on both sides", () => {
    const cells = SHOP_SIGN_LABELS.map((_, slot) => shopSignCell(slot));
    expect(cells[0].top).toBeGreaterThanOrEqual(8);
    expect(SIGN_ATLAS_HEIGHT - cells[cells.length - 1].bottom).toBeGreaterThanOrEqual(8);
    for (let i = 1; i < cells.length; i++) expect(cells[i].top - cells[i - 1].bottom).toBeGreaterThanOrEqual(8);
    for (const cell of cells) {
      expect(cell.bottom - cell.top).toBe(160);
      expect(cell.v0).toBeGreaterThanOrEqual(0);
      expect(cell.v1).toBeLessThanOrEqual(1);
      expect(cell.v1 - cell.v0).toBeCloseTo(160 / SIGN_ATLAS_HEIGHT, 12);
    }
    expect(SIGN_ATLAS_HEIGHT).toBe(2048); // Still a power of two.
  });

  it("draws every sign and its border inside its own cell of the canvas", () => {
    const art = new ArtMaterials();
    try {
      const { material } = art.shopSign("商店");
      const canvas = (material.map as THREE.CanvasTexture).image as { width: number; height: number };
      expect(canvas.height).toBe(SIGN_ATLAS_HEIGHT);
      const drawn = rects.filter((rect) => rect.canvas === canvas);
      SHOP_SIGN_LABELS.forEach((_, slot) => {
        const { top, bottom } = shopSignCell(slot);
        const inCell = drawn.filter((rect) => rect.y >= top && rect.y < bottom);
        expect(inCell.some((rect) => rect.op === "fill" && rect.y === top && rect.h === 160)).toBe(true);
        const border = inCell.find((rect) => rect.op === "stroke");
        expect(border).toBeDefined();
        expect(border!.y + border!.h).toBeLessThanOrEqual(bottom);
      });
      for (const rect of drawn) expect(rect.y + rect.h).toBeLessThanOrEqual(SIGN_ATLAS_HEIGHT);
    } finally {
      art.dispose();
    }
  });

  it("gives every label one shared material and maps each sign quad to its own cell", () => {
    const art = new ArtMaterials();
    try {
      const signs = SHOP_SIGN_LABELS.map((label) => art.shopSign(label));
      expect(new Set(signs.map((sign) => sign.material)).size).toBe(1);
      signs.forEach((sign, slot) => expect(sign).toMatchObject({ v0: shopSignCell(slot).v0, v1: shopSignCell(slot).v1 }));
      const material = signs[0].material;

      const cases: Array<[Building, string]> = [
        [building("shop"), shopSignLabel(building("shop"))], [building("cafe"), shopSignLabel(building("cafe"))], [building("shop", { anchor: true, width: 14 }), "喫茶 青葉"],
      ];
      for (const [b, label] of cases) {
        const group = buildingKit(b, art);
        const [sign, ...rest] = signMeshes(group, material);
        expect(rest).toHaveLength(0);
        const cell = shopSignCell(SHOP_SIGN_LABELS.indexOf(label as typeof SHOP_SIGN_LABELS[number]));
        const range = vRange(sign.geometry);
        expect(range.min).toBeCloseTo(cell.v0, 6);
        expect(range.max).toBeCloseTo(cell.v1, 6);
        // The sign no longer owns per-building GPU resources.
        expect(sign.userData.ownedMaterial).toBeUndefined();
        expect(sign.userData.ownedTexture).toBeUndefined();
        disposeGroup(group);
      }
    } finally {
      art.dispose();
    }
  });

  it("keeps the atlas material shared across styles and releases its resources with ArtMaterials", () => {
    const art = new ArtMaterials();
    const animeSigns = SHOP_SIGN_LABELS.map((label) => art.shopSign(label));
    const material = animeSigns[0].material;
    const texture = material.map;
    if (!texture) throw new Error("Shared sign material is missing its atlas");
    expect(animeSigns.every((sign) => sign.material === material)).toBe(true);
    expect((texture as THREE.CanvasTexture).image).toMatchObject({ width: 512, height: SIGN_ATLAS_HEIGHT });
    expect(art.variant(material)).toBe(material);

    const materialDispose = vi.spyOn(material, "dispose");
    const textureDispose = vi.spyOn(texture, "dispose");
    art.setStyle("miniature");
    const miniatureSigns = SHOP_SIGN_LABELS.map((label) => art.shopSign(label));
    expect(miniatureSigns).toHaveLength(SHOP_SIGN_LABELS.length);
    expect(miniatureSigns.every(({ material: signMaterial }) => signMaterial === material)).toBe(true);
    expect(miniatureSigns.map(({ v0, v1 }) => [v0, v1])).toEqual(animeSigns.map(({ v0, v1 }) => [v0, v1]));
    art.dispose();
    expect(materialDispose).toHaveBeenCalledTimes(1);
    expect(textureDispose).toHaveBeenCalledTimes(1);
  });

  it("keeps sign UVs in their cells after merging into a neighborhood batch", () => {
    const art = new ArtMaterials();
    try {
      const { material } = art.shopSign("商店");
      const chunks = new BuildingChunks((m) => art.buildingBatch(m), (release) => release());
      chunks.set(1, buildingKit(building("cafe", { id: 1, x: 4 }), art));
      chunks.set(2, buildingKit(building("shop", { id: 2, x: 30 }), art));
      for (let i = 0; i < 2000 && chunks.pendingCount > 0; i++) chunks.update();
      expect(chunks.pendingCount).toBe(0);
      const merged = signMeshes(chunks.root, material);
      expect(merged).toHaveLength(1); // Both signs share one draw.
      const geometry = merged[0].geometry, uv = geometry.getAttribute("uv");
      const cafe = shopSignCell(SHOP_SIGN_LABELS.indexOf(shopSignLabel(building("cafe", { id: 1, x: 4 })))), shop = shopSignCell(SHOP_SIGN_LABELS.indexOf(shopSignLabel(building("shop", { id: 2, x: 30 }))));
      const inCafe = (v: number) => v >= cafe.v0 - 1e-6 && v <= cafe.v1 + 1e-6, inShop = (v: number) => v >= shop.v0 - 1e-6 && v <= shop.v1 + 1e-6;
      let cafeVertices = 0, shopVertices = 0;
      for (let i = 0; i < uv.count; i++) {
        const v = uv.getY(i);
        expect(inCafe(v) || inShop(v)).toBe(true);
        if (inCafe(v)) cafeVertices++; else shopVertices++;
      }
      expect(cafeVertices).toBeGreaterThan(0);
      expect(shopVertices).toBeGreaterThan(0);

      // Retiring the batch leaves the shared atlas to ArtMaterials.
      const materialDispose = vi.fn(), textureDispose = vi.fn();
      material.addEventListener("dispose", materialDispose);
      material.map!.addEventListener("dispose", textureDispose);
      chunks.clear();
      expect(materialDispose).not.toHaveBeenCalled();
      expect(textureDispose).not.toHaveBeenCalled();
      art.dispose();
      expect(materialDispose).toHaveBeenCalledTimes(1);
      expect(textureDispose).toHaveBeenCalledTimes(1);
      // A fresh request after dispose rebuilds the atlas rather than reusing a disposed one.
      expect(art.shopSign("商店").material).not.toBe(material);
    } finally {
      art.dispose();
    }
  });
});
