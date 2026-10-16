// Capture the train consist on the architecture review station.
//
// Usage: node scripts/capture-train-review.mjs <out-dir> [port]
// Requires `npx vite --host 127.0.0.1 --port <port>` running. Software WebGL:
// visual evidence only, not frame-rate evidence.
import { chromium } from '@playwright/test';
import { existsSync } from 'node:fs';
import { mkdir } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

const [outDir, port = '5173'] = process.argv.slice(2);
if (!outDir) {
  console.error('Usage: node scripts/capture-train-review.mjs <out-dir> [port]');
  process.exit(1);
}
await mkdir(outDir, { recursive: true });
const cached = join(homedir(), '.cache/ms-playwright/chromium-1148/chrome-linux/chrome');
const browser = await chromium.launch({
  headless: true,
  executablePath: existsSync(cached) ? cached : undefined,
  args: process.env.CAPTURE_GPU
    ? ['--headless=new', '--use-gl=angle', '--use-angle=gl-egl', '--ignore-gpu-blocklist']
    : ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
// The review consist sits on the near rail at x=-119, z=-73.
const shots = [
  ['day-miniature-side', 'miniature', false, [-122, 7, -104], [-118, 1.8, -73]],
  ['day-anime-side', 'anime', false, [-122, 7, -104], [-118, 1.8, -73]],
  ['day-miniature-3q', 'miniature', false, [-92, 9, -97], [-113, 1.9, -73]],
  ['night-miniature-side', 'miniature', true, [-122, 7, -104], [-118, 1.8, -73]],
];
try {
  const page = await browser.newPage({
    viewport: { width: 1400, height: 860 },
    deviceScaleFactor: 1,
  });
  page.on('pageerror', (error) => console.error('pageerror', error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') console.error('console', message.text());
  });
  await page.goto(`http://127.0.0.1:${port}/?review=architecture&webgl=1`, {
    waitUntil: 'domcontentloaded',
  });
  await page.waitForFunction(() => window.__review, null, { timeout: 120000 });
  await page.waitForTimeout(6000);
  for (const [name, style, night, position, target] of shots) {
    await page.evaluate(
      ([style, night, position, target]) => {
        const r = window.__review;
        r.setStyle(style);
        r.setNight(night);
        r.setTrafficPaused(true);
        r.camera.position.set(...position);
        r.controls.target.set(...target);
        r.controls.update();
      },
      [style, night, position, target],
    );
    await page.waitForTimeout(2000);
    await page.screenshot({ path: join(outDir, `${name}.png`) });
    console.log('captured', name);
  }
} finally {
  await browser.close();
}
