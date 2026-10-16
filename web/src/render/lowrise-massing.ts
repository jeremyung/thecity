import type { Building } from '../types';

export type LowriseRoof = 'gable' | 'shed' | 'parapet';

export interface LowriseMass {
  x: number;
  z: number;
  width: number;
  depth: number;
  baseFloor: number;
  floors: number;
  role: 'frontage' | 'service';
  roof: LowriseRoof;
}

export interface LowriseMassing {
  family: 'machiya' | 'shop-house';
  masses: LowriseMass[];
  /** Lot-facing sides receive quieter fenestration. */
  quietSides: Array<-1 | 1>;
}

type LowriseBuilding = Pick<
  Building,
  'kind' | 'floors' | 'width' | 'depth' | 'appearance_seed' | 'anchor'
>;

/** Widen only buildings supported by the low-rise family; others keep the kit's legacy width. */
export function lowriseBuildingWidth(
  b: LowriseBuilding,
  fallbackWidth: number,
  depth: number,
): number {
  if (!lowriseMassing(b, fallbackWidth, depth)) return fallbackWidth;
  return Math.max(Math.min(6, b.width - 0.2), Math.min(b.width * 0.96, b.width - 0.2));
}

/** Seeded compact massing, including the planner's 5 m minimum frontage.
 * The simulation's use, floors and parcel stay authoritative. */
export function lowriseMassing(b: LowriseBuilding, w: number, d: number): LowriseMassing | null {
  const home = b.kind === 'home';
  const shop = b.kind === 'shop' || b.kind === 'cafe';
  if (
    b.anchor ||
    (!home && !shop) ||
    !Number.isInteger(b.floors) ||
    b.floors < 1 ||
    b.floors > 3 ||
    !Number.isFinite(w) ||
    !Number.isFinite(d) ||
    !Number.isFinite(b.width) ||
    !Number.isFinite(b.depth) ||
    b.width < 5 ||
    b.depth < 6 ||
    w < 4 ||
    d < 5.5
  )
    return null;

  const seed = b.appearance_seed >>> 0;
  const parcelW = b.width,
    parcelD = b.depth;
  const bodyW = Math.min(w, parcelW - 0.2),
    bodyD = Math.min(d, parcelD - 0.2);
  if (bodyW < 4 || bodyD < 5.3) return null;
  // Hold a nearly full-width street wall so detached low-rise buildings read
  // as a connected neighborhood; the rear service volume provides the silhouette break.
  const mainWidth = bodyW * 0.96;
  const mainDepth = bodyD * (home ? 0.76 : 0.72);
  const mainZ = d / 2 - mainDepth / 2;
  const mainRear = mainZ - mainDepth / 2;
  const serviceRear = -bodyD / 2 + 0.12;
  const serviceFront = mainRear + 0.1;
  const serviceDepth = Math.max(0.8, serviceFront - serviceRear);
  const side: -1 | 1 = seed & 1 ? 1 : -1;
  // Carry a broad but deliberately off-center rear wing into the back of the
  // shallow lot. Keep a real side court and condenser clearance at every
  // parcel width, including narrow lots where the target must be clipped.
  const serviceOffset = side * mainWidth * 0.04;
  // Leave room for the .72 m condenser projection plus at least .70 m to the
  // parcel edge. The slightly larger value absorbs geometry float error.
  const serviceClearance = 1.45;
  const maxServiceWidth = Math.max(0, parcelW - serviceClearance * 2 - Math.abs(serviceOffset) * 2);
  const serviceWidth = Math.min(mainWidth * 0.78, maxServiceWidth);
  const serviceZ = (serviceFront + serviceRear) / 2;
  // The gable overhang is .16 m per side. Keep a small construction margin
  // while allowing the near-full-width frontage this family is designed for.
  const gableFits = (parcelW - mainWidth) / 2 >= 0.19 && parcelD / 2 - d / 2 >= 0.6;
  const mainRoof: LowriseRoof = home
    ? gableFits
      ? 'gable'
      : (seed >>> 3) % 2
        ? 'shed'
        : 'parapet'
    : gableFits && (seed >>> 3) % 3 === 0
      ? 'gable'
      : (seed >>> 4) % 2
        ? 'parapet'
        : 'shed';
  const serviceRoof: LowriseRoof = (seed >>> 6) % 2 ? 'shed' : 'parapet';

  return {
    family: home ? 'machiya' : 'shop-house',
    masses: [
      {
        x: 0,
        z: mainZ,
        width: mainWidth,
        depth: mainDepth,
        baseFloor: 0,
        floors: b.floors,
        role: 'frontage',
        roof: mainRoof,
      },
      {
        x: serviceOffset,
        z: serviceZ,
        width: serviceWidth,
        depth: serviceDepth,
        baseFloor: 0,
        floors: 1,
        role: 'service',
        roof: serviceRoof,
      },
    ],
    quietSides: [side],
  };
}
