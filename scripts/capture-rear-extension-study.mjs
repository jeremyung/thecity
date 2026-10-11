import { chromium } from '@playwright/test';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
await mkdir('.tools/rear-extension-review',{recursive:true});
await mkdir('validation/screenshots/rear-extension',{recursive:true});
await writeFile('.tools/rear-extension-review/three.ts',"export * from 'three/webgpu';\n");
const before=JSON.parse(await readFile('.tools/rear-extension-review/scheduler-snapshot.json','utf8'));
const after=JSON.parse(await readFile('.tools/rear-extension-review/completed-snapshot.json','utf8'));
const browser=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try {
 const page=await browser.newPage({viewport:{width:1440,height:720},deviceScaleFactor:1});
 page.on('pageerror',error=>console.error(error.message));
 await page.route('**/annex-review.html',route=>route.fulfill({contentType:'text/html',body:`<!doctype html><meta charset="UTF-8"><style>body{margin:0;background:#e0ded6;font:14px system-ui;color:#343d3f}header{position:absolute;top:20px;left:24px}footer{position:absolute;bottom:12px;left:24px}.labels{position:absolute;top:70px;display:flex;width:100%}.labels span{width:33.333%;padding-left:24px;box-sizing:border-box}canvas{display:block}</style><header>Rear annex · asset inspection · same camera, lighting and building</header><div class="labels"><span>Original</span><span>Construction</span><span>Completed</span></div><footer>Production geometry and engine snapshots · isolated studio lighting · software WebGL · not a full-city beauty or performance acceptance</footer><script type="module">
import * as THREE from '/.tools/rear-extension-review/three.ts';
import { ArtMaterials } from '/web/src/render/materials.ts';
import { buildExtendedBuilding } from '/web/src/render/extended-building.ts';
import { RearExtensions } from '/web/src/render/rear-extensions.ts';
const renderer=new THREE.WebGPURenderer({antialias:true,forceWebGL:true});
renderer.setSize(1440,720);renderer.setPixelRatio(1);document.body.append(renderer.domElement);await renderer.init();
window.draw=async(before,after,id,style)=>{
 const art=new ArtMaterials();art.setStyle(style);
 const host=before.buildings.find(b=>b.id===id);const final=after.buildings.find(b=>b.id===id);
 const original=after.building_original_footprints.find(b=>b.building_id===id);
 const rotation=host.rotation,c=Math.cos(rotation),s=Math.sin(rotation);
 const center=new THREE.Vector3(host.x-s*4,0,host.z-c*4);
 const scenes=[0,1,2].map(i=>{
   const scene=new THREE.Scene();scene.background=new THREE.Color('#dddcd4');
   scene.add(new THREE.HemisphereLight('#e6f0ff','#939077',2.1));
   const sun=new THREE.DirectionalLight('#fff1d0',3);sun.position.set(center.x+40,65,center.z+35);scene.add(sun);
   const ground=new THREE.Mesh(new THREE.PlaneGeometry(180,180),new THREE.MeshStandardNodeMaterial({color:'#b5b8a6',roughness:1}));ground.rotation.x=-Math.PI/2;ground.position.copy(center);ground.position.y=-.025;scene.add(ground);
   const b=i===2?final:host;const form=i===2?original:undefined;
   const group=buildExtendedBuilding(b,form,art);group.position.set(b.x,0,b.z);group.rotation.y=b.rotation;scene.add(group);
   if(i===1){const staging=new RearExtensions(art,release=>release());staging.setWorld({...before,rear_extensions:before.rear_extensions.filter(e=>e.building_id===id)});scene.add(staging.root);}
   return scene;
 });
 const camera=new THREE.PerspectiveCamera(40,480/720,.1,500);
 camera.position.set(center.x+c*42-s*58,42,center.z-s*42-c*58);camera.lookAt(center.x,host.floors*1.1,center.z);
 renderer.setScissorTest(true);
 for(let i=0;i<3;i++){renderer.setViewport(i*480,0,480,720);renderer.setScissor(i*480,0,480,720);await renderer.renderAsync(scenes[i],camera);}
 document.querySelector('header').textContent='Rear annex · '+host.kind+' '+id+' · '+style+' · same camera and lighting';
 return {id,style,original,finalDepth:final.depth};
};window.ready=true;
</script>`}));
 await page.goto('http://127.0.0.1:5173/annex-review.html');
 await page.waitForFunction(()=>window.ready,null,{timeout:60000});
 const records=[];
 for(const id of [1343,1369]) for(const style of ['anime','miniature']) {
   records.push(await page.evaluate(async({before,after,id,style})=>window.draw(before,after,id,style),{before,after,id,style}));
   await page.screenshot({path:`validation/screenshots/rear-extension/${id}-${style}.png`});
 }
 await writeFile('.tools/rear-extension-review/capture-record.json',JSON.stringify(records,null,2));
} finally {await browser.close();}
