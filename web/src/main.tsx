import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import Controls, { busLineLabel } from './ui/Controls';
import './ui/style.css';
import './base.css';
import { CityRenderer, type RenderMetrics } from './render/city';
import { ArchitectureAssetReview } from './render/asset-review';
import { SimulationClient } from './worker/client';
import { AudioDirector } from './audio/director';
import { saveWorld, loadWorld, exportWorld, loadViewerPreferences, saveViewerPreferences, validateWorldSave } from './storage/saves';
import { resolveStartupWorld } from './storage/recovery';
import { createGraphicsStartupWatchdog } from './graphics-startup';
import type { LensPresetId } from './camera/lens';
import type { World, Style, Metrics } from './types';

type Subject={id:number;name:string;kind:string;activity:string;destination?:string;history?:string[];passengers?:number;capacity?:number;station_waiting?:number;gateway_waiting?:number;line?:string;nextStop?:string;dwelling?:boolean;waiting?:number};
function inspectedSubject(id:number,data:unknown,world:World|null):Subject|null{
  const info=data as {entityType?:string;name?:string;activity?:string;kind?:string;destination?:number;history?:string[];passengers?:number;capacity?:number;station_waiting?:number;gateway_waiting?:number;line?:string|number;line_id?:number;next_stop?:number|null;dwelling?:boolean;waiting?:number};
  if(info.entityType==='unknown')return null;
  const bus=info.entityType==='bus'||info.kind==='bus';
  const snapshotBus=bus?world?.buses?.find(entity=>entity.id===id):undefined;
  const lineId=info.line_id??snapshotBus?.line_id??(typeof info.line==='number'?info.line:undefined);
  const nextStopId=info.next_stop??snapshotBus?.next_stop??undefined;
  const destinationId=bus?nextStopId:info.destination;
  const node=world?.nodes.find(n=>n.id===destinationId);
  const lineLabel=busLineLabel(world,lineId,typeof info.line==='string'?info.line:undefined);
  const dwelling=info.dwelling??snapshotBus?.dwelling;
  return{id,name:info.name??(bus?'Neighborhood Bus':'A city traveler'),kind:bus?'bus':info.kind==='rail'?'train':info.kind==='car'?'vehicle':'person',activity:(info.activity??(bus?(dwelling?'At stop':'In service'):'On a journey')).replaceAll('_',' '),destination:bus?undefined:world?.buildings.find(b=>b.id===destinationId)?.name??node?.kind.replaceAll('_',' '),history:info.history?.slice(-3),passengers:info.passengers??snapshotBus?.passengers,capacity:info.capacity??snapshotBus?.capacity,station_waiting:info.station_waiting,gateway_waiting:info.gateway_waiting,line:bus?lineLabel:undefined,nextStop:node?.kind.replaceAll('_',' ')??undefined,dwelling:bus?dwelling:undefined,waiting:bus?(info.waiting??snapshotBus?.waiting):undefined};
}

function ArchitectureReviewRoute(){
  const materialCandidate=new URLSearchParams(window.location.search).get('material')==='concrete'?'concrete':'plaster';
  const mount=useRef<HTMLDivElement>(null),viewer=useRef<ArchitectureAssetReview|null>(null);
  const [style,setStyle]=useState<Style>('anime'),[error,setError]=useState<string|null>(null),[materialStatus,setMaterialStatus]=useState<'loading'|'ready'|'error'>('loading'),[shopPlaster,setShopPlaster]=useState(true),[nightReview,setNightReview]=useState(false);
  const [lowriseStage,setLowriseStage]=useState<'open'|'construction'|'renovation'>('open'),[lowriseProgress,setLowriseProgress]=useState(.5);
  useEffect(()=>{
    if(!mount.current)return;
    let disposed=false;
    const review=new ArchitectureAssetReview(mount.current,style,materialCandidate);review.onMaterialStatus=setMaterialStatus;viewer.current=review;
    void review.start().catch(e=>{if(!disposed)setError(e instanceof Error?e.message:String(e));});
    return()=>{disposed=true;review.dispose();viewer.current=null;};
  },[]);
  useEffect(()=>{viewer.current?.setStyle(style);},[style]);
  useEffect(()=>{viewer.current?.setLowriseStage(lowriseStage,lowriseProgress);},[lowriseStage,lowriseProgress]);
  return <main className="architecture-review">
    <style>{`
      .architecture-review{position:fixed;inset:0;overflow:hidden;background:#afc4ca;color:#263d43;font-family:ui-sans-serif,system-ui,-apple-system,sans-serif}
      .architecture-review__canvas{position:absolute;inset:0;touch-action:none}
      .architecture-review__canvas canvas{display:block;width:100%;height:100%;outline:none}
      .architecture-review__head{position:absolute;top:0;left:0;right:0;display:flex;align-items:flex-start;justify-content:space-between;gap:20px;padding:calc(env(safe-area-inset-top) + 18px) max(20px,env(safe-area-inset-right)) 12px max(20px,env(safe-area-inset-left));pointer-events:none;background:linear-gradient(#afc4cae8,transparent)}
      .architecture-review__title{margin:0;font-family:Georgia,'Hiragino Mincho ProN',serif;font-weight:400;font-size:clamp(24px,3vw,38px);letter-spacing:-.035em;line-height:1.02}
      .architecture-review__kicker{display:block;margin-bottom:9px;font-size:10px;letter-spacing:.19em;text-transform:uppercase;color:#a45442}
      .architecture-review__actions{display:flex;align-items:center;gap:7px;pointer-events:auto;flex-wrap:wrap;justify-content:flex-end}
      .architecture-review button,.architecture-review a{min-height:44px;border:1px solid #566b6b80;border-radius:3px;background:#f5f0e7e8;color:#293f43;padding:0 14px;font:inherit;font-size:12px;text-decoration:none;display:inline-flex;align-items:center;justify-content:center;cursor:pointer;backdrop-filter:blur(8px)}
      .architecture-review button[aria-pressed="true"]{background:#344f50;color:#f6eedf;border-color:#344f50}
      .architecture-review button:focus-visible,.architecture-review a:focus-visible{outline:2px solid #b65d4b;outline-offset:2px}
      .architecture-review__toggle{min-height:44px;display:inline-flex;align-items:center;gap:8px;border:1px solid #566b6b80;border-radius:3px;background:#f5f0e7e8;color:#293f43;padding:0 12px;font-size:12px;cursor:pointer;backdrop-filter:blur(8px)}
      .architecture-review__toggle input{width:16px;height:16px;margin:0;accent-color:#a45442}
      .architecture-review__toggle:focus-within{outline:2px solid #b65d4b;outline-offset:2px}
      .architecture-review__stage{min-height:44px;display:inline-flex;align-items:center;gap:7px;border:1px solid #566b6b80;border-radius:3px;background:#f5f0e7e8;color:#293f43;padding:0 9px;font-size:11px;backdrop-filter:blur(8px)}
      .architecture-review__stage select{max-width:116px;border:0;background:transparent;color:inherit;font:inherit;font-size:11px}
      .architecture-review__stage input[type=range]{width:78px;margin:0;accent-color:#a45442}
      .architecture-review__stage output{min-width:30px;font-variant-numeric:tabular-nums}
      .architecture-review__stage:focus-within{outline:2px solid #b65d4b;outline-offset:2px}
      .architecture-review__hint{position:absolute;right:22px;top:92px;max-width:230px;text-align:right;font-size:11px;line-height:1.5;color:#30484d;text-shadow:0 1px 8px #d8e3e1}
      .architecture-review__samples{position:absolute;left:max(20px,env(safe-area-inset-left));bottom:calc(env(safe-area-inset-bottom) + 20px);display:flex;gap:0;flex-wrap:wrap;max-width:min(920px,calc(100vw - 40px));background:#f6f1e8e8;box-shadow:0 12px 35px #23363824;backdrop-filter:blur(12px);border:1px solid #fff9;border-radius:4px;overflow:hidden}
      .architecture-review__sample{padding:12px 16px 11px;border-right:1px solid #7b858033;min-width:142px}
      .architecture-review__sample:last-child{border-right:0}
      .architecture-review__sample b{display:block;font-size:11px;font-weight:550;letter-spacing:.02em}
      .architecture-review__sample small{display:block;margin-top:4px;font-size:10px;color:#607171;letter-spacing:.08em;text-transform:uppercase}
      .architecture-review__error{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);padding:16px 20px;background:#f6eee8;color:#7c332e;border:1px solid #be7768;max-width:min(90vw,520px)}
      @media(max-width:640px){.architecture-review__head{gap:10px;padding-top:calc(env(safe-area-inset-top) + 12px);padding-left:max(12px,env(safe-area-inset-left));padding-right:max(12px,env(safe-area-inset-right))}.architecture-review__title{font-size:25px}.architecture-review__actions{gap:4px}.architecture-review button,.architecture-review a{padding:0 10px;font-size:11px}.architecture-review__toggle{min-height:44px;padding:0 9px;font-size:11px}.architecture-review__stage{min-height:44px;padding:0 7px;gap:5px}.architecture-review__stage select{max-width:100px}.architecture-review__stage input[type=range]{width:66px}.architecture-review__hint{top:84px;right:12px;max-width:180px;font-size:10px}.architecture-review__samples{left:12px;right:12px;bottom:calc(env(safe-area-inset-bottom) + 12px);max-width:none}.architecture-review__sample{flex:1;min-width:calc(50% - 1px);padding:9px 10px;border-bottom:1px solid #7b858033}.architecture-review__sample:nth-child(even){border-right:0}.architecture-review__sample b{font-size:10px}.architecture-review__sample small{font-size:9px}}
      @media(prefers-reduced-motion:reduce){.architecture-review *{scroll-behavior:auto!important;transition:none!important}}
    `}</style>
    <div className="architecture-review__canvas" ref={mount} aria-label="Interactive architectural specimen and station street. Drag to orbit; scroll to zoom." />
    <header className="architecture-review__head">
      <div><span className="architecture-review__kicker">Development review · procedural architecture</span><h1 className="architecture-review__title">Last Train, First Light</h1></div>
      <div className="architecture-review__actions" aria-label="Architecture review controls">
        <button type="button" aria-pressed={style==='anime'} onClick={()=>setStyle('anime')}>Anime</button>
        <button type="button" aria-pressed={style==='miniature'} onClick={()=>setStyle('miniature')}>Miniature</button>
        <label className="architecture-review__stage"><span>Low-rise stage</span><select aria-label="Low-rise specimen stage" value={lowriseStage} onChange={event=>setLowriseStage(event.target.value as 'open'|'construction'|'renovation')}><option value="open">Open</option><option value="construction">Construction</option><option value="renovation">Renovation</option></select></label>
        {lowriseStage==='construction'&&<label className="architecture-review__stage"><span>Progress</span><input aria-label="Low-rise construction progress" type="range" min="0" max="1" step="0.01" value={lowriseProgress} onChange={event=>setLowriseProgress(Number(event.target.value))}/><output>{Math.round(lowriseProgress*100)}%</output></label>}
        <label className="architecture-review__toggle"><input type="checkbox" checked={shopPlaster} onChange={event=>{setShopPlaster(event.target.checked);viewer.current?.setShopPlaster(event.target.checked);}}/>Candidate on shop</label>
        <label className="architecture-review__toggle"><input type="checkbox" checked={nightReview} onChange={event=>{setNightReview(event.target.checked);viewer.current?.setNight(event.target.checked);}}/>Night lighting</label>
        <button type="button" onClick={()=>viewer.current?.resetCamera()}>Reset view</button>
        <button type="button" onClick={()=>viewer.current?.inspectMaterial()}>Inspect material</button>
        <button type="button" onClick={()=>viewer.current?.inspectShop()}>Inspect shop</button>
        <button type="button" onClick={()=>viewer.current?.inspectApartment()}>Inspect apartment</button>
        <button type="button" onClick={()=>viewer.current?.inspectOffice()}>Inspect office</button>
        <button type="button" onClick={()=>viewer.current?.inspectLowrise()}>Inspect low-rise street</button>
        <button type="button" onClick={()=>viewer.current?.inspectLowrise(true)}>Inspect rear service</button>
        <a href="/" aria-label="Return to the living city">Return to city</a>
      </div>
    </header>
    <p className="architecture-review__hint">Drag to orbit · scroll to move closer<br/>Shared building kits · controlled material lighting</p>
    <section className="architecture-review__samples" aria-label="Architecture specimens">
      <div className="architecture-review__sample"><b>Kissa Aoba</b><small>Corner shop · 2 floors</small></div>
      <div className="architecture-review__sample"><b>Apartment tower</b><small>Individual bays · 20 floors</small></div>
      <div className="architecture-review__sample"><b>Office block</b><small>Street frontage · 8 floors</small></div>
      <div className="architecture-review__sample"><b>Mixed-use tower</b><small>Podium · 28 floors</small></div>
      <div className="architecture-review__sample"><b>Aobagawa station</b><small>Platform · canopy · rail</small></div>
      <div className="architecture-review__sample"><b>Material study · candidate</b><small>{materialCandidate==='concrete'?'Pilot concrete · 512 px':'Ivory plaster'} · 2 m tiles · 6 × 6 m · {materialStatus==='ready'?'loaded':materialStatus==='error'?'load failed':'loading'}</small></div>
    </section>
    {error&&<p className="architecture-review__error" role="alert">The architecture review could not start: {error}</p>}
  </main>;
}

function App(){
  if(import.meta.env.DEV&&new URLSearchParams(window.location.search).get('review')==='architecture')return <ArchitectureReviewRoute/>;
  return <CityExperience/>;
}

function CityExperience(){
  const mount=useRef<HTMLDivElement>(null), runtime=useRef<{renderer:CityRenderer,client:SimulationClient,audio:AudioDirector}|null>(null);
  const [world,setWorld]=useState<World|null>(null), [ready,setReady]=useState(false),[error,setError]=useState<string|null>(null),[message,setMessage]=useState<string|null>(null);
  const [preferences,setPreferences]=useState(loadViewerPreferences);
  const [paused,setPaused]=useState(false),[speed,setSpeed]=useState(1),[style,setStyle]=useState<Style>(preferences.style),[mode,setMode]=useState<'watch'|'explore'|'overview'>('watch');
  const [preparingStyle,setPreparingStyle]=useState<Style|null>(null);
  const [preview,setPreview]=useState<number|null>(null),[sound,setSound]=useState(false),[musicLevel,setMusicLevel]=useState(preferences.musicLevel),[cityLevel,setCityLevel]=useState(preferences.cityLevel),[focus,setFocus]=useState(preferences.focus),[development,setDevelopment]=useState(1),[quality,setQuality]=useState(preferences.quality),[contrast,setContrast]=useState(preferences.contrast),[lens,setLens]=useState<LensPresetId>(preferences.lens);
  const [subject,setSubject]=useState<Subject|null>(null);
  const [selectedId,setSelectedId]=useState<number|null>(null),[metrics,setMetrics]=useState<RenderMetrics|null>(null),[sceneLabel,setSceneLabel]=useState('Along the river');
  const [simulationMetrics,setSimulationMetrics]=useState<Metrics|null>(null);
  const preferencesRef=useRef(preferences);
  const styleSwitch=useRef(0);
  const startupOverridden=useRef(false);
  const currentWorld=useRef<World|null>(null),saveBusy=useRef(false),subjectRef=useRef<number|null>(null);subjectRef.current=subject?.id??null;
  function notice(text:string){setMessage(text);window.setTimeout(()=>setMessage(null),5000);}
  function storePreferences(patch:Partial<typeof preferences>){const next={...preferencesRef.current,...patch};preferencesRef.current=next;setPreferences(next);saveViewerPreferences(next);}
  useEffect(()=>{
    if(!mount.current)return;
    let renderer:CityRenderer;
    try{renderer=new CityRenderer(mount.current,new URLSearchParams(location.search).has('webgl'));}
    catch(e){setError(`Graphics could not start: ${e instanceof Error?e.message:String(e)}. Try the WebGL fallback.`);return;}
    const client=new SimulationClient(),audio=new AudioDirector();runtime.current={renderer,client,audio};
    const recoveryRequested=new URLSearchParams(location.search).get('recover')==='1';
    renderer.setLens(lens,false);
    renderer.setFocus(focus);renderer.setQuality(recoveryRequested?'balanced':quality);audio.setLevels(musicLevel,cityLevel);
    if(recoveryRequested)setQuality('balanced');
    let disposed=false,graphicsReady=false,simReady=false,latestMetrics:Metrics|null=null,audioEpoch=0,recoveryInitPending=false;
    const graphicsStartup=createGraphicsStartupWatchdog(setError);
    const isReady=()=>{if(graphicsReady&&simReady&&!disposed)setReady(true);};
    renderer.onPick=(id,kind)=>{if(kind==='building'){setSelectedId(id);setSubject(null);renderer.setSelected(id);}else{setSelectedId(null);renderer.setSelected(null);void client.inspect(id).then(data=>{if(!disposed)setSubject(inspectedSubject(id,data,currentWorld.current));}).catch(()=>{});}};
    renderer.director.onModeChange=setMode;renderer.onMetrics=setMetrics;
    renderer.onError=error=>{if(disposed)return;graphicsStartup.complete();graphicsReady=false;setReady(false);styleSwitch.current++;setPreparingStyle(null);setError(`Graphics stopped: ${error.message}. Try Retry or the WebGL fallback.`);};
    client.onError(e=>{if(!disposed)setError(e.message);});
    client.subscribeWorld(w=>{if(disposed)return;const replaced=audioEpoch!==client.worldEpoch;if(replaced){audioEpoch=client.worldEpoch;setDevelopment(w.development_rate);}currentWorld.current=w;renderer.setWorld(w,replaced);setWorld(w);simReady=true;if(recoveryInitPending){const url=new URL(window.location.href);url.searchParams.delete('recover');window.history.replaceState(null,'',`${url.pathname}${url.search}${url.hash}`);recoveryInitPending=false;}isReady();});
    client.subscribeMovement((data,tick,minute,day,trainDoors,support)=>{renderer.setMovement(data,tick,minute,trainDoors,support);if(currentWorld.current){currentWorld.current.tick=tick;currentWorld.current.minute_of_day=minute;currentWorld.current.day=day;}});
    client.subscribeMetrics(m=>{latestMetrics=m;if(!disposed)setSimulationMetrics(m);});
    renderer.start(style).then(()=>{if(disposed)return;graphicsStartup.complete();graphicsReady=true;isReady();}).catch(e=>{graphicsStartup.complete();if(!disposed){styleSwitch.current++;setPreparingStyle(null);setError(`Graphics could not start: ${e instanceof Error?e.message:String(e)}. Try the WebGL fallback.`);}});
    void resolveStartupWorld(location.search,87231,loadWorld).then(startup=>{
      if(disposed||startupOverridden.current)return;
      if(startup.kind==='recovered'){
        setQuality('balanced');
        recoveryInitPending=true;
        client.init(startup.seed,startup.data);
      }else client.init(startup.seed);
      client.controls({hidden:document.hidden});
    }).catch(e=>{if(!disposed&&!startupOverridden.current)setError(`Could not restore the saved city after graphics failure: ${e instanceof Error?e.message:String(e)}`);});
    let lastAudio=performance.now(),lastInspect=0;
    const pulse=window.setInterval(()=>{
      const now=performance.now();audio.update({...renderer.getAudioContext(),worldKey:audioEpoch,playbackRate:latestMetrics?.achievableSpeed??1},Math.min(.25,(now-lastAudio)/1000));lastAudio=now;setSceneLabel(renderer.director.label);if(currentWorld.current)setWorld({...currentWorld.current});
      const id=subjectRef.current;if(id!==null&&now-lastInspect>5000){lastInspect=now;void client.inspect(id).then(data=>{if(!disposed)setSubject(previous=>previous?.id===id?inspectedSubject(id,data,currentWorld.current):previous);}).catch(()=>{});}
    },250);
    const checkpoint=async()=>{if(!simReady||saveBusy.current)return;saveBusy.current=true;try{const data=await client.save();const seed=currentWorld.current?.seed;if(seed!==undefined)await saveWorld(data,seed);}catch(e){if(!disposed)setError(`Could not save: ${e instanceof Error?e.message:String(e)}`);}finally{saveBusy.current=false;}};
    const autosave=window.setInterval(()=>void checkpoint(),60000);
    const visibility=()=>{client.controls({hidden:document.hidden});audio.setHidden(document.hidden);if(document.hidden)void checkpoint();};
    document.addEventListener('visibilitychange',visibility);
    if(import.meta.env.DEV){Object.assign(window,{__city:{renderer,client,get world(){return currentWorld.current;},get simMetrics(){return latestMetrics;},save:()=>client.save(),export:async()=>exportWorld(await client.save())}});}
    return()=>{disposed=true;styleSwitch.current++;setPreparingStyle(null);graphicsStartup.cleanup();clearInterval(pulse);clearInterval(autosave);document.removeEventListener('visibilitychange',visibility);client.dispose();audio.dispose();renderer.dispose();runtime.current=null;};
  },[]);
  async function save(){try{const r=runtime.current;if(!r||!currentWorld.current)return;const data=await r.client.save();await saveWorld(data,currentWorld.current.seed);notice('Your city is saved on this device.');return true;}catch(e){setError(String(e));return false;}}
  async function recoverGraphics(useWebGL:boolean){
    const r=runtime.current,current=currentWorld.current;
    if(!r){
      // The renderer constructor can fail before a worker exists; in that
      // case there is no live engine state to checkpoint. Keep the seed and
      // any pending recovery marker while switching/retrying backends.
      const url=new URL(window.location.href);
      if(useWebGL)url.searchParams.set('webgl','1');else url.searchParams.delete('webgl');
      window.location.assign(url.toString());
      return;
    }
    if(!current){setError('Graphics recovery cannot checkpoint a city that has not finished opening. The latest saved city is unchanged.');return;}
    try{
      const data=await r.client.save();
      await saveWorld(data,current.seed);
      const url=new URL(window.location.href);url.searchParams.set('recover','1');
      if(useWebGL)url.searchParams.set('webgl','1');else url.searchParams.delete('webgl');
      window.location.assign(url.toString());
    }catch(e){setError(`Graphics recovery could not checkpoint this city. The latest saved city is unchanged. ${e instanceof Error?e.message:String(e)}`);}
  }
  async function exportCity(){try{const r=runtime.current;if(!r)return;exportWorld(await r.client.save());notice('Your city checkpoint is ready to keep.');}catch(e){setError(`Could not export: ${String(e)}`);}}
  function resetWorldSelection(r:{renderer:CityRenderer}){r.renderer.director.setMode('watch');r.renderer.setSelected(null);setMode('watch');setSelectedId(null);setSubject(null);}
  async function importCity(file:File){try{if(file.size>16*1024*1024)throw new Error('Checkpoint exceeds the 16 MiB limit.');const data=await file.text();const checkpoint=validateWorldSave(data);const r=runtime.current;if(!r)throw new Error('The city is not ready to import a checkpoint.');if(await save()!==true)return;resetWorldSelection(r);r.client.init(checkpoint.seed,checkpoint.data);notice('Your current city is saved while the imported checkpoint opens.');}catch(e){setError(`Could not import city: ${e instanceof Error?e.message:String(e)}`);}}
  async function reviewMature(){try{const response=await fetch('/validation/early-reservation-wasm-87231-save.json',{cache:'no-store'});if(!response.ok)throw new Error('The local review fixture is unavailable.');await importCity(new File([await response.text()],'early-reservation-wasm-87231-save.json',{type:'application/json'}));}catch(e){setError(`Could not open the review city: ${String(e)}`);}}
  async function resume(){try{const saved=await loadWorld();if(!saved){notice('There is no saved city on this device yet.');return;}const r=runtime.current;if(r){startupOverridden.current=true;resetWorldSelection(r);r.client.init(saved.seed,saved.data);}notice('Your city continues where you left it.');}catch(e){setError(String(e));}}
  async function newWorld(seed:number){try{if(currentWorld.current&&!(await save()))return;const r=runtime.current;if(!r)throw new Error('The graphics renderer is not available yet. Try the graphics recovery option.');startupOverridden.current=true;const url=new URL(window.location.href);url.searchParams.delete('recover');window.history.replaceState(null,'',`${url.pathname}${url.search}${url.hash}`);resetWorldSelection(r);setError(null);r.client.init(seed);notice('A new beginning, beside a different river.');}catch(e){setError(String(e));}}
  return <main className="city-app">
    <div className="city-canvas" ref={mount} aria-label="Living three-dimensional Tokyo city. Drag to explore; use arrow keys to pan and plus or minus to zoom." />
    <Controls achievableSpeed={simulationMetrics?.achievableSpeed} world={world} ready={ready} error={error} message={message} paused={paused} speed={speed} style={style} preparingStyle={preparingStyle} mode={mode} sceneLabel={sceneLabel} preview={preview} sound={sound} musicLevel={musicLevel} cityLevel={cityLevel} focus={focus} contrast={contrast} lens={lens} selected={world?.buildings.find(b=>b.id===selectedId)??null} metrics={metrics} subject={subject} development={development} quality={quality} onDismissError={()=>setError(null)}
      onDevelopment={value=>{setDevelopment(value);runtime.current?.client.controls({development:value});}}
      onQuality={value=>{setQuality(value);storePreferences({quality:value});runtime.current?.renderer.setQuality(value);}}
      onPause={()=>{const value=!paused;setPaused(value);runtime.current?.client.controls({paused:value});runtime.current?.audio.setPaused(value);}}
      onSpeed={value=>{setSpeed(value);runtime.current?.client.controls({speed:value});}}
      onStyle={value=>{const request=++styleSwitch.current;const r=runtime.current;if(!r){setStyle(value);setPreparingStyle(null);storePreferences({style:value});return;}setPreparingStyle(value);void Promise.resolve().then(()=>r.renderer.setStyle(value)).then(()=>{if(request!==styleSwitch.current)return;setStyle(r.renderer.style);setPreparingStyle(null);storePreferences({style:r.renderer.style});}).catch(e=>{if(request!==styleSwitch.current)return;setPreparingStyle(null);setStyle(r.renderer.style);setError(`Could not prepare this rendering style: ${String(e)}`);});}}
      onMode={value=>{setMode(value);runtime.current?.renderer.director.setMode(value);}}
      onPreview={value=>{setPreview(value);runtime.current?.renderer.setPreview(value);}}
      onSound={()=>{const r=runtime.current;if(!r)return;if(!sound){r.audio.enable().then(()=>{r.audio.setLevels(musicLevel,cityLevel);setSound(true);}).catch(e=>setError(String(e)));}else{r.audio.setLevels(0,0);setSound(false);}}}
      onLevels={(m,c)=>{setMusicLevel(m);setCityLevel(c);storePreferences({musicLevel:m,cityLevel:c});if(sound)runtime.current?.audio.setLevels(m,c);}}
      onFocus={()=>{const value=!focus;setFocus(value);storePreferences({focus:value});runtime.current?.renderer.setFocus(value);}}
      onContrast={value=>{setContrast(value);storePreferences({contrast:value});}}
      onLens={value=>{setLens(value);storePreferences({lens:value});runtime.current?.renderer.setLens(value,true);}}
      onGraphicsRetry={()=>void recoverGraphics(new URLSearchParams(window.location.search).has('webgl'))} onGraphicsFallback={()=>void recoverGraphics(!new URLSearchParams(window.location.search).has('webgl'))}
      onReviewMature={import.meta.env.DEV?()=>void reviewMature():undefined} onSave={()=>void save()} onExport={()=>void exportCity()} onImport={file=>void importCity(file)} onResume={()=>void resume()} onNewWorld={seed=>void newWorld(seed)}
      onReviewSkyline={import.meta.env.DEV?()=>{if(!runtime.current?.renderer.director.reviewSkyline())notice('This city has no completed tall-building scene yet.');}:undefined}
      onReviewBuilding={import.meta.env.DEV?id=>{if(!runtime.current?.renderer.director.reviewBuilding(id))notice('This building is not available to frame.');}:undefined}
      onReviewStation={import.meta.env.DEV?()=>{const station=currentWorld.current?.nodes.find(n=>n.kind==='station');if(!station||!runtime.current?.renderer.director.reviewStation(station.id))notice('This station is not available to frame.');}:undefined}
      onReviewWaterReflection={import.meta.env.DEV?()=>runtime.current?.renderer.toggleWaterReflectionForReview()??false:undefined}
      onReviewStreetLights={import.meta.env.DEV?()=>runtime.current?.renderer.toggleStreetLightsForReview()??false:undefined}
      onReviewWaterfront={import.meta.env.DEV?(preset='overhead')=>{if(!runtime.current?.renderer.director.reviewWaterfront(preset))notice('This city has no river view yet.');}:undefined}
      onCloseInspector={()=>{setSelectedId(null);setSubject(null);runtime.current?.renderer.setSelected(null);runtime.current?.renderer.director.follow(null);}}
      onFollow={id=>{const r=runtime.current;if(!r)return;r.renderer.director.follow(id);if(!currentWorld.current?.buildings.some(b=>b.id===id)){void r.client.inspect(id).then(data=>{setSubject(inspectedSubject(id,data,currentWorld.current));setSelectedId(null);}).catch(()=>{});}}} />
  </main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
