// Frame cost of the traced train consists on the architecture review scene, on
// the real GPU through ANGLE/OpenGL ES (WebGL backend; headless Chromium
// exposes no WebGPU adapter here).
//
// Usage: node scripts/benchmark-trains.mjs [port]
// Requires `npx vite --host 127.0.0.1 --port <port>` running. Each sample
// updates the fleet (CPU) and renders, then reads one pixel so the timing
// includes GPU completion. Use the difference between rows, not the absolute.
import { chromium } from '@playwright/test';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const port = process.argv[2] ?? '5173';
const cached = join(homedir(), '.cache/ms-playwright/chromium-1148/chrome-linux/chrome');
const browser = await chromium.launch({ headless: true, executablePath: existsSync(cached) ? cached : undefined, args: ['--headless=new', '--use-gl=angle', '--use-angle=gl-egl', '--ignore-gpu-blocklist'] });
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 }, deviceScaleFactor: 1 });
  page.on('pageerror', error => console.error('pageerror', error.message));
  await page.goto(`http://127.0.0.1:${port}/?review=architecture&webgl=1`, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => window.__review, null, { timeout: 120000 });
  await page.waitForTimeout(5000);
  const gpu = await page.evaluate(() => {
    const gl = window.__review.renderer.backend.gl, ext = gl.getExtension('WEBGL_debug_renderer_info');
    return ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : 'unknown';
  });
  console.log('renderer:', gpu);
  for (const style of ['miniature', 'anime']) {
    for (const count of [0, 1, 4, 8]) {
      const result = await page.evaluate(async ([style, count]) => {
        const r = window.__review, gl = r.renderer.backend.gl, pixel = new Uint8Array(4);
        r.renderer.setAnimationLoop(null);
        r.setStyle(style); r.setTrafficPaused(true);
        const place = () => {
          r.trainFleet.begin();
          for (let i = 0; i < count; i++) {
            const col = i % 4, row = Math.floor(i / 4);
            r.trainFleet.add(5000 + i, -40 + col * 90, -30 - row * 14, Math.PI / 2, 0, 0, 1 / 60);
          }
          r.trainFleet.finish(count);
        };
        r.camera.position.set(-40, 60, 150); r.controls.target.set(-40, 2, -10); r.controls.update();
        const frame = now => { place(); r.renderer.render(r.scene, r.camera); gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel); };
        let now = performance.now();
        for (let i = 0; i < 30; i++) frame(now += 16.7);
        await new Promise(resolve => setTimeout(resolve, 200));
        const times = [];
        for (let i = 0; i < 120; i++) { const t0 = performance.now(); frame(now += 16.7); times.push(performance.now() - t0); }
        times.sort((a, b) => a - b);
        r.renderer.info.reset(); frame(now += 16.7);
        return { style, count, median: times[60], p95: times[114], calls: r.renderer.info.render.drawCalls, triangles: r.renderer.info.render.triangles };
      }, [style, count]);
      console.log(`${result.style.padEnd(9)} consists ${String(result.count).padStart(2)}  median ${result.median.toFixed(2)} ms  p95 ${result.p95.toFixed(2)} ms  draws ${result.calls}  tris ${result.triangles}`);
    }
  }
} finally {
  await browser.close();
}
