// Frame cost of building cladding and interior-mapped windows in the mature
// city, on the real GPU through ANGLE/OpenGL ES (WebGL backend; headless
// Chromium exposes no WebGPU adapter here).
//
// Usage: node scripts/benchmark/benchmark-building-surfaces.mjs <save.json> [port] [label]
// Requires `npx vite --host 127.0.0.1 --port <port>` running. To compare with
// the previous surfaces, serve a checkout of the base branch on another port
// and run this script against both. Each sample renders the scene pass
// (shadows included) and calls gl.finish so the timing includes GPU
// completion; the post pipeline's render returns without drawing in headless
// Chromium, so it is left out. Use the difference between runs, not the
// absolute.
import { chromium } from '@playwright/test';
import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

const [savePath, port = '5173', label = 'current'] = process.argv.slice(2);
if (!savePath) {
  console.error(
    'Usage: node scripts/benchmark/benchmark-building-surfaces.mjs <save.json> [port] [label]',
  );
  process.exit(1);
}
const saved = await readFile(savePath, 'utf8');
const expected = JSON.parse(saved);
const cached = join(homedir(), '.cache/ms-playwright/chromium-1148/chrome-linux/chrome');
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.CHROMIUM_PATH || (existsSync(cached) ? cached : undefined),
  args: ['--headless=new', '--use-gl=angle', '--use-angle=gl-egl', '--ignore-gpu-blocklist'],
});
try {
  const page = await browser.newPage({
    viewport: { width: 1600, height: 900 },
    deviceScaleFactor: 1,
  });
  page.on('pageerror', (error) => console.error('pageerror', error.message));
  await page.goto(`http://127.0.0.1:${port}/?webgl=1`, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => document.querySelector('canvas')?.dataset.graphicsPhase === 'first-frame-submitted',
    null,
    { timeout: 120000 },
  );
  await page.evaluate(
    ([s, save]) => {
      const c = window.__city;
      c.renderer.renderer.setAnimationLoop(null);
      c.client.controls({ paused: true });
      c.client.init(s, save);
    },
    [expected.seed, saved],
  );
  await page.waitForFunction(
    (count) => window.__city?.world?.buildings.length === count,
    expected.buildings.length,
    { timeout: 60000 },
  );
  const renderer = await page.evaluate(() => {
    const r = window.__city.renderer;
    r.renderer.setAnimationLoop(null);
    r.director.setMode('explore');
    r.setFocus(false);
    r.setQuality('balanced');
    let steps = 0;
    while ((r.objectQueue.size || r.chunks.pendingCount) && steps < 40000) {
      r.animate();
      steps++;
    }
    r.director.update = () => {};
    const gl = r.renderer.backend.gl,
      ext = gl.getExtension('WEBGL_debug_renderer_info');
    return ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : 'unknown';
  });
  console.log(`${label} renderer: ${renderer}`);
  const views = [
    { name: 'street', target: [-24.4, 6, -361.7], position: [-13.05, 1.7, -308.91] },
    { name: 'neighborhood', target: [0, 10, 0], position: [60, 35, -90] },
    { name: 'overview', target: [0, 0, 0], position: [380, 420, -470] },
  ];
  const rows = [];
  for (const style of ['anime', 'miniature']) {
    await page.evaluate(async (s) => {
      await window.__city.renderer.setStyle(s);
    }, style);
    for (const hour of [13, 21]) {
      await page.evaluate((h) => {
        const r = window.__city.renderer;
        r.setPreview(h);
        for (let i = 0; i < 60; i++) {
          r.lastFrame = performance.now() - 100;
          r.animate();
        }
      }, hour);
      for (const view of views) {
        const result = await page.evaluate(async (v) => {
          const r = window.__city.renderer,
            gl = r.renderer.backend.gl;
          r.director.target.set(...v.target);
          r.camera.position.set(...v.position);
          r.camera.lookAt(r.director.target);
          // gl.finish waits for the GPU; a 1x1 readPixels did not on this driver.
          const frame = () => {
            r.renderer.render(r.scene, r.camera);
            gl.finish();
          };
          for (let i = 0; i < 30; i++) frame();
          await new Promise((resolve) => setTimeout(resolve, 200));
          const times = [];
          for (let i = 0; i < 120; i++) {
            const t0 = performance.now();
            frame();
            times.push(performance.now() - t0);
          }
          times.sort((a, b) => a - b);
          r.renderer.info.reset();
          r.renderer.render(r.scene, r.camera);
          return {
            median: times[60],
            p95: times[114],
            calls: r.renderer.info.render.drawCalls,
            triangles: r.renderer.info.render.triangles,
          };
        }, view);
        rows.push({ style, hour, view: view.name, ...result });
        console.log(
          `${label.padEnd(8)} ${style.padEnd(9)} ${String(hour).padStart(2)}h ${view.name.padEnd(12)} median ${result.median.toFixed(2)} ms  p95 ${result.p95.toFixed(2)} ms  draws ${result.calls}  tris ${result.triangles}`,
        );
      }
    }
  }
  console.log(JSON.stringify({ label, renderer, rows }));
} finally {
  await browser.close();
}
