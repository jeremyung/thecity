import type { Building } from '../types';
import type { MidriseMassing } from './building-massing';

/** Broad three-storey residences use a two-storey street wall and an upper
 * terrace. This changes the architectural kit, never the simulation use.
 * Require room for the existing 9m-wide, 12m-deep lowrise frontage before
 * replacing it; compact homes and all other uses retain their current family. */
export function broadHomeMassing(b: Pick<Building,'kind'|'floors'|'anchor'>, w:number, d:number): MidriseMassing|null {
  if(b.anchor || b.kind!=='home' || b.floors!==3 || !Number.isFinite(w) || !Number.isFinite(d)
    || w*.96<9 || d*.76<12)return null;
  const width=w*.96;
  const terraceDepth=2.4;
  return {family:'terrace-home',masses:[
    {x:0,z:0,width,depth:d,baseFloor:0,floors:2,role:'main'},
    {x:0,z:-terraceDepth/2,width,depth:d-terraceDepth,baseFloor:2,floors:1,role:'main'},
  ]};
}
