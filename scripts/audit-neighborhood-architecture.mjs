// Descriptive production-family audit; does not measure screen-space visibility.
// node scripts/audit-neighborhood-architecture.mjs OUTPUT WORLD [WORLD...]
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { resolve, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { build } from 'esbuild';
const [output, ...inputs] = process.argv.slice(2);
if (!output || !inputs.length) throw new Error('Expected OUTPUT WORLD [WORLD...]');
const root = process.cwd();
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const temp = await mkdtemp(resolve('.tools/neighborhood-architecture-'));
try {
  const runtime = resolve(temp, 'runtime.mjs');
  await build({stdin:{contents:`
    export {partyWallBodyWidth,partyWallBodyDepth} from './web/src/render/kit';
    export {midriseMassing} from './web/src/render/building-massing';
    export {lowriseMassing,lowriseBuildingWidth} from './web/src/render/lowrise-massing';
    export {buildingPalette} from './web/src/render/building-palette';
    export {ArtMaterials} from './web/src/render/materials';
  `,resolveDir:root},bundle:true,platform:'node',format:'esm',packages:'external',outfile:runtime});
  const k = await import(pathToFileURL(runtime).href);
  const neutrals = new k.ArtMaterials().palette;
  const count = (rows, key) => rows.reduce((out,row)=>{const v=key(row);out[v]=(out[v]??0)+1;return out;},{});
  const describe = b => {
    const d=k.partyWallBodyDepth(b.depth);
    const w=k.lowriseBuildingWidth(b,k.partyWallBodyWidth(b.width,b.anchor,b.floors,b.kind),d);
    const mid=k.midriseMassing(b,w,d),low=k.lowriseMassing(b,w,d);
    const family=b.floors>=12&&['office','apartment'].includes(b.kind)?`tower-${b.kind}`:mid?.family??low?.family??`generic-${b.kind}`;
    const palette=k.buildingPalette(b,neutrals);
    return {id:b.id,x:b.x,z:b.z,kind:b.kind,floors:b.floors,family,palette:palette.family,wall:palette.wall};
  };
  const summary=rows=>({count:rows.length,kinds:count(rows,b=>b.kind),families:count(rows,b=>b.family),palettes:count(rows,b=>b.palette),wallSwatches:count(rows,b=>b.wall)});
  const worlds=[];
  for(const input of inputs){
    const bytes=await readFile(input),world=JSON.parse(bytes);
    const rows=world.buildings.filter(b=>b.stage==='open').map(describe);
    const tiles=[];
    for(let x=world.bounds.min_x;x<world.bounds.max_x;x+=160)for(let z=world.bounds.min_z;z<world.bounds.max_z;z+=160){
      const members=rows.filter(b=>b.x>=x&&b.x<Math.min(x+160,world.bounds.max_x)&&b.z>=z&&b.z<Math.min(z+160,world.bounds.max_z));
      if(members.length)tiles.push({bounds:[x,z,Math.min(x+160,world.bounds.max_x),Math.min(z+160,world.bounds.max_z)],...summary(members),ids:members.map(b=>b.id)});
    }
    worlds.push({source:relative(root,resolve(input)),sha256:hash(bytes),seed:world.seed,tick:world.tick,wholeCity:summary(rows),tiles});
  }
  const report={method:'Open building centers in fixed 160m world-aligned tiles; production massing, envelope and palette helpers. All occupied tiles retained, no favorable crop selection.',limitations:['Not the prior Watch camera frustum or projected facade area.','Family labels do not measure geometric silhouette diversity within a family.','Palette counts do not establish final shaded appearance or roughness.','Construction is excluded; no artistic acceptance inferred.'],runtimeSha256:hash(await readFile(runtime)),worlds};
  await writeFile(output,JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(worlds.map(w=>({seed:w.seed,tick:w.tick,...w.wholeCity,tiles:w.tiles.length})),null,2));
}finally{await rm(temp,{recursive:true,force:true});}
