// Descriptive architecture mix, not projected facade coverage or art acceptance.
// node scripts/audit-station-architecture.mjs OUTPUT SNAPSHOT [SNAPSHOT...]
import { readFile, writeFile, mkdtemp, mkdir, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, resolve, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { build } from 'esbuild';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const [output, ...inputs] = process.argv.slice(2);
if (!output || !inputs.length) throw new Error('Expected OUTPUT SNAPSHOT [SNAPSHOT...]');
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
await mkdir(resolve(root, '.tools'), { recursive: true });
const temporary = await mkdtemp(resolve(root, '.tools/architecture-audit-'));
try {
  const runtime = resolve(temporary, 'runtime.mjs');
  await build({
    stdin: { contents: `
      export {partyWallBodyWidth,partyWallBodyDepth} from './web/src/render/kit';
      export {midriseMassing} from './web/src/render/building-massing';
      export {lowriseMassing,lowriseBuildingWidth} from './web/src/render/lowrise-massing';
      export {buildingPalette} from './web/src/render/building-palette';`, resolveDir: root },
    bundle: true, platform: 'node', format: 'esm', packages: 'external', outfile: runtime,
  });
  const kit = await import(pathToFileURL(runtime).href);
  const counts = (items, key) => items.reduce((out, item) => {
    const value = key(item); out[value] = (out[value] ?? 0) + 1; return out;
  }, {});
  const family = b => {
    const d = kit.partyWallBodyDepth(b.depth);
    const w = kit.lowriseBuildingWidth(b, kit.partyWallBodyWidth(b.width, b.anchor, b.floors, b.kind), d);
    if (b.floors >= 12 && ['apartment', 'office'].includes(b.kind)) return `tower-${b.kind}`;
    const mid = kit.midriseMassing(b, w, d);
    if (mid) return mid.family;
    const low = kit.lowriseMassing(b, w, d);
    if (low) return low.family;
    return `other-${b.kind}`;
  };
  const rows = [];
  for (const input of inputs) {
    const bytes = await readFile(resolve(input));
    const world = JSON.parse(bytes);
    const station = world.nodes.find(n => n.kind === 'station');
    if (!station) throw new Error(`No station in ${input}`);
    const crops = [250, 150].map(size => {
      const x0 = Math.max(world.bounds.min_x, Math.min(world.bounds.max_x - size, station.x - size / 2));
      const z0 = Math.max(world.bounds.min_z, Math.min(world.bounds.max_z - size, station.z - size / 2));
      const buildings = world.buildings.filter(b => b.stage === 'open' && b.x >= x0 && b.x < x0 + size && b.z >= z0 && b.z < z0 + size);
      const access = new Map(world.building_access.map(a => [a.building_id, a]));
      return {
        sizeM: size, bounds: [x0, z0, x0 + size, z0 + size], count: buildings.length,
        kinds: counts(buildings, b => b.kind), floors: counts(buildings, b => b.floors),
        massingFamilies: counts(buildings, family),
        paletteFamilies: counts(buildings, b => kit.buildingPalette(b, ['#ffffff']).family),
        accessRecords: buildings.filter(b => access.has(b.id)).length,
        activeShopCafeCount: buildings.filter(b => ['shop', 'cafe'].includes(b.kind) && b.business_active !== false).length,
        buildings: buildings.map(b => ({ id: b.id, x: b.x, z: b.z, kind: b.kind, floors: b.floors, family: family(b), accessRoad: access.get(b.id)?.road_id ?? null })),
      };
    });
    rows.push({ source: relative(root, resolve(input)), sha256: digest(bytes), seed: world.seed, tick: world.tick, stage: world.stage, station: station.id, crops });
  }
  const report = {
    method: 'Open building centers in station-centered, world-clamped squares. Families use current production massing functions and kit envelope helpers. Palette family only; dummy neutral swatch does not affect family selection.',
    limitations: ['Counts are not screen-space proportions, facade area or frontage continuity.', 'An access record does not establish visible or unobstructed access.', 'Shop/cafe activity is not a measurement of pedestrian footfall.', 'Seed5 at tick80000 remains developing; construction is excluded.', 'Tower and fallback categories are coarse; no new density or diversity targets are inferred.'],
    runtimeSha256: digest(await readFile(runtime)), rows,
  };
  await writeFile(resolve(output), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(rows.map(row => ({ seed: row.seed, crops: row.crops.map(({ buildings, ...summary }) => summary) })), null, 2));
} finally { await rm(temporary, { recursive: true, force: true }); }
