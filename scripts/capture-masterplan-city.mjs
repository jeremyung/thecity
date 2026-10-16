// Capture a mature master-plan city in the real app (software WebGL).
//
// Usage: node scripts/capture-masterplan-city.mjs <save.json> <out-dir> [seed]
// Requires `npx vite --host 127.0.0.1 --port 5173` running. Geometry queues are
// drained manually with the post pipeline detached, as in the rear-extension
// captures; these images are visual evidence, not frame-rate evidence.
import { chromium } from '@playwright/test';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

const [savePath, outDir, seedArg] = process.argv.slice(2);
if (!savePath || !outDir) {
  console.error('Usage: node scripts/capture-masterplan-city.mjs <save.json> <out-dir> [seed]');
  process.exit(1);
}
const seed = Number(seedArg ?? JSON.parse(await readFile(savePath, 'utf8')).seed);
const saved = await readFile(savePath, 'utf8');
await mkdir(outDir, { recursive: true });

const cached = join(homedir(), '.cache/ms-playwright/chromium-1148/chrome-linux/chrome');
const browser = await chromium.launch({
  headless: true,
  executablePath: existsSync(cached) ? cached : undefined,
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
try {
  const page = await browser.newPage({
    viewport: { width: 1400, height: 900 },
    deviceScaleFactor: 1,
  });
  page.on('pageerror', (error) => console.error(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error' || message.type() === 'warning') console.error(message.text());
  });
  await page.goto('http://127.0.0.1:5173/?webgl=1', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => document.querySelector('canvas')?.dataset.graphicsPhase === 'first-frame-submitted',
    null,
    { timeout: 120000 },
  );
  const expected = JSON.parse(saved);
  await page.evaluate(
    ([s, save]) => {
      const c = window.__city;
      c.renderer.renderer.setAnimationLoop(null);
      c.client.controls({ paused: true });
      c.client.init(s, save);
    },
    [seed, saved],
  );
  await page.waitForFunction(
    (count) => window.__city?.world?.buildings.length === count,
    expected.buildings.length,
    { timeout: 60000 },
  );

  const world = await page.evaluate(() => {
    const r = window.__city.renderer;
    r.renderer.setAnimationLoop(null);
    r.director.setMode('explore');
    r.setPreview(15);
    r.setFocus(false);
    r.setQuality('balanced');
    const pipeline = r.pipeline;
    r.pipeline = null;
    let steps = 0;
    while ((r.objectQueue.size || r.chunks.pendingCount) && steps < 40000) {
      r.animate();
      steps++;
    }
    r.pipeline = pipeline;
    const w = r.worldState;
    const station = w.nodes.find((n) => n.kind === 'station');
    const market = w.nodes.find((n) => n.kind === 'market');
    return {
      tick: w.tick,
      buildings: w.buildings.length,
      steps,
      station,
      market,
      queued: r.objectQueue.size,
      chunks: r.chunks.pendingCount,
    };
  });

  const views = [
    { name: 'overview', target: [0, 0, 0], position: [380, 420, -470] },
    {
      name: 'neighborhood',
      target: [world.market.x + 120, 4, world.market.z + 60],
      position: [world.market.x + 300, 150, world.market.z - 120],
    },
    {
      name: 'station',
      target: [world.station.x, 8, world.station.z],
      position: [world.station.x + 140, 95, world.station.z - 150],
    },
    {
      name: 'street',
      target: [world.market.x + 70, 6, world.market.z + 40],
      position: [world.market.x + 115, 28, world.market.z - 10],
    },
  ];
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
    await page.screenshot({ path: file });
    captured.push({ ...view, file });
  }
  await writeFile(
    join(outDir, 'capture.json'),
    `${JSON.stringify({ source: savePath, backend: 'software WebGL (swiftshader)', method: 'post pipeline detached while draining geometry queues; restored for captures', performanceEvidence: false, previewHour: 15, depthOfField: false, world, views: captured }, null, 2)}\n`,
  );
  console.log(JSON.stringify(world));
} finally {
  await browser.close();
}
