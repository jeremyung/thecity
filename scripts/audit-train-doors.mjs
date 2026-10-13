// Compare native/WASM train views, capture an ordinary open-door stop, and replay it.
// Usage: node scripts/audit-train-doors.mjs /tmp/native-snapshot.json
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {CityEngine,initSync} from '../public/wasm/city_engine.js';
const file=process.argv[2];
if(!file)throw Error('Pass native snapshot and matching .save.json');
initSync({module:readFileSync(new URL('../public/wasm/city_engine_bg.wasm',import.meta.url))});
const native=JSON.parse(readFileSync(file,'utf8'));
const engine=CityEngine.restore(readFileSync(`${file}.save.json`,'utf8'));
let restored;
try {
  let world=JSON.parse(engine.snapshot());
  assert.deepEqual(world.trains,native.trains,'native/WASM train metadata differs');
  let found=false;
  for(let batch=0;batch<400;batch++){
    const t=world.trains[0];
    assert(t && t.door_open_fraction>=0 && t.door_open_fraction<=1);
    if(t.door_open_fraction===1 && t.door_terminal===t.route[0]){found=true;break;}
    engine.step(5);world=JSON.parse(engine.snapshot());
  }
  assert(found,'No ordinary station door-open phase within 2,000 ticks');
  mkdirSync('.tools/fidelity-review',{recursive:true});
  writeFileSync('.tools/fidelity-review/rail-doors-open.json',engine.save());
  const capturedTick=world.tick;
  restored=CityEngine.restore(engine.save());
  for(let batch=0;batch<10;batch++){
    engine.step(20);restored.step(20);
    assert.equal(engine.save(),restored.save());
    assert.deepEqual(JSON.parse(engine.snapshot()).trains,JSON.parse(restored.snapshot()).trains);
    const expected=JSON.parse(engine.snapshot()).trains.slice(0,8).flatMap(t=>[t.id,t.door_terminal??-1,t.door_open_fraction]);
    assert.deepEqual(Array.from(engine.train_doors()),Array.from(new Float32Array(expected)));
    assert.deepEqual(Array.from(engine.train_doors()),Array.from(restored.train_doors()));
    assert.deepEqual(Array.from(engine.movements()),Array.from(restored.movements()));
  }
  console.log(JSON.stringify({seed:world.seed,nativeTick:native.tick,nativeWasmTrainViews:'equal',openTick:capturedTick,train:world.trains[0],replayTicks:200,saveMovementAndDoorReplay:'equal',compactDoorStreamReplay:'equal'},null,2));
}finally{engine.free();restored?.free();}
