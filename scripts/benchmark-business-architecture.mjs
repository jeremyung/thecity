import { build } from 'esbuild';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
const folder = new URL('../.tools/business-benchmark/', import.meta.url);
await mkdir(folder, { recursive: true });
await build({
  stdin: {
    contents: `export {ApartmentResources} from './web/src/render/architecture/resources';export {buildBusinessBuilding} from './web/src/render/architecture/business';export {businessCityStudy} from './web/src/render/architecture/business-study';export {selectBusinessRecipe,resolveBusinessRecipes} from './web/src/render/architecture/business-recipes';export {disposeGroup} from './web/src/render/kit';`,
    resolveDir: process.cwd(),
    sourcefile: 'benchmark.ts',
  },
  outfile: new URL('kit.mjs', folder).pathname,
  bundle: true,
  platform: 'node',
  format: 'esm',
  packages: 'external',
});
globalThis.document = {
  createElement: () => ({
    width: 0,
    height: 0,
    getContext: () =>
      new Proxy(
        {},
        {
          get: (t, k) => t[k] ?? (() => {}),
          set: (t, k, v) => {
            t[k] = v;
            return true;
          },
        },
      ),
  }),
};
const {
  ApartmentResources,
  buildBusinessBuilding,
  businessCityStudy,
  selectBusinessRecipe,
  resolveBusinessRecipes,
  disposeGroup,
} = await import(pathToFileURL(new URL('kit.mjs', folder).pathname));
const owner = new ApartmentResources(),
  report = [];
try {
  for (const family of ['office', 'commercial', 'mixed-use', 'skyscrapers'])
    for (const detail of ['close', 'district', 'far']) {
      const specimens = businessCityStudy(32, family).buildings,
        timings = [],
        triangles = [];
      for (let repeat = 0; repeat < 4; repeat++)
        for (const b of specimens) {
          const recipe = selectBusinessRecipe(b),
            start = performance.now(),
            g = buildBusinessBuilding(b, owner.art, {
              recipe,
              detail,
              signage: { proximity: 2, feature: 'screen' },
            });
          timings.push(performance.now() - start);
          let count = 0;
          g.traverse((o) => {
            if (o.isMesh)
              count += (o.geometry.index?.count ?? o.geometry.getAttribute('position').count) / 3;
          });
          triangles.push(count);
          disposeGroup(g);
        }
      timings.sort((a, b) => a - b);
      report.push({
        family,
        detail,
        samples: timings.length,
        p50BuildMs: timings[Math.floor(timings.length * 0.5)],
        p95BuildMs: timings[Math.floor(timings.length * 0.95)],
        meanTriangles: triangles.reduce((a, b) => a + b) / triangles.length,
        maxTriangles: Math.max(...triangles),
      });
    }
} finally {
  owner.dispose();
}
let census;
try {
  const wasm = await import(new URL('../public/wasm/city_engine.js', import.meta.url));
  await wasm.default({
    module_or_path: await readFile(new URL('../public/wasm/city_engine_bg.wasm', import.meta.url)),
  });
  const engine = wasm.CityEngine.restore(
    await readFile(
      new URL('../tests/fixtures/access-joins/mature-world.json', import.meta.url),
      'utf8',
    ),
  );
  try {
    const world = JSON.parse(engine.snapshot()),
      retained = new Set(world.building_original_footprints?.map((b) => b.building_id)),
      choices = resolveBusinessRecipes(world.buildings, 'mixed', retained);
    census = {
      totalBuildings: world.buildings.length,
      businessBuildings: choices.size,
      recipes: {},
    };
    for (const r of choices.values()) census.recipes[r.id] = (census.recipes[r.id] ?? 0) + 1;
  } finally {
    engine.free();
  }
} catch (error) {
  census = { unavailable: String(error) };
}
const out = process.argv[2] ?? '.tools/business-benchmark/results.json';
const result = {
  method:
    'Node CPU geometry only; four passes across the complete recipe catalog per family/LOD (ten general recipes or five corporate skyscrapers). Shared materials; not a GPU or cross-device benchmark.',
  report,
  census,
};
await writeFile(out, JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify(result, null, 2));
