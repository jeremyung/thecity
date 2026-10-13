import type { Building } from '../types';
import { skylineNeighborhoods } from './skyline-neighbors';

export interface SkylineCluster {
  x:number; z:number; floors:number; spread:number; score:number;
  towers:Building[]; frameBuildings:Building[];
}

/** One latest eligible-building set; ordering is part of the tie-breaking contract. */
export class SkylineClusterCache {
  private key:string|null=null;
  private best:SkylineCluster|null=null;
  get(buildings:Building[]):SkylineCluster|null {
    // Includes every field used by clustering and the director's skylineBounds.
    // Eligibility is filtered by the caller before caching, including road access.
    const key=JSON.stringify(buildings.map(b=>[
      b.id,b.x,b.z,b.floors,b.width,b.depth,b.rotation,b.kind,!!b.anchor,
    ]));
    if(key===this.key)return this.best;
    let best: SkylineCluster | null = null;
    const neighborhood = skylineNeighborhoods(buildings, 90);
    for (const seed of buildings) {
      const local = neighborhood(seed);
      const floors = Math.max(...local.map((building) => building.floors));
      if (floors < 5) continue;
      const towers = local.filter((building) => building.floors >= Math.max(5, floors * 0.68));
      if (towers.length === 0) continue;
      const score = floors * 3 + towers.length * 1.2 + Math.min(local.length, 18) * 0.15;
      // Ties retain the earlier seed; only a winning cluster needs framing work.
      if (best && score <= best.score) continue;
      const totalWeight = towers.reduce((sum, building) => sum + Math.max(1, building.floors), 0);
      const x = towers.reduce((sum, building) => sum + building.x * Math.max(1, building.floors), 0) / totalWeight;
      const z = towers.reduce((sum, building) => sum + building.z * Math.max(1, building.floors), 0) / totalWeight;
      const spread = Math.max(...towers.map((building) => Math.hypot(building.x - x, building.z - z)
        + Math.hypot(building.width, building.depth) / 2));
      // Include adjacent mid-rise roofs that can dominate screen area when
      // they sit closer to the river camera than the tallest tower.
      const frameBuildings = local.filter((building) => building.floors >= Math.max(5, floors * 0.4));
      if (!best || score > best.score) best = { x, z, floors, spread, score, towers, frameBuildings };
    }
    this.key=key;this.best=best;
    return best;
  }
}
