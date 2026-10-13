// Normal seeded growth: no edited parcels, demand, infrastructure or actor state.
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {resolve,dirname} from 'node:path';
import {pathToFileURL} from 'node:url';
const [wasmDir='public/wasm', output='.tools/skyline-growth/current', seedText='87231', tickText='76000']=process.argv.slice(2);
const {CityEngine,initSync}=await import(pathToFileURL(resolve(wasmDir,'city_engine.js')));
initSync({module:readFileSync(resolve(wasmDir,'city_engine_bg.wasm'))});
const seed=Number(seedText),limit=Number(tickText);
if(!Number.isSafeInteger(seed)||seed<0||seed>0xffffffff||!Number.isSafeInteger(limit)||limit<1)throw new Error('Expected uint32 seed and positive tick limit');
const engine=new CityEngine(seed);engine.set_development_rate(16);
const samples=[],seenTowers=new Set();let stationFirstObserved=null;
const started=performance.now();
for(let target=0;target<=limit;target=Math.min(limit,target+2000)){
 if(target)engine.step(target-samples.at(-1).tick);
 const world=JSON.parse(engine.snapshot()),save=JSON.parse(engine.save());
 if(save.station_online&&stationFirstObserved===null)stationFirstObserved=world.tick;
 const towers=world.buildings.filter(b=>b.floors>=14);
 const firstObserved=towers.filter(b=>!seenTowers.has(b.id));for(const b of towers)seenTowers.add(b.id);
 samples.push({tick:world.tick,buildings:world.buildings.length,stationOnline:save.station_online,towers:towers.length,firstObserved:firstObserved.map(b=>({id:b.id,parcel:b.parcel_id,x:b.x,z:b.z,floors:b.floors,stage:b.stage})),realizedLots:save.masterplan.lot_parcels.filter(Boolean).length,plannedLots:save.masterplan.lot_parcels.length});
 if(target===limit)break;
}
const world=JSON.parse(engine.snapshot()),histogram={};for(const b of world.buildings)histogram[b.floors]=(histogram[b.floors]??0)+1;
const saved=engine.save(),restored=CityEngine.restore(saved);
if(restored.snapshot()!==engine.snapshot())throw new Error('Immediate restored snapshot differs');
engine.step(200);restored.step(200);
if(restored.snapshot()!==engine.snapshot())throw new Error('200-tick original/restored replay differs');
const result={seed,developmentRate:16,requestedTicks:limit,elapsedMs:performance.now()-started,scope:'Node WASM normal growth; snapshots every 2000 ticks. First observations are upper bounds, not exact event ticks. No browser performance claim.',stationFirstObserved,samples,histogram,finalTowers:world.buildings.filter(b=>b.floors>=14),replayTicks:200,replayEquivalent:true};
mkdirSync(dirname(output),{recursive:true});writeFileSync(output+'.json',JSON.stringify(result,null,2)+'\n');writeFileSync(output+'-save.json',saved);
console.log(JSON.stringify({output,buildings:world.buildings.length,towers:result.finalTowers.map(b=>b.floors),elapsedMs:result.elapsedMs,replayEquivalent:true}));restored.free();engine.free();
