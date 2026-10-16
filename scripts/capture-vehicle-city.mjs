// Capture road traffic in the real app around its busiest street.
//
// Usage: node scripts/capture-vehicle-city.mjs <save.json> <out-dir> [port]
// Requires `npx vite --host 127.0.0.1 --port <port>` running. Software WebGL:
// visual evidence only, not frame-rate evidence.
import { chromium } from '@playwright/test';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

const [savePath, outDir, port = '5173'] = process.argv.slice(2);
if (!savePath || !outDir) {
  console.error('Usage: node scripts/capture-vehicle-city.mjs <save.json> <out-dir> [port]');
  process.exit(1);
}
const saved = await readFile(savePath, 'utf8');
const seed = JSON.parse(saved).seed;
await mkdir(outDir, { recursive: true });
const cached = join(homedir(), '.cache/ms-playwright/chromium-1148/chrome-linux/chrome');
const browser = await chromium.launch({
  headless: true,
  executablePath: existsSync(cached) ? cached : undefined,
  args: process.env.CAPTURE_GPU
    ? ['--headless=new', '--use-gl=angle', '--use-angle=gl-egl', '--ignore-gpu-blocklist']
    : ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
try {
  const page = await browser.newPage({
    viewport: { width: 1400, height: 860 },
    deviceScaleFactor: 1,
  });
  page.on('pageerror', (error) => console.error('pageerror', error.message));
  await page.goto(`http://127.0.0.1:${port}/?webgl=1`, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => document.querySelector('canvas')?.dataset.graphicsPhase === 'first-frame-submitted',
    null,
    { timeout: 180000 },
  );
  const expected = JSON.parse(saved).buildings.length;
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
    expected,
    { timeout: 120000 },
  );
  await page.evaluate(() => {
    const r = window.__city.renderer,
      pipeline = r.pipeline;
    r.setFocus(false);
    r.setQuality('balanced');
    r.director.setMode('explore');
    r.pipeline = null;
    let steps = 0;
    while ((r.objectQueue.size || r.chunks.pendingCount) && steps < 40000) {
      r.animate();
      steps++;
    }
    r.pipeline = pipeline;
  });
  // Let traffic build at morning rush, then freeze.
  await page.evaluate(() => {
    const c = window.__city;
    c.client.controls({ paused: false, speed: 4 });
  });
  await page.waitForTimeout(25000);
  await page.evaluate(() => window.__city.client.controls({ paused: true }));
  await page.waitForTimeout(500);
  const traffic = await page.evaluate(() => {
    const r = window.__city.renderer,
      kinds = {},
      cells = new Map();
    for (const [, p] of r.current) {
      if (![1, 4, 5, 6, 7, 8].includes(p.k)) continue;
      kinds[p.k] = (kinds[p.k] ?? 0) + 1;
      const key = `${Math.round(p.x / 45)}:${Math.round(p.z / 45)}`;
      const cell = cells.get(key) ?? { n: 0, x: 0, z: 0 };
      cell.n++;
      cell.x += p.x;
      cell.z += p.z;
      cells.set(key, cell);
    }
    const best = [...cells.values()].sort((a, b) => b.n - a.n)[0];
    r.animate();
    const drawn = r.fleet.meshes.map((m) => m.count);
    return {
      kinds,
      drawn,
      busiest: best ? { n: best.n, x: best.x / best.n, z: best.z / best.n } : null,
    };
  });
  console.log(JSON.stringify(traffic));
  if (!traffic.busiest) throw new Error('no road traffic to capture');
  const { x, z } = traffic.busiest;
  // Close-ups render the scene directly: explore mode holds the camera 45 m
  // out and the director would move it between frames.
  const subject = await page.evaluate(() => {
    const r = window.__city.renderer;
    for (const [id, p] of r.current) if ([1, 4, 5, 6, 7, 8].includes(p.k)) return id;
    return null;
  });
  const views = [
    { name: 'close-anime-day', style: 'anime', hour: 10 },
    { name: 'close-miniature-day', style: 'miniature', hour: 10 },
    { name: 'close-miniature-night', style: 'miniature', hour: 21.5 },
  ];
  for (const v of views) {
    await page.evaluate(
      async ([v, id]) => {
        const r = window.__city.renderer,
          p = r.current.get(id);
        await r.setStyle(v.style);
        r.setPreview(v.hour);
        for (let i = 0; i < 60; i++) r.animate();
        r.camera.position.set(p.x + 6, 7, p.z + 6);
        r.camera.lookAt(p.x, 0.8, p.z);
        r.camera.updateMatrixWorld();
        r.renderer.render(r.scene, r.camera);
      },
      [v, subject],
    );
    await page.screenshot({ path: join(outDir, `${v.name}.png`) });
    console.log('captured', v.name);
  }
  await writeFile(
    join(outDir, 'capture.json'),
    `${JSON.stringify({ source: savePath, backend: 'software WebGL (swiftshader)', performanceEvidence: false, traffic, subject, views }, null, 2)}\n`,
  );
} finally {
  await browser.close();
}
