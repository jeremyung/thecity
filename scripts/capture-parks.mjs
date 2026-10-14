// Capture pocket parks, arterial street trees and the shrine grove in the real app (software WebGL).
//
// Usage: node scripts/capture-parks.mjs <save.json> <out-dir> [preview hour]
// Requires `npx vite --host 127.0.0.1 --port 5173` running. As in
// capture-masterplan-city.mjs, geometry queues are drained with the post
// pipeline detached; these images are visual evidence, not frame-rate evidence.
import { chromium } from '@playwright/test';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const [savePath, outDir, hourArg] = process.argv.slice(2);
if (!savePath || !outDir) {
  console.error('Usage: node scripts/capture-parks.mjs <save.json> <out-dir> [preview hour]');
  process.exit(1);
}
const hour = Number(hourArg ?? 15);
const saved = await readFile(savePath, 'utf8');
const expected = JSON.parse(saved);
await mkdir(outDir, { recursive: true });

// CHROMIUM_PATH selects a preinstalled browser when Playwright's pinned build is absent.
const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROMIUM_PATH || undefined, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 }, deviceScaleFactor: 1 });
  page.on('pageerror', (error) => console.error(error.message));
  page.on('console', (message) => { if (message.type() === 'error') console.error(message.text()); });
  await page.goto('http://127.0.0.1:5173/?webgl=1', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => document.querySelector('canvas')?.dataset.graphicsPhase === 'first-frame-submitted', null, { timeout: 120000 });
  await page.evaluate(([s, save]) => {
    const c = window.__city;
    c.renderer.renderer.setAnimationLoop(null);
    c.client.controls({ paused: true });
    c.client.init(s, save);
  }, [expected.seed, saved]);
  await page.waitForFunction((count) => window.__city?.world?.buildings.length === count, expected.buildings.length, { timeout: 60000 });

  const scene = await page.evaluate((previewHour) => {
    const r = window.__city.renderer;
    r.renderer.setAnimationLoop(null);
    r.director.setMode('explore');
    r.setPreview(previewHour);
    r.setFocus(false);
    r.setQuality('balanced');
    const pipeline = r.pipeline;
    r.pipeline = null;
    let steps = 0;
    while ((r.objectQueue.size || r.chunks.pendingCount) && steps < 40000) { r.animate(); steps++; }
    r.pipeline = pipeline;
    const w = r.worldState;
    const built = new Set(w.buildings.map((b) => b.parcel_id));
    const parks = w.parcels.filter((p) => p.protected && !built.has(p.id)).sort((a, b) => Math.hypot(a.x, a.z) - Math.hypot(b.x, b.z));
    const nodes = new Map(w.nodes.map((n) => [n.id, n]));
    const arterial = w.roads.filter((road) => road.kind === 'arterial' && road.stage === 'open' && road.connected)
      .map((road) => ({ road, a: nodes.get(road.from), b: nodes.get(road.to) }))
      .filter(({ a, b }) => a && b)
      .sort((x, y) => Math.hypot(y.b.x - y.a.x, y.b.z - y.a.z) - Math.hypot(x.b.x - x.a.x, x.b.z - x.a.z))[0];
    const layer = r.parks;
    const instanced = []; layer.traverse((o) => { if (o.isInstancedMesh) instanced.push(o.count); });
    const diagnostics = { parkMeshes: layer.children.length, treeInstances: instanced.reduce((a, b) => a + b, 0), batches: instanced.length };
    // Explore mode keeps manual poses clear of buildings; captures pose the camera directly.
    r.director.update = () => {};
    return { diagnostics, tick: w.tick, steps, parks: parks.map((p) => ({ id: p.id, x: p.x, z: p.z, rotation: p.rotation ?? 0 })), arterial: arterial && { a: arterial.a, b: arterial.b }, reserve: w.terrain.protected[0] };
  }, hour);

  const views = [];
  for (const park of scene.parks.slice(0, 3)) {
    // Look into the park from across its frontage street, slightly elevated.
    const fx = Math.sin(park.rotation), fz = Math.cos(park.rotation);
    views.push({ name: `park-${park.id}`, target: [park.x, 1.5, park.z], position: [park.x + fx * 30 + fz * 12, 22, park.z + fz * 30 - fx * 12] });
  }
  if (scene.arterial) {
    const { a, b } = scene.arterial, mx = (a.x + b.x) / 2, mz = (a.z + b.z) / 2, len = Math.hypot(b.x - a.x, b.z - a.z), ux = (b.x - a.x) / len, uz = (b.z - a.z) / len;
    views.push({ name: 'arterial-street-trees', target: [mx, 3, mz], position: [mx - ux * 45 - uz * 18, 16, mz - uz * 45 + ux * 18] });
  }
  if (scene.reserve?.length) {
    const xs = scene.reserve.map((p) => p.x), zs = scene.reserve.map((p) => p.z);
    const cx = (Math.min(...xs) + Math.max(...xs)) / 2, cz = (Math.min(...zs) + Math.max(...zs)) / 2;
    views.push({ name: 'shrine-grove', target: [cx, 4, cz], position: [cx + 40, 55, cz + 95] });
  }
  views.push({ name: 'overview', target: [0, 0, 0], position: [380, 420, -470] });

  const captured = [];
  for (const view of views) {
    await page.evaluate((v) => {
      const r = window.__city.renderer;
      r.director.target.set(...v.target);
      r.camera.position.set(...v.position);
      r.camera.lookAt(r.director.target);
      r.animate();
    }, view);
    const file = join(outDir, `${view.name}.png`);
    await page.screenshot({ path: file, timeout: 300000 });
    captured.push({ ...view, file });
  }
  await writeFile(join(outDir, 'capture.json'), `${JSON.stringify({ source: savePath, backend: 'software WebGL (swiftshader)', performanceEvidence: false, previewHour: hour, depthOfField: false, scene, views: captured }, null, 2)}\n`);
  console.log(JSON.stringify({ diagnostics: scene.diagnostics, tick: scene.tick, steps: scene.steps, parks: scene.parks.length, views: captured.map((v) => v.name) }));
} finally {
  await browser.close();
}
