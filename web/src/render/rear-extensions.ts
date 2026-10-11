import * as THREE from 'three/webgpu';
import type { Building, BuildingOriginalFootprint, RearExtensionView, World } from '../types';
import { ArtMaterials } from './materials';
import { Kit, disposeGroup } from './kit';
import { extendedAdditionEnvelope } from './extended-building';

const MAX_REAR_EXTENSIONS = 2;
const FLOOR_HEIGHT = 3.25;
const PROGRESS_STEPS = 10;
const PROJECT_STAGES = new Set(['survey', 'clearing', 'structure', 'surface', 'finishing']);

/** Match the eventual built addition, including the retained kit's rear inset.
 * The authoritative view still describes reserved land; only visual geometry
 * reaches across the existing setback to join the old rear wall. */
export function rearExtensionRenderView(view:RearExtensionView,host:Building,original?:BuildingOriginalFootprint):RearExtensionView {
  const c=Math.cos(host.rotation),s=Math.sin(host.rotation),delta=view.depth;
  const expanded={...host,x:host.x-s*delta/2,z:host.z-c*delta/2,depth:host.depth+delta};
  const retained=original??{building_id:host.id,x:host.x,z:host.z,width:host.width,depth:host.depth,rotation:host.rotation};
  const envelope=extendedAdditionEnvelope(expanded,retained);
  if(!envelope)return view;
  const previous=original?extendedAdditionEnvelope(host,original):null;
  const front=previous?previous.rearZ+delta/2+.025:envelope.frontZ;
  const centerZ=(front+envelope.rearZ)/2;
  return {...view,x:expanded.x+c*envelope.x+s*centerZ,z:expanded.z-s*envelope.x+c*centerZ,
    width:envelope.width,depth:front-envelope.rearZ};
}

/** Quantize worker progress so tiny simulation updates do not rebuild geometry. */
export function rearExtensionProgressStage(progress: number): number | null {
  if (!Number.isFinite(progress) || progress >= 1) return null;
  return Math.floor(Math.max(0, progress) * PROGRESS_STEPS) / PROGRESS_STEPS;
}

function validView(view: RearExtensionView): boolean {
  return Number.isSafeInteger(view.project_id) && Number.isSafeInteger(view.building_id)
    && [view.x, view.z, view.width, view.depth, view.rotation, view.floors, view.progress]
      .every(Number.isFinite)
    && view.width >= 2 && view.depth >= 2 && view.width <= 80 && view.depth <= 80
    && Number.isInteger(view.floors) && view.floors >= 1 && view.floors <= 30
    && PROJECT_STAGES.has(view.stage);
}

function signature(view: RearExtensionView, progress: number): string {
  return [view.building_id, view.x, view.z, view.width, view.depth, view.rotation,
    view.floors, view.stage, progress].join(':');
}

/** Build only the rear addition. Its transform is supplied in world coordinates;
 * the retained source building is never adapted or mutated. */
export function buildRearExtension(view: RearExtensionView, progress: number, art: ArtMaterials): THREE.Group {
  const kit = new Kit();
  const width = view.width, depth = view.depth;
  const floors = Math.max(1, Math.min(30, Math.floor(view.floors)));
  const maxHeight = floors * FLOOR_HEIGHT;
  const stage = view.stage;
  const isSurvey = stage === 'survey';
  const isClearing = stage === 'clearing';
  const isStructure = stage === 'structure';
  const isSurface = stage === 'surface';
  const isFinishing = stage === 'finishing';
  const structureFraction = Math.max(0, Math.min(1, (progress - .48) / .25));
  const erectedFloors = isSurvey || isClearing ? 0 : isStructure
    ? Math.max(1, Math.min(floors, Math.ceil(structureFraction * floors)))
    : floors;
  const slab = art.get('#aaa99e');
  const concrete = art.get('#c3b9a5');
  const frame = art.get('#536164', .62);
  const scaffold = art.get('#bd8054', .55);
  const caution = art.get('#d6aa51', .62);

  // Survey work is visible as stakes and a perimeter mark only. Foundation
  // and all later stages show the actual added strip.
  const marker = art.get('#8a7650', .9);
  if (isSurvey) {
    const stakeX = Math.max(.25, width / 2 - .12), stakeZ = Math.max(.25, depth / 2 - .12);
    for (const x of [-stakeX, stakeX]) for (const z of [-stakeZ, stakeZ]) {
      kit.box(x, .48, z, .09, .96, .09, marker);
    }
    kit.box(0, .04, -depth / 2, width, .06, .06, marker);
  } else {
    kit.box(0, .29, 0, width, .58, depth, slab);
  }
  const postX = Math.max(.5, width / 2 - .12), postZ = Math.max(.5, depth / 2 - .12);
  const builtHeight = Math.max(0, erectedFloors * FLOOR_HEIGHT);
  if (builtHeight > 0 && !isClearing) {
    for (const x of [-postX, postX]) for (const z of [-postZ, postZ]) {
      kit.box(x, .58 + builtHeight / 2, z, .18, builtHeight, .18, frame);
    }
    for (let floor = 1; floor <= erectedFloors; floor++) {
      kit.box(0, .58 + floor * FLOOR_HEIGHT, 0, width, .14, depth, concrete);
    }
  }

  if (isClearing) {
    kit.box(0, .37, 0, Math.min(width * .5, 2.4), .38, Math.min(depth * .4, 1.8), concrete);
  } else if (!isSurvey) {
    const scaffoldHeight = Math.max(2.2, Math.min(maxHeight + .4, builtHeight + (isFinishing ? .35 : 1.2)));
    for (const x of [-postX, postX]) {
      for (const z of [-postZ, postZ]) kit.box(x, .58 + scaffoldHeight / 2, z, .07, scaffoldHeight, .07, scaffold);
      for (let y = 1.5; y < scaffoldHeight; y += 2.25) {
        kit.box(x, .58 + y, 0, .045, .055, depth - .25, scaffold);
      }
    }
    for (const z of [-postZ, postZ]) for (let y = 1.5; y < scaffoldHeight; y += 2.25) {
      kit.box(0, .58 + y, z, width - .25, .055, .045, scaffold);
    }
  }
  if (isSurface || isFinishing) {
    const cladRatio = isFinishing ? .62 : Math.max(.18, Math.min(.5, (progress - .73) / .19 * .5));
    const panelW = Math.max(.6, width * cladRatio * .42);
    for (let floor = 0; floor < floors; floor++) {
      const y = .58 + floor * FLOOR_HEIGHT + 1.55;
      for (const z of [-depth / 2 + .08, depth / 2 - .08]) {
        kit.box(-width * .28, y, z, panelW, 2.25, .14, concrete);
        kit.box(width * .28, y, z, panelW, 2.25, .14, concrete);
      }
      for (const x of [-width / 2 + .08, width / 2 - .08]) {
        kit.box(x, y, -depth * .25, .14, 2.25, Math.max(.6, depth * cladRatio * .35), concrete);
        kit.box(x, y, depth * .25, .14, 2.25, Math.max(.6, depth * cladRatio * .35), concrete);
      }
    }
  }
  if (!isSurvey) kit.box(0, .43, depth / 2 - .38, Math.min(width * .55, 3.2), .16, .08, caution);

  const group = kit.finish();
  group.position.set(view.x, 0, view.z);
  group.rotation.y = view.rotation;
  group.userData.rearExtensionProjectId = view.project_id;
  group.userData.buildingId = view.building_id;
  return group;
}

/** Owns at most two temporary groups. Geometry retires through the caller's
 * render-safe queue; ArtMaterials owns all shared materials. */
export class RearExtensions {
  readonly root = new THREE.Group();
  private groups = new Map<number, { group: THREE.Group; signature: string }>();
  private awaitingBuilding = false;

  constructor(private readonly art: ArtMaterials, private readonly retire: (release: () => void) => void,
    private readonly isBuildingVisible:(building:Building)=>boolean=()=>true) {}

  setWorld(world: World): void {
    const active = (world?.rear_extensions ?? [])
      .filter(view => validView(view)
        && world.buildings.some(building => building.id === view.building_id && building.stage === 'open'))
      .filter(view => rearExtensionProgressStage(view.progress) !== null)
      .slice(0, MAX_REAR_EXTENSIONS)
      .map(view=>rearExtensionRenderView(view,world.buildings.find(building=>building.id===view.building_id)!,
        world.building_original_footprints?.find(original=>original.building_id===view.building_id)));
    const wanted = new Map(active.map(view => [view.project_id, view]));
    this.awaitingBuilding=false;

    for (const [id, current] of this.groups) {
      const view = wanted.get(id);
      if(!view){
        const host=world.buildings.find(building=>building.id===current.group.userData.buildingId);
        const completed=host?.stage==='open'&&world.building_original_footprints?.some(original=>original.building_id===host.id);
        // The authoritative project ends before the incremental building chunk
        // necessarily reaches the scene. Keep its last construction pose until
        // the exact replacement descriptor is visibly committed.
        if(completed&&host&&!this.isBuildingVisible(host)){this.awaitingBuilding=true;continue;}
      }
      const progress = view ? rearExtensionProgressStage(view.progress) : null;
      const nextSignature = view && progress !== null ? signature(view, progress) : null;
      if (nextSignature === current.signature) continue;
      this.root.remove(current.group);
      this.groups.delete(id);
      this.retire(() => disposeGroup(current.group));
    }

    for (const view of active) {
      const progress = rearExtensionProgressStage(view.progress);
      if (progress === null || this.groups.has(view.project_id)) continue;
      // Pending handoffs share the existing two-group limit. If rendering is
      // behind, show newer sites once the older replacement reaches the scene.
      if(this.groups.size>=MAX_REAR_EXTENSIONS)continue;
      const key = signature(view, progress);
      const group = buildRearExtension(view, progress, this.art);
      this.groups.set(view.project_id, { group, signature: key });
      this.root.add(group);
    }
  }

  flushCompleted(world:World):void { if(this.awaitingBuilding)this.setWorld(world); }

  reset(): void {
    for (const { group } of this.groups.values()) {
      this.root.remove(group);
      this.retire(() => disposeGroup(group));
    }
    this.groups.clear();
    this.awaitingBuilding=false;
  }

  dispose(): void { this.reset(); }
}
