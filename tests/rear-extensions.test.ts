import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three/webgpu';
import { ArtMaterials } from '../web/src/render/materials';
import { buildRearExtension, RearExtensions, rearExtensionProgressStage, rearExtensionRenderView } from '../web/src/render/rear-extensions';
import { extendedAdditionEnvelope } from '../web/src/render/extended-building';
import type { Building, RearExtensionView, World } from '../web/src/types';

function view(project_id: number, overrides: Partial<RearExtensionView> = {}): RearExtensionView {
  return { project_id, building_id: 20 + project_id, x: 14, z: -8, width: 8, depth: 5,
    rotation: .35, floors: 2, stage: 'structure', progress: .3, ...overrides };
}

function host(id: number): Building {
  return { id, parcel_id: id + 100, x: 0, z: 0, rotation: 0, width: 12, depth: 10, floors: 1,
    kind: 'home', stage: 'open', progress: 1, capacity: 2, occupancy: 1, built_day: 0, name: `Host ${id}`,
    appearance_seed: id, history: [], anchor: false, vacant_days: 0 };
}

function world(extensions: RearExtensionView[], buildings = extensions.map(v => host(v.building_id))): World {
  return { schema_version: 1, engine_version: 'test', seed: 1, tick: 0, day: 0, minute_of_day: 0,
    development_rate: 1, revision: 0, stage: 'Founding', bounds: { min_x: -100, max_x: 100, min_z: -100, max_z: 100 },
    terrain: { river: [], protected: [], gateways: [] }, weather: { kind: 'clear', intensity: 0 },
    nodes: [], roads: [], parcels: [], buildings, projects: [], rear_extensions: extensions,
    trains: [], events: [], stats: { population: 0, active_travelers: 0, buildings: 0, active_projects: extensions.length } };
}

function meshes(root: THREE.Object3D): THREE.Mesh[] {
  const found: THREE.Mesh[] = [];
  root.traverse(o => { if (o instanceof THREE.Mesh) found.push(o); });
  return found;
}

function geometrySignature(root: THREE.Object3D): number[][] {
  return meshes(root).map(mesh => Array.from(mesh.geometry.getAttribute('position').array as ArrayLike<number>));
}

describe('rear annex rendering', () => {
  it('keeps construction until the exact permanent building is visible and bounds delayed handoffs',()=>{
    const art=new ArtMaterials(),retired:Array<()=>void>=[];
    const visible=new Set<number>();
    const renderer=new RearExtensions(art,release=>retired.push(release),building=>visible.has(building.id));
    const views=[view(1),view(2)];
    renderer.setWorld(world(views));
    const originalGroups=[...renderer.root.children];
    const done=world([view(3),view(4)],[...views,view(3),view(4)].map(v=>host(v.building_id)));
    done.building_original_footprints=views.map(v=>({...host(v.building_id),building_id:v.building_id}));
    renderer.setWorld(done);
    expect(renderer.root.children).toEqual(originalGroups);
    expect(retired).toHaveLength(0);
    // First replacement commits; a newer active site can now use its slot.
    visible.add(views[0].building_id);renderer.flushCompleted(done);
    expect(renderer.root.children).toHaveLength(2);
    expect(renderer.root.children.map(g=>g.userData.rearExtensionProjectId)).toEqual([2,3]);
    visible.add(views[1].building_id);renderer.flushCompleted(done);
    expect(renderer.root.children.map(g=>g.userData.rearExtensionProjectId)).toEqual([3,4]);
    renderer.reset();retired.forEach(release=>release());art.dispose();
  });
  it('aligns construction with the eventual addition for rotated first and repeated extensions',()=>{
    for(const rotation of [0,.7,Math.PI]) {
      const original={building_id:21,x:10,z:20,width:14,depth:12,rotation};
      const delta=8,s=Math.sin(rotation),c=Math.cos(rotation);
      const building={...host(21),...original,kind:'apartment',floors:7};
      const final={...building,x:building.x-s*delta/2,z:building.z-c*delta/2,depth:20};
      const visual=rearExtensionRenderView(view(1,{rotation,depth:delta}),building);
      const envelope=extendedAdditionEnvelope(final,original)!;
      const localZ=s*(visual.x-final.x)+c*(visual.z-final.z);
      expect(localZ+visual.depth/2).toBeCloseTo(envelope.frontZ,5);
      expect(localZ-visual.depth/2).toBeCloseTo(envelope.rearZ,5);
      expect(visual.width).toBeCloseTo(envelope.width,5);
      const next=rearExtensionRenderView(view(2,{rotation,depth:4}),final,original);
      const localNext=s*(next.x-final.x)+c*(next.z-final.z);
      expect(localNext+next.depth/2).toBeCloseTo(envelope.rearZ+.025,5);
    }
  });
  it('bounds output to two valid in-progress projects and rejects malformed dimensions', () => {
    const art = new ArtMaterials(), retired: Array<() => void> = [];
    const extensions = new RearExtensions(art, release => retired.push(release));
    extensions.setWorld(world([
      view(1), view(2), view(3), view(4, { width: Number.NaN }),
      view(5, { depth: 0 }), view(6, { floors: Number.POSITIVE_INFINITY }),
      view(7, { x: Number.NaN }), view(8, { width: 1000 }),
    ]));
    expect(extensions.root.children).toHaveLength(2);
    expect(extensions.root.children.map(g => g.userData.rearExtensionProjectId)).toEqual([1, 2]);
    expect(rearExtensionProgressStage(Number.NaN)).toBeNull();
    expect(rearExtensionProgressStage(1)).toBeNull();
    extensions.reset(); retired.forEach(release => release()); art.dispose();
  });

  it('keeps the same annex geometry in both styles and changes only shared style materials', () => {
    const art = new ArtMaterials();
    art.setStyle('anime');
    const anime = buildRearExtension(view(1, { stage: 'surface' }), .5, art);
    const animeGeometry = geometrySignature(anime);
    const animeMaterial = meshes(anime)[0].material;
    art.setStyle('miniature');
    const miniature = buildRearExtension(view(1, { stage: 'surface' }), .5, art);
    expect(geometrySignature(miniature)).toEqual(animeGeometry);
    expect(meshes(miniature)[0].material).not.toBe(animeMaterial);
    // Generated meshes borrow ArtMaterials materials; their disposal is geometry-only.
    anime.traverse(o => { if (o instanceof THREE.Mesh) o.geometry.dispose(); });
    miniature.traverse(o => { if (o instanceof THREE.Mesh) o.geometry.dispose(); });
    art.dispose();
  });

  it('quantizes stage updates, replaces and retires groups, and leaves source buildings untouched', () => {
    const art = new ArtMaterials(), retired: Array<() => void> = [];
    const extensions = new RearExtensions(art, release => retired.push(release));
    const original: Building = { id: 9, parcel_id: 4, x: 10, z: -4, rotation: .2, width: 14, depth: 11,
      floors: 2, kind: 'home', stage: 'open', progress: 1, capacity: 3, occupancy: 2, built_day: 1,
      name: 'Old home', appearance_seed: 99, history: ['kept'], anchor: false, vacant_days: 0 };
    const frozen = Object.freeze({ ...original, history: Object.freeze([...original.history]) }) as unknown as Building;
    const before = JSON.stringify(frozen);
    extensions.setWorld(world([view(10, { building_id: frozen.id, stage: 'clearing', progress: .26 })], [frozen]));
    const first = extensions.root.children[0];
    const firstGeometry = meshes(first)[0].geometry;
    const disposed = vi.fn(); firstGeometry.addEventListener('dispose', disposed);
    extensions.setWorld(world([view(10, { building_id: frozen.id, stage: 'clearing', progress: .28 })], [frozen]));
    expect(extensions.root.children[0]).toBe(first);
    expect(retired).toHaveLength(0);

    extensions.setWorld(world([view(10, { building_id: frozen.id, stage: 'structure', progress: .52 })], [frozen]));
    expect(extensions.root.children[0]).not.toBe(first);
    expect(disposed).not.toHaveBeenCalled();
    retired.shift()!();
    expect(disposed).toHaveBeenCalledTimes(1);

    const replacement = extensions.root.children[0];
    const replacementDisposed = vi.fn();
    meshes(replacement)[0].geometry.addEventListener('dispose', replacementDisposed);
    extensions.setWorld(world([], [frozen]));
    expect(extensions.root.children).toHaveLength(0);
    retired.splice(0).forEach(release => release());
    expect(replacementDisposed).toHaveBeenCalledTimes(1);
    expect(JSON.stringify(frozen)).toBe(before);
    extensions.dispose(); art.dispose();
  });

  it('shows survey stakes without a slab and gives one-floor structure and finishing visible height', () => {
    const art = new ArtMaterials();
    const survey = buildRearExtension(view(1, { floors: 1, stage: 'survey', progress: .1 }), .1, art);
    const structure = buildRearExtension(view(1, { floors: 1, stage: 'structure', progress: .6 }), .6, art);
    const finishing = buildRearExtension(view(1, { floors: 1, stage: 'finishing', progress: .95 }), .9, art);
    const slabMaterial = art.get('#aaa99e');
    expect(meshes(survey).some(mesh => mesh.material === slabMaterial)).toBe(false);
    expect(meshes(survey).some(mesh => mesh.material === art.get('#8a7650', .9))).toBe(true);
    const structureBounds = new THREE.Box3().setFromObject(structure);
    const finishingBounds = new THREE.Box3().setFromObject(finishing);
    expect(structureBounds.max.y).toBeGreaterThan(3.5);
    expect(finishingBounds.max.y).toBeGreaterThan(3.5);
    for (const group of [survey, structure, finishing]) group.traverse(o => {
      if (o instanceof THREE.Mesh) o.geometry.dispose();
    });
    art.dispose();
  });

  it('omits annexes whose original host is missing or no longer open', () => {
    const art = new ArtMaterials(), extensions = new RearExtensions(art, () => {});
    const active = view(1);
    extensions.setWorld(world([active], []));
    expect(extensions.root.children).toHaveLength(0);
    extensions.setWorld(world([active], [{ ...host(active.building_id), stage: 'structure' }]));
    expect(extensions.root.children).toHaveLength(0);
    extensions.dispose(); art.dispose();
  });
});
