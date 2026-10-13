// Diagnose actual occupied waterfront fabric without changing growth or camera rules.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
initSync({module:readFileSync('public/wasm/city_engine_bg.wasm')});
const engine=CityEngine.restore(readFileSync('.tools/movement-support/browser/growth-723.json','utf8'));
const results=[];
function measure(){
 const w=JSON.parse(engine.snapshot()),save=JSON.parse(engine.save());
 const river=w.terrain.river;
 const riverX=z=>{const match=river.findIndex((p,j)=>j<river.length-1&&z<=river[j+1].z);const i=match<0?river.length-2:match;const a=river[i],b=river[i+1];return a.x+(b.x-a.x)*(z-a.z)/(b.z-a.z)};
 const bands={west:[],east:[]};
 for(const b of w.buildings){const dx=b.x-riverX(b.z),shoreDistance=Math.abs(dx)-(w.terrain.river_half_width??32);if(shoreDistance<=100)bands[dx<0?'west':'east'].push(b);}
 const mapped=save.masterplan;
 return {tick:w.tick,buildings:w.buildings.length,roads:w.roads.length,realizedLots:mapped.lot_parcels.filter(Boolean).length,plannedLots:mapped.lot_parcels.length,realizedRoads:mapped.road_ids.filter(Boolean).length,plannedRoads:mapped.road_ids.length,waterfront:Object.fromEntries(Object.entries(bands).map(([side,bs])=>[side,{count:bs.length,open:bs.filter(b=>b.stage==='open').length,nearMidpoint:bs.filter(b=>Math.abs(b.z)<100).length,zRange:bs.length?[Math.min(...bs.map(b=>b.z)),Math.max(...bs.map(b=>b.z))]:null}]))};
}
for(let i=0;i<25;i++){if(i)engine.step(2000);results.push(measure());console.log(JSON.stringify(results.at(-1)));}
mkdirSync('.tools/waterfront-growth',{recursive:true});writeFileSync('.tools/waterfront-growth/fully-realized.json',engine.save());engine.free();writeFileSync('validation/waterfront-growth.json',JSON.stringify({scope:'Current WASM normal growth from saved checkpoint; waterfront band uses center distance within100m of X-offset water edge, not footprint coverage. No browser performance claim.',results},null,2)+'\n');
