import fs from 'node:fs/promises';
import { parseEnv } from 'node:util';
const root = new URL('../../', import.meta.url);
const config = JSON.parse(
  await fs.readFile(new URL('art-workshop/texture-pilot.json', root), 'utf8'),
);
const out = new URL('art-workshop/tencent-pilot/', root);
await fs.mkdir(out, { recursive: true });
const ledgerPath = new URL('ledger.json', out);
let ledger;
try {
  ledger = JSON.parse(await fs.readFile(ledgerPath, 'utf8'));
} catch (e) {
  if (e.code !== 'ENOENT') throw e;
  ledger = { model: config.proposedModel, capUSD: 2, requests: [] };
}
const save = () => fs.writeFile(ledgerPath, JSON.stringify(ledger, null, 2) + '\n');
const env = parseEnv(await fs.readFile(new URL('.env.local', root), 'utf8'));
const key = env.OPENROUTER_API_KEY;
if (!key || config.status !== 'approved') throw new Error('Missing credential or pilot approval');
const endpoint = await fetch(
  `https://openrouter.ai/api/v1/images/models/${config.proposedModel}/endpoints`,
);
if (!endpoint.ok) throw new Error(`Public pricing HTTP ${endpoint.status}`);
const pricing = await endpoint.json();
const provider = pricing.endpoints.find((e) => e.provider_tag === 'tencent');
if (
  provider?.pricing.length !== 1 ||
  provider.pricing[0].billable !== 'output_image' ||
  provider.pricing[0].unit !== 'token' ||
  provider.pricing[0].cost_usd > 0.0000016
)
  throw new Error('Pricing changed; stopped before generation');
await fs.writeFile(new URL('endpoint-pricing.json', out), JSON.stringify(pricing, null, 2));
for (const candidate of config.candidates.slice(0, 4)) {
  if (ledger.requests.some((r) => r.id === candidate.id)) continue;
  if (ledger.requests.some((r) => r.status !== 'complete') || ledger.requests.length >= 4)
    throw new Error('Unresolved request or request limit; no retry');
  const spent = ledger.requests.reduce((s, r) => s + r.costUSD, 0);
  if (spent + 0.024 > 2) throw new Error('Pilot spend cap');
  const prompt = config.commonPrompt + ' ' + candidate.prompt;
  const entry = {
    id: candidate.id,
    status: 'submitted',
    submittedAt: new Date().toISOString(),
    prompt,
    expectedCostUSD: 0.024,
  };
  ledger.requests.push(entry);
  await save();
  console.log(`Generating ${candidate.id} (1K, expected $0.024)`);
  let response;
  try {
    response = await fetch('https://openrouter.ai/api/v1/images', {
      method: 'POST',
      headers: { Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: config.proposedModel,
        prompt,
        resolution: '1K',
        aspect_ratio: '1:1',
        n: 1,
        provider: { only: ['tencent'], allow_fallbacks: false },
      }),
    });
  } catch {
    throw new Error('Network outcome unknown; no automatic retry');
  }
  if (!response.ok) {
    entry.status = 'http-error';
    entry.httpStatus = response.status;
    await save();
    throw new Error(`Generation HTTP ${response.status}; no retry`);
  }
  const result = await response.json();
  const item = result.data?.[0];
  const ext = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/webp': 'webp' }[item?.media_type];
  if (!ext || !item.b64_json) throw new Error('Unexpected image response; no retry');
  await fs.writeFile(new URL(`${candidate.id}.${ext}`, out), Buffer.from(item.b64_json, 'base64'));
  entry.file = `${candidate.id}.${ext}`;
  entry.usage = result.usage;
  const cost = result.usage?.cost;
  if (typeof cost !== 'number' || !Number.isFinite(cost) || cost < 0) {
    entry.status = 'cost-unverified';
    await save();
    throw new Error('Image saved but cost unverified; stopped');
  }
  entry.costUSD = cost;
  entry.status = 'complete';
  await save();
  console.log(`Saved ${entry.file}; reported cost $${cost}`);
  if (cost > 0.0241) throw new Error('Reported cost exceeds estimate; stopped');
}
console.log(
  `Pilot complete: ${ledger.requests.length} candidates, $${ledger.requests.reduce((s, r) => s + (r.costUSD ?? 0), 0).toFixed(4)} reported total`,
);
