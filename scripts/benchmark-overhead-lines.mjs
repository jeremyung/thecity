// Frame and rebuild cost of utility poles, street wiring and the rail overhead
// line in the mature city, on the real GPU through ANGLE/OpenGL ES (WebGL
// backend; headless Chromium exposes no WebGPU adapter here).
//
// Usage: node scripts/benchmark-overhead-lines.mjs <save.json> [port]
// Requires `npx vite --host 127.0.0.1 --port <port>` running. Each view is
// sampled with the overhead-line layer shown and hidden; every sample renders
// the scene pass (shadows included) and calls gl.finish so the timing includes
// GPU completion. The post pipeline is left out: in headless Chromium its
// render returns without drawing. Use the difference between rows, not the
// absolute.
import { chromium } from '@playwright/test';
import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

const [savePath, port = '5173'] = process.argv.slice(2);
if (!savePath) {
  console.error('Usage: node scripts/benchmark-overhead-lines.mjs <save.json> [port]');
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
  const setup = await page.evaluate(async () => {
    const r = window.__city.renderer;
    r.renderer.setAnimationLoop(null);
    r.director.setMode('explore');
    r.setPreview(15);
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
    // Rebuild cost: the whole layer, from world to finished group, as the park rebuild calls it.
    const { buildOverheadLines } = await import('/web/src/render/overhead-lines.ts');
    const { streetTreePlacements } = await import('/web/src/render/parks.ts');
    const w = r.worldState,
      trees = streetTreePlacements(w),
      times = [];
    let summary = null;
    for (let i = 0; i < 7; i++) {
      const t0 = performance.now();
      const group = buildOverheadLines(w, r.art, trees);
      times.push(performance.now() - t0);
      if (!summary) {
        summary = { meshes: 0, instances: {}, wireTriangles: 0 };
        group.traverse((o) => {
          if (o.isInstancedMesh) {
            summary.meshes++;
            summary.instances[o.name] = o.count;
          } else if (o.isMesh) {
            summary.meshes++;
            summary.wireTriangles += o.geometry.index.count / 3;
          }
        });
      }
      group.traverse((o) => {
        if (o.isMesh) {
          o.geometry.dispose();
          if (o.isInstancedMesh) o.dispose();
        }
      });
    }
    times.sort((a, b) => a - b);
    return {
      renderer: ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : 'unknown',
      rebuildMedianMs: times[3],
      rebuildMaxMs: times[6],
      summary,
    };
  });
  console.log('renderer:', setup.renderer);
  console.log(
    `layer: ${setup.summary.meshes} meshes, wire triangles ${setup.summary.wireTriangles}, rebuild median ${setup.rebuildMedianMs.toFixed(1)} ms, max ${setup.rebuildMaxMs.toFixed(1)} ms`,
  );
  console.log('instances:', JSON.stringify(setup.summary.instances));
  const views = [
    { name: 'street', target: [214, 7, 190], position: [214.5, 1.7, 240] },
    { name: 'neighborhood', target: [217, 5, 224.9], position: [245, 24, 200] },
    { name: 'overview', target: [0, 0, 0], position: [380, 420, -470] },
  ];
  const rows = [];
  for (const style of ['anime', 'miniature']) {
    await page.evaluate(async (s) => {
      const r = window.__city.renderer;
      await r.setStyle(s);
    }, style);
    for (const view of views) {
      for (const shown of [false, true]) {
        const result = await page.evaluate(
          async ([v, shown]) => {
            const r = window.__city.renderer,
              gl = r.renderer.backend.gl;
            r.parks.getObjectByName('overhead-lines').visible = shown;
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
            const calls = r.renderer.info.render.drawCalls,
              triangles = r.renderer.info.render.triangles;
            r.parks.getObjectByName('overhead-lines').visible = true;
            return { median: times[60], p95: times[114], calls, triangles };
          },
          [view, shown],
        );
        rows.push({ style, view: view.name, shown, ...result });
        console.log(
          `${style.padEnd(9)} ${view.name.padEnd(12)} lines ${shown ? 'on ' : 'off'}  median ${result.median.toFixed(2)} ms  p95 ${result.p95.toFixed(2)} ms  draws ${result.calls}  tris ${result.triangles}`,
        );
      }
    }
  }
  console.log(
    JSON.stringify({
      renderer: setup.renderer,
      rebuild: { medianMs: setup.rebuildMedianMs, maxMs: setup.rebuildMaxMs },
      layer: setup.summary,
      rows,
    }),
  );
} finally {
  await browser.close();
}
