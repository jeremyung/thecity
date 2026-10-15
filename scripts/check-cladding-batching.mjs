// Browser regression: authored and shared-batch cladding must produce the same image.
// Start Vite first. CLADDING_URL selects its URL/backend; CLADDING_OUTPUT stores local evidence.
import {chromium} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const dir=process.env.CLADDING_OUTPUT??'.tools/cladding-parity';await mkdir(dir,{recursive:true});
const browser=await chromium.launch({headless:true,args:['--use-angle=metal','--ignore-gpu-blocklist']});
const errors=[],report={errors,captures:[]};
try{
const page=await browser.newPage({viewport:{width:1200,height:800}});
page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
await page.goto(process.env.CLADDING_URL??'http://127.0.0.1:5173/?webgl=1');
await page.waitForFunction(()=>window.__city?.renderer&&document.querySelector('canvas')?.dataset.graphicsPhase==='first-frame-submitted',null,{timeout:120000});
report.setup=await page.evaluate(async()=>{
const r=window.__city.renderer;window.__city.client.controls({paused:true});r.renderer.setAnimationLoop(null);
const T=await import('/node_modules/three/build/three.webgpu.js');const {tagCladding}=await import('/web/src/render/building-surfaces.ts');
r.director.setMode('explore');r.director.update=()=>{};r.setFocus(false);r.setQuality('balanced');r.setPreview(13);
const g=new T.Group();window.__matrix=g;r.scene.add(g);window.__matrixSources=[];
for(const [i,kind]of ['siding','tile','alc','concrete'].entries()){
const mat=r.art.get('#c2b69d',.85,false,'ambient',kind);const geo=new T.BoxGeometry(8,14,3);tagCladding(geo,kind);
let out=mat;{const b=r.art.buildingBatch(mat);out=b.material;const rgb=new Float32Array(geo.attributes.position.count*3);for(let k=0;k<rgb.length;k+=3){rgb[k]=b.color.r;rgb[k+1]=b.color.g;rgb[k+2]=b.color.b;}geo.setAttribute('color',new T.BufferAttribute(rgb,3));}
const mesh=new T.Mesh(geo,out);mesh.position.set((i-1.5)*11,7,0);g.add(mesh);window.__matrixSources.push({mesh,mat});}
for(const child of r.scene.children)if(child!==g&&!child.isLight)child.visible=false;
r.camera.position.set(35,35,65);r.camera.lookAt(0,7,5);
return{gpuAvailable:!!navigator.gpu,renderer:r.renderer.backend.constructor.name};
});
await page.addStyleTag({content:'* { visibility: hidden !important; } canvas { visibility: visible !important; }'});
for(const style of ['anime','miniature']){const pictures=[];for(const mode of ['authored','batched']){
await page.evaluate(async ([style,mode])=>{const r=window.__city.renderer;await r.setStyle(style);for(const {mesh,mat} of window.__matrixSources){mesh.material=mode==='batched'?r.art.buildingBatch(r.art.variant(mat,style)).material:r.art.variant(mat,style);}r.setPreview(13);const p=r.pipeline;r.pipeline=null;for(let i=0;i<20;i++){r.lastFrame=performance.now()-100;r.animate()}r.pipeline=p;for(const c of r.scene.children)if(c!==window.__matrix&&!c.isLight)c.visible=false;r.camera.position.set(35,35,65);r.camera.lookAt(0,7,5);r.renderer.render(r.scene,r.camera);r.renderer.backend.gl?.finish()},[style,mode]);
pictures.push((await page.locator('canvas').first().screenshot({path:`${dir}/${style}-${mode}.png`})).toString('base64'));report.captures.push(`${style}-${mode}`);}
const comparison=await page.evaluate(async pictures=>{const arrays=[];for(const p of pictures){const img=new Image();img.src='data:image/png;base64,'+p;await img.decode();const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);arrays.push(ctx.getImageData(0,0,c.width,c.height).data);}let sum=0,n=0,large=0;for(let i=0;i<arrays[0].length;i++){if(i%4===3)continue;const d=Math.abs(arrays[0][i]-arrays[1][i]);sum+=d;n++;if(d>10)large++;}return{meanAbsoluteError:sum/n,largeDifferenceFraction:large/n};},pictures);
report[style]=comparison;if(comparison.meanAbsoluteError>1||comparison.largeDifferenceFraction>.005)errors.push(`${style} authored/batched pixel mismatch: ${JSON.stringify(comparison)}`);
}
report.browser=browser.version();
}finally{await writeFile(`${dir}/report.json`,JSON.stringify(report,null,2));await browser.close();}
if(errors.length)throw Error(errors.join('\n'));console.log(JSON.stringify(report));
