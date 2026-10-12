// Capture the vehicle lineup on the architecture review street.
//
// Usage: node scripts/capture-vehicle-review.mjs <out-dir> [port]
// Requires `npx vite --host 127.0.0.1 --port <port>` running. Software WebGL:
// visual evidence only, not frame-rate evidence.
import { chromium } from '@playwright/test';
import { existsSync } from 'node:fs';
import { mkdir } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

const [outDir, port = '5173'] = process.argv.slice(2);
if (!outDir) { console.error('Usage: node scripts/capture-vehicle-review.mjs <out-dir> [port]'); process.exit(1); }
await mkdir(outDir, { recursive: true });
const cached = join(homedir(), '.cache/ms-playwright/chromium-1148/chrome-linux/chrome');
const browser = await chromium.launch({ headless: true, executablePath: existsSync(cached) ? cached : undefined, args: process.env.CAPTURE_GPU ? ['--headless=new', '--use-gl=angle', '--use-angle=gl-egl', '--ignore-gpu-blocklist'] : ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const shots = [
  // name, style, night, camera position, target
  ['day-anime-overview', 'anime', false, [-6, 26, 52], [-6, 1, 8]],
  ['day-miniature-overview', 'miniature', false, [-6, 26, 52], [-6, 1, 8]],
  ['day-miniature-close', 'miniature', false, [-30, 7, 26], [-20, 1, 7]],
  ['day-miniature-bus', 'miniature', false, [44, 9, 30], [36, 1.6, 8]],
  ['night-miniature-overview', 'miniature', true, [-6, 22, 46], [-6, 1, 8]],
];
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 860 }, deviceScaleFactor: 1 });
  page.on('pageerror', error => console.error('pageerror', error.message));
  page.on('console', message => { if (message.type() === 'error') console.error('console', message.text()); });
  await page.goto(`http://127.0.0.1:${port}/?review=architecture&webgl=1`, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => window.__review, null, { timeout: 120000 });
  // Let the vehicle atlas decode and a few frames settle.
  await page.waitForTimeout(6000);
  for (const [name, style, night, position, target] of shots) {
    await page.evaluate(([style, night, position, target]) => {
      const r = window.__review;
      r.setStyle(style); r.setNight(night); r.setTrafficPaused(false);
      r.camera.position.set(...position); r.controls.target.set(...target); r.controls.update();
    }, [style, night, position, target]);
    await page.waitForTimeout(2500);
    await page.evaluate(() => window.__review.setTrafficPaused(true));
    await page.waitForTimeout(800);
    await page.screenshot({ path: join(outDir, `${name}.png`) });
    console.log('captured', name);
  }
} finally {
  await browser.close();
}
