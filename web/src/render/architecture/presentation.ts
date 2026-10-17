import {
  selectApartmentArchitecture,
  resolveApartmentArchitectures,
  type ApartmentArchitecture,
  apartmentArchitectureKey,
  parseApartmentEra,
  type ApartmentEra,
} from './era';
import { apartmentPhase, APARTMENT_RECIPE_VERSION } from './recipes';
import { ApartmentResources } from './resources';
import { apartmentDetail, type ApartmentDetail } from './detail';
import type * as THREE from 'three/webgpu';
import type { Building, BuildingOriginalFootprint, World } from '../../types';
import type { SignageContext } from '../signage';
import type { ArtMaterials } from '../materials';
import { buildingRenderSignature } from '../building-signature';

export class ArchitecturePresentation {
  readonly apartmentResources = new ApartmentResources();
  apartmentDetails = new Map<number, ApartmentDetail>();
  apartmentDetailAt = 0;
  apartmentEra: ApartmentEra = 'mixed';
  apartmentChoices = new Map<number, ApartmentArchitecture>();
  apartmentLayout = '';
  selectLayout(world: World, originals: ReadonlyMap<number, BuildingOriginalFootprint>) {
    const layout = world.buildings
      .map(
        (b) =>
          `${b.id}:${b.kind}:${b.anchor}:${b.appearance_seed}:${b.floors}:${b.width}:${b.depth}:${b.x}:${b.z}:${b.stage}:${originals.has(b.id)}`,
      )
      .join('|');
    if (layout !== this.apartmentLayout) {
      this.apartmentLayout = layout;
      this.apartmentChoices = resolveApartmentArchitectures(
        world.buildings,
        this.apartmentEra,
        new Set(originals.keys()),
      );
    }
  }
  signature(
    b: Building,
    original: BuildingOriginalFootprint | undefined,
    signage: SignageContext | undefined,
  ) {
    const choice =
      this.apartmentChoices.get(b.id) ??
      selectApartmentArchitecture(b, this.apartmentEra, !!original);
    return (
      buildingRenderSignature(b, original, signage) +
      (choice.kind !== 'legacy'
        ? `:architecture-${APARTMENT_RECIPE_VERSION}:${apartmentArchitectureKey(choice)}:${this.apartmentDetails.get(b.id) ?? 'district'}:${apartmentPhase(b)}`
        : '')
    );
  }
  setEra(value: ApartmentEra) {
    const mode = parseApartmentEra(value);
    if (mode === this.apartmentEra) return false;
    this.apartmentEra = mode;
    this.apartmentLayout = '';
    return true;
  }
  updateDetail(world: World | null, camera: THREE.PerspectiveCamera, now: number) {
    if (!world || now - this.apartmentDetailAt < 1000) return;
    this.apartmentDetailAt = now;
    const originals = new Set(world.building_original_footprints?.map((b) => b.building_id));
    let changed = false;
    for (const b of world.buildings)
      if (
        (
          this.apartmentChoices.get(b.id) ??
          selectApartmentArchitecture(b, this.apartmentEra, originals.has(b.id))
        ).kind !== 'legacy'
      ) {
        const previous = this.apartmentDetails.get(b.id),
          next = apartmentDetail(
            Math.hypot(
              b.x - camera.position.x,
              b.z - camera.position.z,
              b.floors * 1.625 - camera.position.y,
            ),
            previous,
          );
        if (previous !== next) {
          this.apartmentDetails.set(b.id, next);
          changed = true;
        }
      }
    return changed;
  }
  syncLighting(art: ArtMaterials) {
    const apartmentArt = this.apartmentResources.art;
    apartmentArt.night.value = art.night.value;
    apartmentArt.wet.value = art.wet.value;
    apartmentArt.time.value = art.time.value;
    apartmentArt.sunDirection.value.copy(art.sunDirection.value);
    apartmentArt.cloudShadow.value = art.cloudShadow.value;
    for (const activity of ['home', 'office', 'shop'] as const)
      apartmentArt.occupancy[activity].value = art.occupancy[activity].value;
    apartmentArt.litHome.value = art.litHome.value;
    apartmentArt.litOffice.value = art.litOffice.value;
    apartmentArt.litShop.value = art.litShop.value;
  }
}
