// Hosted half of the vehicle texture pilot. One request per (model, vehicle),
// no retries, ledger written before each request so an unknown outcome is
// never silently repeated. Local Hunyuan runs separately via ComfyUI.
import fs from 'node:fs/promises';
import { parseEnv } from 'node:util';
const root = new URL('../', import.meta.url);
const config = JSON.parse(await fs.readFile(new URL(process.env.VEHICLE_CONFIG ?? 'art-workshop/vehicle-pilot.json', root), 'utf8'));
const out = new URL('art-workshop/vehicle-pilot/', root);
await fs.mkdir(out, {recursive:true});
const ledgerPath = new URL('ledger.json', out);
let ledger;
try { ledger = JSON.parse(await fs.readFile(ledgerPath, 'utf8')); } catch(e) { if(e.code !== 'ENOENT') throw e; ledger = {capUSD:config.capUSD, requests:[]}; }
const save = () => fs.writeFile(ledgerPath, JSON.stringify(ledger,null,2)+'\n');
const envPath = process.env.ENV_FILE ? new URL(`file://${process.env.ENV_FILE}`) : new URL('.env.local', root);
const key = parseEnv(await fs.readFile(envPath,'utf8')).OPENROUTER_API_KEY;
if (!key || config.status !== 'approved') throw new Error('Missing credential or pilot approval');
const only = process.argv[2];
const spent = () => ledger.requests.reduce((s,r)=>s+(r.costUSD??r.estimateUSD),0);
for (const model of config.hostedModels.filter(m=>!only||m.id===only)) {
  for (const vehicle of config.vehicles) {
    const id = `${model.id.split('/')[1]}--${vehicle.id}`;
    if (ledger.requests.some(r=>r.id===id)) continue;
    if (ledger.requests.some(r=>r.status!=='complete'&&!r.status.startsWith('skipped'))) throw new Error('Unresolved request; no retry');
    if (spent()+model.estimateUSD>config.capUSD) throw new Error('Spend cap');
    const prompt = `${config.commonPrompt} ${vehicle.prompt}`;
    const entry = {id,model:model.id,vehicle:vehicle.id,status:'submitted',submittedAt:new Date().toISOString(),estimateUSD:model.estimateUSD};
    ledger.requests.push(entry); await save();
    const started = Date.now();
    let response;
    try {
      response = await fetch('https://openrouter.ai/api/v1/images',{method:'POST',headers:{Authorization:`Bearer ${key}`,'Content-Type':'application/json'},
        body:JSON.stringify({model:model.id,prompt,aspect_ratio:config.aspectRatio,n:1,...model.request,provider:{only:[model.provider],allow_fallbacks:false}})});
    } catch { throw new Error('Network outcome unknown; no automatic retry'); }
    if (!response.ok) { entry.status='http-error'; entry.httpStatus=response.status; entry.body=(await response.text()).slice(0,400); await save(); console.log(`${id}: HTTP ${response.status}`); continue; }
    const result = await response.json();
    const item = result.data?.[0];
    const ext = {'image/png':'png','image/jpeg':'jpg','image/webp':'webp'}[item?.media_type] ?? 'png';
    if (!item?.b64_json) { entry.status='bad-response'; await save(); throw new Error('Unexpected image response; no retry'); }
    entry.file = `${id}.${ext}`; entry.seconds = (Date.now()-started)/1000; entry.usage = result.usage;
    await fs.writeFile(new URL(entry.file,out), Buffer.from(item.b64_json,'base64'));
    const cost = result.usage?.cost;
    if (typeof cost!=='number'||!Number.isFinite(cost)||cost<0) { entry.status='cost-unverified'; await save(); throw new Error('Image saved but cost unverified; stopped'); }
    entry.costUSD = cost; entry.status = 'complete'; await save();
    console.log(`${entry.file} $${cost} ${entry.seconds}s`);
    if (cost>model.estimateUSD*1.5) throw new Error('Reported cost exceeds estimate; stopped');
  }
}
console.log(`Total reported $${spent().toFixed(4)}`);
