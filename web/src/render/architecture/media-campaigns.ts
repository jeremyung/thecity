import * as THREE from 'three/webgpu';
import {texture,uv,mix,fract,smoothstep,sin,vec2,vec3,floor,fwidth} from 'three/tsl';
import {canvasTexture,type ArtMaterials} from '../materials';
export const MEDIA_CAMPAIGNS=[
 {name:'星屑サイダー',en:'HOSHIKUZU CIDER',line:['ひとくち、夏になる。','街に、はじける。'],bg:'#10384d',ink:'#fff4d7',accent:'#f2bb45',spill:'#80c9d5',kind:'drink'},
 {name:'灯りレコード',en:'AKARI RECORDS',line:['次の一曲に、出会う。','夜を、鳴らそう。'],bg:'#332039',ink:'#fff0db',accent:'#ed8b6d',spill:'#de9e91',kind:'record'},
 {name:'空色旅行',en:'SORAIRO TRAVEL',line:['次の休日は、空の下。','知らない街へ。'],bg:'#1e5360',ink:'#fff7de',accent:'#f2b866',spill:'#8fc6cd',kind:'travel'},
 {name:'山手珈琲',en:'YAMATE COFFEE',line:['ひと息の、いい時間。','今日の一杯を。'],bg:'#49382c',ink:'#fff2d4',accent:'#c8a378',spill:'#ddba91',kind:'coffee'},
] as const;
export const MEDIA_CAMPAIGN_ATLAS_SIZE=1024;
export function campaignCell(program:number,scene:number){const row=((program%4)+4)%4,col=scene?1:0;return {u0:(col*512+3)/1024,u1:((col+1)*512-3)/1024,v0:1-((row+1)*256-3)/1024,v1:1-(row*256+3)/1024};}
function ellipse(c:CanvasRenderingContext2D,x:number,y:number,rx:number,ry:number,fill:string){c.fillStyle=fill;c.beginPath();c.ellipse(x,y,rx,ry,0,0,Math.PI*2);c.fill();}
function drawCampaign(c:CanvasRenderingContext2D,index:number,scene:number){
 const a=MEDIA_CAMPAIGNS[index];c.fillStyle=a.bg;c.fillRect(0,0,512,256);
 // Deliberate print-like composition: stable typography, a recognizable product,
 // controlled palette and a secondary scene, rather than animated colour noise.
 c.fillStyle=a.accent;c.fillRect(116,116,38,4);c.fillStyle=a.ink;c.textAlign='left';c.textBaseline='middle';
 c.font='700 13px sans-serif';c.fillText(a.en,116,49,174);
 c.font='800 31px "Hiragino Kaku Gothic ProN",sans-serif';c.fillText(a.name,116,94,174);
 c.font='600 16px "Hiragino Kaku Gothic ProN",sans-serif';c.fillText(a.line[scene],116,139,174);
 c.font='500 10px sans-serif';c.fillText(scene?'DISCOVER THE COLLECTION':'A LITTLE SOMETHING FOR YOUR DAY',116,179,174);
 c.fillStyle=a.accent;c.fillRect(116,211,168,1);c.fillStyle=a.ink;c.font='700 11px sans-serif';c.fillText(scene?'NEW · 街で見つけよう':'NOW OPEN · 新しい毎日へ',116,229,174);
 const x=scene?379:392;
 if(a.kind==='drink'){
  const shade=c.createLinearGradient(x-45,0,x+45,0);shade.addColorStop(0,'#a7c4c9');shade.addColorStop(.18,'#f0f4e3');shade.addColorStop(.55,'#f9fbf1');shade.addColorStop(1,'#6b969e');
  c.fillStyle=shade;c.beginPath();c.roundRect(x-43,48,86,153,12);c.fill();ellipse(c,x,48,41,8,'#d6e2da');ellipse(c,x,48,25,4,'#7f9695');
  c.fillStyle=a.bg;c.fillRect(x-42,88,84,70);c.fillStyle=a.ink;c.textAlign='center';c.font='800 18px sans-serif';c.fillText('HOSHI',x,111);c.font='700 11px sans-serif';c.fillText('CIDER',x,133);
  ellipse(c,x+31,193,30,28,a.accent);ellipse(c,x+31,193,23,21,'#fff0a9');
  c.strokeStyle=a.accent;c.lineWidth=1;for(let n=0;n<8;n++){const t=n*Math.PI/4;c.beginPath();c.moveTo(x+31,193);c.lineTo(x+31+Math.cos(t)*22,193+Math.sin(t)*20);c.stroke();}
  for(let n=0;n<9;n++)ellipse(c,x-60+(n*29%132),30+n*23,2+(n%3),2+(n%3),'#9fc5cc');
 }else if(a.kind==='record'){
  ellipse(c,x,133,94,94,'#151b21');c.strokeStyle='#48515a';c.lineWidth=1;for(let r=30;r<90;r+=5){c.beginPath();c.arc(x,133,r,0,Math.PI*2);c.stroke();}
  ellipse(c,x,133,30,30,a.accent);ellipse(c,x,133,5,5,a.ink);c.fillStyle=a.ink;c.textAlign='center';c.font='700 10px sans-serif';c.fillText('AKARI',x,122);
  c.strokeStyle=a.ink;c.lineWidth=5;c.beginPath();c.moveTo(482,38);c.lineTo(460,145);c.lineTo(432,168);c.stroke();
 }else if(a.kind==='travel'){
  c.fillStyle='#a1c4c2';c.beginPath();c.roundRect(304,28,184,196,80);c.fill();ellipse(c,445,77,22,22,a.accent);
  c.fillStyle='#467e83';c.beginPath();c.moveTo(304,154);c.lineTo(364,79);c.lineTo(423,157);c.lineTo(488,117);c.lineTo(488,224);c.lineTo(304,224);c.fill();
  c.fillStyle='#254e59';c.beginPath();c.moveTo(304,200);c.lineTo(408,145);c.lineTo(488,186);c.lineTo(488,224);c.lineTo(304,224);c.fill();
  c.strokeStyle='#e8dac0';c.lineWidth=9;c.beginPath();c.moveTo(398,224);c.bezierCurveTo(485,189,351,187,402,160);c.stroke();
 }else{
  ellipse(c,x,200,91,18,'#af977c');c.fillStyle='#efe6ce';c.beginPath();c.roundRect(x-60,95,120,100,17);c.fill();ellipse(c,x,99,59,15,'#e9dcc0');ellipse(c,x,99,49,11,'#4b2c1e');
  c.strokeStyle='#efe6ce';c.lineWidth=11;c.beginPath();c.arc(x+61,136,26,-Math.PI/2,Math.PI/2);c.stroke();
  c.strokeStyle='#c4b294';c.lineWidth=3;for(let i=0;i<3;i++){c.beginPath();c.moveTo(x-23+i*24,77);c.bezierCurveTo(x-45+i*24,55,x+i*24,48,x-20+i*24,26);c.stroke();}
  c.fillStyle=a.bg;c.textAlign='center';c.font='700 13px sans-serif';c.fillText('YAMATE',x,145);c.font='500 10px sans-serif';c.fillText('COFFEE',x,164);
 }
}
export function flagshipCampaignSurface(art:ArtMaterials,program=0){const variant=((program%4)+4)%4;return art.shared(`flagship-campaign-${variant}`,()=>{
 const atlas=flagshipCampaignAtlas(art),p=uv(),phase=fract(art.displayTime.div(14).add(variant*.13));
 const blend=smoothstep(.43,.49,phase).mul(smoothstep(.93,.99,phase).oneMinus());
 const sample=(scene:number)=>{const c=campaignCell(variant,scene);return texture(atlas,p.mul(vec2(c.u1-c.u0,c.v1-c.v0)).add(vec2(c.u0,c.v0))).rgb;};
 const poster=mix(sample(0),sample(1),blend);
 // Stable cabinet calibration, separate from the advertisement. Derivatives use
 // unwrapped coordinates so seams cannot produce false footprints at each repeat.
 const cabinetUV=p.mul(vec2(12,8)),cabinet=floor(cabinetUV),cell=fract(cabinetUV);
 const hash=fract(sin(cabinet.x.mul(127.1).add(cabinet.y.mul(311.7)).add(variant*19.3)).mul(43758.5453));
 const tint=fract(sin(cabinet.x.mul(269.5).add(cabinet.y.mul(183.3))).mul(43758.5453));
 const footprint=fwidth(cabinetUV).x.max(fwidth(cabinetUV).y);
 const cabinetVisibility=smoothstep(.2,.7,footprint).oneMinus();
 const edge=cell.x.min(cell.y).min(cell.x.oneMinus()).min(cell.y.oneMinus());
 const seam=smoothstep(.008,footprint.mul(.8).add(.01),edge);
 const joints=mix(1,seam.mul(.12).add(.88),cabinetVisibility);
 const calibration=mix(vec3(1),vec3(hash.mul(.07).add(.945),hash.mul(.06).add(.95),tint.mul(.065).add(.945)),cabinetVisibility);
 // Fine emitters resolve only in close views. Fade to their average before
 // minification; no scanline animation, random flicker or additional texture.
 const pixelUV=p.mul(vec2(192,96)),pixel=fract(pixelUV);
 const pixelFootprint=fwidth(pixelUV).x.max(fwidth(pixelUV).y);
 const pixelVisibility=smoothstep(.65,1.4,pixelFootprint).oneMinus();
 const pixelEdge=pixel.x.min(pixel.y).min(pixel.x.oneMinus()).min(pixel.y.oneMinus());
 const emitter=smoothstep(.10,.27,pixelEdge);
 const emitters=mix(1,emitter.mul(.56).add(.57),pixelVisibility);
 // A fixed ordered threshold belongs to the LED lattice, so the pattern
 // follows the curved screen rather than crawling with the camera or time.
 const address=floor(pixelUV),parity=fract(address.mul(.5)).mul(2);
 const threshold=parity.x.mul(2).add(parity.y.mul(3)).mod(4).add(.5).div(4);
 const quantized=floor(poster.mul(40).add(threshold)).div(40);
 const dithered=mix(poster,quantized,pixelVisibility);
 const picture=dithered.mul(joints).mul(calibration).mul(emitters).mul(art.night.mul(.18).add(.85)).mul(art.litShop);
 const anime=new THREE.MeshToonNodeMaterial({color:'#070b0d',gradientMap:art.toonRamp}),miniature=new THREE.MeshStandardNodeMaterial({color:'#070b0d',roughness:.64});
 for(const m of [anime,miniature]){(m as unknown as {emissiveNode:typeof picture}).emissiveNode=picture;m.userData.flagshipCampaign=variant;}
 return {anime,miniature};
});}
const atlases=new WeakMap<ArtMaterials,THREE.CanvasTexture>();
export function flagshipCampaignAtlas(art:ArtMaterials){let atlas=atlases.get(art);if(!atlas){atlas=canvasTexture(1024,1024,c=>{for(let row=0;row<4;row++)for(let scene=0;scene<2;scene++){c.save();c.translate(scene*512,row*256);drawCampaign(c,row,scene);c.restore();}});atlas.anisotropy=8;atlases.set(art,atlas);const owned=atlas;art.onDispose(()=>{owned.dispose();atlases.delete(art);});}return atlas;}
export function flagshipPosterSurface(art:ArtMaterials){return art.shared('flagship-campaign-poster',()=>{
 const atlas=flagshipCampaignAtlas(art),anime=new THREE.MeshToonNodeMaterial({map:atlas,gradientMap:art.toonRamp}),miniature=new THREE.MeshStandardNodeMaterial({map:atlas,roughness:.5});
 const emission=texture(atlas).rgb.mul(art.night.mul(.45).add(.05)).mul(art.litShop);for(const m of [anime,miniature])(m as unknown as {emissiveNode:typeof emission}).emissiveNode=emission;
 return {anime,miniature};
});}
