import {chromium} from '@playwright/test';
import {readFile,writeFile} from 'node:fs/promises';
const saved=await readFile('.tools/rear-extension-review/scheduler-save.json','utf8');
const completed=JSON.parse(await readFile('.tools/rear-extension-review/completed-snapshot.json','utf8'));
const browser=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});
try{
 const page=await browser.newPage({viewport:{width:1200,height:800},deviceScaleFactor:1});
 page.on('pageerror',error=>console.error(error.message));
 await page.goto('http://127.0.0.1:5173/?webgl=1',{waitUntil:'domcontentloaded'});
 await page.waitForFunction(()=>document.querySelector('canvas')?.dataset.graphicsPhase==='first-frame-submitted',null,{timeout:90000});
 await page.evaluate(save=>{const c=window.__city;c.renderer.renderer.setAnimationLoop(null);c.client.controls({paused:true});c.client.init(87231,save);},saved);
 await page.waitForFunction(()=>window.__city?.world?.tick>=275520&&window.__city.world.buildings.length===871,null,{timeout:30000});
 const initial=await page.evaluate(()=>{
  const r=window.__city.renderer;r.renderer.setAnimationLoop(null);r.director.setMode('explore');r.setPreview(12);r.setFocus(false);r.setQuality('balanced');
  const pipeline=r.pipeline;r.pipeline=null;let steps=0;
  while((r.objectQueue.size||r.chunks.pendingCount)&&steps<10000){r.animate();steps++;}
  r.pipeline=pipeline;r.director.target.set(106,4,20);r.camera.position.set(190,105,-75);r.camera.lookAt(r.director.target);r.animate();
  return {tick:r.worldState.tick,steps,buildings:r.objectQueue.size,chunks:r.chunks.pendingCount,staging:r.rearExtensions.root.children.length};
 });
 await page.screenshot({path:'validation/screenshots/rear-extension/city-construction-anime.png'});
 const transition=await page.evaluate(after=>{
  const r=window.__city.renderer;const pipeline=r.pipeline;r.pipeline=null;r.setWorld(after,false);
  const initialStaging=r.rearExtensions.root.children.length;const handoffs=[];let steps=0,last=initialStaging;
  while((r.objectQueue.size||r.chunks.pendingCount)&&steps<10000){r.animate();steps++;const count=r.rearExtensions.root.children.length;if(count!==last){handoffs.push({step:steps,from:last,to:count,remainingChunks:r.chunks.pendingCount});last=count;}}
  r.pipeline=pipeline;r.director.target.set(106,4,20);r.camera.position.set(190,105,-75);r.camera.lookAt(r.director.target);r.animate();
  return {steps,initialStaging,finalStaging:r.rearExtensions.root.children.length,handoffs,buildings:r.objectQueue.size,chunks:r.chunks.pendingCount};
 },completed);
 await page.screenshot({path:'validation/screenshots/rear-extension/city-completed-anime.png'});
 await writeFile('validation/rear-extension-render-handoff.json',JSON.stringify({source:'mature871 scheduler fixture, real engine snapshots',backend:'software WebGL',method:'pipeline detached while manually draining bounded update queues; restored for captures',performanceEvidence:false,settings:{previewHour:12,depthOfField:false,style:'anime',quality:'balanced',camera:[190,105,-75],target:[106,4,20],viewport:[1200,800],uiClock:'unchanged simulation time, not preview time'},miniatureCapture:'Separate attempt completed style-switch await but screenshot readback timed out after 30s; no accepted full-city miniature capture',initial,transition},null,2)+'\n');
 console.log(JSON.stringify({initial,transition}));
}finally{await browser.close();}
