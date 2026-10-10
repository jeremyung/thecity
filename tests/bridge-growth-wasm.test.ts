// @ts-expect-error Node built-in types are not included in the application config.
import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { CityEngine, initSync } from '../public/wasm/city_engine.js';
import type { World } from '../web/src/types';

it('uses an opened bridge to establish occupied far-bank frontage through ordinary construction', () => {
  initSync({module:readFileSync(new URL('../public/wasm/city_engine_bg.wasm',import.meta.url))});
  const engine=CityEngine.restore(readFileSync(new URL('../validation/mature-world.multiblock-candidate.json',import.meta.url),'utf8'));
  let replay:CityEngine|undefined;
  try {
    const initial:World=JSON.parse(engine.snapshot());
    const bridge=initial.roads.find(r=>r.kind==='bridge'&&r.stage==='open'&&r.connected)!;
    expect(bridge).toBeDefined();
    const endpoints=[bridge.from,bridge.to].map(id=>initial.nodes.find(n=>n.id===id)!);
    const west=endpoints.reduce((a,b)=>a.x<b.x?a:b);
    expect(initial.buildings.filter(b=>b.x<west.x).length).toBe(0);
    let roadWasUnderConstruction=false,buildingWasUnderConstruction=false;
    let settled=false;
    for(let elapsed=0;elapsed<64000;elapsed+=20){
      engine.step(20);
      const world:World=JSON.parse(engine.snapshot());
      const westernIds=new Set(world.nodes.filter(n=>n.x<=west.x).map(n=>n.id));
      roadWasUnderConstruction ||= world.projects.some(p=>p.kind==='road'&&p.progress<1&&p.from!==null&&westernIds.has(p.from));
      buildingWasUnderConstruction ||= world.buildings.some(b=>b.x<west.x&&b.stage!=='open');
      settled=world.buildings.some(b=>b.x<west.x&&b.stage==='open'&&['home','apartment'].includes(b.kind)&&b.occupancy>0);
      if(settled)break;
    }
    expect(roadWasUnderConstruction).toBe(true);
    expect(buildingWasUnderConstruction).toBe(true);
    expect(settled).toBe(true);
    replay=CityEngine.restore(engine.save());engine.step(2000);replay.step(2000);
    expect(replay.save()).toBe(engine.save());
  } finally {replay?.free();engine.free();}
},60000);
